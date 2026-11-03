//! Evaluation of the spec's encoding and alias conditions, compiled by the generator
//! into [`EXPRS`](crate::generated::tables::EXPRS).

use crate::bits::decode_bit_masks;
use crate::feature_set::FeatureSet;
use crate::generated::Feature;
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

/// System instruction aliases (same list as the generator's `SYSOP_NAMES`).
pub(crate) const SYSOP_NAMES: &[&str] = &[
    "SYS", "DC", "IC", "AT", "TLBI", "TLBIP", "BRB", "CFP", "CPP", "DVP", "COSP", "APAS", "GIC",
    "GICR", "GSB", "PLBI", "TRCIT", "GCSPUSHM", "GCSPOPM", "GCSPUSHX", "GCSPOPX", "GCSPOPCX",
    "GCSSS1", "GCSSS2",
];

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
enum Val {
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

    fn truthy(self) -> bool {
        match self {
            Val::Bool(b) => b,
            Val::Bits { value, .. } => value != 0,
            Val::Int(i) => i != 0,
        }
    }

    fn int(self) -> i64 {
        match self {
            Val::Bool(b) => b as i64,
            Val::Bits { value, .. } => value as i64,
            Val::Int(i) => i,
        }
    }

    fn raw(self) -> (u64, u8) {
        match self {
            Val::Bits { value, width, .. } => (value, width),
            Val::Bool(b) => (b as u64, 1),
            Val::Int(i) => (i as u64, 64),
        }
    }
}

fn equal(a: Val, b: Val) -> bool {
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

fn eval(id: u16, cx: &Ctx<'_>) -> Val {
    match EXPRS[id as usize] {
        Expr::Bool(b) => Val::Bool(b),
        Expr::Field { lsb, width } => Val::bits(u64::from(cx.word >> lsb), width),
        Expr::Bits { value, mask, width } => Val::Bits {
            value: u64::from(value),
            mask: u64::from(mask),
            width,
        },
        Expr::Int(i) => Val::Int(i),
        Expr::Feature(f) => Val::Bool(cx.features.has(f)),
        Expr::Not(a) => Val::Bool(!eval(a, cx).truthy()),
        Expr::BitNot(a) => {
            let (v, w) = eval(a, cx).raw();
            Val::bits(!v, w)
        }
        Expr::Bin(op, a, b) => {
            let x = eval(a, cx);
            // Short-circuit the logical operators.
            match op {
                BinOp::And if !x.truthy() => return Val::Bool(false),
                BinOp::Or if x.truthy() => return Val::Bool(true),
                _ => {}
            }
            let y = eval(b, cx);
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
        Expr::In(a, start, len) => {
            let x = eval(a, cx);
            let members = &EXPR_ARGS[start as usize..(start + len) as usize];
            Val::Bool(members.iter().any(|&m| equal(x, eval(m, cx))))
        }
        Expr::Slice(a, hi, lo) => {
            let (v, _) = eval(a, cx).raw();
            Val::bits(v >> lo, hi - lo + 1)
        }
        Expr::UInt(a) => Val::Int(eval(a, cx).raw().0 as i64),
        Expr::Call(f, start, n) => {
            let args = &EXPR_ARGS[start as usize..start as usize + n as usize];
            call(f, args, cx)
        }
    }
}

fn arg(args: &[u16], i: usize, cx: &Ctx<'_>) -> (u64, u8) {
    args.get(i).map(|&a| eval(a, cx).raw()).unwrap_or((0, 0))
}

fn call(f: Func, args: &[u16], cx: &Ctx<'_>) -> Val {
    match f {
        // BFXPreferred(sf, uns, imms, immr): whether UBFM/SBFM should print as
        // UBFX/SBFX rather than the other aliases.
        Func::BFXPreferred => {
            let (sf, _) = arg(args, 0, cx);
            let (uns, _) = arg(args, 1, cx);
            let (imms, _) = arg(args, 2, cx);
            let (immr, _) = arg(args, 3, cx);
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
            let (sf, _) = arg(args, 0, cx);
            let (n, _) = arg(args, 1, cx);
            let (imms, _) = arg(args, 2, cx);
            let (immr, _) = arg(args, 3, cx);
            Val::Bool(move_wide_preferred(
                sf == 1,
                n as u32,
                imms as u32,
                immr as u32,
            ))
        }
        Func::SVEMoveMaskPreferred => Val::Bool(false),
        Func::DecodeBitMasksCheckUndefined => {
            let (n, _) = arg(args, 0, cx);
            let (imms, _) = arg(args, 1, cx);
            Val::Bool(decode_bit_masks(n as u32, imms as u32, 0, true, 64).is_none())
        }
        Func::IsZero => Val::Bool(arg(args, 0, cx).0 == 0),
        Func::IsOnes => {
            let (v, w) = arg(args, 0, cx);
            let m = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
            Val::Bool(v & m == m)
        }
        Func::BitCount => Val::Int(arg(args, 0, cx).0.count_ones() as i64),
        Func::SysOpIs => {
            let a = |i| arg(args, i, cx).0 as u32;
            let enc = crate::sysreg::sys_encoding(1, a(0), a(1), a(2), a(3));
            let kind = match a(4) {
                0 => crate::sysreg::SysKind::Sys,
                1 => crate::sysreg::SysKind::Sysl,
                _ => crate::sysreg::SysKind::Sysp,
            };
            let want = SYSOP_NAMES.get(a(5) as usize).copied().unwrap_or("");
            let have = crate::sysreg::lookup(kind, enc).map_or("SYS", |e| e.insn);
            Val::Bool(have == want)
        }
        Func::ZeroExtend => {
            let (v, _) = arg(args, 0, cx);
            let (w, _) = arg(args, 1, cx);
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

/// Evaluates a compiled condition.
pub(crate) fn holds(id: u16, cx: &Ctx<'_>) -> bool {
    eval(id, cx).truthy()
}
