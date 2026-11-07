//! Types of the generated tables (`src/generated/tables.rs`).

use crate::desc::Op;
use crate::generated::{Code, Feature, Mnemonic};

/// No condition: always true.
pub(crate) const NO_COND: u16 = u16::MAX;

/// A named encoding field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: &'static str,
    pub lsb: u8,
    pub width: u8,
}

/// A preferred-disassembly alias of an encoding.
#[derive(Debug)]
pub(crate) struct AliasData {
    pub mnemonic: Mnemonic,
    pub template: &'static str,
    /// When the alias applies (the spec's alias condition).
    pub cond: u16,
    /// When the alias is the preferred disassembly.
    pub preferred: u16,
    /// Field values the alias condition pins (`Rn == 11111` for `MOV`), as (mask,
    /// value): the assembler starts from them.
    #[cfg_attr(not(feature = "code_asm"), allow(dead_code))]
    pub fixed: (u32, u32),
    /// Fields the alias condition makes equal (`Rm == Rn` for `ROR`).
    #[cfg_attr(not(feature = "code_asm"), allow(dead_code))]
    pub copies: &'static [(crate::bits::F, crate::bits::F)],
    /// Operand descriptors, or `None` when the alias template is not supported.
    pub ops: Option<&'static [Op]>,
}

#[derive(Debug)]
pub(crate) struct EncodingData {
    pub name: &'static str,
    pub iclass: &'static str,
    pub group: &'static str,
    pub mnemonic: Mnemonic,
    pub mask: u32,
    pub value: u32,
    pub sb_mask: u32,
    pub sb_value: u32,
    /// Conditions on the decode path (ancestor groups and the encoding itself).
    pub conds: &'static [u16],
    /// Conditions on the path that name only features.
    pub feature_conds: &'static [u16],
    /// Every feature the path conditions name.
    pub features: &'static [Feature],
    pub fields: &'static [FieldInfo],
    pub template: &'static str,
    /// Encodings that win where their pattern overlaps this one.
    pub precedence: &'static [Code],
    /// Operand descriptors, or `None` when operands are not supported yet.
    pub ops: Option<&'static [Op]>,
    pub aliases: &'static [AliasData],
}

/// A node of the decode tree. Children are tried in order (most specific first).
/// The decoder uses the dispatch built from it; the tree is the tests' reference.
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Node {
    pub mask: u32,
    pub value: u32,
    pub cond: u16,
    pub first: u16,
    pub count: u16,
    /// The encoding for a leaf, `Code::Invalid` for a group.
    pub code: Code,
}

/// A decode-dispatch candidate: an encoding's fixed bits and the conjunction of
/// its path conditions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cand {
    pub mask: u32,
    pub value: u32,
    pub cond: u16,
    pub code: Code,
}

/// Dispatch slots (`DISPATCH`): a list of candidates in `CANDIDATES`, tried in
/// order (`LIST | len << 20 | start`), or a node indexing further slots by a field
/// of the word (`lsb << 24 | width << 20 | base`).
pub(crate) const SLOT_LIST: u32 = 1 << 31;
