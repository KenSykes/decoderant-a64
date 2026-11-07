//! `aarch64-isa`: an A64 decoder, formatter, encoder and instruction-info library
//! whose tables are generated from Arm's machine-readable specification (AARCHMRS).
//! The interface is modelled on `iced-x86`.
//!
//! ```
//! use aarch64_isa::{Decoder, DecoderOptions, Code};
//!
//! let bytes = 0x8b02_0020u32.to_le_bytes(); // ADD X0, X1, X2
//! let insn = Decoder::new(&bytes, 0x1000, DecoderOptions::default()).decode();
//! assert_eq!(insn.code(), Code::Add_64_addsub_shift);
//! assert_eq!(insn.op_code().iclass(), "addsub_shift");
//! ```
//!
//! The generated tables (`src/generated/`) are BSD-3-Clause derived data from
//! AARCHMRS; the code is MIT. See docs/arch/aarch64-isa.md.

#![no_std]

// The unit tests use `std` whatever the features.
#[cfg(any(test, feature = "std"))]
extern crate std;

mod alias;
mod bits;
#[cfg(feature = "code_asm")]
pub mod code_asm;
mod cond;
mod constraints;
#[cfg(feature = "encoder")]
mod cover;
mod decoder;
mod desc;
mod determinism;
#[cfg(feature = "encoder")]
mod encoder;
mod feature_set;
#[cfg(feature = "formatter")]
mod formatter;
mod generated;
mod info;
#[cfg(feature = "instr_info")]
mod instr_info;
mod instruction;
mod op_code_info;
mod operand;
mod regs;
mod sysreg;
mod tables;
mod undefined;

pub use decoder::{Decoder, DecoderOptions, decode_word};
#[cfg(feature = "encoder")]
pub use encoder::{EncodeError, Encoder, encode};
pub use feature_set::FeatureSet;
#[cfg(feature = "formatter")]
pub use formatter::{FmtOutput, Formatter, FormatterOptions, FormatterOutput, FormatterTextKind};
pub use generated::{Code, Feature, Mnemonic, Register};
#[cfg(feature = "instr_info")]
pub use instr_info::{InstructionInfo, InstructionInfoFactory, OpAccess, UsedMemory, UsedRegister};
pub use instruction::{Determinism, FlowControl, Instruction, MAX_OPERANDS};
pub use op_code_info::{AliasInfo, OpCodeInfo};
pub use operand::{
    Arrangement, ConditionCode, ExtendKind, MemoryMode, OpKind, Operand, PredQual, ShiftKind,
};
pub use regs::RegisterClass;
pub use sysreg::{SysEntry, SysKind, sys_encoding};
pub use tables::FieldInfo;

/// System register and system operation lookups.
pub mod system {
    pub use crate::sysreg::{entries, lookup};
}

/// The number of [`Code`] values, `Code::Invalid` included.
pub const CODE_COUNT: usize = generated::CODE_COUNT;
/// The number of [`Mnemonic`] values.
pub const MNEMONIC_COUNT: usize = generated::MNEMONIC_COUNT;
/// The number of [`Feature`] values.
pub const FEATURE_COUNT: usize = generated::FEATURE_COUNT;
/// The number of [`Register`] values.
pub const REGISTER_COUNT: usize = generated::REGISTER_COUNT;

/// The AARCHMRS release the tables were generated from.
pub const SPEC_RELEASE: &str = "AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09";
