# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1]

- Decoding is about 2× faster on real code (78 → 39 ns per instruction on Windows
  system DLLs, Snapdragon): operands are decoded by generated per-list functions
  instead of interpreting the descriptors.
- Decoding valid random encodings is about 40% faster (111 → 67 ns per instruction),
  and random words about 30% faster (66 → 46 ns).
- Binaries that use the decoder are about 18% larger.

## [0.1.0]

First release, generated from `AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09`.

- `Decoder`, `decode_word`, `DecoderOptions` with `FeatureSet` filtering.
- `Instruction`: operands, memory, condition, flow control, features, `min_el()`,
  `is_unpredictable()`, `determinism()`.
- `Code`, `Mnemonic`, `Feature`, `Register` for every encoding, mnemonic, feature and
  register of the release; `OpCodeInfo` per encoding.
- `Formatter` (Arm assembler syntax with preferred aliases), `Encoder`,
  `code_asm::CodeAssembler`, `InstructionInfoFactory`.
- `no_std`: only the `alloc` feature (implied by `std`) allocates, for
  `FormatterOutput` for `String` and `CodeAssembler::new`. `CodeAssembler` is
  generic over a `CodeBuffer` (`Vec<u32>` by default with `alloc`; `FixedBuffer` or
  `SliceBuffer` via `CodeAssembler::with_buffer` without it).
- Operand support for all 4,429 encodings: base, FP, Advanced SIMD, SVE/SVE2 and
  SME/SME2.

[Unreleased]: https://github.com/KenSykes/decoderant-a64/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/KenSykes/decoderant-a64/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/KenSykes/decoderant-a64/releases/tag/v0.1.0
