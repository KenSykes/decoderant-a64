# decoderant-a64 design

An A64 decoder, formatter, encoder, assembler and instruction-info library whose
tables are generated from Arm's machine-readable specification (AARCHMRS). This
document explains where its data comes from, what is hand-maintained and why, how the
interface maps to `iced-x86`, and how it is verified.

## Goals

The crate was written for differential testing of CPU emulators, where a library
has to do more than disassemble. A test generator has to **enumerate every encoding
form**, **know each form's fields and their legal values** to build instances, and
**reason about what an instance does** (which registers it writes, whether it touches
memory, whether it branches, whether it is usable at EL0, whether its result is
architecturally defined). Recorded words have to be **classified back** to their
encoding form. These are the jobs of an encoder/decoder library, and they are the
jobs `iced-x86` does for x86, so the interface follows `iced-x86`.

- **Generated from the spec.** Encoding names, encoding patterns, the decode tree and
  feature requirements come from `Instructions.json`, `Features.json` and
  `Registers.json` of a pinned AARCHMRS release. The decode tree is not hand-written.
- **Complete.** Every encoding of the pinned release has a `Code`, an `OpCodeInfo`
  and operand support.
- **Bit-exact.** Every valid word re-encodes to itself, including bits that do not
  affect any operand.
- **Portable.** Pure Rust, `#![no_std]` with an optional `std` feature, no `unsafe`,
  no allocation in the decoder or encoder.
- **Honest.** What the spec does not say in machine-readable form is a reviewed,
  hand-maintained table with a reason per entry, and every table is checked by
  independent oracles.

## Spec pin and licence

`spec/REVISION` pins `AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09`: the tarball's
SHA-256 and each used file's. `spec/fetch.cmake` downloads and verifies it (into
`$AARCHMRS_DATA_DIR/<release>` or `spec/data/`, which git ignores). Only Arm's
open-source (BSD 3-clause) package is used. The licence was checked for this release:
`docs/notice.html` and the `_meta` block of all three JSON files state BSD 3-clause,
and the generator refuses to run if a pinned file or the notice changes.

The code is MIT; the generated tables are BSD-3 derived data, so the crate's
`license` field is `MIT AND BSD-3-Clause`. Attribution: `THIRD_PARTY_NOTICES`,
`LICENSE-AARCHMRS` and a header on every generated file.

## Code generation

```
AARCHMRS release (spec/data/<release>, pinned in spec/REVISION)
        │  decoderant-a64-gen (gen/, not published)
        ▼
src/generated/        checked in: Code, Mnemonic, Feature, Register enums, decode
                      dispatch, conditions, encodings, operand descriptors, system
                      registers, instruction-info flags, CodeAssembler methods
spec/report.md        operand-mapping report
spec/encodings.txt    one line per encoding, so a release change is a reviewable diff
```

- The generator is run by hand when the pin or a hand-maintained table in `gen/`
  changes: `cargo run -p decoderant-a64-gen --release -- --spec <dir>`. Normal builds
  do not need the spec (`Instructions.json` alone is over 100 MB).
- Output is deterministic: everything is sorted and nothing iterates a `HashMap`, so
  the same inputs always give byte-identical files. `--check` writes nothing and
  fails if anything differs from the checked-in files.
- When the pin changes, `spec/changes.md` lists the encodings added, removed,
  renamed and changed, for review with the regenerated tables.
- Tables are `static` data walked by hand-written code, not large generated `match`
  expressions, to keep compile times reasonable with several thousand encodings.

### What the open-source release contains

- It has **no ASL**: decode and operation pseudocode are omitted. The decode tree,
  encoding patterns, field conditions, feature conditions, assembler grammar and
  alias conditions are machine-readable; operand semantics and decode-time UNDEFINED
  rules are not.
- Sibling nodes of the decode tree overlap (the `HINT` encoding covers `NOP`,
  `PACIASP`, ...). The decoder tries children most specific first, and a child whose
  feature condition fails is skipped, so a missing feature falls back to `HINT`
  exactly as hardware does. 129 encodings overlap a higher-priority one
  (`OpCodeInfo::precedence()`).
- The base groups' assembler rule names carry field values (`T_01_1_8H`: size 01,
  Q 1), so most value tables come from the data. They are assembler hints, not
  always complete value sets (`<R>` lists only `00x` and `x11`); the differential
  tests found every such case and a hand rule replaced it.
- The SVE/SME rule names carry no field values (`T_B__2`). For those the generator
  evaluates each encoding's path conditions and maps a table's alternatives, in spec
  order, to the field values the encoding allows; values the missing decode ASL
  reserves get no entry and decode as UNALLOCATED.

### Hand-maintained tables

Each is reviewed like code and checked by the oracles (see "Verification").

| Table | Where | What |
|---|---|---|
| Operand mapping | `gen/src/operands.rs`, `gen/src/sve.rs` | Symbol → operand kind and decoding rule: register 31 as SP or ZR, offset scaling, PC-relative targets, bitmask and FP immediates, `immh:immb` shifts, element indices, alias formulas such as `LSL #n` = `UBFM #(-n MOD 64), #(63-n)`, SVE/SME element sizes. 135 distinct rules; `spec/report.md` lists them and their uses. |
| Operand access | `gen/src/info.rs` | Which operands are read, written or both, and the implicit registers (flags, PC, FFR, ...), by mnemonic and iclass. The generator fails on a name no encoding has. |
| UNDEFINED rules | `src/undefined.rs` | Decode-time UNDEFINED rules from the missing ASL (a 32-bit shift of 32 or more, `CASP` with an odd register, MOPS `sz != 00`, SVE `size` values that fixed-size encodings pin, ...), applied by the decoder, `matches()` and the encoder. |
| Constraints | `src/constraints.rs` | UNPREDICTABLE and CONSTRAINED UNPREDICTABLE instances, with the Arm ARM reference. |
| Determinism | `src/determinism.rs` | Instances whose results can differ between runs or machines, with the reason. |
| `MSR (immediate)` ELs | `gen/src/sysreg.rs` | Its accessors have no access tree in `Registers.json`. |

## Interface (modelled on iced-x86)

Names follow `iced-x86` unless A64 makes them misleading.

| iced-x86 | decoderant-a64 | Notes |
|---|---|---|
| `Decoder::new(bitness, data, options)`, `decode`, iterator | `Decoder::new(data, pc, options)`, same methods; `decode_word` | A64 only. Words are little-endian. |
| `DecoderOptions` | `DecoderOptions` | Feature filtering (`FeatureSet`, see below). Default: everything in the pinned release. |
| `Instruction` (`Copy`) | `Instruction` | `code()`, `mnemonic()`, `pc()`, `next_pc()`, operands, memory accessors, `condition_code()`, `flow_control()`, `features()`, `min_el()`, `is_unpredictable()`, `determinism()`; setters and `with` constructors. Keeps the decoded should-be and don't-care bits. |
| `Code` | `Code` | One variant per AARCHMRS **encoding**, named from its encoding name (`ADD_64_addsub_shift` → `Code::Add_64_addsub_shift`). `Code::values()` enumerates the space. Discriminants are not stable across releases: persist the name. |
| `Mnemonic` | `Mnemonic` | Architectural mnemonic. Preferred aliases (`MOV`, `CMP`, `LSL`, ...) are a formatter option, not separate `Code`s. |
| `Register` | `Register` | `X`/`W` with `XZR`/`WZR` and `SP`/`WSP` distinct, SIMD&FP views, `Z`, `P`, `PN`, ZA tiles, `ZT0`, and implicit registers (`NZCV`, `FPCR`, `FPSR`, `FFR`, `PC`). `#[non_exhaustive]`. |
| `OpKind` | `OpKind` | `Register`, `RegisterList`, `Immediate`, `BitmaskImmediate`, `FpImmediate`, `ShiftedRegister`, `ExtendedRegister`, `Memory`, `Label`, `Condition`, `SystemRegister`, `PState`, `SystemOp`, `ControlRegister`, `Barrier`, `Prefetch`, `Name`, `ZaTile`, `ZaSlice`, `ZaArray`, `ZaMask`. `#[non_exhaustive]`. |
| `InstructionInfoFactory` | `InstructionInfoFactory` | Registers and memory used, with access kinds; implicit state included. |
| `Instruction::cpuid_features()` | `Instruction::features()` | `FEAT_*` names from `Features.json`, per instance (a system register's feature included). |
| `Code::op_code()` → `OpCodeInfo` | same | Encoding name, iclass, pattern, fields, features, template, aliases, precedence. |
| `Formatter`, `FormatterOutput` | same, `FormatterOptions` | Arm assembler syntax from the AARCHMRS templates. Options: preferred aliases, hex immediates, case. |
| `Encoder`, `encode(&instr, rip)` | `Encoder::encode(&instr, pc) -> Result<u32, EncodeError>` | Re-encodes from operands and the kept bits, never from a stored raw word. Out-of-range or reserved operands are typed errors. |
| `CodeAssembler` (`code_asm`) | same | `a.adds(x0, x1, x2)?`. Tries encodings and aliases with the requested mnemonic; an alias is accepted only when the word decodes back to it. |
| `BlockEncoder` | not yet | |

Unlike `iced-x86` there are no `decoder` or `op_code_info` features: the decoder and
the per-encoding metadata are what everything else is built on (the encoder
re-decodes what it encodes).

### Beyond iced-x86: what a test generator needs

- **Encoding pattern**: mask/value plus the excluded patterns from the spec's field
  conditions (`Rn != 11111`, reserved `size`/`opc` values), as
  `OpCodeInfo::matches(word)`. Mask/value alone is not enough: most encodings share
  their mask/value space with conditions or with other encodings.
- **Field layout**: every field's name, bit range and the operand it feeds, including
  should-be bits (`OpCodeInfo::fields()`).
- **Privilege**: `Instruction::min_el()`. For `MRS`, `MSR`, `SYS`, `SYSL`, `DC`, `IC`,
  `AT` and `TLBI` it depends on the system register or operation the operand names,
  and comes from the accessor definitions in `Registers.json`: the lowest EL at which
  some path performs the access (traps and UNDEFINED count as no access).

### Constraints and determinism

`Instruction::is_unpredictable()` reports instances whose result is UNPREDICTABLE or
CONSTRAINED UNPREDICTABLE. Should-be bit conditions are generated from the spec; the
rest are the constraints table, which covers at least:

- pair loads with `Rt == Rt2`;
- writeback with `Rt == Rn` (or `Rt2 == Rn` for pairs), `LDRAA`/`LDRAB` included;
- store-exclusive with `Rs` equal to `Rt`, `Rt2` or `Rn`, and load-exclusive pairs
  with `Rt == Rt2`;
- `CPY*` and `SET*` with overlapping registers or register 31;
- `MSR (immediate)` to a 1-bit PSTATE field with an immediate above 1;
- should-be `(0)`/`(1)` bits not at their stated value.

`Instruction::determinism()` reports instances whose results can differ between runs,
cores or machines on the same input, with a reason
(`Instruction::nondeterminism_reason()`). The determinism table covers at least:
store-exclusive (spurious failure), `RNDR`/`RNDRRS`, the system counter
(`CNTVCT_EL0`, `CNTFRQ_EL0`, ...), identification registers (`MIDR_EL1`, `CTR_EL0`,
`DCZID_EL0`, `ID_*`, ...), thread pointers, performance and activity monitor
counters, the IMPLEMENTATION DEFINED system register space
(`S3_<op1>_C<n>_C<m>_<op2>`), and `DC ZVA`/`GVA`/`GZVA`.

### Feature filtering

Decoding with a `FEAT_*` disabled behaves as if that feature's encodings were removed
from the decode tree: the word decodes as whatever remains, and is `Code::Invalid`
only if nothing does. In the hint space, `PACIASP`, `BTI`, `XPACLRI` and similar
execute as `NOP` on cores without the feature, so with the feature disabled they
decode as `HINT`. An `MRS`/`MSR` of a system register whose feature is disabled still
decodes as `MRS`/`MSR`; the instance's `features()` reports the missing feature.

### Should-be and don't-care bits

Some encodings have bits the spec marks `(0)` or `(1)`. A word with a different value
there decodes to the same encoding, but the behaviour is CONSTRAINED UNPREDICTABLE.
`Instruction` keeps those bits as decoded, `is_unpredictable()` reports any that are
not at their stated value, and the encoder writes them back (default: the stated
value).

A few variable bits affect no operand value (the `imm4`/`imm5` bits of `INS`/`DUP`
below the element size). They are kept like the should-be bits, never used to choose
operand values, and dropped when an operand is edited.

Mnemonic suffixes (`SADDL2`, `BFMLALB`/`BFMLALT`) are not operands: the instruction
keeps the suffix and the encoder sets its bits when no operand does.

## SVE and SME

- `Z`, `P` and `PN` registers are `OpKind::Register` with an element size
  (`Arrangement::B`..`Q`) and a predicate qualifier (`Operand::predicate()`: `/Z`,
  `/M`). Indexed elements use the lane.
- Memory operands carry a vector base or index with its element size, `MUL VL`
  (counting whole transfer lists: `LD2B ..., #8, MUL VL` encodes 4), and an optional
  `XZR` index.
- Register lists are consecutive, SME2 multi-vector (aligned to their length) or
  strided. Lists of three or more consecutive registers that do not wrap print as
  ranges (`{ z0.s-z3.s }`), others element by element.
- Patterns are `OpKind::Name` with `NameKind::SvePattern`; `MUL #<imm>` follows as an
  immediate. Both are left out when they are `ALL` and 1, as LLVM and the Arm ARM
  print them.
- SME has `OpKind::ZaTile`, `ZaSlice`, `ZaArray` and `ZaMask` (`ZERO { <mask> }`,
  printed as the largest tiles that cover it); `ZT0` is a register. ZA indices print
  as the templates write them (`za1h.s[w12, 3]`, `za.s[w8, 0:1, vgx2]`).
- Instruction info reports Z/P/PN/ZA/ZT0 like V registers; merging predication and
  the spec's accumulating names (`<Zda>`, `<Zdn>`, `<ZAda>`) make the destination
  read too. Memory accesses report size 0 (it depends on the vector length).
- Two readings have no reference yet: `LDAPRO`/`STLRO` offsets (shown as the field
  value; the open-source spec gives no scaling) and `LUTI6`'s three-register list
  (read as starting at a multiple of 4, the SME2 convention).
- Thirteen SVE aliases (`ORN`/`EON`/`BIC` immediates, `CMPLE` and similar,
  `FMOV ..., #0.0`) have the preferred condition `false` in the spec: they are
  assembler-only forms, never the disassembly.

## Decoder speed

The decoder does not walk the decode tree. The generator emits a dispatch
(`gen/src/dispatch.rs`): tables indexed by fields of the word, ending in short
candidate lists kept in decode-tree order, so the first candidate whose fixed bits and
path conditions match is the tree's answer. Conditions are compiled to Rust functions
(`src/generated/conds.rs`). Decoding re-encodes to find don't-care bits only where the
operands may not reproduce every variable bit (`src/cover.rs`). The tree and the
expression table are still generated, for the unit tests, which check the dispatch
and the compiled conditions against them. `benches/decode.rs` measures throughput.

## Verification

A wrong table silently mislabels or loses coverage downstream, so correctness comes
before speed.

- **Exhaustive sweep** over all 2^32 words (`tests/sweep.rs`, ignored by default;
  minutes in release): every word decodes without panicking; every word that decodes
  to `C` satisfies `C`'s `matches()`; every word that satisfies `C`'s `matches()`
  decodes to `C` or to an encoding `OpCodeInfo` lists as taking precedence; every
  valid word re-encodes to the identical word.
- **Encoder round trip** per `Code` (`tests/encoder.rs`): random operand values within
  the field layout, encode, decode, compare.
- **Differential decode** against `fARM64` and `yaxpeax-arm` (`tests/oracles.rs`,
  dev-dependencies; both have hand-written decode trees): classification and
  operands, compared through a normalised text form. SVE and SME are compared with
  `fARM64` only (`yaxpeax-arm` 0.5 decodes neither). Disagreements are grouped by
  (encoding, class) and every group is triaged in `tests/triage/*.txt` with the reason
  and which side is right. Groups an oracle does not implement, or that the other
  oracle settles, get a generated triage line reviewed by sampling.
- **Instruction info** against `fARM64`'s `instruction_info()` (`tests/info.rs`),
  triaged the same way.
- **Golden snapshots** of the formatter for representative encodings of every iclass,
  EL0 availability of system registers, and hint fallback (`tests/golden.rs`).
- **Fingerprint** (`tests/fingerprint.rs`, ignored): a hash of everything observable
  through the public API for all 2^32 words, bucketed by encoding, to show that an
  optimisation changes no behaviour.
- **Hardware.** Recordings on a Cortex-A78C, replayed against Arm's Sail model,
  cross-checked the decoder's classification with the recording's features. Findings
  so far: `CFINV`/`XAFLAG`/`AXFLAG` share the `pstate` iclass with `MSR (immediate)`
  but are not system accesses; `MSR (immediate)` to a 1-bit PSTATE field with an
  immediate above 1 is UNDEFINED on the A78C (now a constraint); the IMPLEMENTATION
  DEFINED system register space and some counters are nondeterministic (now in the
  determinism table). Open: `RPRFM` without `FEAT_RPRFM` (see the README's known
  limitations).

No code is copied from `fARM64`, `yaxpeax-arm` or LLVM; they are used only as oracles.
