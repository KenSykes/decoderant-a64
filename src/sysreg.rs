//! System registers and system operations, from `Registers.json`.

use crate::generated::Feature;
use crate::generated::sysreg::SYSREGS;

/// How a system register or operation is accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SysKind {
    /// `MRS` (read).
    Mrs,
    /// `MSR` (register write).
    Msr,
    /// `MSR` (immediate): a PSTATE field.
    MsrImm,
    /// `MRRS` (128-bit read).
    Mrrs,
    /// `MSRR` (128-bit write).
    Msrr,
    /// `SYS` and its aliases (`DC`, `IC`, `AT`, `TLBI`, ...).
    Sys,
    /// `SYSL`.
    Sysl,
    /// `SYSP` and `TLBIP`.
    Sysp,
}

/// One system register accessor or system operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SysEntry {
    pub kind: SysKind,
    /// The instruction that performs the access (`MRS`, `DC`, `TLBI`, ...).
    pub insn: &'static str,
    /// The assembler name (`NZCV`, `CIVAC`, ...).
    pub name: &'static str,
    /// `op0:op1:CRn:CRm:op2`, 16 bits, with `mask` covering the bits the accessor
    /// fixes.
    pub value: u16,
    pub mask: u16,
    /// The lowest exception level at which some configuration allows the access
    /// (4: none).
    pub min_el: u8,
    /// Features the register or accessor requires.
    pub features: &'static [Feature],
}

/// Packs `op0:op1:CRn:CRm:op2` into the 16-bit form used by [`SysEntry`].
pub const fn sys_encoding(op0: u32, op1: u32, crn: u32, crm: u32, op2: u32) -> u16 {
    ((op0 & 3) << 14 | (op1 & 7) << 11 | (crn & 15) << 7 | (crm & 15) << 3 | (op2 & 7)) as u16
}

/// The entries of a kind matching an encoding, most specific first.
pub fn lookup(kind: SysKind, enc: u16) -> Option<&'static SysEntry> {
    let lo = SYSREGS.partition_point(|e| e.kind < kind);
    let hi = SYSREGS.partition_point(|e| e.kind <= kind);
    SYSREGS[lo..hi]
        .iter()
        .filter(|e| enc & e.mask == e.value)
        .max_by_key(|e| e.mask.count_ones())
}

/// Every entry, sorted by kind and encoding.
pub fn entries() -> &'static [SysEntry] {
    &SYSREGS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_by_kind() {
        assert!(
            SYSREGS
                .windows(2)
                .all(|w| (w[0].kind, w[0].value) <= (w[1].kind, w[1].value))
        );
        // TTBR0_EL1 has 64- and 128-bit accessors.
        let ttbr0 = sys_encoding(3, 0, 2, 0, 0);
        for k in [SysKind::Mrs, SysKind::Msr, SysKind::Mrrs, SysKind::Msrr] {
            assert_eq!(lookup(k, ttbr0).map(|e| e.name), Some("TTBR0_EL1"), "{k:?}");
        }
    }
}
