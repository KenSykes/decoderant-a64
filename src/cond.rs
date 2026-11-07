//! Evaluation of the spec's encoding and alias conditions. The generator compiles
//! each condition into a function (`crate::generated::conds`) built from the
//! helpers here; the expression table it also emits
//! ([`EXPRS`](crate::generated::tables::EXPRS)) is interpreted only by the tests,
//! as the reference the compiled functions are checked against.

use crate::bits::decode_bit_masks;
use crate::feature_set::FeatureSet;
#[cfg(test)]
use crate::generated::Feature;
use crate::generated::conds::{CONDS, SYSOP_NAMES};
#[cfg(test)]
use crate::generated::tables::{EXPR_ARGS, EXPRS};

// The evaluator covers every operator and helper the generator compiles, whether or
// not the pinned release uses it.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Eq,
    Ne,
    And,
    Or,
    Lt,
    Gt,
    Ge,
    Le,
    Add,
    Sub,
    Mod,
    Xor,
    Concat,
}

/// Pseudocode helpers used by alias preferences, implemented by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum Func {
    BFXPreferred,
    MoveWidePreferred,
    SVEMoveMaskPreferred,
    DecodeBitMasksCheckUndefined,
    IsZero,
    IsOnes,
    BitCount,
    ZeroExtend,
    /// `SysOpIs(op1, CRn, CRm, op2, kind, alias)`: whether the operation is the
    /// named system instruction alias (`kind`: 0 SYS, 1 SYSL, 2 SYSP).
    SysOpIs,
}

#[cfg(test)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Expr {
    Bool(bool),
    Field {
        lsb: u8,
        width: u8,
    },
    Bits {
        value: u32,
        mask: u32,
        width: u8,
    },
    Int(i64),
    Feature(Feature),
    Not(u16),
    BitNot(u16),
    Bin(BinOp, u16, u16),
    /// Operand, then the members in `EXPR_ARGS[start..start + len]`.
    In(u16, u16, u16),
    Slice(u16, u8, u8),
    UInt(u16),
    Call(Func, u16, u8),
}

/// A value during evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Val {
    Bool(bool),
    /// A bit string; `mask` has the fixed bits (constants may contain `x`).
    Bits {
        value: u64,
        mask: u64,
        width: u8,
    },
    Int(i64),
}

impl Val {
    #[inline(always)]
    fn bits(value: u64, width: u8) -> Val {
        let mask = if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        Val::Bits {
            value: value & mask,
            mask,
            width,
        }
    }

    #[inline(always)]
    pub(crate) fn truthy(self) -> bool {
        match self {
            Val::Bool(b) => b,
            Val::Bits { value, .. } => value != 0,
            Val::Int(i) => i != 0,
        }
    }

    #[inline(always)]
    fn int(self) -> i64 {
        match self {
            Val::Bool(b) => b as i64,
            Val::Bits { value, .. } => value as i64,
            Val::Int(i) => i,
        }
    }

    #[inline(always)]
    fn raw(self) -> (u64, u8) {
        match self {
            Val::Bits { value, width, .. } => (value, width),
            Val::Bool(b) => (b as u64, 1),
            Val::Int(i) => (i as u64, 64),
        }
    }
}

#[inline(always)]
pub(crate) fn equal(a: Val, b: Val) -> bool {
    match (a, b) {
        (
            Val::Bits {
                value: av,
                mask: am,
                ..
            },
            Val::Bits {
                value: bv,
                mask: bm,
                ..
            },
        ) => {
            let m = am & bm;
            av & m == bv & m
        }
        (Val::Bool(x), Val::Bool(y)) => x == y,
        _ => a.int() == b.int(),
    }
}

/// Context for evaluating a condition on one word.
pub(crate) struct Ctx<'a> {
    pub word: u32,
    pub features: &'a FeatureSet,
}

// One helper per kind of expression, shared by the compiled conditions and the
// reference evaluator.

#[inline(always)]
pub(crate) fn field(cx: &Ctx<'_>, lsb: u8, width: u8) -> Val {
    Val::bits(u64::from(cx.word >> lsb), width)
}

#[inline(always)]
pub(crate) fn not(a: Val) -> Val {
    Val::Bool(!a.truthy())
}

// Like `BinOp` and `Func`, covers an operator the pinned release may not use.
#[allow(dead_code)]
#[inline(always)]
pub(crate) fn bit_not(a: Val) -> Val {
    let (v, w) = a.raw();
    Val::bits(!v, w)
}

/// A binary operator. The callers short-circuit the logical ones first, so here
/// `And` and `Or` only see a left operand that does not decide the result.
#[inline(always)]
pub(crate) fn bin(op: BinOp, x: Val, y: Val) -> Val {
    match op {
        BinOp::Eq => Val::Bool(equal(x, y)),
        BinOp::Ne => Val::Bool(!equal(x, y)),
        BinOp::And | BinOp::Or => Val::Bool(y.truthy()),
        BinOp::Lt => Val::Bool(x.int() < y.int()),
        BinOp::Gt => Val::Bool(x.int() > y.int()),
        BinOp::Ge => Val::Bool(x.int() >= y.int()),
        BinOp::Le => Val::Bool(x.int() <= y.int()),
        BinOp::Add => Val::Int(x.int().wrapping_add(y.int())),
        BinOp::Sub => Val::Int(x.int().wrapping_sub(y.int())),
        BinOp::Mod => Val::Int(x.int().checked_rem_euclid(y.int()).unwrap_or(0)),
        BinOp::Xor => {
            let (a, w) = x.raw();
            let (b, _) = y.raw();
            Val::bits(a ^ b, w)
        }
        BinOp::Concat => {
            let (a, aw) = x.raw();
            let (b, bw) = y.raw();
            Val::bits(a << bw | b, aw + bw)
        }
    }
}

#[inline(always)]
pub(crate) fn slice(a: Val, hi: u8, lo: u8) -> Val {
    let (v, _) = a.raw();
    Val::bits(v >> lo, hi - lo + 1)
}

#[inline(always)]
pub(crate) fn uint(a: Val) -> Val {
    Val::Int(a.raw().0 as i64)
}

/// The reference evaluator: interprets `EXPRS`.
#[cfg(test)]
fn eval(id: u16, cx: &Ctx<'_>) -> Val {
    match EXPRS[id as usize] {
        Expr::Bool(b) => Val::Bool(b),
        Expr::Field { lsb, width } => field(cx, lsb, width),
        Expr::Bits { value, mask, width } => Val::Bits {
            value: u64::from(value),
            mask: u64::from(mask),
            width,
        },
        Expr::Int(i) => Val::Int(i),
        Expr::Feature(f) => Val::Bool(cx.features.has(f)),
        Expr::Not(a) => not(eval(a, cx)),
        Expr::BitNot(a) => bit_not(eval(a, cx)),
        Expr::Bin(op, a, b) => {
            let x = eval(a, cx);
            // Short-circuit the logical operators.
            match op {
                BinOp::And if !x.truthy() => return Val::Bool(false),
                BinOp::Or if x.truthy() => return Val::Bool(true),
                _ => {}
            }
            bin(op, x, eval(b, cx))
        }
        Expr::In(a, start, len) => {
            let x = eval(a, cx);
            let members = &EXPR_ARGS[start as usize..(start + len) as usize];
            Val::Bool(members.iter().any(|&m| equal(x, eval(m, cx))))
        }
        Expr::Slice(a, hi, lo) => slice(eval(a, cx), hi, lo),
        Expr::UInt(a) => uint(eval(a, cx)),
        Expr::Call(f, start, n) => {
            let args: std::vec::Vec<Val> = EXPR_ARGS[start as usize..start as usize + n as usize]
                .iter()
                .map(|&a| eval(a, cx))
                .collect();
            call(f, &args)
        }
    }
}

fn arg(args: &[Val], i: usize) -> (u64, u8) {
    args.get(i).map(|a| a.raw()).unwrap_or((0, 0))
}

pub(crate) fn call(f: Func, args: &[Val]) -> Val {
    match f {
        // BFXPreferred(sf, uns, imms, immr): whether UBFM/SBFM should print as
        // UBFX/SBFX rather than the other aliases.
        Func::BFXPreferred => {
            let (sf, _) = arg(args, 0);
            let (uns, _) = arg(args, 1);
            let (imms, _) = arg(args, 2);
            let (immr, _) = arg(args, 3);
            let bfx = if imms < immr || imms == if sf == 1 { 63 } else { 31 } {
                false
            } else if immr == 0 {
                // UXTB/UXTH/SXTB/SXTH/SXTW take precedence.
                !((sf == 0 && (imms == 7 || imms == 15))
                    || (sf == 1 && uns == 0 && (imms == 7 || imms == 15 || imms == 31)))
            } else {
                true
            };
            Val::Bool(bfx)
        }
        // MoveWidePreferred(sf, immN, imms, immr): whether ORR (immediate) or
        // MOVZ/MOVN can express the bitmask as a single move.
        Func::MoveWidePreferred => {
            let (sf, _) = arg(args, 0);
            let (n, _) = arg(args, 1);
            let (imms, _) = arg(args, 2);
            let (immr, _) = arg(args, 3);
            Val::Bool(move_wide_preferred(
                sf == 1,
                n as u32,
                imms as u32,
                immr as u32,
            ))
        }
        Func::SVEMoveMaskPreferred => {
            let (imm13, _) = arg(args, 0);
            let imm13 = imm13 as u32;
            Val::Bool(sve_move_mask_preferred(
                imm13 >> 12 & 1,
                imm13 >> 6 & 63,
                imm13 & 63,
            ))
        }
        Func::DecodeBitMasksCheckUndefined => {
            let (n, _) = arg(args, 0);
            let (imms, _) = arg(args, 1);
            Val::Bool(decode_bit_masks(n as u32, imms as u32, 0, true, 64).is_none())
        }
        Func::IsZero => Val::Bool(arg(args, 0).0 == 0),
        Func::IsOnes => {
            let (v, w) = arg(args, 0);
            let m = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
            Val::Bool(v & m == m)
        }
        Func::BitCount => Val::Int(arg(args, 0).0.count_ones() as i64),
        Func::SysOpIs => {
            let a = |i| arg(args, i).0 as u32;
            let enc = crate::sysreg::sys_encoding(1, a(0), a(1), a(2), a(3));
            let kind = match a(4) {
                0 => crate::sysreg::SysKind::Sys,
                1 => crate::sysreg::SysKind::Sysl,
                _ => crate::sysreg::SysKind::Sysp,
            };
            let want = SYSOP_NAMES.get(a(5) as usize).copied().unwrap_or("");
            let have = crate::sysreg::lookup_insn(kind, enc).map_or("SYS", |e| e.insn);
            Val::Bool(have == want)
        }
        Func::ZeroExtend => {
            let (v, _) = arg(args, 0);
            let (w, _) = arg(args, 1);
            Val::bits(v, w as u8)
        }
    }
}

/// The pseudocode's `MoveWidePreferred()`.
pub(crate) fn move_wide_preferred(sf: bool, n: u32, imms: u32, immr: u32) -> bool {
    let width = if sf { 64 } else { 32 };
    if sf && n != 1 {
        return false;
    }
    if !sf && !(n == 0 && imms & 0x20 == 0) {
        return false;
    }
    // MOVZ: up to 16 consecutive ones, rotated into one 16-bit halfword.
    if imms < 16 {
        return (immr.wrapping_neg() % 16) <= 15 - imms;
    }
    // MOVN: up to 16 consecutive zeros.
    if imms >= width - 15 {
        return immr % 16 <= imms - (width - 15);
    }
    false
}

/// Evaluates a condition.
#[inline]
pub(crate) fn holds(id: u16, cx: &Ctx<'_>) -> bool {
    CONDS[id as usize](cx)
}

/// The pseudocode's `SVEMoveMaskPreferred(imm13)`: whether `DUPM` should print as
/// `MOV`, which it should not when a `DUP` (immediate) of a signed 8-bit value,
/// optionally shifted left by 8, gives the same 64-bit pattern.
pub(crate) fn sve_move_mask_preferred(n: u32, immr: u32, imms: u32) -> bool {
    let Some((imm, _)) = decode_bit_masks(n, imms, immr, true, 64) else {
        return false;
    };
    let bits = |hi: u32, lo: u32| imm >> lo & (u64::MAX >> (63 - (hi - lo)));
    let zero = |hi: u32, lo: u32| bits(hi, lo) == 0;
    let ones = |hi: u32, lo: u32| bits(hi, lo) == u64::MAX >> (63 - (hi - lo));
    let rep32 = bits(63, 32) == bits(31, 0);
    let rep16 = rep32 && bits(31, 16) == bits(15, 0);
    if bits(7, 0) != 0 {
        // `ffffffffffffffxy`, `ffffffxyffffffxy`, `ffxyffxyffxyffxy`, `xyxy...`.
        !(zero(63, 7)
            || ones(63, 7)
            || rep32 && (zero(31, 7) || ones(31, 7))
            || rep16 && (zero(15, 7) || ones(15, 7))
            || rep16 && bits(15, 8) == bits(7, 0))
    } else {
        // `ffffffffffffxy00`, `ffffxy00ffffxy00`, `xy00xy00xy00xy00`.
        !(zero(63, 15) || ones(63, 15) || rep32 && (zero(31, 15) || ones(31, 15)) || rep16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The compiled conditions agree with the reference evaluator on random words,
    /// with every feature, none, and random feature subsets.
    #[test]
    fn compiled_conditions_match_the_evaluator() {
        let mut next = crate::bits::xorshift(0x9e37_79b9_7f4a_7c15);
        let mut sets = std::vec![FeatureSet::ALL, FeatureSet::EMPTY];
        for _ in 0..30 {
            let mut s = FeatureSet::EMPTY;
            for f in Feature::values() {
                if next() % 3 != 0 {
                    s.insert(f);
                }
            }
            sets.push(s);
        }
        for features in &sets {
            for _ in 0..20_000 {
                let cx = Ctx {
                    word: next() as u32,
                    features,
                };
                for id in 0..EXPRS.len() as u16 {
                    assert_eq!(
                        holds(id, &cx),
                        eval(id, &cx).truthy(),
                        "expression {id} on {:#010x}",
                        cx.word
                    );
                }
            }
        }
    }
}
