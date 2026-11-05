//! Instruction info against `fARM64`'s `instruction_info()`: the registers read and
//! written, per encoding, over the words both decoders agree on. Disagreement
//! groups are triaged in `tests/triage/farm64_info.txt` (same format as the decode
//! triage). `AARCH64_ISA_TRIAGE=print` lists untriaged groups. The base groups and,
//! where we decode their operands, SVE and SME.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use aarch64_isa::{DecoderOptions, InstructionInfoFactory, OpAccess, decode_word};
use common::{
    GROUPS, Oracle, PC, SVE_GROUPS, load_triage, ours, same, sample_words, sve_sample_words,
};

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
    // The base groups, then SVE and SME (fARM64 decodes them with its `sve` and
    // `sme` features).
    for (w, _) in sample_words(16).into_iter().chain(sve_sample_words(16)) {
        let i = decode_word(w, PC, &DecoderOptions::default());
        let g = i.op_code().group();
        let sve = SVE_GROUPS.contains(&g) && i.has_operands();
        if i.is_invalid() || !(GROUPS.contains(&g) || sve) {
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

/// SVE: merging predication reads the destination, zeroing does not; accumulating
/// (`<Zda>`), destructive (`<Zdn>`) and top-narrowing destinations are read; the
/// flag-setting predicate instructions write NZCV; FFR is implicit.
#[test]
fn sve_predication_and_implicit_registers() {
    use aarch64_isa::Register as R;
    let info = |w: u32| {
        let i = decode_word(w, PC, &DecoderOptions::default());
        assert!(i.has_operands(), "{w:08x}");
        let mut f = InstructionInfoFactory::new();
        let mut v: Vec<(R, OpAccess)> = f
            .info(&i)
            .used_registers()
            .iter()
            .map(|u| (u.register, u.access))
            .collect();
        v.sort_by_key(|x| x.0 as u16);
        (i.to_string(), v)
    };
    use OpAccess::{Read as Rd, ReadWrite as Rw, Write as Wr};
    for (w, text, expect) in [
        (
            0x0400_0597u32,
            "add z23.b, p1/m, z23.b, z12.b",
            vec![(R::Z12, Rd), (R::Z23, Rw), (R::P1, Rd)],
        ),
        (
            0x04d0_3ad5,
            "movprfx z21.d, p6/z, z22.d",
            vec![(R::Z21, Wr), (R::Z22, Rd), (R::P6, Rd)],
        ),
        (
            0x04d1_3ad5,
            "movprfx z21.d, p6/m, z22.d",
            vec![(R::Z21, Rw), (R::Z22, Rd), (R::P6, Rd)],
        ),
        (
            0x6462_017c,
            "fmla z28.h, z11.h, z2.h[4]",
            vec![(R::Z2, Rd), (R::Z11, Rd), (R::Z28, Rw)],
        ),
        (
            0x4560_450a,
            "sqxtnt z10.s, z8.d",
            vec![(R::Z8, Rd), (R::Z10, Rw)],
        ),
        (
            0x25d9_094c,
            "cmpge p12.d, p2/z, z10.d, #-7",
            vec![(R::Z10, Rd), (R::P2, Rd), (R::P12, Wr), (R::NZCV, Wr)],
        ),
        (0x256c_88a7, "incp x7, p5.h", vec![(R::X7, Rw), (R::P5, Rd)]),
        (0x2528_9000, "wrffr p0.b", vec![(R::P0, Rd), (R::FFR, Wr)]),
        (
            0x2518_f1a3,
            "rdffr p3.b, p13/z",
            vec![(R::P3, Wr), (R::P13, Rd), (R::FFR, Rd)],
        ),
    ] {
        let (t, v) = info(w);
        assert_eq!(t, text, "{w:08x}");
        assert_eq!(v, expect, "{w:08x} {text}");
    }
}

/// SVE loads, stores and prefetches: transfer lists written by loads and read by
/// stores, the governing predicate read, the base and a vector index read, and the
/// access size unknown (0: it depends on the vector length). `ADR` accesses no
/// memory.
#[test]
fn sve_memory() {
    use OpAccess::{Read as Rd, ReadWrite as Rw, Write as Wr};
    use aarch64_isa::Register as R;
    let info = |w: u32| {
        let i = decode_word(w, PC, &DecoderOptions::default());
        let mut f = InstructionInfoFactory::new();
        let info = f.info(&i);
        let mut v: Vec<(R, OpAccess)> = info
            .used_registers()
            .iter()
            .map(|u| (u.register, u.access))
            .collect();
        v.sort_by_key(|x| x.0 as u16);
        let m: Vec<(R, R, u32, OpAccess)> = info
            .used_memory()
            .iter()
            .map(|m| (m.base, m.index, m.size, m.access))
            .collect();
        (i.to_string(), v, m)
    };
    for (w, text, regs, mem) in [
        (
            0x8531_48d5u32,
            "ld1w { z21.s }, p2/z, [x6, z17.s, uxtw #2]",
            vec![(R::X6, Rd), (R::Z17, Rd), (R::Z21, Wr), (R::P2, Rd)],
            vec![(R::X6, R::Z17, 0, Rd)],
        ),
        (
            0xe57a_fd9d,
            "st4w { z29.s, z30.s, z31.s, z0.s }, p7, [x12, #-0x18, mul vl]",
            vec![
                (R::X12, Rd),
                (R::Z0, Rd),
                (R::Z29, Rd),
                (R::Z30, Rd),
                (R::Z31, Rd),
                (R::P7, Rd),
            ],
            vec![(R::X12, R::None, 0, Wr)],
        ),
        (
            0xa41f_7fff,
            "ldff1b { z31.b }, p7/z, [sp]",
            vec![(R::SP, Rd), (R::Z31, Wr), (R::P7, Rd), (R::FFR, Rw)],
            vec![(R::SP, R::None, 0, Rd)],
        ),
        (
            0x8468_192c,
            "prfb pstl3keep, p6, [x9, z8.s, sxtw]",
            vec![(R::X9, Rd), (R::Z8, Rd), (R::P6, Rd)],
            vec![(R::X9, R::Z8, 0, OpAccess::None)],
        ),
        (
            0x043e_abb6,
            "adr z22.d, [z29.d, z30.d, sxtw #2]",
            vec![(R::Z22, Wr), (R::Z29, Rd), (R::Z30, Rd)],
            vec![],
        ),
        (
            0x85b2_4bec,
            "ldr z12, [sp, #-0x6e, mul vl]",
            vec![(R::SP, Rd), (R::Z12, Wr)],
            vec![(R::SP, R::None, 0, Rd)],
        ),
    ] {
        let (t, v, m) = info(w);
        assert_eq!(t, text, "{w:08x}");
        assert_eq!(v, regs, "{w:08x} {text}");
        assert_eq!(m, mem, "{w:08x} {text}");
    }
}

/// Register lists: every register of a multi-vector or strided list, and `PSEL`'s
/// index register.
#[test]
fn sve_lists_and_psel() {
    use OpAccess::{Read as Rd, Write as Wr};
    use aarch64_isa::Register as R;
    let regs = |w: u32| {
        let i = decode_word(w, PC, &DecoderOptions::default());
        let mut f = InstructionInfoFactory::new();
        let mut v: Vec<(R, OpAccess)> = f
            .info(&i)
            .used_registers()
            .iter()
            .map(|u| (u.register, u.access))
            .collect();
        v.sort_by_key(|x| x.0 as u16);
        (i.to_string(), v)
    };
    let (t, v) = regs(0xa111_9342);
    assert_eq!(t, "ld1b { z2.b, z6.b, z10.b, z14.b }, pn12/z, [x26, x17]");
    assert_eq!(
        v,
        [
            (R::X17, Rd),
            (R::X26, Rd),
            (R::Z2, Wr),
            (R::Z6, Wr),
            (R::Z10, Wr),
            (R::Z14, Wr),
            (R::PN12, Rd)
        ]
    );
    let (t, v) = regs(0x2571_5162);
    assert_eq!(t, "psel p2, p4, p11.s[w13, 1]");
    assert_eq!(v, [(R::W13, Rd), (R::P2, Wr), (R::P4, Rd), (R::P11, Rd)]);
    let (t, v) = regs(0x257b_5010);
    assert_eq!(t, "whilege { p0.h, p1.h }, x0, x27");
    assert_eq!(
        v,
        [
            (R::X0, Rd),
            (R::X27, Rd),
            (R::P0, Wr),
            (R::P1, Wr),
            (R::NZCV, Wr)
        ]
    );
}
