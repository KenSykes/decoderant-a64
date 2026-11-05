//! Operand descriptors: how each operand of an encoding is decoded from (and encoded
//! into) the instruction word. The generator emits one list per encoding from the
//! hand-maintained operand mapping (`gen/src/operands.rs`); value tables come from
//! the spec's assembly rules where the rule names carry field values.

use crate::bits::{Fs, decode_bit_masks, fp_imm8_value, highest_set_bit, sext};
use crate::generated::Register;
use crate::operand::{
    Arrangement, ConditionCode, ExtendKind, MemoryMode, OpKind, Operand, PredQual, ShiftKind,
};

/// A value table: the first entry whose bits match (`field & mask == value`) gives
/// the code. Codes index the meaning the descriptor gives them (register kind,
/// arrangement, shift, extend, name, number).
#[derive(Debug)]
pub(crate) struct Tab {
    pub f: Fs,
    pub e: &'static [(u32, u32, u8)],
    /// Names for name tables (`targets`, barrier options, ...), by code.
    pub names: &'static [&'static str],
    /// Numbers for number tables (`#<imm>` from `Q`, ...), by code.
    pub nums: &'static [i64],
}

impl Tab {
    pub fn find(&self, word: u32) -> Option<u8> {
        let v = self.f.get(word);
        self.e.iter().find(|e| v & e.1 == e.0).map(|e| e.2)
    }

    /// The first entry for `code`: the bits to set and which of them are fixed.
    pub fn bits(&self, code: u8) -> Option<(u32, u32)> {
        self.e.iter().find(|e| e.2 == code).map(|e| (e.0, e.1))
    }

    /// The word bits that re-encoding the code `word` decodes to reproduces: the
    /// fixed bits of the matching entry, when it is also the entry the encoder
    /// picks for the code (the first). `None` otherwise.
    fn cover(&self, word: u32) -> Option<u32> {
        let v = self.f.get(word);
        let i = self.e.iter().position(|e| v & e.1 == e.0)?;
        let code = self.e[i].2;
        if self.e.iter().position(|e| e.2 == code) != Some(i) {
            return None;
        }
        let mut m = 0;
        self.f.set(&mut m, self.e[i].1);
        Some(m)
    }
}

/// Register kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Rk {
    /// `Xn`, 31 = `XZR`.
    X,
    /// `Wn`, 31 = `WZR`.
    W,
    /// `Xn`, 31 = `SP`.
    XSp,
    /// `Wn`, 31 = `WSP`.
    WSp,
    B,
    H,
    S,
    D,
    Q,
    V,
    Z,
    P,
    PN,
    /// `ZT0` (SME2), whatever the field.
    Zt0,
}

pub(crate) const RK: [Rk; 14] = [
    Rk::X,
    Rk::W,
    Rk::XSp,
    Rk::WSp,
    Rk::B,
    Rk::H,
    Rk::S,
    Rk::D,
    Rk::Q,
    Rk::V,
    Rk::Z,
    Rk::P,
    Rk::PN,
    Rk::Zt0,
];

impl Rk {
    pub fn reg(self, n: u32) -> Register {
        match self {
            Rk::X => Register::x(n),
            Rk::W => Register::w(n),
            Rk::XSp => Register::x_or_sp(n),
            Rk::WSp => Register::w_or_sp(n),
            Rk::B => Register::b(n),
            Rk::H => Register::h(n),
            Rk::S => Register::s(n),
            Rk::D => Register::d(n),
            Rk::Q => Register::q(n),
            Rk::V => Register::v(n),
            Rk::Z => Register::z(n),
            Rk::P => Register::p(n),
            Rk::PN => Register::pn(n),
            Rk::Zt0 => Register::ZT0,
        }
    }
}

/// A register kind: fixed, or selected by a table (code = `Rk as u8`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum RegSel {
    Fixed(Rk),
    Tab(&'static Tab),
}

impl RegSel {
    fn get(self, word: u32) -> Option<Rk> {
        match self {
            RegSel::Fixed(k) => Some(k),
            RegSel::Tab(t) => t.find(word).map(|c| RK[c as usize]),
        }
    }
}

pub(crate) const ARR: [Arrangement; 18] = [
    Arrangement::None,
    Arrangement::B8,
    Arrangement::B16,
    Arrangement::H4,
    Arrangement::H8,
    Arrangement::S2,
    Arrangement::S4,
    Arrangement::D1,
    Arrangement::D2,
    Arrangement::Q1,
    Arrangement::H2,
    Arrangement::B4,
    Arrangement::B2,
    Arrangement::B,
    Arrangement::H,
    Arrangement::S,
    Arrangement::D,
    Arrangement::Q,
];

/// An arrangement: fixed, or selected by a table (code = `Arrangement as u8`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Arr {
    Fixed(Arrangement),
    Tab(&'static Tab),
}

impl Arr {
    fn get(self, word: u32) -> Option<Arrangement> {
        match self {
            Arr::Fixed(a) => Some(a),
            Arr::Tab(t) => t.find(word).map(|c| ARR[c as usize]),
        }
    }
}

/// An element index.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Idx {
    /// Concatenated fields shifted right (the low bits belong to the element size).
    Bits(Fs, u8),
    /// By element size, for the by-element forms: the index fields for `H`, `S`,
    /// `D` (and `B`) elements.
    ByEs {
        b: Fs,
        h: Fs,
        s: Fs,
        d: Fs,
        zs: Fs,
        dz: Fs,
    },
    /// `imm5` style: the bits above the lowest set bit.
    Imm5(Fs),
    /// `imm4` of `INS (element)`, scaled by the element size from `imm5`.
    Imm4(Fs, Fs),
    /// A constant (`<Vd>.D[1]`).
    Const(u8),
    /// An optional index, left out when 0 (`PMOV <Zd>{[<imm>]}, <Pn>.H`).
    Opt(Fs),
    /// The field times a constant (`MOVT ZT0[<offs>]`: bytes, in eights).
    Mul(Fs, u8),
}

/// How a register number comes from its field.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Rn {
    /// `base + mul * field` (`FMOP4A`: `Zn` is `Z0`-`Z14` even, `Zm` `Z16`-`Z30`).
    Lin(u8, u8),
    /// `base + field<1:0> + 8 * field<2>` (`FTMOPA ..., <Zk>[<index>]`: `K:Zk` names
    /// `Z20`-`Z23` and `Z28`-`Z31`).
    K8(u8),
}

impl Rn {
    pub(crate) fn number(self, v: u32) -> u32 {
        match self {
            Rn::Lin(base, mul) => u32::from(base) + u32::from(mul) * v,
            Rn::K8(base) => u32::from(base) + (v & 3) + 8 * (v >> 2),
        }
    }

    /// The field value for register number `n`.
    pub(crate) fn field(self, n: u32) -> Option<u32> {
        match self {
            Rn::Lin(base, mul) => {
                let d = n.checked_sub(u32::from(base))?;
                (d % u32::from(mul) == 0).then_some(d / u32::from(mul))
            }
            Rn::K8(base) => {
                let d = n.checked_sub(u32::from(base))?;
                (d & 4 == 0).then_some((d & 3) | (d >> 3) << 2)
            }
        }
    }
}

impl Idx {
    fn get(self, word: u32, es: Arrangement) -> Option<u8> {
        Some(match self {
            Idx::Bits(f, shr) => (f.get(word) >> shr) as u8,
            Idx::ByEs { b, h, s, d, zs, dz } => {
                if es.element_bytes() == 8 && dz.get(word) != 0 {
                    return None;
                }
                if es.element_bytes() == 4 && zs.get(word) != 0 {
                    return None;
                }
                let f = match es.element_bytes() {
                    1 => b,
                    2 => h,
                    4 => s,
                    8 => d,
                    _ => return None,
                };
                if f.is_none() {
                    return None;
                }
                f.get(word) as u8
            }
            Idx::Imm5(f) => {
                let v = f.get(word);
                let low = v.trailing_zeros();
                if low >= 5 {
                    return None;
                }
                (v >> (low + 1)) as u8
            }
            Idx::Imm4(imm4, imm5) => {
                let low = imm5.get(word).trailing_zeros();
                if low >= 4 {
                    return None;
                }
                (imm4.get(word) >> low) as u8
            }
            Idx::Const(c) => c,
            Idx::Opt(f) => match f.get(word) {
                0 => Operand::NO_LANE,
                v => v as u8,
            },
            Idx::Mul(f, k) => (f.get(word) * u32::from(k)) as u8,
        })
    }
}

/// An immediate value.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Imm {
    Const(i64),
    /// Unsigned field value, shifted left.
    U(Fs, u8),
    /// Signed field value, shifted left.
    S(Fs, u8),
    /// Field value plus a constant.
    Add(Fs, i8),
    /// A constant minus the field value.
    Rsub(i16, Fs),
    /// `-value MOD m`.
    NegMod(Fs, u8),
    /// A number table.
    Tab(&'static Tab),
    /// Right-shift amount from `immh:immb`: `2 * esize - value`.
    ShrEs(Fs),
    /// Left-shift amount from `immh:immb`: `value - esize`.
    ShlEs(Fs),
    /// Logical immediate: `N:immr:imms`, `datasize` 32 or 64.
    Bitmask {
        n: Fs,
        immr: Fs,
        imms: Fs,
        size: u8,
    },
    /// 8-bit FP immediate (the value as `f64` bits).
    Fp8(Fs),
    /// Negated unsigned field, shifted (`AUTIASPPC <label>`: PC minus the offset).
    NegU(Fs, u8),
    /// Bitfield width `imms + 1 - immr` (`UBFX`, `SBFX`, `BFXIL`).
    Width(Fs, Fs),
    /// `LSL #n` as `UBFM`: `n = -immr MOD size`, with `imms = size - 1 - n`.
    LslShift {
        immr: Fs,
        imms: Fs,
        size: u8,
    },
    /// AdvSIMD 64-bit byte mask: each bit of `a:b:c:d:e:f:g:h` becomes a byte of
    /// ones (`MOVI Dd, #imm`, `MOVI Vd.2D, #imm`).
    ByteMask(Fs),
    /// `MOV` (wide immediate): `imm16 << hw * 16`, inverted for `MOVN`, truncated to
    /// `size` bits.
    WideMov {
        imm16: Fs,
        hw: Fs,
        inv: bool,
        size: u8,
    },
    /// Signed field value times a constant (`LD3B ..., #<imm>, MUL VL`: in multiples
    /// of the three registers).
    SMul(Fs, u8),
    /// SVE logical immediate (`DUPM`, `AND`, ...): `N:immr:imms` as a 64-bit
    /// `DecodeBitMasks`, shown as one element (the size `N:imms` gives).
    SveBitmask {
        n: Fs,
        immr: Fs,
        imms: Fs,
    },
}

impl Imm {
    fn get(self, word: u32) -> Option<u64> {
        Some(match self {
            Imm::Const(c) => c as u64,
            Imm::U(f, shl) => u64::from(f.get(word)) << shl,
            Imm::S(f, shl) => (sext(u64::from(f.get(word)), f.width()) << shl) as u64,
            Imm::SMul(f, k) => (sext(u64::from(f.get(word)), f.width()) * i64::from(k)) as u64,
            Imm::Add(f, k) => (i64::from(f.get(word)) + i64::from(k)) as u64,
            Imm::Rsub(k, f) => (i64::from(k) - i64::from(f.get(word))) as u64,
            Imm::NegMod(f, m) => {
                let m = u32::from(m);
                ((m - f.get(word) % m) % m) as u64
            }
            Imm::Tab(t) => *t.nums.get(t.find(word)? as usize)? as u64,
            Imm::ShrEs(f) => {
                let v = f.get(word);
                let esize = 8u32 << highest_set_bit(v >> 3)?;
                u64::from(2 * esize - v)
            }
            Imm::ShlEs(f) => {
                let v = f.get(word);
                let esize = 8u32 << highest_set_bit(v >> 3)?;
                u64::from(v - esize)
            }
            Imm::Bitmask {
                n,
                immr,
                imms,
                size,
            } => {
                decode_bit_masks(
                    n.get(word),
                    imms.get(word),
                    immr.get(word),
                    true,
                    u32::from(size),
                )?
                .0
            }
            Imm::Fp8(f) => fp_imm8_value(f.get(word)).to_bits(),
            Imm::ByteMask(f) => {
                let b = f.get(word);
                (0..8).fold(0u64, |v, i| {
                    if b >> i & 1 == 1 {
                        v | 0xff << (8 * i)
                    } else {
                        v
                    }
                })
            }
            Imm::NegU(f, shl) => (u64::from(f.get(word)) << shl).wrapping_neg(),
            Imm::Width(imms, immr) => {
                (i64::from(imms.get(word)) + 1 - i64::from(immr.get(word))) as u64
            }
            Imm::LslShift { immr, size, .. } => {
                let m = u32::from(size);
                u64::from((m - immr.get(word) % m) % m)
            }
            Imm::WideMov {
                imm16,
                hw,
                inv,
                size,
            } => {
                let v = u64::from(imm16.get(word)) << (16 * hw.get(word));
                let v = if inv { !v } else { v };
                if size == 32 { v & 0xffff_ffff } else { v }
            }
            Imm::SveBitmask { n, immr, imms } => {
                let (n, imms) = (n.get(word), imms.get(word));
                let v = decode_bit_masks(n, imms, immr.get(word), true, 64)?.0;
                let esize = sve_bitmask_esize(n, imms)?;
                if esize == 64 {
                    v
                } else {
                    v & ((1 << esize) - 1)
                }
            }
        })
    }
}

/// A shift applied to an immediate.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ImmShift {
    None,
    /// `LSL #amount`, printed when non-zero.
    Lsl(Imm),
    /// `MSL #amount`, always printed.
    Msl(Imm),
}

/// The offset part of a memory operand.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MemOff {
    None,
    Imm(Imm),
    /// Register offset: index register, its kind, the extend (`None`: always
    /// `LSL`), and the amount.
    Reg {
        m: Fs,
        k: RegSel,
        ext: Option<&'static Tab>,
        amount: Amount,
    },
    /// An optional `X` index with a fixed `LSL` amount (0: none), left out when it is
    /// XZR (`[<Xn|SP>{, <Xm>, LSL #1}]`, `[<Zn>.D{, <Xm>}]`).
    OptReg(Fs, u8),
}

/// The shift amount of a register offset. (`None` and `Fixed` are for operand
/// shapes the mapping handles but the pinned release does not use.)
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum Amount {
    /// No amount (`[Xn, Xm]`).
    None,
    /// Always present (`LSL #3`).
    Fixed(u8),
    /// Present when the bit is set (`S`), with this amount; an absent amount is
    /// printed as nothing.
    OnBit(Fs, u8),
    /// A table (`amount_0_0=#0 amount_1_3=#3`): present when the bit is set.
    Tab(&'static Tab),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Mem {
    pub base: Fs,
    pub base_k: Rk,
    pub off: MemOff,
    pub mode: MemoryMode,
    /// The post-index part, for [`MemoryMode::PostIndex`].
    pub post: Post,
    /// `[Xn]!` without an offset (`CPY*`, `SET*`).
    pub writeback: bool,
    /// The element size of a vector base or index (`[<Zn>.D]`, `[<Xn|SP>, <Zm>.S,
    /// SXTW]`).
    pub arr: Arr,
    /// The immediate offset is in multiples of the vector length (`MUL VL`).
    pub mul_vl: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Post {
    None,
    Imm(Imm),
    /// A register (`Xm`; 31 is not allowed in these encodings).
    Reg(Fs),
}

/// Kinds of named-operand tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NameKind {
    Barrier,
    Prefetch,
    Plain,
    /// An SVE predicate pattern (`VL4`, `ALL`); a value without a name is an
    /// immediate (`#14`).
    SvePattern,
}

/// The qualifier of a governing predicate.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Pq {
    None,
    Zeroing,
    Merging,
    /// From a table (`<Pg>/<ZM>`): code 0 is `/Z`, 1 is `/M`.
    Tab(&'static Tab),
}

impl Pq {
    fn get(self, word: u32) -> Option<PredQual> {
        Some(match self {
            Pq::None => PredQual::None,
            Pq::Zeroing => PredQual::Zeroing,
            Pq::Merging => PredQual::Merging,
            Pq::Tab(t) => match t.find(word)? {
                0 => PredQual::Zeroing,
                _ => PredQual::Merging,
            },
        })
    }
}

/// The element size `DecodeBitMasks` gives `N:imms`, at least 8 (the SVE
/// logical immediates show 2- and 4-bit patterns as bytes).
pub(crate) fn sve_bitmask_esize(n: u32, imms: u32) -> Option<u32> {
    let len = highest_set_bit(n << 6 | (!imms & 0x3f))?;
    if len < 1 {
        return None;
    }
    Some((1u32 << len).max(8))
}

/// One operand descriptor.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Op {
    /// A register (`f` + `add`, mod 32).
    Reg(Fs, RegSel, u8),
    /// A fixed register (`X16` in `SYSP` templates, ...).
    RegFixed(Register),
    /// The second register of a pair named by one field (`SYSP ..., <Xt1>, <Xt2>`):
    /// the next register, or `XZR` again when the field is 31.
    RegPair(Fs, Rk),
    /// A vector with an arrangement.
    Vec(Fs, Arr, u8),
    /// A vector element (`V0.S[1]`), the element size and the index. With the
    /// flag set, `H` elements use only the low four bits of the register field
    /// (`M` is then an index bit).
    Elem(Fs, Arr, Idx, bool),
    /// A list of `n` consecutive vectors (`stride` apart), optionally indexed.
    List(Fs, u8, Arr, Option<Idx>),
    Imm(Imm, ImmShift),
    /// Expanded logical immediate.
    BitImm(Imm),
    FpImm(Imm),
    /// A shifted register: register, kind, shift table (code = `ShiftKind`), amount.
    Shifted(Fs, RegSel, &'static Tab, Fs),
    /// An extended register: register, kind, the `option` field, amount.
    Extended(Fs, RegSel, Fs, Fs),
    Mem(Mem),
    /// PC-relative target: offset, and whether it is a 4 KiB page (`ADRP`).
    Label(Imm, bool),
    /// A condition: the field, whether it is a mnemonic suffix (`B.EQ`), and whether
    /// the operand is its inverse (`<invcond>` in `CSET`, `CINC`, ...).
    Cond(Fs, bool, bool),
    /// A system operation (`DC`, `IC`, `AT`, `TLBI`, ... operand): `op1:CRn:CRm:op2`.
    SysOperation(Fs),
    /// `MRS`/`MSR` system register: `o0:op1:CRn:CRm:op2` (op0 is `2 + o0`).
    SysReg(Fs),
    /// `MSR` (immediate) PSTATE field: `op1:CRm:op2`.
    PState(Fs),
    /// `CRn`/`CRm`.
    Cr(Fs),
    /// A name from a table (barrier option, prefetch operation, BTI target, ...),
    /// or `#imm` when the table has no name for the value.
    Name(&'static Tab, NameKind),
    /// A name with no encoding (`CSYNC`).
    Lit(&'static str),
    /// A register with writeback (`X1!` in `CPY*`/`SET*`).
    RegWb(Fs, Rk),
    /// An optional operand: left out of the text when the field has the default.
    Opt(&'static Op, Fs, u32),
    /// A mnemonic suffix from a name table (`2` in `SADDL2`, `B`/`T` in `BFMLALB`).
    /// Not an operand: decoding only checks the value is allocated.
    Suffix(&'static Tab),
    /// An SVE or SME register (`Z`, `P` or `PN`; field + `add`) with an element size
    /// and a predicate qualifier: `Z0.S`, `P1/M`, `PN8.B`.
    Zr(Fs, Rk, Arr, u8, Pq),
    /// An indexed SVE or SME register (field + `add`): `Z1.S[2]`, `Z3[1]`, `PN9[0]`.
    ZElem(Fs, Rk, Arr, Idx, u8),
    /// `MUL #<imm>` after an SVE pattern: an immediate printed with `MUL`.
    MulImm(Imm),
    /// A list of SVE or SME registers: field, kind, count, stride and multiplier,
    /// element size. The first register is the field times the multiplier
    /// (consecutive lists; SME2 multi-vector lists start at a multiple of their
    /// length), or for a stride of 4 or 8 the field's top bit gives bit 4 and its
    /// other bits the low bits (`{ Z0.S, Z8.S }`, `{ Z17.S, Z21.S, Z25.S, Z29.S }`).
    /// Consecutive lists wrap from Z31 to Z0. The last number is added to the first
    /// register (`FMOP4A ..., { <Zm1>.S-<Zm2>.S }`: `Z16`-`Z30`).
    ZList(Fs, Rk, u8, u8, u8, u8, Arr, Option<Idx>),
    /// A `Z` register numbered by a map from its field, optionally indexed
    /// (`FMOP4A ..., <Zm>.S`, `FTMOPA ..., <Zk>[<index>]`).
    ZrMap(Fs, Rn, Arr, Option<Idx>),
    /// A register indexed by a `W` register and an immediate (`PSEL ..., <Pm>.<T>[<Wv>,
    /// <imm>]`): field, kind, element size, the index register field (a number
    /// added to `W12`), the immediate.
    ZSel(Fs, Rk, Arr, Fs, Idx),
    /// An SME ZA tile: the tile number field and the element size (`<ZAda>.S`).
    ZaTile(Fs, Arr),
    /// A ZA tile slice: tile number field (`F::NONE` for `ZA0.B`), element size, the
    /// H/V bit, the index register field and the first register it numbers (W12),
    /// the offset field, and the number of slices (the offset is the field times
    /// it: `[<Ws>, <offs1>:<offs2>]`).
    ZaSlice(Fs, Arr, Fs, Fs, u8, Fs, u8),
    /// ZA array vectors: element size, index register field and first register (W8
    /// or W12), offset field, number of vectors, vector group (`VGx2`; 0: none).
    ZaArray(Arr, Fs, u8, Fs, u8, u8),
    /// The `ZERO` tile mask.
    ZaMask(Fs),
    /// An operand printed in braces as a list of one (`{ ZT0 }`).
    Braced(&'static Op),
    /// `ZT0{[<offs>, MUL VL]}`: an optional index in multiples of the vector length.
    Zt0Vl(Fs),
}

fn reg_op(r: Register) -> Operand {
    Operand {
        kind: OpKind::Register,
        reg: r,
        ..Operand::none()
    }
}

fn plus(f: Fs, word: u32, add: u8) -> u32 {
    (f.get(word) + u32::from(add)) % 32
}

fn decode_imm_shift(s: ImmShift, word: u32, o: &mut Operand) -> Option<()> {
    match s {
        ImmShift::None => {}
        ImmShift::Lsl(i) => {
            o.shift = ShiftKind::Lsl;
            o.amount = i.get(word)? as u8;
            o.has_amount = true;
        }
        ImmShift::Msl(i) => {
            o.shift = ShiftKind::Msl;
            o.amount = i.get(word)? as u8;
            o.has_amount = true;
        }
    }
    Some(())
}

fn decode_one(op: &Op, word: u32, pc: u64) -> Option<Operand> {
    Some(match *op {
        Op::Reg(f, k, add) => reg_op(k.get(word)?.reg(plus(f, word, add))),
        Op::RegFixed(r) => reg_op(r),
        Op::RegPair(f, k) => {
            let n = f.get(word);
            reg_op(k.reg(if n == 31 { 31 } else { n + 1 }))
        }
        Op::Vec(f, a, add) => Operand {
            arr: a.get(word)?,
            ..reg_op(Register::v(plus(f, word, add)))
        },
        Op::Elem(f, a, idx, mask_h) => {
            let es = a.get(word)?;
            let mut n = f.get(word);
            if mask_h && es.element_bytes() == 2 {
                n &= 15;
            }
            Operand {
                arr: es,
                lane: idx.get(word, es)?,
                ..reg_op(Register::v(n))
            }
        }
        Op::List(f, n, a, idx) => {
            let arr = a.get(word)?;
            Operand {
                kind: OpKind::RegisterList,
                reg: Register::v(f.get(word)),
                arr,
                count: n,
                lane: match idx {
                    Some(i) => i.get(word, arr)?,
                    None => Operand::NO_LANE,
                },
                ..Operand::none()
            }
        }
        Op::Imm(i, s) => {
            let mut o = Operand {
                kind: OpKind::Immediate,
                imm: i.get(word)?,
                signed: matches!(i, Imm::S(..)),
                ..Operand::none()
            };
            decode_imm_shift(s, word, &mut o)?;
            o
        }
        Op::BitImm(i) => Operand {
            kind: OpKind::BitmaskImmediate,
            imm: i.get(word)?,
            raw_fields: match i {
                Imm::Bitmask { n, immr, imms, .. } | Imm::SveBitmask { n, immr, imms } => {
                    (1 << 15 | n.get(word) << 12 | immr.get(word) << 6 | imms.get(word)) as u16
                }
                _ => 0,
            },
            ..Operand::none()
        },
        Op::FpImm(i) => Operand {
            kind: OpKind::FpImmediate,
            imm: i.get(word)?,
            ..Operand::none()
        },
        Op::Shifted(f, k, t, amount) => Operand {
            kind: OpKind::ShiftedRegister,
            reg: k.get(word)?.reg(f.get(word)),
            shift: shift_kind(t.find(word)?),
            amount: amount.get(word) as u8,
            ..Operand::none()
        },
        Op::Extended(f, k, option, amount) => {
            let amt = amount.get(word);
            if amt > 4 {
                return None;
            }
            Operand {
                kind: OpKind::ExtendedRegister,
                reg: k.get(word)?.reg(f.get(word)),
                extend: ExtendKind::from_option(option.get(word)),
                amount: amt as u8,
                has_amount: true,
                ..Operand::none()
            }
        }
        Op::Mem(m) => decode_mem(&m, word)?,
        Op::Label(i, page) => {
            let off = i.get(word)?;
            let base = if page { pc & !0xfff } else { pc };
            Operand {
                kind: OpKind::Label,
                imm: base.wrapping_add(off),
                ..Operand::none()
            }
        }
        Op::Cond(f, suffix, invert) => Operand {
            kind: OpKind::Condition,
            imm: u64::from(ConditionCode::from_bits(f.get(word) ^ u32::from(invert)).bits()),
            suffix,
            ..Operand::none()
        },
        Op::SysReg(f) => {
            let v = f.get(word);
            // o0:op1:CRn:CRm:op2 (15 bits) → op0:op1:CRn:CRm:op2 with op0 = 2 + o0.
            Operand {
                kind: OpKind::SystemRegister,
                imm: u64::from(v | 1 << 15),
                ..Operand::none()
            }
        }
        Op::SysOperation(f) => Operand {
            kind: OpKind::SystemOp,
            imm: u64::from(1u32 << 14 | f.get(word)),
            ..Operand::none()
        },
        Op::PState(f) => {
            // An unallocated PSTATE field is UNDEFINED (`MSR (immediate)`: `otherwise
            // UNDEFINED`).
            let v = f.get(word);
            let enc = crate::sysreg::sys_encoding(0, v >> 7 & 7, 4, v >> 3 & 15, v & 7);
            crate::sysreg::lookup(crate::sysreg::SysKind::MsrImm, enc)?;
            Operand {
                kind: OpKind::PState,
                imm: u64::from(v),
                ..Operand::none()
            }
        }
        Op::Cr(f) => Operand {
            kind: OpKind::ControlRegister,
            imm: u64::from(f.get(word)),
            ..Operand::none()
        },
        Op::Name(t, kind) => {
            let v = t.f.get(word);
            let name = t.find(word).and_then(|c| t.names.get(c as usize)).copied();
            Operand {
                kind: match kind {
                    NameKind::Barrier => OpKind::Barrier,
                    NameKind::Prefetch => OpKind::Prefetch,
                    NameKind::Plain => OpKind::Name,
                    NameKind::SvePattern if name.is_none() => OpKind::Immediate,
                    NameKind::SvePattern => OpKind::Name,
                },
                imm: u64::from(v),
                name: name.unwrap_or(""),
                ..Operand::none()
            }
        }
        Op::Lit(s) => Operand {
            kind: OpKind::Name,
            name: s,
            ..Operand::none()
        },
        Op::RegWb(f, k) => Operand {
            writeback: true,
            ..reg_op(k.reg(f.get(word)))
        },
        Op::Opt(inner, f, default) => Operand {
            elide: f.get(word) == default,
            ..decode_one(inner, word, pc)?
        },
        Op::Zr(f, k, a, add, pq) => Operand {
            arr: a.get(word)?,
            pred: pq.get(word)?,
            ..reg_op(k.reg(plus(f, word, add)))
        },
        Op::ZElem(f, k, a, idx, add) => {
            let es = a.get(word)?;
            Operand {
                arr: es,
                lane: idx.get(word, es)?,
                ..reg_op(k.reg(plus(f, word, add)))
            }
        }
        Op::MulImm(i) => Operand {
            kind: OpKind::Immediate,
            imm: i.get(word)?,
            name: "MUL",
            ..Operand::none()
        },
        Op::ZList(f, k, n, stride, mul, add, a, idx) => {
            let arr = a.get(word)?;
            Operand {
                kind: OpKind::RegisterList,
                reg: k.reg(zlist_first(f.get(word), stride, mul) + u32::from(add)),
                arr,
                count: n,
                stride: if stride > 1 { stride } else { 0 },
                lane: match idx {
                    Some(i) => i.get(word, arr)?,
                    None => Operand::NO_LANE,
                },
                ..Operand::none()
            }
        }
        Op::ZaTile(f, a) => {
            let es = a.get(word)?;
            Operand {
                kind: OpKind::ZaTile,
                reg: Register::za_tile(es.element_bytes().trailing_zeros(), f.get(word))?,
                arr: es,
                ..Operand::none()
            }
        }
        Op::ZaSlice(t, a, hv, ws, base, offs, count) => {
            let es = a.get(word)?;
            Operand {
                kind: OpKind::ZaSlice,
                reg: Register::za_tile(es.element_bytes().trailing_zeros(), t.get(word))?,
                arr: es,
                vertical: hv.get(word) != 0,
                index_reg: Register::w(u32::from(base) + ws.get(word)),
                imm: u64::from(offs.get(word) * u32::from(count)),
                count,
                ..Operand::none()
            }
        }
        Op::ZaArray(a, wv, base, offs, count, vg) => Operand {
            kind: OpKind::ZaArray,
            reg: Register::ZA,
            arr: a.get(word)?,
            index_reg: Register::w(u32::from(base) + wv.get(word)),
            imm: u64::from(offs.get(word) * u32::from(count)),
            count,
            vg,
            ..Operand::none()
        },
        Op::ZaMask(f) => Operand {
            kind: OpKind::ZaMask,
            imm: u64::from(f.get(word)),
            ..Operand::none()
        },
        Op::Braced(inner) => Operand {
            braced: true,
            ..decode_one(inner, word, pc)?
        },
        Op::Zt0Vl(f) => Operand {
            lane: match f.get(word) {
                0 => Operand::NO_LANE,
                v => v as u8,
            },
            mul_vl: true,
            ..reg_op(Register::ZT0)
        },
        Op::ZrMap(f, rn, a, idx) => {
            let es = a.get(word)?;
            Operand {
                arr: es,
                lane: match idx {
                    Some(i) => i.get(word, es)?,
                    None => Operand::NO_LANE,
                },
                ..reg_op(Register::z(rn.number(f.get(word))))
            }
        }
        Op::ZSel(f, k, a, v, idx) => {
            let es = a.get(word)?;
            Operand {
                arr: es,
                lane: idx.get(word, es)?,
                index_reg: Register::w(12 + v.get(word)),
                ..reg_op(k.reg(f.get(word)))
            }
        }
        Op::Suffix(_) => return None,
    })
}

/// The first register of an `Op::ZList` from its field value.
pub(crate) fn zlist_first(v: u32, stride: u8, mul: u8) -> u32 {
    if stride <= 1 {
        v * u32::from(mul)
    } else {
        let s = u32::from(stride).trailing_zeros();
        (v >> s) << 4 | (v & ((1 << s) - 1))
    }
}

pub(crate) fn shift_kind(code: u8) -> ShiftKind {
    match code {
        0 => ShiftKind::Lsl,
        1 => ShiftKind::Lsr,
        2 => ShiftKind::Asr,
        3 => ShiftKind::Ror,
        _ => ShiftKind::Msl,
    }
}

fn decode_mem(m: &Mem, word: u32) -> Option<Operand> {
    let mut o = Operand {
        kind: OpKind::Memory,
        reg: m.base_k.reg(m.base.get(word)),
        mode: m.mode,
        writeback: m.writeback,
        arr: m.arr.get(word)?,
        mul_vl: m.mul_vl,
        ..Operand::none()
    };
    match m.off {
        MemOff::None => {}
        MemOff::Imm(i) => o.imm = i.get(word)?,
        MemOff::OptReg(f, a) => {
            let n = f.get(word);
            if n != 31 {
                o.index_reg = Register::x(n);
                o.extend = ExtendKind::Lsl;
                if a != 0 {
                    o.amount = a;
                    o.has_amount = true;
                }
            }
        }
        MemOff::Reg {
            m: f,
            k,
            ext,
            amount,
        } => {
            o.index_reg = k.get(word)?.reg(f.get(word));
            o.extend = match ext {
                Some(t) => extend_code(t.find(word)?),
                None => ExtendKind::Lsl,
            };
            match amount {
                Amount::None => {}
                Amount::Fixed(a) => {
                    o.amount = a;
                    o.has_amount = true;
                }
                Amount::OnBit(f, a) => {
                    if f.get(word) == 1 {
                        o.amount = a;
                        o.has_amount = true;
                    }
                }
                Amount::Tab(t) => {
                    let v = *t.nums.get(t.find(word)? as usize)?;
                    o.amount = v as u8;
                    o.has_amount = t.f.get(word) != 0;
                }
            }
        }
    }
    match m.post {
        Post::None => {}
        Post::Imm(i) => o.imm = i.get(word)?,
        Post::Reg(f) => o.index_reg = Register::x(f.get(word)),
    }
    Some(o)
}

/// Extend codes of `<extend>` tables: `ExtendKind as u8`.
pub(crate) fn extend_code(c: u8) -> ExtendKind {
    match c {
        0 => ExtendKind::Uxtb,
        1 => ExtendKind::Uxth,
        2 => ExtendKind::Uxtw,
        3 => ExtendKind::Uxtx,
        4 => ExtendKind::Sxtb,
        5 => ExtendKind::Sxth,
        6 => ExtendKind::Sxtw,
        7 => ExtendKind::Sxtx,
        _ => ExtendKind::Lsl,
    }
}

/// Decodes `ops` from `word`. `None` when an operand field holds a reserved value
/// (the word is then UNALLOCATED for this encoding).
pub(crate) fn decode_ops(ops: &[Op], word: u32, pc: u64, out: &mut [Operand]) -> Option<u8> {
    let mut n = 0;
    for op in ops {
        if let Op::Suffix(t) = op {
            t.find(word)?;
            continue;
        }
        out[n] = decode_one(op, word, pc)?;
        n += 1;
    }
    Some(n as u8)
}

fn sel_cover(k: RegSel, word: u32) -> Option<u32> {
    match k {
        RegSel::Fixed(_) => Some(0),
        RegSel::Tab(t) => t.cover(word),
    }
}

fn arr_cover(a: Arr, word: u32) -> Option<u32> {
    match a {
        Arr::Fixed(_) => Some(0),
        Arr::Tab(t) => t.cover(word),
    }
}

fn imm_cover(i: Imm, word: u32) -> Option<u32> {
    Some(match i {
        Imm::Const(_) => 0,
        Imm::U(f, _)
        | Imm::S(f, _)
        | Imm::SMul(f, _)
        | Imm::Add(f, _)
        | Imm::Rsub(_, f)
        | Imm::NegU(f, _)
        | Imm::ShrEs(f)
        | Imm::ShlEs(f)
        | Imm::Fp8(f)
        | Imm::ByteMask(f) => f.mask(),
        Imm::NegMod(f, m) if 1u32 << f.width() <= u32::from(m) => f.mask(),
        // `imms` from the value and `immr`, which another operand sets.
        Imm::Width(imms, _) => imms.mask(),
        Imm::Bitmask { n, immr, imms, .. } => n.mask() | immr.mask() | imms.mask(),
        Imm::Tab(t) => {
            // The encoder looks the value up by its first code.
            let code = t.find(word)?;
            let v = *t.nums.get(code as usize)?;
            if t.nums.iter().position(|&n| n == v) != Some(code as usize) {
                return None;
            }
            t.cover(word)?
        }
        Imm::SveBitmask { .. } => return None,
        Imm::NegMod(..) | Imm::LslShift { .. } | Imm::WideMov { .. } => return None,
    })
}

fn op_cover(op: &Op, word: u32) -> Option<u32> {
    Some(match *op {
        Op::Reg(f, k, _) => f.mask() | sel_cover(k, word)?,
        Op::RegFixed(_) | Op::RegPair(..) | Op::Lit(_) => 0,
        Op::Vec(f, a, _) => f.mask() | arr_cover(a, word)?,
        Op::List(f, _, a, None) => f.mask() | arr_cover(a, word)?,
        Op::Elem(..) | Op::List(_, _, _, Some(_)) => return None,
        Op::Imm(i, s) => {
            imm_cover(i, word)?
                | match s {
                    ImmShift::None => 0,
                    ImmShift::Lsl(a) | ImmShift::Msl(a) => imm_cover(a, word)?,
                }
        }
        Op::BitImm(i) | Op::FpImm(i) | Op::Label(i, _) => imm_cover(i, word)?,
        Op::Shifted(f, k, t, amount) => {
            // Codes above `MSL` all decode to `MSL`.
            if t.find(word)? > 4 {
                return None;
            }
            f.mask() | sel_cover(k, word)? | t.cover(word)? | amount.mask()
        }
        Op::Extended(f, k, option, amount) => {
            f.mask() | sel_cover(k, word)? | option.mask() | amount.mask()
        }
        Op::Mem(ref m) => {
            let off = match m.off {
                MemOff::None => 0,
                MemOff::OptReg(f, _) => f.mask(),
                MemOff::Imm(i) => imm_cover(i, word)?,
                MemOff::Reg {
                    m: f,
                    k,
                    ext,
                    amount,
                } => {
                    let ext = match ext {
                        Some(t) => t.cover(word)?,
                        None => 0,
                    };
                    let amount = match amount {
                        Amount::None | Amount::Fixed(_) => 0,
                        Amount::OnBit(f, _) if f.width() == 1 => f.mask(),
                        Amount::OnBit(..) | Amount::Tab(_) => return None,
                    };
                    f.mask() | sel_cover(k, word)? | ext | amount
                }
            };
            let post = match m.post {
                Post::None => 0,
                Post::Imm(i) => imm_cover(i, word)?,
                Post::Reg(f) => f.mask(),
            };
            m.base.mask() | off | post | arr_cover(m.arr, word)?
        }
        Op::Cond(f, ..) if f.width() <= 4 => f.mask(),
        Op::SysReg(f) if f.width() <= 15 => f.mask(),
        Op::SysOperation(f) if f.width() <= 14 => f.mask(),
        Op::PState(f) | Op::Cr(f) | Op::RegWb(f, _) => f.mask(),
        Op::Name(t, _) => t.f.mask(),
        Op::Opt(inner, ..) => op_cover(inner, word)?,
        Op::Suffix(t) => t.cover(word)?,
        Op::Zr(f, _, a, _, pq) => {
            f.mask()
                | arr_cover(a, word)?
                | match pq {
                    Pq::Tab(t) => t.cover(word)?,
                    _ => 0,
                }
        }
        Op::ZElem(..) => return None,
        Op::MulImm(i) => imm_cover(i, word)?,
        Op::ZList(f, _, _, _, _, _, a, None) => f.mask() | arr_cover(a, word)?,
        Op::ZList(.., Some(_)) | Op::ZSel(..) => return None,
        Op::ZrMap(f, _, a, None) => f.mask() | arr_cover(a, word)?,
        Op::ZrMap(..) => return None,
        Op::ZaTile(f, a) => f.mask() | arr_cover(a, word)?,
        Op::ZaSlice(t, a, hv, ws, _, offs, _) => {
            t.mask() | arr_cover(a, word)? | hv.mask() | ws.mask() | offs.mask()
        }
        Op::ZaArray(a, wv, _, offs, _, _) => arr_cover(a, word)? | wv.mask() | offs.mask(),
        Op::ZaMask(f) | Op::Zt0Vl(f) => f.mask(),
        Op::Braced(inner) => op_cover(inner, word)?,
        Op::Cond(..) | Op::SysReg(_) | Op::SysOperation(_) => return None,
    })
}

/// The word bits that re-encoding the operands decoded from `word` is certain to
/// reproduce, or `None` when some operand's encoding may differ from the word (a
/// table with several entries for one value, a value with several encodings, ...).
/// Variable bits outside the result are don't-care bits; the decoder finds them by
/// re-encoding, which this lets it skip when there are none.
pub(crate) fn cover(ops: &[Op], word: u32) -> Option<u32> {
    let mut m = 0;
    for op in ops {
        m |= op_cover(op, word)?;
    }
    Some(m)
}

// A lower bound of `cover` over all words (usable in constants), or `None` where
// `cover` can be `None`, or where a bound needs the word.

/// For a table whose codes each have one entry, re-encoding writes back whichever
/// entry a word matches: its fixed bits, at least the bits every entry fixes.
const fn tab_cover_const(t: &Tab) -> Option<u32> {
    let mut common = u32::MAX;
    let mut i = 0;
    while i < t.e.len() {
        common &= t.e[i].1;
        let mut j = 0;
        while j < i {
            if t.e[j].2 == t.e[i].2 {
                return None;
            }
            j += 1;
        }
        i += 1;
    }
    let mut m = 0;
    t.f.set(&mut m, common);
    Some(m)
}

const fn sel_cover_const(k: RegSel) -> Option<u32> {
    match k {
        RegSel::Fixed(_) => Some(0),
        RegSel::Tab(t) => tab_cover_const(t),
    }
}

const fn arr_cover_const(a: Arr) -> Option<u32> {
    match a {
        Arr::Fixed(_) => Some(0),
        Arr::Tab(t) => tab_cover_const(t),
    }
}

/// `a | b` when both are known.
const fn or(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a | b),
        _ => None,
    }
}

const fn imm_cover_const(i: Imm) -> Option<u32> {
    match i {
        Imm::Const(_) => Some(0),
        Imm::U(f, _)
        | Imm::S(f, _)
        | Imm::SMul(f, _)
        | Imm::Add(f, _)
        | Imm::Rsub(_, f)
        | Imm::NegU(f, _)
        | Imm::ShrEs(f)
        | Imm::ShlEs(f)
        | Imm::Fp8(f)
        | Imm::ByteMask(f) => Some(f.mask()),
        Imm::NegMod(f, m) if 1u32 << f.width() <= m as u32 => Some(f.mask()),
        Imm::Width(imms, _) => Some(imms.mask()),
        Imm::Bitmask { n, immr, imms, .. } => Some(n.mask() | immr.mask() | imms.mask()),
        Imm::Tab(t) => {
            // As in `imm_cover`, each number must have one code.
            let mut i = 0;
            while i < t.nums.len() {
                let mut j = 0;
                while j < i {
                    if t.nums[j] == t.nums[i] {
                        return None;
                    }
                    j += 1;
                }
                i += 1;
            }
            tab_cover_const(t)
        }
        Imm::SveBitmask { .. } => None,
        Imm::NegMod(..) | Imm::LslShift { .. } | Imm::WideMov { .. } => None,
    }
}

const fn op_cover_const(op: &Op) -> Option<u32> {
    match *op {
        Op::Reg(f, k, _) => or(Some(f.mask()), sel_cover_const(k)),
        Op::RegFixed(_) | Op::RegPair(..) | Op::Lit(_) => Some(0),
        Op::Vec(f, a, _) | Op::List(f, _, a, None) => or(Some(f.mask()), arr_cover_const(a)),
        Op::Elem(..) | Op::List(_, _, _, Some(_)) => None,
        Op::Imm(i, s) => or(
            imm_cover_const(i),
            match s {
                ImmShift::None => Some(0),
                ImmShift::Lsl(a) | ImmShift::Msl(a) => imm_cover_const(a),
            },
        ),
        Op::BitImm(i) | Op::FpImm(i) | Op::Label(i, _) => imm_cover_const(i),
        Op::Shifted(f, k, t, amount) => {
            // As in `op_cover`: codes above `MSL` all decode to `MSL`.
            let mut i = 0;
            while i < t.e.len() {
                if t.e[i].2 > 4 {
                    return None;
                }
                i += 1;
            }
            or(
                Some(f.mask() | amount.mask()),
                or(sel_cover_const(k), tab_cover_const(t)),
            )
        }
        Op::Extended(f, k, option, amount) => or(
            Some(f.mask() | option.mask() | amount.mask()),
            sel_cover_const(k),
        ),
        Op::Mem(ref m) => {
            let off = match m.off {
                MemOff::None => Some(0),
                MemOff::OptReg(f, _) => Some(f.mask()),
                MemOff::Imm(i) => imm_cover_const(i),
                MemOff::Reg {
                    m: f,
                    k,
                    ext,
                    amount,
                } => {
                    let ext = match ext {
                        Some(t) => tab_cover_const(t),
                        None => Some(0),
                    };
                    let amount = match amount {
                        Amount::None | Amount::Fixed(_) => Some(0),
                        Amount::OnBit(f, _) if f.width() == 1 => Some(f.mask()),
                        Amount::OnBit(..) | Amount::Tab(_) => None,
                    };
                    or(or(Some(f.mask()), sel_cover_const(k)), or(ext, amount))
                }
            };
            let post = match m.post {
                Post::None => Some(0),
                Post::Imm(i) => imm_cover_const(i),
                Post::Reg(f) => Some(f.mask()),
            };
            or(
                or(Some(m.base.mask()), arr_cover_const(m.arr)),
                or(off, post),
            )
        }
        Op::Cond(f, ..) if f.width() <= 4 => Some(f.mask()),
        Op::SysReg(f) if f.width() <= 15 => Some(f.mask()),
        Op::SysOperation(f) if f.width() <= 14 => Some(f.mask()),
        Op::PState(f) | Op::Cr(f) | Op::RegWb(f, _) => Some(f.mask()),
        Op::Name(t, _) => Some(t.f.mask()),
        Op::Opt(inner, ..) => op_cover_const(inner),
        Op::Suffix(t) => tab_cover_const(t),
        Op::Zr(f, _, a, _, pq) => or(
            or(Some(f.mask()), arr_cover_const(a)),
            match pq {
                Pq::Tab(t) => tab_cover_const(t),
                _ => Some(0),
            },
        ),
        Op::ZElem(..) => None,
        Op::MulImm(i) => imm_cover_const(i),
        Op::ZList(f, _, _, _, _, _, a, None) => or(Some(f.mask()), arr_cover_const(a)),
        Op::ZList(.., Some(_)) | Op::ZSel(..) => None,
        Op::ZrMap(f, _, a, None) => or(Some(f.mask()), arr_cover_const(a)),
        Op::ZrMap(..) => None,
        Op::ZaTile(f, a) => or(Some(f.mask()), arr_cover_const(a)),
        Op::ZaSlice(t, a, hv, ws, _, offs, _) => or(
            Some(t.mask() | hv.mask() | ws.mask() | offs.mask()),
            arr_cover_const(a),
        ),
        Op::ZaArray(a, wv, _, offs, _, _) => or(Some(wv.mask() | offs.mask()), arr_cover_const(a)),
        Op::ZaMask(f) | Op::Zt0Vl(f) => Some(f.mask()),
        Op::Braced(inner) => op_cover_const(inner),
        Op::Cond(..) | Op::SysReg(_) | Op::SysOperation(_) => None,
    }
}

/// A lower bound of [`cover`] over all words, where `cover` is never `None`.
pub(crate) const fn cover_const(ops: &[Op]) -> Option<u32> {
    let mut m = 0;
    let mut i = 0;
    while i < ops.len() {
        match op_cover_const(&ops[i]) {
            Some(c) => m |= c,
            None => return None,
        }
        i += 1;
    }
    Some(m)
}

/// The mnemonic suffix of an encoding for `word` (`""` when it has none).
pub(crate) fn suffix(ops: &[Op], word: u32) -> &'static str {
    for op in ops {
        if let Op::Suffix(t) = op {
            return t
                .find(word)
                .and_then(|c| t.names.get(c as usize))
                .copied()
                .unwrap_or("");
        }
    }
    ""
}

/// The index of the mnemonic suffix in its table (0 when there is none).
pub(crate) fn suffix_code(ops: &[Op], word: u32) -> u8 {
    for op in ops {
        if let Op::Suffix(t) = op {
            return t.find(word).unwrap_or(0);
        }
    }
    0
}
