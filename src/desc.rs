//! Operand descriptors: how each operand of an encoding is decoded from (and encoded
//! into) the instruction word. The generator emits one list per encoding from the
//! hand-maintained operand mapping (`gen/src/operands.rs`); value tables come from
//! the spec's assembly rules where the rule names carry field values.

use crate::bits::{Fs, decode_bit_masks, fp_imm8_value, highest_set_bit, sext};
use crate::generated::Register;
use crate::instruction::MAX_OPERANDS;
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
    /// The index of the entry `word` matches.
    #[inline(always)]
    pub const fn entry(&self, word: u32) -> Option<usize> {
        let v = self.f.get(word);
        let mut i = 0;
        while i < self.e.len() {
            if v & self.e[i].1 == self.e[i].0 {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    #[inline(always)]
    pub const fn find(&self, word: u32) -> Option<u8> {
        match self.entry(word) {
            Some(i) => Some(self.e[i].2),
            None => None,
        }
    }

    /// The first entry for `code`: the bits to set and which of them are fixed.
    #[cfg(feature = "encoder")]
    pub fn bits(&self, code: u8) -> Option<(u32, u32)> {
        self.e.iter().find(|e| e.2 == code).map(|e| (e.0, e.1))
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
    #[inline(always)]
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

// `RK` and `ARR` map table codes to values; the encoder maps back with `as u8`.
const _: () = {
    let mut i = 0;
    while i < RK.len() {
        assert!(RK[i] as usize == i);
        i += 1;
    }
    let mut i = 0;
    while i < ARR.len() {
        assert!(ARR[i] as usize == i);
        i += 1;
    }
};

/// An arrangement: fixed, or selected by a table (code = `Arrangement as u8`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Arr {
    Fixed(Arrangement),
    Tab(&'static Tab),
}

impl Arr {
    #[inline(always)]
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
    #[cfg(feature = "encoder")]
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
    #[inline(always)]
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
        /// Only the encoder writes `imms`.
        #[cfg_attr(not(feature = "encoder"), allow(dead_code))]
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
    #[inline(always)]
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
    #[inline(always)]
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

#[inline(always)]
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

fn decode_one(op: &'static Op, word: u32, pc: u64) -> Option<Operand> {
    decode_op(op, word, pc)
}

/// [`decode_one`], inlined so that a constant descriptor folds away (the generated
/// per-list decoders, `DECODE_OPS`). Nested operands (`Opt`, `Braced`) go through
/// `decode_one`.
#[inline(always)]
fn decode_op(op: &'static Op, word: u32, pc: u64) -> Option<Operand> {
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
        Op::Mem(ref m) => decode_mem(m, word)?,
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
            let name = t.find(word).and_then(|c| t.names.get(c as usize));
            Operand {
                kind: match kind {
                    NameKind::Barrier => OpKind::Barrier,
                    NameKind::Prefetch => OpKind::Prefetch,
                    NameKind::Plain => OpKind::Name,
                    NameKind::SvePattern if name.is_none() => OpKind::Immediate,
                    NameKind::SvePattern => OpKind::Name,
                },
                imm: u64::from(v),
                name: name.unwrap_or(&""),
                ..Operand::none()
            }
        }
        Op::Lit(ref s) => Operand {
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
            name: &"MUL",
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

/// Shift codes of shift tables: `ShiftKind as u8` (codes above `MSL` decode to it).
pub(crate) fn shift_kind(code: u8) -> ShiftKind {
    match code {
        0 => ShiftKind::Lsl,
        1 => ShiftKind::Lsr,
        2 => ShiftKind::Asr,
        3 => ShiftKind::Ror,
        _ => ShiftKind::Msl,
    }
}

#[inline(always)]
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

/// Extend codes of `<extend>` tables: `ExtendKind as u8` (the `option` encoding,
/// then 8 for `LSL`).
pub(crate) fn extend_code(c: u8) -> ExtendKind {
    if c < 8 {
        ExtendKind::from_option(u32::from(c))
    } else {
        ExtendKind::Lsl
    }
}

/// The number of operands `ops` decodes to (a mnemonic suffix is not one).
pub(crate) const fn operand_count(ops: &[Op]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < ops.len() {
        if !matches!(ops[i], Op::Suffix(_)) {
            n += 1;
        }
        i += 1;
    }
    n
}

/// Decodes `ops` from `word`. `None` when an operand field holds a reserved value
/// (the word is then UNALLOCATED for this encoding).
pub(crate) fn decode_ops(
    ops: &'static [Op],
    word: u32,
    pc: u64,
    out: &mut [Operand],
) -> Option<u8> {
    let mut n = 0;
    for op in ops {
        decode_step(op, word, pc, out, &mut n)?;
    }
    Some(n as u8)
}

/// A decoder for one operand list: [`decode_ops`] with the list built in, which the
/// generator emits for each distinct list (`DECODE_OPS`, by encoding).
pub(crate) type OpsFn = fn(u32, u64, &mut [Operand; MAX_OPERANDS]) -> Option<u8>;

/// The [`OpsFn`] of encodings without operands.
pub(crate) fn no_ops(_: u32, _: u64, _: &mut [Operand; MAX_OPERANDS]) -> Option<u8> {
    Some(0)
}

/// One entry of an operand list: decodes it into `out[*n]`, or for a mnemonic suffix
/// checks that its value is allocated. Inlined, so that a generated decoder's
/// constant descriptors fold into straight-line code.
#[inline(always)]
pub(crate) fn decode_step(
    op: &'static Op,
    word: u32,
    pc: u64,
    out: &mut [Operand],
    n: &mut usize,
) -> Option<()> {
    if let Op::Suffix(t) = op {
        t.find(word)?;
        return Some(());
    }
    out[*n] = decode_op(op, word, pc)?;
    *n += 1;
    Some(())
}

/// The mnemonic suffix table of an encoding (`2` in `SADDL2`, `B`/`T` in
/// `BFMLALB`/`BFMLALT`), if it has one.
pub(crate) fn suffix_table(ops: &[Op]) -> Option<&'static Tab> {
    ops.iter().find_map(|op| match op {
        Op::Suffix(t) => Some(*t),
        _ => None,
    })
}

/// The mnemonic suffix of an encoding for `word` (`""` when it has none).
#[cfg(any(feature = "formatter", feature = "instr_info"))]
pub(crate) fn suffix(ops: &[Op], word: u32) -> &'static str {
    suffix_table(ops)
        .and_then(|t| t.names.get(t.find(word)? as usize).copied())
        .unwrap_or("")
}

/// The index of the mnemonic suffix in its table (0 when there is none).
pub(crate) fn suffix_code(ops: &[Op], word: u32) -> u8 {
    suffix_table(ops).and_then(|t| t.find(word)).unwrap_or(0)
}
