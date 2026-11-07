//! Flow control, privilege and the system-register lookup for an instance.

use crate::generated::info::ICLASSES;
use crate::generated::{Code, Mnemonic};
use crate::instruction::{FlowControl, Instruction};
use crate::sysreg::{SysEntry, SysKind, sys_encoding};

/// The iclass of an encoding, as far as this module needs it, from a table the
/// generator writes (`gen/src/info.rs`) so that no instance compares strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Iclass {
    Other,
    /// `systemmove`: `MRS` or `MSR` by `L` (bit 21); likewise below.
    SystemMove,
    /// `systemmovepr`: `MRRS` or `MSRR`.
    SystemMovePair,
    /// `systeminstrs`: `SYSL` or `SYS`.
    SystemInstrs,
    /// `syspairinstrs`: `SYSP`.
    SysPair,
    /// `MSR (immediate)`. (`CFINV`, `XAFLAG` and `AXFLAG` share the `pstate`
    /// iclass but name no PSTATE field.)
    MsrImm,
    /// `compbranch*`: compare and branch.
    CompBranch,
}

/// The system-register access kind of an encoding, by iclass.
pub(crate) fn sys_kind(code: Code, word: u32) -> Option<SysKind> {
    let l = word >> 21 & 1 == 1;
    Some(match ICLASSES[code as usize] {
        Iclass::SystemMove => {
            if l {
                SysKind::Mrs
            } else {
                SysKind::Msr
            }
        }
        Iclass::SystemMovePair => {
            if l {
                SysKind::Mrrs
            } else {
                SysKind::Msrr
            }
        }
        Iclass::SystemInstrs => {
            if l {
                SysKind::Sysl
            } else {
                SysKind::Sys
            }
        }
        Iclass::SysPair => SysKind::Sysp,
        Iclass::MsrImm => SysKind::MsrImm,
        Iclass::Other | Iclass::CompBranch => return None,
    })
}

/// `op0:op1:CRn:CRm:op2` of a system instruction word.
pub(crate) fn sys_fields(word: u32) -> u16 {
    sys_encoding(
        word >> 19 & 3,
        word >> 16 & 7,
        word >> 12 & 15,
        word >> 8 & 15,
        word >> 5 & 7,
    )
}

/// The system register or operation the instance names, if any.
pub(crate) fn sys_entry(insn: &Instruction) -> Option<&'static SysEntry> {
    let kind = sys_kind(insn.code, insn.word)?;
    crate::sysreg::lookup_insn(kind, sys_fields(insn.word))
}

/// Encodings that are UNDEFINED at EL0 whatever their operands.
fn encoding_min_el(m: Mnemonic) -> u8 {
    match m {
        Mnemonic::Eret
        | Mnemonic::Eretaa
        | Mnemonic::Eretab
        | Mnemonic::Drps
        | Mnemonic::Hvc
        | Mnemonic::Smc
        | Mnemonic::Dcps1
        | Mnemonic::Dcps2
        | Mnemonic::Dcps3 => 1,
        _ => 0,
    }
}

pub(crate) fn min_el(insn: &Instruction) -> u8 {
    if insn.is_invalid() {
        return 0;
    }
    if sys_kind(insn.code, insn.word).is_some() {
        // An unknown (IMPLEMENTATION DEFINED) register or operation: assume EL1.
        return sys_entry(insn).map_or(1, |e| e.min_el);
    }
    encoding_min_el(insn.mnemonic())
}

pub(crate) fn flow_control(insn: &Instruction) -> FlowControl {
    use Mnemonic as M;
    match insn.mnemonic() {
        M::B | M::Bc => {
            // `B.cond` and `BC.cond` share the mnemonic with a condition operand.
            if insn.condition_code().is_some() {
                FlowControl::ConditionalBranch
            } else {
                FlowControl::UnconditionalBranch
            }
        }
        M::Cbz | M::Cbnz | M::Tbz | M::Tbnz => FlowControl::ConditionalBranch,
        _ if ICLASSES[insn.code as usize] == Iclass::CompBranch => FlowControl::ConditionalBranch,
        M::Bl => FlowControl::Call,
        M::Blr | M::Blraa | M::Blraaz | M::Blrab | M::Blrabz => FlowControl::IndirectCall,
        M::Br | M::Braa | M::Braaz | M::Brab | M::Brabz => FlowControl::IndirectBranch,
        M::Ret
        | M::Retaa
        | M::Retab
        | M::Retaasppc
        | M::Retabsppc
        | M::Eret
        | M::Eretaa
        | M::Eretab
        | M::Drps => FlowControl::Return,
        M::Svc | M::Hvc | M::Smc | M::Brk | M::Hlt | M::Udf | M::Dcps1 | M::Dcps2 | M::Dcps3 => {
            FlowControl::Exception
        }
        _ => FlowControl::Next,
    }
}
