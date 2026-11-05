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
use common::{GROUPS, Rng};

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
