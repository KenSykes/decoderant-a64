//! Flow control, privilege and the system-register lookup for an instance.

use crate::generated::{Code, Mnemonic};
use crate::instruction::{FlowControl, Instruction};
use crate::sysreg::{SysEntry, SysKind, lookup, sys_encoding};

/// The system-register access kind of an encoding, by iclass.
pub(crate) fn sys_kind(code: Code, word: u32) -> Option<SysKind> {
    let l = word >> 21 & 1 == 1;
    Some(match code.op_code().iclass() {
        "systemmove" => {
            if l {
                SysKind::Mrs
            } else {
                SysKind::Msr
            }
        }
        "systemmovepr" => {
            if l {
                SysKind::Mrrs
            } else {
                SysKind::Msrr
            }
        }
        "systeminstrs" => {
            if l {
                SysKind::Sysl
            } else {
                SysKind::Sys
            }
        }
        "syspairinstrs" => SysKind::Sysp,
        "pstate" => SysKind::MsrImm,
        _ => return None,
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
    lookup(kind, sys_fields(insn.word))
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
        m if insn.op_code().iclass().starts_with("compbranch") && m != M::Invalid => {
            FlowControl::ConditionalBranch
        }
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
