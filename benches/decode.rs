//! Throughput of the decoder, encoder, formatter and instruction info:
//! `cargo bench -p aarch64-isa --bench decode`.
//!
//! Corpora:
//! - `code`: the code sections of ARM64 PE files (`AARCH64_ISA_BENCH_PE`, a
//!   `;`-separated list; on Windows the default is a few system DLLs), skipped when
//!   there are none;
//! - `valid`: random words that decode to an instruction with operands;
//! - `random`: uniformly random words (mostly SVE/SME and unallocated space).
//!
//! Each figure is the best of several runs, in nanoseconds per instruction.
//! `AARCH64_ISA_BENCH_FILTER` runs only the benchmarks whose name contains it.

use std::hint::black_box;
use std::time::Instant;

use aarch64_isa::{
    Decoder, DecoderOptions, Formatter, Instruction, InstructionInfoFactory, decode_word,
};

const RUNS: usize = 7;
const MAX_WORDS: usize = 1 << 20;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }
}

/// The words of the code sections of an ARM64 PE file.
fn pe_code(path: &str) -> Option<Vec<u32>> {
    let b = std::fs::read(path).ok()?;
    let u16_at = |o: usize| Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?));
    let pe = u32_at(0x3c)? as usize;
    if u32_at(pe)? != 0x4550 || u16_at(pe + 4)? != 0xaa64 {
        return None;
    }
    let nsec = u16_at(pe + 6)? as usize;
    let sec0 = pe + 24 + u16_at(pe + 20)? as usize;
    let mut words = Vec::new();
    for s in 0..nsec {
        let h = sec0 + 40 * s;
        let (vsize, raw_size, raw) = (u32_at(h + 8)?, u32_at(h + 16)?, u32_at(h + 20)?);
        if u32_at(h + 36)? & 0x2000_0020 == 0 {
            continue;
        }
        let len = vsize.min(raw_size) as usize & !3;
        let code = b.get(raw as usize..raw as usize + len)?;
        words.extend(
            code.as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_le_bytes(*c)),
        );
    }
    Some(words)
}

fn code_corpus() -> Vec<u32> {
    let paths = std::env::var("AARCH64_ISA_BENCH_PE").unwrap_or_else(|_| {
        if cfg!(windows) {
            let sys = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
            ["ntdll.dll", "KernelBase.dll", "ucrtbase.dll"]
                .map(|f| format!(r"{sys}\System32\{f}"))
                .join(";")
        } else {
            String::new()
        }
    });
    let mut words = Vec::new();
    for p in paths.split(';').filter(|p| !p.is_empty()) {
        match pe_code(p) {
            Some(w) => words.extend(w),
            None => eprintln!("skipping {p}: not an ARM64 PE file"),
        }
    }
    words.truncate(MAX_WORDS);
    words
}

fn valid_corpus() -> Vec<u32> {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let opts = DecoderOptions::default();
    let mut words = Vec::with_capacity(MAX_WORDS / 4);
    while words.len() < MAX_WORDS / 4 {
        let w = rng.next();
        let i = decode_word(w, 0, &opts);
        if !i.is_invalid() && i.has_operands() {
            words.push(w);
        }
    }
    words
}

fn random_corpus() -> Vec<u32> {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    (0..MAX_WORDS / 4).map(|_| rng.next()).collect()
}

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// Best-of-`RUNS` nanoseconds per item of `f`, which processes `n` items.
fn time(name: &str, n: usize, mut f: impl FnMut()) {
    if let Ok(filter) = std::env::var("AARCH64_ISA_BENCH_FILTER")
        && !name.contains(&filter)
    {
        return;
    }
    f();
    let mut best = f64::MAX;
    for _ in 0..RUNS {
        let t = Instant::now();
        f();
        best = best.min(t.elapsed().as_secs_f64());
    }
    println!(
        "{name:<24} {:>9.2} ns/insn  ({n} insns)",
        best * 1e9 / n as f64
    );
}

fn run(corpus: &str, words: &[u32]) {
    if words.is_empty() {
        return;
    }
    let data = bytes(words);
    let n = words.len();
    let opts = DecoderOptions::default();

    time(&format!("{corpus}/decode"), n, || {
        let mut d = Decoder::new(&data, 0x1_0000, opts);
        let mut insn = Instruction::default();
        while d.can_decode() {
            d.decode_out(&mut insn);
            black_box(&insn);
        }
    });

    let insns: Vec<Instruction> = Decoder::new(&data, 0x1_0000, opts).collect();
    let encodable: Vec<Instruction> = insns
        .iter()
        .filter(|i| !i.is_invalid() && i.has_operands())
        .copied()
        .collect();
    time(&format!("{corpus}/encode"), encodable.len(), || {
        for i in &encodable {
            black_box(black_box(i).encode().ok());
        }
    });

    let fmt = Formatter::new();
    let mut s = String::with_capacity(128);
    time(&format!("{corpus}/format"), encodable.len(), || {
        for i in &encodable {
            s.clear();
            fmt.format(black_box(i), &mut s);
            black_box(&s);
        }
    });

    let mut f = InstructionInfoFactory::new();
    time(&format!("{corpus}/info"), encodable.len(), || {
        for i in &encodable {
            black_box(f.info(black_box(i)));
        }
    });
}

fn main() {
    println!(
        "size_of::<Instruction>() = {}",
        std::mem::size_of::<Instruction>()
    );
    run("code", &code_corpus());
    run("valid", &valid_corpus());
    run("random", &random_corpus());
}
