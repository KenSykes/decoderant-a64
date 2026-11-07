//! The encoder: operands back to a word, through the same operand descriptors the
//! decoder uses.
//!
//! The word is built from the encoding's fixed bits, the instruction's operands,
//! its should-be bits and its don't-care bits (bits no operand value depends on,
//! such as the `immr` bits above a bitmask immediate's element size); never from
//! the word it was decoded from. Editing an operand clears the don't-care bits.

use crate::bits::{F, Fs, encode_bit_mask, fp_imm8_encode, highest_set_bit};
use crate::desc::{
    Amount, Arr, Idx, Imm, ImmShift, Mem, MemOff, NameKind, Op, Post, Pq, RegSel, Rk, Tab,
    sve_bitmask_esize,
};
use crate::generated::{Code, Register};
use crate::instruction::Instruction;
use crate::operand::{
    ARR_ES, Arrangement, ExtendKind, MemoryMode, OpKind, Operand, PredQual, ShiftKind,
};
use crate::regs::RegisterClass;

/// Why an instruction cannot be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("Code::Invalid cannot be encoded")]
    InvalidCode,
    #[error("{0:?} has no operand support yet")]
    NoOperandSupport(Code),
    #[error("expected {expected} operands, got {got}")]
    OperandCount { expected: usize, got: usize },
    #[error("operand {0} has the wrong kind")]
    WrongKind(usize),
    #[error("operand {0}: register not encodable here")]
    Register(usize),
    #[error("operand {0}: arrangement or element size not encodable here")]
    Arrangement(usize),
    #[error("operand {0}: value out of range")]
    Range(usize),
    #[error("operand {0}: value not representable (alignment, bitmask or FP immediate)")]
    Unrepresentable(usize),
    #[error("operand {0} conflicts with an earlier operand or a fixed bit")]
    Conflict(usize),
}

type R = Result<(), EncodeError>;

/// A word being built: the bits set so far and which bits they are.
struct W {
    word: u32,
    set: u32,
    op: usize,
}

impl W {
    fn put(&mut self, f: Fs, v: u32) -> R {
        let width = f.width();
        if width < 32 && v >> width != 0 {
            return Err(EncodeError::Range(self.op));
        }
        let mut tmp = 0u32;
        f.set(&mut tmp, v);
        self.put_bits(f.mask(), tmp)
    }

    /// [`put`](Self::put) for an operand's 64-bit value: a value that does not fit
    /// is out of range, not truncated.
    fn put_u64(&mut self, f: Fs, v: u64) -> R {
        let v = u32::try_from(v).map_err(|_| EncodeError::Range(self.op))?;
        self.put(f, v)
    }

    fn put_bits(&mut self, mask: u32, value: u32) -> R {
        let value = value & mask;
        let both = self.set & mask;
        if self.word & both != value & both {
            return Err(EncodeError::Conflict(self.op));
        }
        self.word = (self.word & !mask) | value;
        self.set |= mask;
        Ok(())
    }

    /// Sets a table entry's bits (its fixed bits only; `x` bits belong to another
    /// operand).
    fn tab(&mut self, t: &Tab, code: u8) -> R {
        let (v, m) = t.bits(code).ok_or(EncodeError::Arrangement(self.op))?;
        // Spread the table's concatenated value back over its fields.
        let mut tv = 0u32;
        t.f.set(&mut tv, v);
        let mut tm = 0u32;
        t.f.set(&mut tm, m);
        self.put_bits(tm, tv)
    }
}

/// The register kind `r` is a register of.
fn rk_of(r: Register) -> Option<Rk> {
    Some(match r.class() {
        RegisterClass::Gp64 if r == Register::SP => Rk::XSp,
        RegisterClass::Gp64 => Rk::X,
        RegisterClass::Gp32 if r == Register::WSP => Rk::WSp,
        RegisterClass::Gp32 => Rk::W,
        RegisterClass::B => Rk::B,
        RegisterClass::H => Rk::H,
        RegisterClass::S => Rk::S,
        RegisterClass::D => Rk::D,
        RegisterClass::Q => Rk::Q,
        RegisterClass::V => Rk::V,
        RegisterClass::Z => Rk::Z,
        RegisterClass::P => Rk::P,
        RegisterClass::PN => Rk::PN,
        _ => return None,
    })
}

/// Whether `r` is a register of kind `k`: `XSp` accepts `X0`–`X30` and `SP`, `X`
/// accepts `X0`–`X30` and `XZR`.
fn fits(r: Register, k: Rk) -> bool {
    k.reg(r.number()) == r && r != Register::None
}

fn reg(w: &mut W, f: Fs, sel: RegSel, add: u8, r: Register) -> R {
    let op = w.op;
    let n = r.number();
    match sel {
        RegSel::Fixed(k) => {
            if !fits(r, k) {
                return Err(EncodeError::Register(op));
            }
        }
        // The table's entry for the register's kind (X/W tables accept ZR; XSp/WSp
        // tables do not occur).
        RegSel::Tab(t) => w.tab(t, rk_of(r).ok_or(EncodeError::Register(op))? as u8)?,
    }
    w.put(f, (n + 32 - u32::from(add)) % 32)
}

fn arr(w: &mut W, a: Arr, have: Arrangement) -> R {
    match a {
        Arr::Fixed(x) if x == have => Ok(()),
        Arr::Fixed(_) => Err(EncodeError::Arrangement(w.op)),
        Arr::Tab(t) => w.tab(t, have as u8),
    }
}

fn index(w: &mut W, idx: Idx, es: Arrangement, i: u8) -> R {
    let i = u32::from(i);
    match idx {
        Idx::Bits(f, shr) => w.put(f, i << shr),
        Idx::ByEs { b, h, s, d, zs, dz } => {
            let f = match es.element_bytes() {
                1 => b,
                2 => h,
                4 => s,
                8 => d,
                _ => return Err(EncodeError::Arrangement(w.op)),
            };
            if f.is_none() {
                return Err(EncodeError::Arrangement(w.op));
            }
            w.put(f, i)?;
            if es.element_bytes() == 4 && !zs.is_none() {
                w.put(zs, 0)?;
            }
            if es.element_bytes() == 8 && !dz.is_none() {
                w.put(dz, 0)?;
            }
            Ok(())
        }
        Idx::Imm5(f) => {
            // Up to Q elements (`DUP <Zd>.Q, <Zn>.Q[<imm>]`: `tsz` = 10000).
            let low = es.element_bytes().trailing_zeros();
            if low > 4 || es.element_bytes() == 0 {
                return Err(EncodeError::Arrangement(w.op));
            }
            w.put(f, (i << (low + 1)) | (1 << low))
        }
        Idx::Imm4(imm4, _imm5) => {
            // B, H, S or D elements, as decoding allows.
            let low = es.element_bytes().trailing_zeros();
            if low >= 4 {
                return Err(EncodeError::Arrangement(w.op));
            }
            w.put(imm4, i << low)
        }
        Idx::Const(c) => {
            if u32::from(c) != i {
                return Err(EncodeError::Range(w.op));
            }
            Ok(())
        }
        Idx::Opt(f) => w.put(
            f,
            if i == u32::from(Operand::NO_LANE) {
                0
            } else {
                i
            },
        ),
        Idx::Mul(f, k) => {
            let k = u32::from(k);
            if i % k != 0 {
                return Err(EncodeError::Unrepresentable(w.op));
            }
            w.put(f, i / k)
        }
    }
}

/// The esize of an `immh:immb` shift from the `immh` bits already set (by the
/// arrangement or the encoding's fixed bits).
fn immh_esize(word: u32, f: Fs) -> Option<u32> {
    let v = f.get(word);
    Some(8 << highest_set_bit(v >> 3)?)
}

fn imm(w: &mut W, i: Imm, v: u64) -> R {
    let op = w.op;
    match i {
        Imm::Const(c) => {
            if v != c as u64 {
                return Err(EncodeError::Range(op));
            }
            Ok(())
        }
        Imm::U(f, shl) => {
            if v & ((1u64 << shl) - 1) != 0 {
                return Err(EncodeError::Unrepresentable(op));
            }
            let x = v >> shl;
            if x >> f.width() != 0 {
                return Err(EncodeError::Range(op));
            }
            w.put(f, x as u32)
        }
        Imm::S(f, shl) => {
            let s = v as i64;
            if s & ((1i64 << shl) - 1) != 0 {
                return Err(EncodeError::Unrepresentable(op));
            }
            let x = s >> shl;
            let width = f.width();
            let (lo, hi) = (-(1i64 << (width - 1)), (1i64 << (width - 1)) - 1);
            if x < lo || x > hi {
                return Err(EncodeError::Range(op));
            }
            w.put(f, (x as u32) & ((1u64 << width) - 1) as u32)
        }
        Imm::SMul(f, k) => {
            let s = v as i64;
            let k = i64::from(k);
            if s % k != 0 {
                return Err(EncodeError::Unrepresentable(op));
            }
            let x = s / k;
            let width = f.width();
            if x < -(1i64 << (width - 1)) || x >= 1i64 << (width - 1) {
                return Err(EncodeError::Range(op));
            }
            w.put(f, (x as u32) & ((1u64 << width) - 1) as u32)
        }
        Imm::Add(f, k) => {
            let x = v as i64 - i64::from(k);
            if x < 0 || x >> f.width() != 0 {
                return Err(EncodeError::Range(op));
            }
            w.put(f, x as u32)
        }
        Imm::Rsub(k, f) => {
            let x = i64::from(k) - v as i64;
            if x < 0 || x >> f.width() != 0 {
                return Err(EncodeError::Range(op));
            }
            w.put(f, x as u32)
        }
        Imm::NegMod(f, m) => {
            let m = u64::from(m);
            if v >= m {
                return Err(EncodeError::Range(op));
            }
            w.put(f, ((m - v) % m) as u32)
        }
        Imm::Tab(t) => {
            let code = t
                .nums
                .iter()
                .position(|&n| n as u64 == v)
                .ok_or(EncodeError::Range(op))?;
            w.tab(t, code as u8)
        }
        Imm::ShrEs(f) => {
            let esize = immh_esize(w.word, f).ok_or(EncodeError::Arrangement(op))?;
            if v == 0 || v > u64::from(esize) {
                return Err(EncodeError::Range(op));
            }
            w.put(f, 2 * esize - v as u32)
        }
        Imm::ShlEs(f) => {
            let esize = immh_esize(w.word, f).ok_or(EncodeError::Arrangement(op))?;
            if v >= u64::from(esize) {
                return Err(EncodeError::Range(op));
            }
            w.put(f, esize + v as u32)
        }
        Imm::Bitmask {
            n,
            immr,
            imms,
            size,
        } => bitmask(w, n, immr, imms, u32::from(size), v, None),
        Imm::Fp8(f) => {
            let x = fp_imm8_encode(f64::from_bits(v)).ok_or(EncodeError::Unrepresentable(op))?;
            w.put(f, x)
        }
        Imm::ByteMask(f) => {
            let mut b = 0u32;
            for k in 0..8 {
                match v >> (8 * k) & 0xff {
                    0 => {}
                    0xff => b |= 1 << k,
                    _ => return Err(EncodeError::Unrepresentable(op)),
                }
            }
            w.put(f, b)
        }
        Imm::NegU(f, shl) => imm(w, Imm::U(f, shl), v.wrapping_neg()),
        Imm::Width(imms, immr) => {
            let r = u64::from(immr.get(w.word));
            let s = (v + r).checked_sub(1).ok_or(EncodeError::Range(op))?;
            if s >> imms.width() != 0 {
                return Err(EncodeError::Range(op));
            }
            w.put(imms, s as u32)
        }
        Imm::LslShift { immr, imms, size } => {
            let m = u64::from(size);
            if v >= m {
                return Err(EncodeError::Range(op));
            }
            w.put(immr, ((m - v) % m) as u32)?;
            w.put(imms, (m - 1 - v) as u32)
        }
        Imm::WideMov {
            imm16,
            hw,
            inv,
            size,
        } => {
            // A 32-bit value with higher bits set is not silently truncated.
            let ones = if size == 32 { 0xffff_ffff } else { u64::MAX };
            if v & !ones != 0 {
                return Err(EncodeError::Range(op));
            }
            let v = if inv { !v & ones } else { v };
            for h in 0..u64::from(size / 16) {
                if v & !(0xffff << (16 * h)) == 0 {
                    w.put(hw, h as u32)?;
                    return w.put(imm16, (v >> (16 * h)) as u32);
                }
            }
            Err(EncodeError::Unrepresentable(op))
        }
        Imm::SveBitmask { n, immr, imms } => sve_bitmask(w, n, immr, imms, v, None),
    }
}

/// An SVE logical immediate: `v` is one element. The element size is the one whose
/// encoding agrees with the bits already set (the arrangement sets `N` and the top
/// of `imms`); the decoded fields are kept when they still give the value.
fn sve_bitmask(w: &mut W, n: Fs, immr: Fs, imms: Fs, v: u64, raw: Option<u16>) -> R {
    let op = w.op;
    let value_of = |rn: u32, rr: u32, rs: u32| {
        let full = crate::bits::decode_bit_masks(rn, rs, rr, true, 64)?.0;
        let es = sve_bitmask_esize(rn, rs)?;
        Some(if es == 64 {
            full
        } else {
            full & ((1 << es) - 1)
        })
    };
    let mut tries: [Option<(u32, u32, u32)>; 5] = [None; 5];
    if let Some(raw) = raw {
        let f = (
            u32::from(raw >> 12 & 1),
            u32::from(raw >> 6 & 63),
            u32::from(raw & 63),
        );
        if value_of(f.0, f.1, f.2) == Some(v) {
            tries[0] = Some(f);
        }
    }
    for (k, es) in [64u32, 32, 16, 8].into_iter().enumerate() {
        if es < 64 && v >> es != 0 {
            continue;
        }
        let mut rep = v;
        let mut s = es;
        while s < 64 {
            rep |= rep << s;
            s *= 2;
        }
        if let Some(f) = encode_bit_mask(rep, 64)
            && value_of(f.0, f.1, f.2) == Some(v)
        {
            tries[k + 1] = Some((f.0, f.1, f.2));
        }
    }
    for (rn, rr, rs) in tries.into_iter().flatten() {
        let mut t = W { ..*w };
        if t.put(n, rn).is_ok() && t.put(immr, rr).is_ok() && t.put(imms, rs).is_ok() {
            *w = t;
            return Ok(());
        }
    }
    Err(EncodeError::Unrepresentable(op))
}

fn bitmask(w: &mut W, n: Fs, immr: Fs, imms: Fs, size: u32, v: u64, raw: Option<u16>) -> R {
    let op = w.op;
    // A 32-bit value with higher bits set is not silently truncated.
    if size < 64 && v >> size != 0 {
        return Err(EncodeError::Unrepresentable(op));
    }
    // Prefer the decoded fields when they still give the value: their don't-care
    // `immr` bits are then kept.
    if let Some(raw) = raw {
        let (rn, rr, rs) = (
            u32::from(raw >> 12 & 1),
            u32::from(raw >> 6 & 63),
            u32::from(raw & 63),
        );
        if crate::bits::decode_bit_masks(rn, rs, rr, true, size).map(|x| x.0) == Some(v) {
            if !n.is_none() {
                w.put(n, rn)?;
            }
            w.put(immr, rr)?;
            return w.put(imms, rs);
        }
    }
    let (en, er, es) = encode_bit_mask(v, size).ok_or(EncodeError::Unrepresentable(op))?;
    if n.is_none() {
        if en != 0 {
            return Err(EncodeError::Unrepresentable(op));
        }
    } else {
        w.put(n, en)?;
    }
    w.put(immr, er)?;
    w.put(imms, es)
}

/// The index register (`base`..`base + 3`) and the offset of a ZA slice or array:
/// the offset is a multiple of the number of slices or vectors.
fn za_index(w: &mut W, f: Fs, base: u8, o: &Operand, count: u8, offs: Fs) -> R {
    let i = w.op;
    let n = o.index_reg.number();
    let base = u32::from(base);
    if o.index_reg.class() != RegisterClass::Gp32 || !(base..base + 4).contains(&n) {
        return Err(EncodeError::Register(i));
    }
    if o.count.max(1) != count.max(1) {
        return Err(EncodeError::WrongKind(i));
    }
    let c = u64::from(count.max(1));
    if o.imm % c != 0 {
        return Err(EncodeError::Unrepresentable(i));
    }
    w.put(f, n - base)?;
    if offs.is_none() {
        // A constant offset (`<offs1>:<offs4>` of `0:3`).
        return if o.imm == 0 {
            Ok(())
        } else {
            Err(EncodeError::Range(i))
        };
    }
    let v = o.imm / c;
    if v >> offs.width() != 0 {
        return Err(EncodeError::Range(i));
    }
    w.put(offs, v as u32)
}

/// A predicate qualifier: what the encoding fixes, or a table entry.
fn pred(w: &mut W, pq: Pq, have: PredQual) -> R {
    let ok = match pq {
        Pq::None => have == PredQual::None,
        Pq::Zeroing => have == PredQual::Zeroing,
        Pq::Merging => have == PredQual::Merging,
        Pq::Tab(t) => {
            return match have {
                PredQual::Zeroing => w.tab(t, 0),
                PredQual::Merging => w.tab(t, 1),
                PredQual::None => Err(EncodeError::WrongKind(w.op)),
            };
        }
    };
    if ok {
        Ok(())
    } else {
        Err(EncodeError::WrongKind(w.op))
    }
}

fn want(o: &Operand, k: OpKind, i: usize) -> R {
    if o.kind == k {
        Ok(())
    } else {
        Err(EncodeError::WrongKind(i))
    }
}

fn mem(w: &mut W, m: &Mem, o: &Operand) -> R {
    let i = w.op;
    want(o, OpKind::Memory, i)?;
    // `[x0]` is `[x0, #0, MUL VL]`.
    let vl_ok = o.mul_vl == m.mul_vl || (m.mul_vl && o.imm == 0);
    if o.mode != m.mode || o.writeback != m.writeback || !vl_ok {
        return Err(EncodeError::WrongKind(i));
    }
    if !fits(o.reg, m.base_k) {
        return Err(EncodeError::Register(i));
    }
    w.put(m.base, o.reg.number())?;
    arr(w, m.arr, o.arr)?;
    match m.off {
        MemOff::None => {
            if m.mode != MemoryMode::PostIndex && (o.imm != 0 || o.index_reg != Register::None) {
                return Err(EncodeError::Range(i));
            }
        }
        MemOff::Imm(x) => {
            // An index the encoding has no place for is not silently dropped.
            if o.index_reg != Register::None {
                return Err(EncodeError::WrongKind(i));
            }
            imm(w, x, o.imm)?
        }
        MemOff::OptReg(f, a) => {
            if o.imm != 0 {
                return Err(EncodeError::WrongKind(i));
            }
            if o.index_reg == Register::None || o.index_reg == Register::XZR {
                if o.has_amount {
                    return Err(EncodeError::Range(i));
                }
                w.put(f, 31)?;
            } else {
                if !fits(o.index_reg, Rk::X)
                    || o.extend != ExtendKind::Lsl
                    || o.has_amount != (a != 0)
                    || (a != 0 && o.amount != a)
                {
                    return Err(EncodeError::Register(i));
                }
                w.put(f, o.index_reg.number())?;
            }
        }
        MemOff::Reg {
            m: f,
            k,
            ext,
            amount,
        } => {
            if o.imm != 0 && m.mode != MemoryMode::PostIndex {
                return Err(EncodeError::WrongKind(i));
            }
            reg(w, f, k, 0, o.index_reg)?;
            match ext {
                Some(t) => {
                    // A table that spells `011` `LSL` has no `UXTX` entry.
                    let code = match o.extend as u8 {
                        c if o.extend == ExtendKind::Uxtx && t.bits(c).is_none() => {
                            ExtendKind::Lsl as u8
                        }
                        c => c,
                    };
                    w.tab(t, code)?;
                }
                None => {
                    if o.extend != ExtendKind::Lsl {
                        return Err(EncodeError::WrongKind(i));
                    }
                }
            }
            match amount {
                Amount::None => {
                    if o.has_amount {
                        return Err(EncodeError::Range(i));
                    }
                }
                Amount::Fixed(a) => {
                    if !o.has_amount || o.amount != a {
                        return Err(EncodeError::Range(i));
                    }
                }
                Amount::OnBit(f, a) => {
                    if o.has_amount && o.amount != a {
                        return Err(EncodeError::Range(i));
                    }
                    w.put(f, u32::from(o.has_amount))?;
                }
                Amount::Tab(t) => {
                    let e = t
                        .e
                        .iter()
                        .find(|e| {
                            (e.0 != 0) == o.has_amount
                                && t.nums.get(e.2 as usize).copied() == Some(i64::from(o.amount))
                        })
                        .ok_or(EncodeError::Range(i))?;
                    w.tab(t, e.2)?;
                }
            }
        }
    }
    match m.post {
        Post::None => {}
        Post::Imm(x) => imm(w, x, o.imm)?,
        Post::Reg(f) => {
            if !fits(o.index_reg, Rk::X) || o.index_reg == Register::XZR {
                return Err(EncodeError::Register(i));
            }
            w.put(f, o.index_reg.number())?;
        }
    }
    Ok(())
}

/// Whether an operand's encoding reads bits other operands set (it goes in the
/// second pass).
fn deferred(op: &Op) -> bool {
    match op {
        Op::Imm(Imm::ShrEs(_) | Imm::ShlEs(_) | Imm::Width(..), _) => true,
        Op::Imm(_, ImmShift::Lsl(Imm::ShrEs(_))) => true,
        Op::Opt(inner, ..) => deferred(inner),
        _ => false,
    }
}

fn encode_one(w: &mut W, op: &Op, o: &Operand, pc: u64) -> R {
    let i = w.op;
    match *op {
        Op::Reg(f, sel, add) => {
            want(o, OpKind::Register, i)?;
            if o.arr != Arrangement::None || o.lane != Operand::NO_LANE {
                return Err(EncodeError::WrongKind(i));
            }
            reg(w, f, sel, add, o.reg)
        }
        Op::RegFixed(r) => {
            want(o, OpKind::Register, i)?;
            if o.reg != r {
                return Err(EncodeError::Register(i));
            }
            Ok(())
        }
        Op::RegPair(f, k) => {
            want(o, OpKind::Register, i)?;
            let n = f.get(w.word);
            let expect = k.reg(if n == 31 { 31 } else { n + 1 });
            if o.reg != expect {
                return Err(EncodeError::Register(i));
            }
            Ok(())
        }
        Op::Vec(f, a, add) => {
            want(o, OpKind::Register, i)?;
            if o.reg.class() != RegisterClass::V || o.lane != Operand::NO_LANE {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            w.put(f, (o.reg.number() + 32 - u32::from(add)) % 32)
        }
        Op::Elem(f, a, idx, mask_h) => {
            want(o, OpKind::Register, i)?;
            if o.reg.class() != RegisterClass::V || o.lane == Operand::NO_LANE {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            let n = o.reg.number();
            if mask_h && o.arr.element_bytes() == 2 {
                if n > 15 {
                    return Err(EncodeError::Register(i));
                }
                // The top bit of the register field is an index bit here: set the
                // register's four bits only.
                w.put(Fs::one(F::new(f.last().lsb, 4)), n)?;
            } else {
                w.put(f, n)?;
            }
            index(w, idx, o.arr, o.lane)
        }
        Op::List(f, n, a, idx) => {
            want(o, OpKind::RegisterList, i)?;
            if o.count != n {
                return Err(EncodeError::WrongKind(i));
            }
            if o.reg.class() != RegisterClass::V {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            w.put(f, o.reg.number())?;
            match idx {
                Some(x) => index(w, x, o.arr, o.lane),
                None => {
                    if o.lane != Operand::NO_LANE {
                        return Err(EncodeError::WrongKind(i));
                    }
                    Ok(())
                }
            }
        }
        Op::Imm(x, s) => {
            want(o, OpKind::Immediate, i)?;
            match s {
                ImmShift::None => {
                    if o.has_amount && o.amount != 0 {
                        return Err(EncodeError::Range(i));
                    }
                }
                ImmShift::Lsl(a) => {
                    if o.has_amount && o.shift != ShiftKind::Lsl {
                        return Err(EncodeError::WrongKind(i));
                    }
                    imm(w, a, u64::from(o.amount))?;
                }
                ImmShift::Msl(a) => {
                    if o.shift != ShiftKind::Msl {
                        return Err(EncodeError::WrongKind(i));
                    }
                    imm(w, a, u64::from(o.amount))?;
                }
            }
            imm(w, x, o.imm)
        }
        Op::BitImm(x) => {
            want(o, OpKind::BitmaskImmediate, i)?;
            match x {
                Imm::Bitmask {
                    n,
                    immr,
                    imms,
                    size,
                } => bitmask(w, n, immr, imms, u32::from(size), o.imm, o.raw()),
                Imm::SveBitmask { n, immr, imms } => sve_bitmask(w, n, immr, imms, o.imm, o.raw()),
                other => imm(w, other, o.imm),
            }
        }
        Op::FpImm(x) => {
            want(o, OpKind::FpImmediate, i)?;
            imm(w, x, o.imm)
        }
        Op::Shifted(f, k, t, amount) => {
            want(o, OpKind::ShiftedRegister, i)?;
            reg(w, f, k, 0, o.reg)?;
            w.tab(t, o.shift as u8)?;
            w.put(amount, u32::from(o.amount))
        }
        Op::Extended(f, k, option, amount) => {
            want(o, OpKind::ExtendedRegister, i)?;
            reg(w, f, k, 0, o.reg)?;
            let wide = o.reg.class() == RegisterClass::Gp64;
            let ext = match o.extend {
                ExtendKind::Lsl => {
                    if wide {
                        ExtendKind::Uxtx
                    } else {
                        ExtendKind::Uxtw
                    }
                }
                e => e,
            };
            w.put(option, u32::from(ext as u8))?;
            if o.amount > 4 {
                return Err(EncodeError::Range(i));
            }
            w.put(amount, u32::from(o.amount))
        }
        Op::Mem(ref m) => mem(w, m, o),
        Op::Label(x, page) => {
            want(o, OpKind::Label, i)?;
            let base = if page { pc & !0xfff } else { pc };
            imm(w, x, o.imm.wrapping_sub(base))
        }
        Op::Cond(f, _suffix, invert) => {
            want(o, OpKind::Condition, i)?;
            if o.imm > 15 {
                return Err(EncodeError::Range(i));
            }
            w.put(f, o.imm as u32 ^ u32::from(invert))
        }
        Op::SysReg(f) => {
            want(o, OpKind::SystemRegister, i)?;
            if o.imm >> 15 != 1 {
                return Err(EncodeError::Range(i));
            }
            w.put(f, (o.imm & 0x7fff) as u32)
        }
        Op::SysOperation(f) => {
            want(o, OpKind::SystemOp, i)?;
            if o.imm >> 14 != 1 {
                return Err(EncodeError::Range(i));
            }
            w.put(f, (o.imm & 0x3fff) as u32)
        }
        Op::PState(f) => {
            want(o, OpKind::PState, i)?;
            w.put_u64(f, o.imm)
        }
        Op::Cr(f) => {
            want(o, OpKind::ControlRegister, i)?;
            w.put_u64(f, o.imm)
        }
        Op::Name(t, kind) => {
            let pattern_imm = kind == NameKind::SvePattern && o.kind == OpKind::Immediate;
            if !matches!(o.kind, OpKind::Barrier | OpKind::Prefetch | OpKind::Name) && !pattern_imm
            {
                return Err(EncodeError::WrongKind(i));
            }
            w.put_u64(t.f, o.imm)
        }
        Op::Lit(_) => want(o, OpKind::Name, i),
        Op::RegWb(f, k) => {
            want(o, OpKind::Register, i)?;
            if !fits(o.reg, k) {
                return Err(EncodeError::Register(i));
            }
            w.put(f, o.reg.number())
        }
        Op::Opt(inner, _, _) => encode_one(w, inner, o, pc),
        Op::Suffix(_) => Ok(()),
        Op::Zr(f, k, a, add, pq) => {
            want(o, OpKind::Register, i)?;
            if o.lane != Operand::NO_LANE || !fits(o.reg, k) {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            pred(w, pq, o.pred)?;
            w.put(f, (o.reg.number() + 32 - u32::from(add)) % 32)
        }
        Op::MulImm(x) => {
            want(o, OpKind::Immediate, i)?;
            imm(w, x, o.imm)
        }
        Op::ZrMap(f, rn, a, idx) => {
            want(o, OpKind::Register, i)?;
            if o.reg.class() != RegisterClass::Z || (o.lane != Operand::NO_LANE) != idx.is_some() {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            let v = rn.field(o.reg.number()).ok_or(EncodeError::Register(i))?;
            w.put(f, v)?;
            match idx {
                Some(x) => index(w, x, o.arr, o.lane),
                None => Ok(()),
            }
        }
        Op::ZaTile(f, a) => {
            want(o, OpKind::ZaTile, i)?;
            let (es, n) = o.reg.za_tile_info().ok_or(EncodeError::Register(i))?;
            arr(w, a, ARR_ES[es as usize])?;
            w.put(f, n)
        }
        Op::ZaSlice(t, a, hv, ws, base, offs, count) => {
            want(o, OpKind::ZaSlice, i)?;
            let (es, n) = o.reg.za_tile_info().ok_or(EncodeError::Register(i))?;
            arr(w, a, ARR_ES[es as usize])?;
            w.put(t, n)?;
            w.put(hv, u32::from(o.vertical))?;
            za_index(w, ws, base, o, count, offs)
        }
        Op::ZaArray(a, wv, base, offs, count, vg) => {
            want(o, OpKind::ZaArray, i)?;
            // `{, VGx2}` may be left out.
            if o.reg != Register::ZA || (o.vg != vg && o.vg != 0) {
                return Err(EncodeError::WrongKind(i));
            }
            arr(w, a, o.arr)?;
            za_index(w, wv, base, o, count, offs)
        }
        Op::ZaMask(f) => {
            want(o, OpKind::ZaMask, i)?;
            w.put_u64(f, o.imm)
        }
        Op::Braced(inner) => encode_one(w, inner, o, pc),
        Op::Zt0Vl(f) => {
            want(o, OpKind::Register, i)?;
            if o.reg != Register::ZT0 {
                return Err(EncodeError::Register(i));
            }
            w.put(
                f,
                if o.lane == Operand::NO_LANE {
                    0
                } else {
                    u32::from(o.lane)
                },
            )
        }
        Op::ZSel(f, k, a, v, idx) => {
            want(o, OpKind::Register, i)?;
            let wn = o.index_reg.number();
            if !fits(o.reg, k)
                || o.lane == Operand::NO_LANE
                || o.index_reg.class() != RegisterClass::Gp32
                || !(12..=15).contains(&wn)
            {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            w.put(f, o.reg.number())?;
            w.put(v, wn - 12)?;
            index(w, idx, o.arr, o.lane)
        }
        Op::ZList(f, k, n, stride, mul, add, a, idx) => {
            want(o, OpKind::RegisterList, i)?;
            if o.count != n
                || o.list_stride() != stride.max(1)
                || (o.lane != Operand::NO_LANE) != idx.is_some()
            {
                return Err(EncodeError::WrongKind(i));
            }
            if !fits(o.reg, k) {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            if let Some(x) = idx {
                index(w, x, o.arr, o.lane)?;
            }
            let first = o
                .reg
                .number()
                .checked_sub(u32::from(add))
                .ok_or(EncodeError::Register(i))?;
            let v = if stride <= 1 {
                if first % u32::from(mul) != 0 {
                    return Err(EncodeError::Register(i));
                }
                first / u32::from(mul)
            } else {
                let s = u32::from(stride).trailing_zeros();
                if (first & 15) >> s != 0 {
                    return Err(EncodeError::Register(i));
                }
                (first >> 4) << s | (first & ((1 << s) - 1))
            };
            w.put(f, v)
        }
        Op::ZElem(f, k, a, idx, add) => {
            want(o, OpKind::Register, i)?;
            let no_lane = o.lane == Operand::NO_LANE && !matches!(idx, Idx::Opt(_));
            if no_lane || !fits(o.reg, k) || o.pred != PredQual::None {
                return Err(EncodeError::Register(i));
            }
            arr(w, a, o.arr)?;
            w.put(f, (o.reg.number() + 32 - u32::from(add)) % 32)?;
            index(w, idx, o.arr, o.lane)
        }
    }
}

/// Encodes operands into the variable bits of an encoding's word (no should-be or
/// don't-care bits).
pub(crate) fn encode_ops(
    ops: &[Op],
    operands: &[Operand],
    suffix: u8,
    pc: u64,
    base: u32,
    mask: u32,
) -> Result<u32, EncodeError> {
    let n = crate::desc::operand_count(ops);
    if operands.len() != n {
        return Err(EncodeError::OperandCount {
            expected: n,
            got: operands.len(),
        });
    }
    let mut w = W {
        word: base,
        set: mask,
        op: 0,
    };
    for pass in 0..2 {
        let mut i = 0;
        for op in ops {
            if matches!(op, Op::Suffix(_)) {
                continue;
            }
            if deferred(op) == (pass == 1) {
                w.op = i;
                encode_one(&mut w, op, &operands[i], pc)?;
            }
            i += 1;
        }
    }
    // The mnemonic suffix: its bits, unless the operands already set them (Q in
    // `SADDL2` comes from the arrangements); then they must agree.
    for op in ops {
        if let Op::Suffix(t) = op {
            if w.set & t.f.mask() == t.f.mask() {
                t.find(w.word).ok_or(EncodeError::Conflict(0))?;
            } else {
                w.op = 0;
                w.tab(t, suffix)?;
            }
        }
    }
    Ok(w.word)
}

/// Encodes an instruction.
pub fn encode(insn: &Instruction, pc: u64) -> Result<u32, EncodeError> {
    if insn.code == Code::Invalid {
        return Err(EncodeError::InvalidCode);
    }
    let d = insn.op_code().data().ok_or(EncodeError::InvalidCode)?;
    let ops = d.ops.ok_or(EncodeError::NoOperandSupport(insn.code))?;
    let sb = insn.sb & d.sb_mask;
    let (pm, pv) = crate::undefined::pinned(d);
    let extra = insn.extra & !(d.mask | d.sb_mask | pm);
    let w = encode_ops(
        ops,
        insn.operands(),
        insn.suffix,
        pc,
        d.value | sb | pv,
        d.mask | d.sb_mask | pm,
    )? | extra;
    // An operand value the decode pseudocode rejects (`imm6` >= 32 in a 32-bit
    // shift, ...).
    if crate::undefined::undefined(d, w) {
        return Err(EncodeError::Range(0));
    }
    Ok(w)
}

/// Encodes instructions, in the manner of `iced_x86::Encoder`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Encoder;

impl Encoder {
    pub fn new() -> Self {
        Encoder
    }

    /// Encodes `insn` at `pc` (for PC-relative operands).
    pub fn encode(&self, insn: &Instruction, pc: u64) -> Result<u32, EncodeError> {
        encode(insn, pc)
    }
}

impl Instruction {
    /// Builds an instruction of encoding `code` from operands (and a mnemonic suffix
    /// index, 0 for none), at `pc`. Should-be bits take their stated values. The
    /// result is checked by encoding it.
    pub fn with(code: Code, operands: &[Operand], pc: u64) -> Result<Instruction, EncodeError> {
        Self::with_suffix(code, 0, operands, pc)
    }

    /// [`Instruction::with`] with a mnemonic suffix (the index into the encoding's
    /// suffix table: `SADDL`/`SADDL2`, `BFMLALB`/`BFMLALT`).
    pub fn with_suffix(
        code: Code,
        suffix: u8,
        operands: &[Operand],
        pc: u64,
    ) -> Result<Instruction, EncodeError> {
        let d = code.op_code().data().ok_or(EncodeError::InvalidCode)?;
        if operands.len() > crate::MAX_OPERANDS {
            return Err(EncodeError::OperandCount {
                expected: crate::MAX_OPERANDS,
                got: operands.len(),
            });
        }
        let mut insn = Instruction {
            code,
            pc,
            sb: d.sb_value & d.sb_mask,
            suffix,
            has_operands: d.ops.is_some(),
            op_count: operands.len() as u8,
            ..Instruction::default()
        };
        insn.ops[..operands.len()].copy_from_slice(operands);
        insn.refresh()?;
        Ok(insn)
    }

    /// Replaces operand `i` and re-encodes. The don't-care bits are dropped. On
    /// error the instruction is unchanged.
    pub fn set_op(&mut self, i: usize, op: Operand) -> Result<(), EncodeError> {
        if i >= self.op_count as usize {
            return Err(EncodeError::OperandCount {
                expected: self.op_count as usize,
                got: i + 1,
            });
        }
        let mut next = Instruction { extra: 0, ..*self };
        next.ops[i] = op;
        next.refresh()?;
        *self = next;
        Ok(())
    }

    /// Sets the should-be bits (within the encoding's should-be mask) and re-encodes.
    pub fn set_should_be_bits(&mut self, bits: u32) -> Result<(), EncodeError> {
        let d = self.op_code().data().ok_or(EncodeError::InvalidCode)?;
        let mut next = Instruction {
            sb: bits & d.sb_mask,
            ..*self
        };
        next.refresh()?;
        *self = next;
        Ok(())
    }

    /// Re-encodes and brings the decode-derived state (word, preferred alias)
    /// up to date.
    fn refresh(&mut self) -> Result<(), EncodeError> {
        let w = encode(self, self.pc)?;
        let fresh = crate::decoder::decode_word(
            w,
            self.pc,
            &crate::DecoderOptions::with_features(self.features),
        );
        if fresh.code != self.code {
            return Err(EncodeError::Conflict(0));
        }
        self.word = w;
        self.alias = fresh.alias;
        // Decoding normalises operands (label targets, raw bitmask fields).
        self.ops = fresh.ops;
        self.op_count = fresh.op_count;
        Ok(())
    }

    /// A copy without the kept don't-care bits: what an instruction built from its
    /// operands alone would encode to.
    pub fn without_kept_bits(&self) -> Instruction {
        Instruction { extra: 0, ..*self }
    }

    /// Encodes the instruction at its own PC.
    pub fn encode(&self) -> Result<u32, EncodeError> {
        encode(self, self.pc)
    }
}
