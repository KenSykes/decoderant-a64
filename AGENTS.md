# AGENTS.md

Guidance for AI coding agents working in this repository. Humans: see
[CONTRIBUTING.md](CONTRIBUTING.md), which this file summarises.

## What this is

`decoderant-a64`, a crate published on crates.io: an A64 decoder, formatter, encoder,
assembler and instruction-info library modelled on `iced-x86`, whose tables are
generated from Arm's AARCHMRS by `decoderant-a64-gen` (`gen/`). Read
[docs/DESIGN.md](docs/DESIGN.md) before making changes.

## Rules

- Never hand-edit `src/generated/`, `spec/report.md` or `spec/encodings.txt`. Change
  the generator or the hand-maintained tables (operand mapping, operand access,
  UNDEFINED rules, constraints, determinism) and regenerate (CONTRIBUTING.md).
- Classify by decoding bytes, never by trusting what produced them.
- Every oracle disagreement is fixed or triaged in `tests/triage/` with a reason.
- `#![no_std]`; no `unsafe`; no allocation in the decoder or encoder; typed errors.
- The public API is semver-checked by users: removing or renaming public items is a
  breaking change and goes in `CHANGELOG.md`.
- Do not copy code from GPL or EPL projects or from the oracle crates.

## Commands

- Build: `cargo build --workspace`
- Test: `cargo test --workspace --all-features`
- Lint: `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Regenerate: `cargo run -p decoderant-a64-gen --release -- --spec <unpacked release>`
