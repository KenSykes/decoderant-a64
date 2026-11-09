//! `decoderant-a64`: an A64 decoder, formatter, encoder and instruction-info library
//! whose tables are generated from Arm's machine-readable specification (AARCHMRS).
//! The interface is modelled on `iced-x86`.
//!
//! Every A64 encoding of the pinned release is covered (base, FP, Advanced SIMD,
//! SVE/SVE2 and SME/SME2), with per-encoding metadata ([`OpCodeInfo`]), bit-exact
//! round trips and UNPREDICTABLE and determinism classification.
//!
//! ```
//! use decoderant_a64::{Decoder, DecoderOptions, Code};
//!
//! let bytes = 0x8b02_0020u32.to_le_bytes(); // ADD X0, X1, X2
//! let insn = Decoder::new(&bytes, 0x1000, DecoderOptions::default()).decode();
//! assert_eq!(insn.code(), Code::Add_64_addsub_shift);
//! assert_eq!(insn.op_code().iclass(), "addsub_shift");
//! ```
//!
//! # Cargo features
//!
//! - `std` (default): `FormatterOutput` for `String`. The crate is `#![no_std]`
//!   without it, and the decoder and encoder never allocate.
//! - `formatter` (default): `Formatter`, Arm assembler syntax.
//! - `encoder` (default): `Encoder`, `encode` and the `Instruction` constructors.
//! - `instr_info` (default): `InstructionInfoFactory`, registers and memory used.
//! - `code_asm`: the fluent `code_asm::CodeAssembler` (implies `encoder`).
//!
//! The decoder and [`OpCodeInfo`] are always present.
//!
//! # Licence
//!
//! Licensed under either of the MIT licence or the Apache License, Version 2.0, at
//! your option. The generated tables (`src/generated/`) are derived from Arm's
//! AARCHMRS, published under the BSD 3-clause licence; see `THIRD_PARTY_NOTICES` and
//! `LICENSE-AARCHMRS`.

#![no_std]

// The unit tests use `std` whatever the features.
#[cfg(any(test, feature = "std"))]
extern crate std;

// The README's examples run as doctests (they use every feature).
#[cfg(all(
    doctest,
    feature = "std",
    feature = "code_asm",
    feature = "formatter",
    feature = "instr_info"
))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// `from_name` (a binary search over a name-sorted index) finds every value by
    /// its name, and nothing else.
    #[test]
    fn from_name_round_trips() {
        fn check<T: Copy + PartialEq + core::fmt::Debug>(
            values: impl Iterator<Item = T>,
            name: impl Fn(T) -> &'static str,
            from_name: impl Fn(&str) -> Option<T>,
        ) {
            for v in values {
                assert_eq!(from_name(name(v)), Some(v), "{v:?}");
            }
            for s in ["", "nonexistent", "\u{10ffff}"] {
                assert!(from_name(s).is_none_or(|v| name(v) == s), "{s:?}");
            }
        }
        check(Code::values(), Code::name, Code::from_name);
        check(Mnemonic::values(), Mnemonic::name, Mnemonic::from_name);
        check(Feature::values(), Feature::name, Feature::from_name);
        check(Register::values(), Register::name, Register::from_name);
    }
}
