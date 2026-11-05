//! Golden tests: formatter output for representative encodings of every iclass,
//! `min_el()` for the system registers and operations accessible at EL0, the
//! hint-space fallback with the Dev Kit's feature set, and the constraints table.
//!
//! The snapshot files under `tests/golden/` are reviewed like code; regenerate them
//! with `AARCH64_ISA_BLESS=1 cargo test -p aarch64-isa --test golden` and review the
//! diff.

mod common;

use std::collections::BTreeSet;
use std::fmt::Write as _;

use aarch64_isa::{
    Code, DecoderOptions, Feature, FeatureSet, SysKind, decode_word, system::entries,
};
use common::{GROUPS, Rng, SVE_GROUPS};

fn snapshot(name: &str, actual: &str) {
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var("AARCH64_ISA_BLESS").is_ok() {
        std::fs::create_dir_all(format!("{}/tests/golden", env!("CARGO_MANIFEST_DIR"))).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if expected != actual {
        let first = expected
            .lines()
            .zip(actual.lines())
            .find(|(a, b)| a != b)
            .map(|(a, b)| format!("expected {a:?}\n  actual {b:?}"))
            .unwrap_or_else(|| "length differs".into());
        panic!("{name} differs from the snapshot (AARCH64_ISA_BLESS=1 to update):\n{first}");
    }
}

#[test]
fn formatter_per_iclass() {
    let mut rng = Rng::new(7);
    let mut seen = BTreeSet::new();
    let mut out = String::new();
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        if !GROUPS.contains(&oc.group()) || !oc.has_operands() || !seen.insert(oc.iclass()) {
            continue;
        }
        for w in [oc.value(), oc.value() | (rng.next() & !oc.mask())] {
            let i = decode_word(w, 0x1000, &DecoderOptions::default());
            let _ = writeln!(out, "{w:08x} {:<28} {}", i.op_code().name(), i);
        }
    }
    snapshot("format.txt", &out);
}

/// The same for the SVE and SME encodings with operand support (a snapshot of its
/// own, so the base one is unchanged as encodings gain operands).
#[test]
fn formatter_per_iclass_sve_sme() {
    let mut rng = Rng::new(11);
    let mut seen = BTreeSet::new();
    let mut out = String::new();
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        if !SVE_GROUPS.contains(&oc.group()) || !oc.has_operands() || !seen.insert(oc.iclass()) {
            continue;
        }
        for w in [oc.value(), oc.value() | (rng.next() & !oc.mask())] {
            let i = decode_word(w, 0x1000, &DecoderOptions::default());
            let _ = writeln!(out, "{w:08x} {:<28} {}", i.op_code().name(), i);
        }
    }
    snapshot("format_sve.txt", &out);
}

#[test]
fn el0_system_registers_and_operations() {
    let mut out = String::new();
    for e in entries().iter().filter(|e| e.min_el == 0) {
        let _ = writeln!(out, "{:?} {} {} {:#06x}", e.kind, e.insn, e.name, e.value);
    }
    snapshot("el0.txt", &out);
}

#[test]
fn min_el_per_instance() {
    let el = |w: u32| decode_word(w, 0, &DecoderOptions::default()).min_el();
    // MRS X0, NZCV / FPCR / FPSR / TPIDR_EL0 / CNTVCT_EL0 / CTR_EL0.
    assert_eq!(el(0xd53b_4200), 0);
    assert_eq!(el(0xd53b_4400), 0);
    assert_eq!(el(0xd53b_4420), 0);
    assert_eq!(el(0xd53b_d040), 0);
    assert_eq!(el(0xd53b_e040), 0);
    assert_eq!(el(0xd53b_0020), 0);
    // MSR NZCV, X0; MSR FPCR, X0.
    assert_eq!(el(0xd51b_4200), 0);
    assert_eq!(el(0xd51b_4400), 0);
    // MRS X0, MIDR_EL1 / SCTLR_EL1; MSR SCTLR_EL1, X0.
    assert_eq!(el(0xd538_0000), 1);
    assert_eq!(el(0xd538_1000), 1);
    assert_eq!(el(0xd518_1000), 1);
    // DC ZVA / CVAU / CIVAC and IC IVAU are EL0 when enabled; DC IVAC is not.
    assert_eq!(el(0xd50b_7420), 0);
    assert_eq!(el(0xd50b_7b20), 0);
    assert_eq!(el(0xd50b_7e20), 0);
    assert_eq!(el(0xd50b_7520), 0);
    assert_eq!(el(0xd508_7620), 1);
    // TLBI VMALLE1; AT S1E1R, X0.
    assert_eq!(el(0xd508_871f), 1);
    assert_eq!(el(0xd508_7800), 1);
    // ERET, HVC, SMC; SVC is EL0.
    assert_eq!(el(0xd69f_03e0), 1);
    assert_eq!(el(0xd400_0002), 1);
    assert_eq!(el(0xd400_0003), 1);
    assert_eq!(el(0xd400_0001), 0);
    // MSR SPSel, #1 and MSR PAN, #1 are EL1; MSR DIT, #1 is EL0.
    assert_eq!(el(0xd500_41bf), 1);
    assert_eq!(el(0xd500_419f), 1);
    assert_eq!(el(0xd503_415f), 0);
    // Ordinary instructions.
    assert_eq!(el(0x8b02_0020), 0);
    assert_eq!(el(0xf940_0420), 0);
}

/// The Windows Dev Kit 2023 (Cortex-X1/A78): Armv8.2 with DotProd, FP16, LSE and
/// RCpc, and neither PAuth nor BTI.
fn devkit() -> FeatureSet {
    let mut s = FeatureSet::ALL;
    for f in Feature::values() {
        if matches!(
            f.name(),
            "FEAT_PAuth" | "FEAT_BTI" | "FEAT_PAuth_LR" | "FEAT_CHK" | "FEAT_GCS"
        ) {
            s.remove(f);
        }
    }
    s
}

#[test]
fn missing_hint_features_decode_as_hint() {
    let opts = DecoderOptions::with_features(devkit());
    for (w, name) in [
        (0xd503_233fu32, "PACIASP"),
        (0xd503_23bf, "AUTIASP"),
        (0xd503_20ff, "XPACLRI"),
        (0xd503_245f, "BTI c"),
        (0xd503_241f, "BTI"),
        (0xd503_251f, "CHKFEAT"),
    ] {
        let i = decode_word(w, 0, &opts);
        assert_eq!(
            i.code(),
            Code::Hint_HM_hints,
            "{name} ({w:08x}) is {:?}",
            i.code()
        );
        assert!(i.to_string().starts_with("hint #"), "{name}: {i}");
        // With every feature it is the named instruction.
        let all = decode_word(w, 0, &DecoderOptions::default());
        assert_ne!(all.code(), Code::Hint_HM_hints, "{name}");
    }
    // A missing feature outside the hint space: PACIA X0, X1 is unallocated.
    let i = decode_word(0xdac1_0020, 0, &opts);
    assert!(i.is_invalid());
    // NOP is NOP either way.
    assert_eq!(
        decode_word(0xd503_201f, 0, &opts).code(),
        Code::Nop_HI_hints
    );
}

#[test]
fn constraints_table() {
    let why = |w: u32| decode_word(w, 0, &DecoderOptions::default()).unpredictable_reason();
    // LDP X0, X0, [X1]
    assert!(why(0xa940_0020).is_some());
    // LDP X0, X2, [X1] is fine; STP X0, X0, [X1] is fine (stores may repeat).
    assert!(why(0xa940_0820).is_none());
    assert!(why(0xa900_0020).is_none());
    // LDR X1, [X1], #8 (post-index writeback with Rt == Rn).
    assert!(why(0xf840_8421).is_some());
    // LDR X0, [X1], #8
    assert!(why(0xf840_8420).is_none());
    // LDR X1, [SP], #8: Rn = 31 is SP, no overlap.
    assert!(why(0xf840_87e1).is_none());
    // STP X1, X2, [X1, #16]!
    assert!(why(0xa981_0821).is_some());
    // STXR W0, X0, [X1]
    assert!(why(0xc800_7c20).is_some());
    // LDAR X0, [X1] with a should-be bit cleared (Rs = 11110).
    let ldar = 0xc8df_fc20u32;
    assert!(why(ldar).is_none());
    assert!(why(ldar & !(1 << 16)).is_some());
    // CPYP [X0]!, [X1]!, X0!
    assert!(why(0x1d00_0420).is_some());
    // CPYP [X0]!, [X1]!, X2!
    assert!(why(0x1d02_0420).is_none());
}

#[test]
fn hint_feature_lookup() {
    assert_eq!(
        Feature::from_name("FEAT_LSE").map(|f| f.name()),
        Some("FEAT_LSE")
    );
    assert_eq!(
        Code::from_name("ADD_64_addsub_shift"),
        Some(Code::Add_64_addsub_shift)
    );
    assert_eq!(
        Code::Add_64_addsub_shift.op_code().name(),
        "ADD_64_addsub_shift"
    );
    let _ = SysKind::Mrs;
}

/// `CFINV`, `XAFLAG` and `AXFLAG` are in the `pstate` iclass with `MSR (immediate)`
/// but name no PSTATE field: they are ordinary EL0 instructions.
#[test]
fn flag_manipulation_is_el0() {
    let el = |w: u32| decode_word(w, 0, &DecoderOptions::default()).min_el();
    // CFINV, XAFLAG, AXFLAG.
    assert_eq!(el(0xd500_401f), 0);
    assert_eq!(el(0xd500_403f), 0);
    assert_eq!(el(0xd500_405f), 0);
    // MSR SPSel, #1 is still EL1.
    assert_eq!(el(0xd500_41bf), 1);
}

/// Instances found by recording the full coverage set on a Cortex-A78C and replaying
/// it against the Sail model (Phase 4), and the table entries that now cover them.
#[test]
fn constraints_and_determinism_from_the_hardware_replay() {
    let i = |w: u32| decode_word(w, 0, &DecoderOptions::default());
    // MSR SSBS, #0xb and MSR DIT, #7: only #0 and #1 are defined for 1-bit fields.
    assert!(i(0xd503_4b3f).unpredictable_reason().is_some());
    assert!(i(0xd503_475f).unpredictable_reason().is_some());
    // MSR SSBS, #1, MSR DIT, #0 and MSR DAIFSet, #3 (a 4-bit field) are fine.
    assert!(i(0xd503_413f).unpredictable_reason().is_none());
    assert!(i(0xd503_405f).unpredictable_reason().is_none());
    assert!(i(0xd503_43df).unpredictable_reason().is_none());
    // CFINV shares the iclass and is fine.
    assert!(i(0xd500_401f).unpredictable_reason().is_none());
    // MRS X0, S3_7_C15_C9_0 and SYS #6, C11, C15, #4: the IMPLEMENTATION DEFINED
    // register and operation space.
    assert_eq!(
        i(0xd53f_f900).nondeterminism_reason(),
        Some("IMPLEMENTATION DEFINED system register or operation")
    );
    assert!(i(0xd50e_bf9f).nondeterminism_reason().is_some());
    // MRS X9, SPMEVCNTR7_EL0: a counter.
    assert!(i(0xd533_e0e9).nondeterminism_reason().is_some());
    // MRS X14, PMSELR_EL0: performance monitor state set by the OS.
    assert!(i(0xd53b_9cae).nondeterminism_reason().is_some());
    // MRS X0, FPCR stays deterministic.
    assert!(i(0xd53b_4400).nondeterminism_reason().is_none());
}

/// The four encodings and alias outside SVE/SME that had no operand support:
/// `TEXIT {NB}`, `CTERMEQ`/`CTERMNE` (`<R>` from `sz`), `GCSPOPM {<Xt>}` and the
/// `GICR` alias of `SYSL` (its condition names `Sysl_GICR`). Text checked against
/// the Arm ARM syntax (and fARM64, which agrees); each word re-encodes bit-exactly
/// and from its operands alone.
#[test]
fn stragglers() {
    for (w, text, min_el) in [
        (0xd6ff_03e0u32, "texit", 0),
        (0xd6ff_07e0, "texit nb", 0),
        (0x25a0_2000, "ctermeq w0, w0", 0),
        (0x25e3_2040, "ctermeq x2, x3", 0),
        (0x25e3_2050, "ctermne x2, x3", 0),
        (0x25bf_23f0, "ctermne wzr, wzr", 0),
        (0xd52b_773f, "gcspopm", 0),
        (0xd52b_7725, "gcspopm x5", 0),
        (0xd52b_7765, "gcsss2 x5", 0),
        (0xd528_c300, "gicr x0, cdia", 1),
        (0xd528_c325, "gicr x5, cdnmia", 1),
        // SYSL with an operation no alias names stays SYSL (an unknown operation
        // outside the `S1_*` space: EL1).
        (0xd528_c345, "sysl x5, #0, c12, c3, #2", 1),
    ] {
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        assert!(i.has_operands(), "{w:08x}");
        assert_eq!(i.to_string(), text, "{w:08x}");
        assert_eq!(i.min_el(), min_el, "{w:08x} {text}");
        assert_eq!(i.encode(), Ok(w), "{w:08x} {text}");
        let built = aarch64_isa::Instruction::with(i.code(), i.operands(), 0x1000).unwrap();
        assert_eq!(built.encode(), Ok(w), "{w:08x} {text} from operands");
    }
}

/// SVE data processing, one word per operand family (text per the Arm ARM syntax;
/// fARM64 agrees except where `tests/triage/farm64.txt` says otherwise).
#[test]
fn sve_data_processing() {
    for (w, text) in [
        // Predicated, merging and zeroing; destructive `<Zdn>`.
        (0x0400_0597u32, "add z23.b, p1/m, z23.b, z12.b"),
        (0x04d0_3ad5, "movprfx z21.d, p6/z, z22.d"),
        // `<Pg>/<ZM>` from the `M` bit (CPY, SEL as MOV).
        (0x057e_c5de, "mov z30.h, p1/m, z14.h"),
        // Unpredicated, wide elements, `<Tb>` from the same `size`.
        (0x0418_8d24, "asr z4.b, p3/m, z4.b, z9.d"),
        (0x45d4_43e6, "saddwb z6.d, z31.d, z20.s"),
        (0x45cc_0044, "saddlb z4.d, z2.s, z12.s"),
        // Element size and shift from `tsz:imm3`; narrowing (exact one-hot `tsz`).
        (0x04b0_934f, "asr z15.d, z26.d, #0x30"),
        (0x4574_022e, "sqshrunb z14.s, z17.d, #0xc"),
        (0x4560_410a, "sqxtnb z10.s, z8.d"),
        // Indexed elements: `i2`, `i3h:i3l`, and `imm2:tsz` (DUP as MOV).
        (0x44b0_028b, "sdot z11.s, z20.b, z0.b[2]"),
        (0x4467_0874, "mla z20.h, z3.h, z7.h[4]"),
        (0x05fe_220d, "mov z13.h, z16.h[0x1f]"),
        // Immediates: signed and unsigned, `LSL #8`, logical, FP.
        (0x25d9_094c, "cmpge p12.d, p2/z, z10.d, #-7"),
        (0x24b5_8e0e, "cmphs p14.s, p3/z, z16.s, #0x56"),
        (0x2568_d460, "smax z0.h, z0.h, #-0x5d"),
        (0x2563_e0cb, "subr z11.h, z11.h, #6, lsl #8"),
        (0x0428_5741, "addvl x1, x8, #-6"),
        (0x0502_40ad, "orr z13.d, z13.d, #0x3f00000000000000"),
        (0x05c0_0000, "dupm z0.s, #0x1"),
        (0x05c2_a6f6, "mov z22.d, #0xfffff00fffffffff"),
        (0x0554_d205, "fmov z5.h, p4/m, #-4.0"),
        (0x6558_8c1c, "fadd z28.h, p3/m, z28.h, #0.5"),
        (0x6550_2222, "fcmge p2.h, p0/z, z17.h, #0.0"),
        // Rotations.
        (0x44a3_6dc4, "cmla z4.h, z14.h, z3.h[0], #0x10e"),
        // Predicates, scalars and vectors of SIMD&FP registers.
        (0x2508_68cc, "and p12.b, p10/z, p6.b, p8.b"),
        (0x2558_c10a, "pfirst p10.b, p8, p10.b"),
        (0x2560_9da7, "cntp x7, p7, p13.h"),
        (0x25e8_80e5, "sqincp z5.d, p7.d"),
        (0x2520_7810, "ptrue pn8.b"),
        (0x04c5_29d7, "addqv v23.2d, p2, z14.d"),
        (0x65d8_32b6, "fadda d22, p4, d22, z21.d"),
        (0x0574_3952, "insr z18.h, h10"),
    ] {
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        assert_eq!(i.to_string(), text, "{w:08x}");
        assert_eq!(i.encode(), Ok(w), "{w:08x} {text}");
    }
    // RESERVED sizes the decode pseudocode rejects: a byte immediate shifted by 8,
    // SXTW of a non-doubleword, HISTSEG of a non-byte, SADDV of doublewords.
    for w in [0x2520_e06fu32, 0x0404_a000, 0x45e9_a2ac, 0x04c0_2000] {
        assert!(
            decode_word(w, 0, &DecoderOptions::default()).is_invalid(),
            "{w:08x}"
        );
    }
}

/// SVE patterns, counts, prefetches and memory operands (text per the Arm ARM
/// syntax; fARM64 agrees except where `tests/triage/farm64.txt` says otherwise).
#[test]
fn sve_patterns_and_memory() {
    for (w, text) in [
        // `{, <pattern>{, MUL #<imm>}}`: left out at ALL and 1; unnamed patterns
        // are immediates.
        (0x0420_e3e0u32, "cntb x0"),
        (0x0420_e000, "cntb x0, pow2"),
        (0x0431_e063, "incb x3, vl3, mul #2"),
        (0x042e_e20d, "cntb x13, #0x10, mul #0xf"),
        (0x042a_f281, "sqincb x1, w1, #0x14, mul #0xb"),
        (0x047e_c332, "inch z18.h, #0x19, mul #0xf"),
        (0x2598_e3e0, "ptrue p0.s"),
        (0x2558_e060, "ptrue p0.h, vl3"),
        // Prefetch operations: named, or an immediate.
        (0x8468_192c, "prfb pstl3keep, p6, [x9, z8.s, sxtw]"),
        (0x85d5_1c4f, "prfb #0xf, p7, [x2, #0x15, mul vl]"),
        // Contiguous: `MUL VL`, scalar index with LSL, multiples of a list's length.
        (0xa540_a000, "ld1w { z0.s }, p0/z, [x0]"),
        (0xa40e_a4ae, "ld1b { z14.b }, p1/z, [x5, #-2, mul vl]"),
        (0xa50b_8095, "ld1w { z21.q }, p0/z, [x4, x11, lsl #2]"),
        (
            0xa424_e5d9,
            "ld2b { z25.b, z26.b }, p1/z, [x14, #8, mul vl]",
        ),
        (0xa560_f67a, "ld4w { z26.s-z29.s }, p5/z, [x19]"),
        (
            0xe57a_fd9d,
            "st4w { z29.s, z30.s, z31.s, z0.s }, p7, [x12, #-0x18, mul vl]",
        ),
        // Gather and scatter: vector index with extend and amount, vector base.
        (0x8531_48d5, "ld1w { z21.s }, p2/z, [x6, z17.s, uxtw #2]"),
        (0xc5e0_c000, "ld1d { z0.d }, p0/z, [x0, z0.d, lsl #3]"),
        (0x8401_e2c0, "prfb pldl1keep, p0, [z22.s, #1]"),
        // The optional index, left out when it is XZR.
        (0xa41f_7fff, "ldff1b { z31.b }, p7/z, [sp]"),
        (0xc59f_d956, "ldnt1d { z22.d }, p6/z, [z10.d]"),
        // Replicating loads: scaled by the access size, by 16 (`LD1RQ`), by 32.
        (0xa403_20b3, "ld1rqb { z19.b }, p0/z, [x5, #0x30]"),
        (0xa42f_36a4, "ld1rob { z4.b }, p5/z, [x21, #-0x20]"),
        // Whole registers.
        (0x85b2_4bec, "ldr z12, [sp, #-0x6e, mul vl]"),
        (0xe5a9_1860, "str p0, [x3, #-0xb2, mul vl]"),
        // SME2 multi-vector and strided lists with a predicate-as-counter.
        (
            0xa00f_b010,
            "ld1h { z16.h-z19.h }, pn12/z, [x0, x15, lsl #1]",
        ),
        (
            0xa147_37c1,
            "ld1h { z1.h, z9.h }, pn13/z, [x30, #0xe, mul vl]",
        ),
        // ADR: vector base and index.
        (0x043e_abb6, "adr z22.d, [z29.d, z30.d, sxtw #2]"),
    ] {
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        assert_eq!(i.to_string(), text, "{w:08x}");
        assert_eq!(i.encode(), Ok(w), "{w:08x} {text}");
    }
}

/// SVE2, SVE2.1 and the multi-vector forms: lists of two (element by element) and
/// four (ranges), strided lists, predicate pairs and predicate-as-counter
/// registers, indexed lists, and `PSEL`'s register-indexed predicate.
#[test]
fn sve2_and_multi_vector() {
    for (w, text) in [
        (0x2571_5162u32, "psel p2, p4, p11.s[w13, 1]"),
        (0x45fc_ae8a, "luti6 z10.h, { z20.h, z21.h }, z28[1]"),
        (0x453a_e886, "aese { z6.b, z7.b }, { z6.b, z7.b }, z4.q[3]"),
        (0x4526_e888, "aese { z8.b-z11.b }, { z8.b-z11.b }, z4.q[0]"),
        (
            0xc179_9498,
            "sel { z24.h-z27.h }, pn13, { z4.h-z7.h }, { z24.h-z27.h }",
        ),
        (0xc1ac_ab14, "add { z20.s-z23.s }, { z20.s-z23.s }, z12.s"),
        (0xc1e6_d98a, "sqrshr z10.h, { z12.d-z15.d }, #0x1a"),
        (0x0575_1ab3, "ext z19.b, { z21.b, z22.b }, #0xae"),
        (0x257b_5010, "whilege { p0.h, p1.h }, x0, x27"),
        (0x25e0_87fa, "cntp x26, pn15.d, vlx4"),
        (
            0xa111_9342,
            "ld1b { z2.b, z6.b, z10.b, z14.b }, pn12/z, [x26, x17]",
        ),
        (0xa108_1281, "ld1b { z1.b, z9.b }, pn12/z, [x20, x8]"),
    ] {
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        assert_eq!(i.to_string(), text, "{w:08x}");
        assert_eq!(i.encode(), Ok(w), "{w:08x} {text}");
    }
}

/// SME: tiles, horizontal and vertical slices (single and ranges), array vectors
/// with vector groups, `ZT0` forms, the `ZERO` mask and braced single operands (as
/// the templates write them: no `#` in ZA indices).
#[test]
fn sme_operands() {
    for (w, text) in [
        (0x8080_a640u32, "fmopa za0.s, p1/m, p5/m, z18.s, z0.s"),
        (0xc090_00a2, "addha za2.s, p0/m, p0/m, z5.s"),
        (0xc0c3_81c8, "mov z8.q, p0/m, za14v.q[w12, 0]"),
        (0xc000_0f27, "mov za0h.b[w12, 7], p3/m, z25.b"),
        (0xc004_0400, "mov za0h.b[w12, 0:3], { z0.b-z3.b }"),
        (0xc006_0a00, "movaz { z0.d, z1.d }, za.d[w8, 0, vgx2]"),
        (
            0xe0c2_ba45,
            "ld1d { za2v.d[w13, 1] }, p6/z, [x18, x2, lsl #3]",
        ),
        (
            0xe1d9_e183,
            "ld1q { za3v.q[w15, 0] }, p0/z, [x12, x25, lsl #4]",
        ),
        (0xe100_6103, "ldr za[w15, 3], [x8, #3, mul vl]"),
        (
            0xc15a_2d07,
            "fmla za.s[w9, 7, vgx2], { z8.s, z9.s }, z10.s[3]",
        ),
        (
            0xc195_5a97,
            "bfmlal za.s[w10, 6:7, vgx2], { z20.h, z21.h }, z5.h[5]",
        ),
        (
            0xc117_0141,
            "smlall za.s[w8, 4:7, vgx2], { z10.b, z11.b }, z7.b[0]",
        ),
        (0xc00c_0000, "zero za.d[w8, 0, vgx2]"),
        (0xc008_0000, "zero { }"),
        (0xc008_00ff, "zero { za }"),
        (0xc008_00c6, "zero { za2.s, za1.d, za7.d }"),
        (0xc048_0001, "zero { zt0 }"),
        (0xc04c_73ff, "movt xzr, zt0[0x38]"),
        (0xc04f_13e8, "movt zt0[1, mul vl], z8"),
        (0xe11f_80c0, "ldr zt0, [x6]"),
        (0xc0cc_e2aa, "luti2 z10.s, zt0, z21[3]"),
        // Quarter-tile outer products: Zn in Z0-Z14, Zm in Z16-Z30; K:Zk in
        // Z20-Z23 and Z28-Z31.
        (0xa0ce_001b, "smop4s za3.d, z0.h, z30.h"),
        (0x8053_979b, "stmopa za3.s, { z28.h, z29.h }, z19.h, z29[1]"),
    ] {
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        assert_eq!(i.to_string(), text, "{w:08x}");
        assert_eq!(i.encode(), Ok(w), "{w:08x} {text}");
    }
    // A ZT0 byte offset is a multiple of 8.
    let i = decode_word(0xc04c_73ff, 0, &DecoderOptions::default());
    assert_eq!(i.op(1).index(), Some(56));
}
