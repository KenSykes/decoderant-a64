//! Instruction info against `fARM64`'s `instruction_info()`: the registers read and
//! written, per encoding, over the words both decoders agree on. Disagreement
//! groups are triaged in `tests/triage/farm64_info.txt` (same format as the decode
//! triage). `AARCH64_ISA_TRIAGE=print` lists untriaged groups.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use aarch64_isa::{DecoderOptions, InstructionInfoFactory, OpAccess, decode_word};
use common::{GROUPS, Oracle, PC, load_triage, ours, same, sample_words};

fn acc(read: bool, write: bool) -> &'static str {
    match (read, write) {
        (true, true) => "rw",
        (true, false) => "r",
        (false, true) => "w",
        _ => "-",
    }
}

fn mine(w: u32) -> BTreeSet<String> {
    let i = decode_word(w, PC, &DecoderOptions::default());
    let mut f = InstructionInfoFactory::new();
    f.info(&i)
        .used_registers()
        .iter()
        .filter(|u| u.access != OpAccess::None)
        .map(|u| {
            format!(
                "{}:{}",
                u.register.name(),
                acc(u.access.reads(), u.access.writes())
            )
        })
        .collect()
}

fn theirs(w: u32) -> BTreeSet<String> {
    use fARM64::info::OpAccess as A;
    let b = w.to_le_bytes();
    let i = fARM64::Decoder::new(&b, PC, fARM64::DecoderOptions::default()).decode();
    let info = fARM64::info::instruction_info(&i);
    info.used_registers()
        .iter()
        .filter(|u| u.access != A::None)
        .filter(|u| !matches!(u.register.name(), "xzr" | "wzr"))
        .map(|u| {
            let (r, wr) = match u.access {
                A::Read | A::CondRead => (true, false),
                A::Write | A::CondWrite => (false, true),
                A::ReadWrite => (true, true),
                A::None => (false, false),
            };
            format!("{}:{}", u.register.name().to_ascii_lowercase(), acc(r, wr))
        })
        .collect()
}

#[test]
fn farm64_instruction_info() {
    let mut groups: BTreeMap<(String, String), (usize, Vec<String>)> = BTreeMap::new();
    for (w, _) in sample_words(16) {
        let i = decode_word(w, PC, &DecoderOptions::default());
        if i.is_invalid() || !GROUPS.contains(&i.op_code().group()) {
            continue;
        }
        // Only words both decoders read the same way.
        let (Some(a), Some(b)) = (ours(w), Oracle::Farm64.text(w)) else {
            continue;
        };
        if !same(&a, &b) {
            continue;
        }
        let (m, t) = (mine(w), theirs(w));
        if m != t {
            let only_m: Vec<_> = m.difference(&t).cloned().collect();
            let only_t: Vec<_> = t.difference(&m).cloned().collect();
            let g = groups
                .entry((i.op_code().name().to_string(), "registers".to_string()))
                .or_default();
            g.0 += 1;
            if g.1.len() < 3 {
                g.1.push(format!("{w:08x} {a}: ours +{only_m:?} theirs +{only_t:?}"));
            }
        }
    }
    let triage = load_triage_file("farm64_info");
    let untriaged: Vec<_> = groups
        .iter()
        .filter(|(k, _)| !triage.contains_key(*k))
        .collect();
    if std::env::var("AARCH64_ISA_TRIAGE").as_deref() == Ok("print") {
        for (k, (n, ex)) in &untriaged {
            println!("{} {} ({n} samples)", k.0, k.1);
            for e in ex {
                println!("    {e}");
            }
        }
        return;
    }
    assert!(
        untriaged.is_empty(),
        "{} untriaged instruction-info groups, e.g.:\n{}",
        untriaged.len(),
        untriaged
            .iter()
            .take(20)
            .map(|(k, (_, ex))| format!("{}: {}", k.0, ex.join(" ; ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let _ = load_triage;
}

fn load_triage_file(name: &str) -> BTreeMap<(String, String), String> {
    let path = format!("{}/tests/triage/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut it = l.splitn(4, ' ');
            let enc = it.next().unwrap_or_default().to_string();
            let cl = it.next().unwrap_or_default().to_string();
            let side = it.next().unwrap_or_default();
            assert!(
                matches!(side, "isa" | "oracle" | "both"),
                "{path}: bad side in {l}"
            );
            ((enc, cl), it.next().unwrap_or_default().to_string())
        })
        .collect()
}
