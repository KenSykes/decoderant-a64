//! Differential decode against independent decoders: `fARM64` (MIT) and
//! `yaxpeax-arm` (0BSD), whose decode trees are hand-written from the Arm ARM.
//!
//! Words are sampled per `Code` (random values of the variable bits, plus all-zero
//! and all-one), plus random words for the valid/invalid classification.
//! Disagreements are grouped by (encoding name, class) and every group must be
//! triaged in `tests/triage/<oracle>.txt` with which side is right and why.
//!
//! - `cargo test -p aarch64-isa --test oracles` runs the sampled comparison.
//! - `AARCH64_ISA_TRIAGE=print` lists the untriaged groups with examples instead of
//!   failing on the first.
//! - `AARCH64_ISA_SAMPLES=<n>` changes the samples per encoding (default 48).
//!
//! Scope: the base, FP and Advanced SIMD groups (`control`, `dpimm`, `dpreg`,
//! `ldst`, `simd_dp`). SVE and SME come later (docs/arch/aarch64-isa.md, "Scope").

mod common;

use std::collections::BTreeMap;
use std::sync::Mutex;

use common::{Oracle, compare_groups, finish, load_triage, merge, tally};

fn run(oracle: Oracle) {
    let n: u32 = std::env::var("AARCH64_ISA_SAMPLES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(48);
    check(oracle, compare_groups(oracle, n));
}

/// Every word (or every `AARCH64_ISA_STRIDE`th), in parallel.
fn exhaustive(oracle: Oracle) -> common::Groups {
    let stride: u64 = std::env::var("AARCH64_ISA_STRIDE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()) as u64;
    let total = Mutex::new((common::Groups::new(), BTreeMap::new()));
    std::thread::scope(|s| {
        for t in 0..threads {
            let total = &total;
            s.spawn(move || {
                let words = (t * stride..1u64 << 32)
                    .step_by((stride * threads) as usize)
                    .map(|w| (w as u32, String::new()));
                let part = tally(oracle, words);
                merge(&mut total.lock().unwrap(), part);
            });
        }
    });
    let (g, a) = total.into_inner().unwrap();
    finish(g, &a)
}

fn check(oracle: Oracle, groups: common::Groups) {
    check_with(oracle, groups, false);
}

/// The exhaustive tests also accept `tests/triage/<oracle>_exhaustive.txt`.
fn check_with(oracle: Oracle, groups: common::Groups, exhaustive: bool) {
    let mut triage = load_triage(oracle);
    if exhaustive {
        triage.extend(common::load_triage_named(&format!(
            "{}_exhaustive",
            oracle.name()
        )));
    }
    let mut untriaged: Vec<_> = groups
        .iter()
        .filter(|(k, _)| !triage.contains_key(*k))
        .collect();
    untriaged.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(a.0.cmp(b.0)));
    let stale: Vec<_> = triage.keys().filter(|k| !groups.contains_key(*k)).collect();
    if std::env::var("AARCH64_ISA_TRIAGE").as_deref() == Ok("print") {
        for (k, g) in &untriaged {
            let suggest = std::env::var("AARCH64_ISA_SUGGEST").is_ok();
            if suggest && k.1 == "valid-vs-invalid:unpredictable" {
                println!(
                    "SUGGEST {} {} both {} rejects CONSTRAINED UNPREDICTABLE register choices; we decode them and report is_unpredictable()",
                    k.0,
                    k.1,
                    oracle.name()
                );
                continue;
            }
            if suggest && !g.oracle_unknown && g.other_agrees {
                let other = if oracle.name() == "farm64" {
                    "yaxpeax"
                } else {
                    "farm64"
                };
                println!(
                    "SUGGEST {} {} isa {other} agrees with us on every sample; {} is wrong here",
                    k.0,
                    k.1,
                    oracle.name()
                );
                continue;
            }
            if g.oracle_unknown && suggest {
                let feats: Vec<String> = aarch64_isa::Code::from_name(&k.0)
                    .map(|c| {
                        c.op_code()
                            .features()
                            .iter()
                            .map(|f| f.name().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                println!(
                    "SUGGEST {} {} isa {} does not implement this encoding ({})",
                    k.0,
                    k.1,
                    oracle.name(),
                    if feats.is_empty() {
                        "base".to_string()
                    } else {
                        feats.join(", ")
                    }
                );
                continue;
            }
            println!("{} {} ({} samples)", k.0, k.1, g.count);
            for e in &g.examples {
                println!("    {e}");
            }
        }
        for k in &stale {
            println!("stale: {} {}", k.0, k.1);
        }
        return;
    }
    assert!(
        untriaged.is_empty(),
        "{} untriaged disagreement groups with {}, e.g.:\n{}",
        untriaged.len(),
        oracle.name(),
        untriaged
            .iter()
            .take(20)
            .map(|(k, g)| format!("{} {}: {}", k.0, k.1, g.examples.join(" ; ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let _ = stale;
}

#[test]
fn farm64() {
    run(Oracle::Farm64);
}

#[test]
fn yaxpeax() {
    run(Oracle::Yaxpeax);
}

/// The comparison over all 2^32 words (nightly; hours). `AARCH64_ISA_STRIDE=<n>`
/// takes every nth word for a quicker pass.
#[test]
#[ignore]
fn farm64_exhaustive() {
    check_with(Oracle::Farm64, exhaustive(Oracle::Farm64), true);
}

#[test]
#[ignore]
fn yaxpeax_exhaustive() {
    check_with(Oracle::Yaxpeax, exhaustive(Oracle::Yaxpeax), true);
}
