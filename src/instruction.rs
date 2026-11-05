//! The decoded instruction.

use crate::feature_set::FeatureSet;
use crate::generated::{Code, Feature, Mnemonic, Register};
use crate::op_code_info::OpCodeInfo;
use crate::operand::{
    Arrangement, ConditionCode, ExtendKind, MemoryMode, OpKind, Operand, ShiftKind,
};

/// Operand slots in an [`Instruction`].
pub const MAX_OPERANDS: usize = 8;

/// Control flow of an instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowControl {
    /// Falls through to the next instruction.
    Next,
    /// `B`, `BC.cond`'s unconditional relatives.
    UnconditionalBranch,
    /// `B.cond`, `CBZ`, `TBZ`, `CB<cc>`, ...
    ConditionalBranch,
    /// `BL`.
    Call,
    /// `BLR` and the authenticated forms.
    IndirectCall,
    /// `BR` and the authenticated forms.
    IndirectBranch,
    /// `RET`, `RETAA`, `ERET`, ...
    Return,
    /// `SVC`, `HVC`, `SMC`, `BRK`, `HLT`, `UDF`, `DCPS*`: takes an exception.
    Exception,
}

/// Whether an instruction's result can differ between runs on the same input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Determinism {
    Deterministic,
    /// Results depend on time, randomness, the implementation or other agents (a
    /// reason is in [`Instruction::nondeterminism_reason`]).
    Nondeterministic,
}

/// A decoded A64 instruction. A value type: `Copy`, no allocation.
///
/// Operands are decoded from the word once; the encoder (`encoder` feature) works
/// from them and the kept should-be bits, never from the original word.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instruction {
    pub(crate) code: Code,
    pub(crate) pc: u64,
    /// The word as decoded. Used for alias selection and the constraint tables;
    /// never by the encoder.
    pub(crate) word: u32,
    pub(crate) op_count: u8,
    pub(crate) ops: [Operand; MAX_OPERANDS],
    /// Should-be bits as decoded (only those in the encoding's should-be mask).
    pub(crate) sb: u32,
    /// Don't-care bits as decoded: variable bits no operand value depends on (the
    /// `immr` bits above a bitmask element, index bits below an element size).
    /// Kept like the should-be bits so re-encoding is bit-exact; editing an operand
    /// clears them.
    pub(crate) extra: u32,
    /// Whether operand decoding is supported for this encoding.
    pub(crate) has_operands: bool,
    /// Index + 1 of the preferred alias (0: none).
    pub(crate) alias: u8,
    /// The mnemonic suffix (`2` in `SADDL2`, `B`/`T` in `BFMLALB`/`BFMLALT`) as an
    /// index into the encoding's suffix table.
    pub(crate) suffix: u8,
    pub(crate) features: FeatureSet,
}

impl Default for Instruction {
    fn default() -> Self {
        Instruction {
            code: Code::Invalid,
            pc: 0,
            word: 0,
            op_count: 0,
            ops: [Operand::none(); MAX_OPERANDS],
            sb: 0,
            extra: 0,
            has_operands: false,
            alias: 0,
            suffix: 0,
            features: FeatureSet::ALL,
        }
    }
}

impl Instruction {
    pub fn code(&self) -> Code {
        self.code
    }

    /// The architectural mnemonic (not the alias; see the formatter for aliases).
    pub fn mnemonic(&self) -> Mnemonic {
        self.code.op_code().mnemonic()
    }

    pub fn is_invalid(&self) -> bool {
        self.code == Code::Invalid
    }

    pub fn pc(&self) -> u64 {
        self.pc
    }

    pub fn set_pc(&mut self, pc: u64) {
        // Labels are stored as absolute targets, so they stay put.
        self.pc = pc;
    }

    pub fn next_pc(&self) -> u64 {
        self.pc.wrapping_add(4)
    }

    /// Always 4.
    pub fn len(&self) -> usize {
        4
    }

    /// Never true (A64 instructions are four bytes); for API symmetry.
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn op_code(&self) -> OpCodeInfo {
        self.code.op_code()
    }

    /// Whether operands are decoded for this encoding (every encoding of the pinned
    /// release has them; false for `Code::Invalid`).
    pub fn has_operands(&self) -> bool {
        self.has_operands
    }

    pub fn op_count(&self) -> usize {
        self.op_count as usize
    }

    pub fn operands(&self) -> &[Operand] {
        &self.ops[..self.op_count as usize]
    }

    pub fn op(&self, i: usize) -> Operand {
        if i < self.op_count as usize {
            self.ops[i]
        } else {
            Operand::none()
        }
    }

    pub fn op_kind(&self, i: usize) -> OpKind {
        self.op(i).kind
    }

    pub fn op_register(&self, i: usize) -> Register {
        self.op(i).reg
    }

    pub fn op_arrangement(&self, i: usize) -> Arrangement {
        self.op(i).arr
    }

    pub fn immediate(&self, i: usize) -> u64 {
        self.op(i).imm
    }

    fn memory(&self) -> Option<&Operand> {
        self.operands().iter().find(|o| o.kind == OpKind::Memory)
    }

    /// The memory base register (`Register::None` without a memory operand).
    pub fn memory_base(&self) -> Register {
        self.memory().map_or(Register::None, |m| m.reg)
    }

    /// The index register (offset or post-index register).
    pub fn memory_index(&self) -> Register {
        self.memory().map_or(Register::None, |m| m.index_reg)
    }

    pub fn memory_index_extend(&self) -> Option<(ExtendKind, u8)> {
        self.memory()
            .filter(|m| m.index_reg != Register::None)
            .map(|m| (m.extend, m.amount))
    }

    /// The immediate offset (pre/post-index amounts included).
    pub fn memory_displacement(&self) -> i64 {
        self.memory().map_or(0, |m| m.imm as i64)
    }

    pub fn memory_mode(&self) -> Option<MemoryMode> {
        self.memory().map(|m| m.mode)
    }

    /// The first condition operand (`B.cond`, `CSEL`, `CCMP`, ...).
    pub fn condition_code(&self) -> Option<ConditionCode> {
        self.operands()
            .iter()
            .find(|o| o.kind == OpKind::Condition)
            .map(|o| ConditionCode::from_bits(o.imm as u32))
    }

    /// The target of a PC-relative operand (branch target or literal address).
    pub fn label_target(&self) -> Option<u64> {
        self.operands()
            .iter()
            .find(|o| o.kind == OpKind::Label)
            .map(|o| o.imm)
    }

    pub fn shift(&self, i: usize) -> Option<(ShiftKind, u8)> {
        let o = self.op(i);
        matches!(o.kind, OpKind::ShiftedRegister | OpKind::Immediate).then_some((o.shift, o.amount))
    }

    /// The decoded should-be bits (inside the encoding's should-be mask).
    pub fn should_be_bits(&self) -> u32 {
        self.sb
    }

    /// Features the encoding (and, for system register and system operation
    /// accesses, the named register or operation) requires.
    pub fn features(&self) -> impl Iterator<Item = Feature> + '_ {
        let enc = self.op_code().features().iter().copied();
        let sys = crate::info::sys_entry(self)
            .map(|e| e.features)
            .unwrap_or(&[])
            .iter()
            .copied();
        enc.chain(sys)
    }

    /// The system register or system operation the instance names (`MRS`, `MSR`,
    /// `SYS`, `SYSL`, `SYSP` and their aliases), if the encoding is a system access
    /// and the operands name a known one.
    pub fn system_entry(&self) -> Option<&'static crate::SysEntry> {
        crate::info::sys_entry(self)
    }

    pub fn flow_control(&self) -> FlowControl {
        crate::info::flow_control(self)
    }

    /// The lowest exception level at which this instance can execute: a property
    /// of the encoding, except for `MRS`, `MSR`, `SYS`, `SYSL` and their aliases,
    /// where it comes from the named system register or operation.
    pub fn min_el(&self) -> u8 {
        crate::info::min_el(self)
    }

    /// Whether the instance is UNPREDICTABLE or CONSTRAINED UNPREDICTABLE: should-be
    /// bits not at their stated value, or a condition from the constraints table.
    pub fn is_unpredictable(&self) -> bool {
        self.unpredictable_reason().is_some()
    }

    /// The first reason this instance is unpredictable.
    pub fn unpredictable_reason(&self) -> Option<&'static str> {
        crate::constraints::unpredictable(self)
    }

    pub fn determinism(&self) -> Determinism {
        if self.nondeterminism_reason().is_some() {
            Determinism::Nondeterministic
        } else {
            Determinism::Deterministic
        }
    }

    pub fn nondeterminism_reason(&self) -> Option<&'static str> {
        crate::determinism::reason(self)
    }

    /// The word this instruction was decoded from (for diagnostics; the encoder
    /// never reads it).
    pub fn decoded_word(&self) -> u32 {
        self.word
    }
}
