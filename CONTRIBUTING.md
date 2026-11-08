# Contributing

Contributions are accepted under the MIT licence (see [LICENSE](LICENSE)). Read
[docs/DESIGN.md](docs/DESIGN.md) first.

## Layout

```
src/               the library (src/generated/ is generated: never edit it by hand)
gen/               decoderant-a64-gen, the table generator (not published)
spec/              REVISION (the pinned AARCHMRS release), fetch.cmake, and the
                   generator's checked-in reports (report.md, encodings.txt)
tests/             integration tests; golden/ snapshots, triage/ oracle triage files
benches/, examples/
scripts/test.ps1   runs every check and reports what it skipped
```

## Checks

Every change must pass:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

`scripts/test.ps1` (Windows PowerShell 5.1 or PowerShell 7) runs these, the docs
and the generated-files check, and lists what it skipped; `-Sweep`,
`-Fingerprint <file>` and `-Exhaustive` add the slow checks.

Slow checks, for changes to the decoder, encoder, generator or tables (release
builds; minutes to hours):

```sh
cargo test --release --all-features --test sweep -- --ignored     # all 2^32 words
cargo test --release --all-features --test oracles -- --ignored   # exhaustive oracle comparison
```

Environment variables used by the tests and benches:

| Variable | Effect |
|---|---|
| `DECODERANT_A64_BLESS=1` | `tests/golden.rs` rewrites its snapshots; review the diff. |
| `DECODERANT_A64_TRIAGE=print` | `tests/oracles.rs` and `tests/info.rs` list untriaged groups with examples. |
| `DECODERANT_A64_SUGGEST=1` | With `TRIAGE=print`, also suggests triage lines. |
| `DECODERANT_A64_SAMPLES=<n>` | Samples per encoding in the sampled oracle test (default 48). |
| `DECODERANT_A64_STRIDE=<n>` | The exhaustive oracle test checks every `n`th word. |
| `DECODERANT_A64_FINGERPRINT=<file>` | `tests/fingerprint.rs` records, or compares with, a behaviour fingerprint. |
| `DECODERANT_A64_BENCH_PE=<files>` | `benches/decode.rs` decodes the code sections of these ARM64 PE files. |
| `DECODERANT_A64_BENCH_FILTER=<s>` | Runs only the benchmarks whose name contains `s`. |

To show that an optimisation changes no behaviour, record a fingerprint before the
change and compare after:

```sh
DECODERANT_A64_FINGERPRINT=before.txt cargo test --release --all-features --test fingerprint -- --ignored
```

## Regenerating the tables

Needed when the spec pin or a hand-maintained table in `gen/` changes.

```sh
cmake -P spec/fetch.cmake        # downloads and verifies the pinned release (~200 MB unpacked)
cargo run -p decoderant-a64-gen --release -- --spec spec/data/AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09
cargo run -p decoderant-a64-gen --release -- --spec spec/data/AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09 --check
```

Commit the regenerated `src/generated/`, `spec/report.md` and `spec/encodings.txt`
with the change that caused them, and review their diff. `--check` fails if any
generated file differs from the checked-in one.

To move to a new AARCHMRS release, update `spec/REVISION` (the release name, tarball
URL and SHA-256, and each file's SHA-256), check the licence in `docs/notice.html`
and each JSON file's `_meta` block is still BSD 3-clause, update `SPEC_RELEASE` in
`src/lib.rs` and `THIRD_PARTY_NOTICES`, regenerate, and review `spec/changes.md`.

## Rules

- **Correctness first.** Downstream users classify recorded hardware results with
  this crate; a wrong table silently mislabels them.
- **Generated files are never edited by hand.** Change the generator or a
  hand-maintained table instead.
- **Hand-maintained tables** (operand mapping, operand access, UNDEFINED rules,
  constraints, determinism) give a reason and, where there is one, an Arm ARM
  reference per entry.
- **Oracle disagreements are triaged**, never ignored: add a line to
  `tests/triage/*.txt` saying which side is right and why, or fix the bug.
- **Spec licence.** Only Arm's open-source BSD 3-clause AARCHMRS package is used; the
  spec itself is never committed.
- **No copied code** from GPL or EPL projects (QEMU, binutils, ...) or from the oracle
  crates. Referencing their behaviour is fine.
- `#![no_std]`, no `unsafe` (`unsafe_code = "forbid"`), no allocation in the decoder
  or encoder. Errors are typed (`thiserror`); only the generator uses `anyhow`.
- Tests, benches and examples declare the features they need with
  `required-features`.

## Releasing

1. Update `CHANGELOG.md` and the version in `Cargo.toml` (`[workspace.package]`).
   Breaking changes, including removed or renamed `Code` variants after a spec
   update, need a minor-version bump while the version is `0.x`.
2. Run the checks above, the slow checks, and `cargo package --list` (the package
   holds `src/`, the licences, the notices, the README and the changelog).
3. `cargo publish --dry-run`, then `cargo publish`.
4. Tag the release `vX.Y.Z`.
