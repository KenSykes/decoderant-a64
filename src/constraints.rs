//! UNPREDICTABLE and CONSTRAINED UNPREDICTABLE instances.
//!
//! Should-be bits come from the spec. The other conditions are not all flagged by
//! the machine-readable spec, so they are this hand-maintained table: each entry
//! names the encodings (by iclass and fixed bits), the condition, the reason and the
//! Arm ARM reference (the encoding's pseudocode or "Architectural Constraints on
//! UNPREDICTABLE behaviors", Arm ARM K1). It covers at least the cases in
//! docs/arch/aarch64.md ("Determinism and unpredictable behaviour"); the generator
//! excludes every instance it flags.

use crate::instruction::Instruction;

/// One table entry.
struct Constraint {
    /// iclasses the entry applies to.
    iclass: &'static [&'static str],
    /// The word must also match `word & mask == value` (e.g. loads only).
    mask: u32,
    value: u32,
    test: fn(u32) -> bool,
    reason: &'static str,
}

fn f(w: u32, lsb: u32, width: u32) -> u32 {
    w >> lsb & ((1 << width) - 1)
}

fn rt(w: u32) -> u32 {
    f(w, 0, 5)
}
fn rn(w: u32) -> u32 {
    f(w, 5, 5)
}
fn rt2(w: u32) -> u32 {
    f(w, 10, 5)
}
fn rs(w: u32) -> u32 {
    f(w, 16, 5)
}
fn rd(w: u32) -> u32 {
    f(w, 0, 5)
}

/// Bit 26 (`VR`): set for SIMD&FP transfer registers, which never alias the base.
const VR: u32 = 1 << 26;
/// Bit 22 (`L`): set for loads in the pair and single-register classes.
const L: u32 = 1 << 22;
/// Bits 23:22 (`op1`) of the CPY*/SET* classes: 11 for the SET* family.
const SET: u32 = 3 << 22;

static TABLE: &[Constraint] = &[
    // C6.2 LDP/LDNP/LDPSW: `if t == t2 then Constraint_UNKNOWN/UNDEF/NOP` (loads).
    Constraint {
        iclass: &[
            "ldstpair_off",
            "ldstpair_pre",
            "ldstpair_post",
            "ldstnapair_offs",
        ],
        mask: L,
        value: L,
        test: |w| rt(w) == rt2(w),
        reason: "load pair with Rt == Rt2 (CONSTRAINED UNPREDICTABLE)",
    },
    // LDP/STP writeback: `if wback && (t == n || t2 == n) && n != 31` (GP only).
    Constraint {
        iclass: &["ldstpair_pre", "ldstpair_post"],
        mask: VR,
        value: 0,
        test: |w| rn(w) != 31 && (rt(w) == rn(w) || rt2(w) == rn(w)),
        reason: "pair writeback with Rt or Rt2 == Rn (CONSTRAINED UNPREDICTABLE)",
    },
    // LDR/STR (immediate) pre/post-index: `if wback && n == t && n != 31`.
    Constraint {
        iclass: &["ldst_immpre", "ldst_immpost"],
        mask: VR,
        value: 0,
        test: |w| rn(w) != 31 && rt(w) == rn(w),
        reason: "single-register writeback with Rt == Rn (CONSTRAINED UNPREDICTABLE)",
    },
    // LDRAA/LDRAB with writeback (W, bit 11): `if wback && n == t && n != 31`.
    Constraint {
        iclass: &["ldst_pac"],
        mask: 1 << 11,
        value: 1 << 11,
        test: |w| rn(w) != 31 && rt(w) == rn(w),
        reason: "LDRAA/LDRAB writeback with Rt == Rn (CONSTRAINED UNPREDICTABLE)",
    },
    // STXR/STLXR (and byte, halfword): `if s == t || (s == n && n != 31)`.
    // Store exclusives have L (bit 22) clear.
    Constraint {
        iclass: &["ldstexclr", "ldstexclr_unpriv"],
        mask: L,
        value: 0,
        test: |w| rs(w) == rt(w) || (rs(w) == rn(w) && rn(w) != 31),
        reason: "store exclusive with Rs == Rt or Rs == Rn (CONSTRAINED UNPREDICTABLE)",
    },
    // STXP/STLXP: `if s == t || s == t2 || (s == n && n != 31)`.
    Constraint {
        iclass: &["ldstexclp"],
        mask: L,
        value: 0,
        test: |w| rs(w) == rt(w) || rs(w) == rt2(w) || (rs(w) == rn(w) && rn(w) != 31),
        reason: "store exclusive pair with Rs == Rt, Rt2 or Rn (CONSTRAINED UNPREDICTABLE)",
    },
    // LDXP/LDAXP: `if t == t2`.
    Constraint {
        iclass: &["ldstexclp"],
        mask: L,
        value: L,
        test: |w| rt(w) == rt2(w),
        reason: "load exclusive pair with Rt == Rt2 (CONSTRAINED UNPREDICTABLE)",
    },
    // CPY*: `if d == s || s == n || d == n || d == 31 || s == 31 || n == 31`
    // (Rd destination, Rs source, Rn size; the registers must be distinct and not
    // 31, Arm ARM C6.2 CPYFP and K1.2.9).
    Constraint {
        iclass: &["memcms"],
        mask: 0,
        value: 0,
        test: |w| {
            let (d, s, n) = (rd(w), rs(w), rn(w));
            !is_set(w) && (d == s || s == n || d == n || d == 31 || s == 31 || n == 31)
        },
        reason: "CPY* with overlapping or 31 registers (CONSTRAINED UNPREDICTABLE)",
    },
    // MSR (immediate) to a 1-bit PSTATE field (UAO, PAN, SSBS, DIT, TCO): the
    // immediate is CRm, and only #0 and #1 are defined (C6.2 MSR (immediate),
    // `<imm>`). Hardware (Cortex-A78C) takes such words as UNDEFINED while the
    // ASL-derived Sail model executes them with CRm<0>, so no result is architectural.
    Constraint {
        iclass: &["pstate"],
        mask: 0,
        value: 0,
        test: |w| {
            let (op1, crm, op2) = (f(w, 16, 3), f(w, 8, 4), f(w, 5, 3));
            matches!((op1, op2), (0, 3) | (0, 4) | (3, 1) | (3, 2) | (3, 4)) && crm >> 1 != 0
        },
        reason: "MSR (immediate) to a 1-bit PSTATE field with an immediate above 1",
    },
    // SET*: `if d == n || d == s || s == n || d == 31 || n == 31` (Rd destination,
    // Rn size, Rs data; Rs may be XZR).
    Constraint {
        iclass: &["memcms", "memset_go"],
        mask: SET,
        value: SET,
        test: |w| {
            let (d, s, n) = (rd(w), rs(w), rn(w));
            d == n || d == s || s == n || d == 31 || n == 31
        },
        reason: "SET* with overlapping or 31 registers (CONSTRAINED UNPREDICTABLE)",
    },
];

/// Whether a `memcms` word is a SET* (`op1`, bits 23:22, is 11) rather than a CPY*:
/// the two families share the iclass.
fn is_set(w: u32) -> bool {
    w & SET == SET
}

pub(crate) fn unpredictable(insn: &Instruction) -> Option<&'static str> {
    let d = insn.op_code().data()?;
    if insn.sb != d.sb_value & d.sb_mask {
        return Some("should-be bits not at their stated value");
    }
    let w = insn.word;
    TABLE
        .iter()
        .find(|c| c.iclass.contains(&d.iclass) && w & c.mask == c.value && (c.test)(w))
        .map(|c| c.reason)
}
