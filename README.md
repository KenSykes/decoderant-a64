# decoderant-a64

[![crates.io](https://img.shields.io/crates/v/decoderant-a64.svg)](https://crates.io/crates/decoderant-a64)
[![docs.rs](https://docs.rs/decoderant-a64/badge.svg)](https://docs.rs/decoderant-a64)
[![CI](https://github.com/KenSykes/decoderant-a64/actions/workflows/ci.yml/badge.svg)](https://github.com/KenSykes/decoderant-a64/actions/workflows/ci.yml)
![MSRV 1.88](https://img.shields.io/badge/rustc-1.88+-blue.svg)

An A64 (AArch64) decoder, formatter, encoder, assembler and instruction-info library
whose tables are **generated from Arm's machine-readable specification** (AARCHMRS),
not written by hand. The interface is modelled on
[`iced-x86`](https://crates.io/crates/iced-x86).

- **Complete.** Every one of the 4,429 A64 encodings of the pinned release
  (`AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09`) decodes, formats and encodes: base,
  FP, Advanced SIMD, crypto, SVE/SVE2 and SME/SME2, with the spec's preferred
  aliases.
- **Bit-exact.** Every valid 32-bit word re-encodes to the identical word, should-be
  bits included (checked over all 2^32 words).
- **Per-encoding metadata.** `OpCodeInfo` gives each encoding's name, iclass,
  encoding pattern (mask, value and the spec's excluded patterns), field layout,
  required `FEAT_*` features, assembler template and alias conditions.
- **Semantics for test generators.** Registers and memory read and written,
  flow control, the lowest exception level an instance runs at (`min_el()`, system
  registers included), and whether an instance is CONSTRAINED UNPREDICTABLE or
  nondeterministic.
- **Feature filtering.** Decode as a core without some features would: `PACIASP`
  on a core without PAuth decodes as the `HINT` it executes as.
- **`no_std`**, no allocation in the decoder or encoder, no `unsafe`.

## Usage

```toml
[dependencies]
decoderant-a64 = "0.1"
```

### Decode and format

```rust
use decoderant_a64::{Code, Decoder, DecoderOptions, Formatter, Mnemonic};

// add x0, x1, x2; ldr x3, [sp, #16]; ret
let bytes = [
    0x20, 0x00, 0x02, 0x8b, 0xe3, 0x0b, 0x40, 0xf9, 0xc0, 0x03, 0x5f, 0xd6,
];
let formatter = Formatter::new();
let mut text = Vec::new();
for insn in Decoder::new(&bytes, 0x1000, DecoderOptions::default()) {
    let mut line = String::new();
    formatter.format(&insn, &mut line);
    text.push(format!("{:x}: {line}", insn.pc()));
}
assert_eq!(text, ["1000: add x0, x1, x2", "1004: ldr x3, [sp, #0x10]", "1008: ret"]);

let insn = decoderant_a64::decode_word(0x8b02_0020, 0, &DecoderOptions::default());
assert_eq!(insn.code(), Code::Add_64_addsub_shift);
assert_eq!(insn.mnemonic(), Mnemonic::Add);
assert_eq!(insn.op_code().iclass(), "addsub_shift");
```

### Assemble and encode

```rust
use decoderant_a64::code_asm::{CodeAssembler, registers::*};
use decoderant_a64::Operand;

let mut a = CodeAssembler::new(0x1000);
a.add(x0, x1, x2)?.ldr(x3, Operand::mem_off(sp, 16))?.ret(x30)?;
assert_eq!(a.words(), [0x8b02_0020, 0xf940_0be3, 0xd65f_03c0]);
# Ok::<(), decoderant_a64::EncodeError>(())
```

`CodeAssembler` needs the `code_asm` feature. Decoded instructions re-encode with
`Instruction::encode()` or `decoderant_a64::encode(&insn, pc)`; `Instruction::with`
builds one from a `Code` and its operands.

### Instruction info and classification

```rust
use decoderant_a64::{
    DecoderOptions, Determinism, FlowControl, InstructionInfoFactory, OpAccess, Register,
    decode_word,
};

let opts = DecoderOptions::default();
let insn = decode_word(0xab02_0020, 0, &opts); // adds x0, x1, x2
let mut factory = InstructionInfoFactory::new();
let info = factory.info(&insn);
assert_eq!(info.access(Register::X0), OpAccess::Write);
assert_eq!(info.access(Register::X1), OpAccess::Read);
assert_eq!(info.access(Register::NZCV), OpAccess::Write);
assert_eq!(insn.flow_control(), FlowControl::Next);

// mrs x0, cntvct_el0: readable at EL0, but not deterministic.
let insn = decode_word(0xd53b_e040, 0, &opts);
assert_eq!(insn.min_el(), 0);
assert_eq!(insn.determinism(), Determinism::Nondeterministic);

// ldp x0, x0, [x1]: CONSTRAINED UNPREDICTABLE (Rt == Rt2).
assert!(decode_word(0xa940_0020, 0, &opts).is_unpredictable());
```

### Feature filtering

```rust
use decoderant_a64::{Code, DecoderOptions, Feature, decode_word};

let mut opts = DecoderOptions::default();
assert_eq!(decode_word(0xd503_233f, 0, &opts).to_string(), "paciasp");
opts.features.remove(Feature::PAuth);
let insn = decode_word(0xd503_233f, 0, &opts);
assert_eq!(insn.code(), Code::Hint_HM_hints);
```

More in [`examples/`](examples) (`cargo run --example dis -- 8b020020`) and in the
[API documentation](https://docs.rs/decoderant-a64).

## Cargo features

| Feature | Default | Enables |
|---|---|---|
| `std` | yes | `FormatterOutput` for `String`. Without it the crate is `#![no_std]`. |
| `formatter` | yes | `Formatter`: Arm assembler syntax from the spec's templates. |
| `encoder` | yes | `Encoder`, `encode`, `Instruction::with` and the operand setters. |
| `instr_info` | yes | `InstructionInfoFactory`: registers and memory used. |
| `code_asm` | no | `code_asm::CodeAssembler`, a fluent assembler (implies `encoder`). |

The decoder and `OpCodeInfo` are always present: everything else is built on them.

## How it is built

The tables in `src/generated/` are produced by `decoderant-a64-gen` (in `gen/`, not
published) from a pinned AARCHMRS release (`spec/REVISION`, with SHA-256 hashes).
The open-source release has no ASL pseudocode, so what the data does not say
(operand semantics, decode-time UNDEFINED rules, operand access, UNPREDICTABLE and
nondeterministic cases) is kept in reviewed hand-maintained tables, and checked by
differential tests against two independent decoders with hand-written decode trees
([`fARM64`](https://crates.io/crates/fARM64) and
[`yaxpeax-arm`](https://crates.io/crates/yaxpeax-arm)), golden snapshots and
exhaustive sweeps. See [docs/DESIGN.md](docs/DESIGN.md) for the design and
[CONTRIBUTING.md](CONTRIBUTING.md) for how to regenerate and test.

`Code` discriminants are not stable across spec releases: persist the encoding name
(`OpCodeInfo::name()`, `Code::from_name`), not the number.

## Known limitations

- `BlockEncoder` (re-encoding a sequence at a new address) is not implemented yet.
- A64 only: no A32 or T32.
- Memory accesses of SVE and SME instructions report size 0 (they depend on the
  vector length).
- With `FEAT_RPRFM` disabled, `RPRFM` words decode as unallocated, where cores
  without the feature execute them as prefetch hints.

## Minimum supported Rust version

Rust 1.88. Raising it is a minor-version change.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT licence ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

The generated tables (`src/generated/`) are derived from Arm's AARCHMRS, which Arm
publishes under the BSD 3-clause licence: see [LICENSE-AARCHMRS](LICENSE-AARCHMRS)
and [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES), both included in the published crate.
Keep that notice when you redistribute the crate or a binary built with it. Arm's
name is not used to endorse this project.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
