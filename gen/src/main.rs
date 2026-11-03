//! aarch64-isa-gen: generates `aarch64-isa`'s tables from the pinned AARCHMRS release.
//!
//! ```text
//! aarch64-isa-gen --spec <dir> [--out <dir>] [--report <file>] [--check]
//! ```
//!
//! `--spec` is the unpacked release (see `arch/aarch64-isa/spec/fetch.cmake`).
//! `--out` defaults to `arch/aarch64-isa/src/generated` and `--report` to
//! `arch/aarch64-isa/spec/report.md` (the operand-mapping report, checked in so a
//! new release shows up as a diff). Output is deterministic: the same release always
//! produces byte-identical files. `--check` writes nothing and fails if any generated
//! file (tables, report, `spec/encodings.txt`) differs from the checked-in one: the CI
//! job for hosts that have the spec data.

mod dispatch;
mod emit;
mod expr;
mod model;
mod names;
mod operands;
mod spec;
mod sysreg;
mod tables;
mod template;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    // The spec JSON nests deeply; parsing and dropping it recurses.
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(run)?
        .join()
        .map_err(|_| anyhow::anyhow!("generator thread panicked"))?
}

fn run() -> Result<()> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut spec_dir = None;
    let mut out_dir = crate_dir.join("src/generated");
    let mut report = crate_dir.join("spec/report.md");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--spec" => spec_dir = Some(PathBuf::from(args.next().context("--spec <dir>")?)),
            "--out" => out_dir = PathBuf::from(args.next().context("--out <dir>")?),
            "--report" => report = PathBuf::from(args.next().context("--report <file>")?),
            "--check" => *emit::CHECK.lock().expect("check lock") = Some(Vec::new()),
            _ => bail!("unknown argument {a}"),
        }
    }
    let spec_dir = spec_dir.context("--spec <dir> is required")?;
    let rev = spec::Revision::load(&crate_dir.join("spec/REVISION"))?;
    let spec = spec::Spec::load(&spec_dir, &rev)?;
    let model = model::Model::build(&spec.instructions)?;
    std::fs::create_dir_all(&out_dir)?;
    let encodings = crate_dir.join("spec/encodings.txt");
    let previous = std::fs::read_to_string(&encodings).unwrap_or_default();
    let (rep, list) = tables::generate(&spec, &model, &out_dir, &previous)?;
    emit::write(&report, &rep)?;
    emit::write(&encodings, &list)?;
    if let Some(diff) = emit::CHECK.lock().expect("check lock").take() {
        if !diff.is_empty() {
            bail!(
                "generated files differ from the checked-in ones: {}",
                diff.join(", ")
            );
        }
        eprintln!("generated files are up to date");
    }
    Ok(())
}
