//! The hand-maintained determinism table: instances whose results can differ
//! between runs, cores or machines on the same input, so a recording cannot be the
//! expected result for an emulator. The spec does not say this; each entry gives
//! the reason. It covers at least the instructions listed in docs/DESIGN.md
//! ("Constraints and determinism"), so a test generator can exclude every instance
//! it flags.

use crate::generated::Mnemonic;
use crate::info::sys_entry;
use crate::instruction::Instruction;
use crate::sysreg::SysKind;

/// System registers whose value is not a function of the test's inputs.
fn sysreg_reason(name: &str) -> Option<&'static str> {
    Some(match name {
        "RNDR" | "RNDRRS" => "random number",
        "CNTVCT_EL0" | "CNTPCT_EL0" | "CNTVCTSS_EL0" | "CNTPCTSS_EL0" | "CNTFRQ_EL0" => {
            "system counter (time-dependent or machine-dependent)"
        }
        "PMCCNTR_EL0" | "PMXEVCNTR_EL0" => "performance counter",
        "MIDR_EL1" | "MPIDR_EL1" | "REVIDR_EL1" | "AIDR_EL1" | "CTR_EL0" | "DCZID_EL0"
        | "CLIDR_EL1" | "CCSIDR_EL1" | "CCSIDR2_EL1" | "CSSELR_EL1" | "GMID_EL1" | "SMIDR_EL1" => {
            "identification register (implementation-dependent)"
        }
        "TPIDR_EL0" | "TPIDRRO_EL0" | "TPIDR2_EL0" => "thread pointer set by the OS",
        _ if name.starts_with("ID_") => "identification register (implementation-dependent)",
        _ if name.starts_with("PMEVCNTR") || name.starts_with("SPMEVCNTR") => "performance counter",
        "PMICNTR_EL0" | "SPMCCNTR_EL0" => "performance counter",
        // PMSELR_EL0, PMCR_EL0, PMCNTENSET_EL0, ...: configured by the OS (Windows
        // allows EL0 reads; the values are not test inputs).
        _ if name.starts_with("PM") && name.ends_with("_EL0") => {
            "performance monitor state (configured by the OS)"
        }
        _ if name.starts_with("AMEVCNTR") => "activity monitor counter",
        _ => return None,
    })
}

/// The `S3_<op1>_C<Cn>_C<Cm>_<op2>` and `S1_<op1>_<Cn>_<Cm>_<op2>` accessors: the
/// IMPLEMENTATION DEFINED system register and operation space, whose behaviour is
/// the implementation's own (an emulator cannot be expected to match it).
fn is_implementation_defined(name: &str) -> bool {
    name.contains('<')
}

pub(crate) fn reason(insn: &Instruction) -> Option<&'static str> {
    use Mnemonic as M;
    if insn.is_invalid() {
        return None;
    }
    let m = insn.mnemonic();
    match m {
        // Store exclusives may fail spuriously (Arm ARM B2.9: the exclusive monitor
        // may be cleared at any time), so the status result is not determined.
        M::Stxr
        | M::Stlxr
        | M::Stxrb
        | M::Stlxrb
        | M::Stxrh
        | M::Stlxrh
        | M::Stxp
        | M::Stlxp
        | M::Sttxr
        | M::Stltxr => return Some("exclusive store can fail spuriously"),
        _ => {}
    }
    if let Some(e) = sys_entry(insn) {
        if is_implementation_defined(e.name) {
            return Some("IMPLEMENTATION DEFINED system register or operation");
        }
        match e.kind {
            SysKind::Mrs | SysKind::Mrrs => {
                if let Some(r) = sysreg_reason(e.name) {
                    return Some(r);
                }
            }
            // DC ZVA/GVA/GZVA write a block whose size is IMPLEMENTATION DEFINED
            // (DCZID_EL0).
            SysKind::Sys if e.insn == "DC" && matches!(e.name, "ZVA" | "GVA" | "GZVA") => {
                return Some("DC ZVA block size is implementation-dependent");
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::{DecoderOptions, Determinism, decode_word};

    fn det(w: u32) -> Determinism {
        decode_word(w, 0, &DecoderOptions::default()).determinism()
    }

    #[test]
    fn table() {
        // STXR W2, X0, [X1]
        assert_eq!(det(0xc802_7c20), Determinism::Nondeterministic);
        // MRS X0, RNDR
        assert_eq!(det(0xd53b_2400), Determinism::Nondeterministic);
        // MRS X0, CNTVCT_EL0
        assert_eq!(det(0xd53b_e040), Determinism::Nondeterministic);
        // MRS X0, MIDR_EL1
        assert_eq!(det(0xd538_0000), Determinism::Nondeterministic);
        // MRS X0, CTR_EL0
        assert_eq!(det(0xd53b_0020), Determinism::Nondeterministic);
        // DC ZVA, X0
        assert_eq!(det(0xd50b_7420), Determinism::Nondeterministic);
        // MRS X0, NZCV; ADD X0, X1, X2; LDXR X0, [X1]
        assert_eq!(det(0xd53b_4200), Determinism::Deterministic);
        assert_eq!(det(0x8b02_0020), Determinism::Deterministic);
        assert_eq!(det(0xc85f_7c20), Determinism::Deterministic);
    }
}
