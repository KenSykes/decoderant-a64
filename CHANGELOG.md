# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `alloc` feature (implied by `std`): `FormatterOutput` for `String` and
  `CodeAssembler::new` need it; nothing else allocates.
- `code_asm::CodeBuffer`, `FixedBuffer` and `SliceBuffer`, and
  `CodeAssembler::with_buffer`, `bytes` and `assemble_into`: `code_asm` works
  without an allocator.

### Changed (breaking)

- `CodeAssembler` is generic over its buffer (`CodeAssembler<B = Vec<u32>>`, the
  default only with `alloc`). `CodeAssembler::new` and `assemble` need `alloc`.
- `CodeAssembler::word` returns `Result`, as it can now fail on a full buffer.
- `EncodeError::BufferFull` is a new variant.

## [0.1.0]

First release, generated from `AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09`.

- `Decoder`, `decode_word`, `DecoderOptions` with `FeatureSet` filtering.
- `Instruction`: operands, memory, condition, flow control, features, `min_el()`,
  `is_unpredictable()`, `determinism()`.
- `Code`, `Mnemonic`, `Feature`, `Register` for every encoding, mnemonic, feature and
  register of the release; `OpCodeInfo` per encoding.
- `Formatter` (Arm assembler syntax with preferred aliases), `Encoder`,
  `code_asm::CodeAssembler`, `InstructionInfoFactory`.
- Operand support for all 4,429 encodings: base, FP, Advanced SIMD, SVE/SVE2 and
  SME/SME2.

[Unreleased]: https://github.com/KenSykes/decoderant-a64/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/KenSykes/decoderant-a64/releases/tag/v0.1.0
