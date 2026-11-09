//! A fingerprint of everything observable about all 2^32 words, for checking that a
//! refactor or optimisation changes no behaviour. Slow (minutes in release):
//!
//! ```text
//! DECODERANT_A64_FINGERPRINT=<file> cargo test -p decoderant-a64 --release --test fingerprint -- --ignored
//! ```
//!
//! The first run writes `<file>` (one hash per value of the top 16 bits); later runs
//! compare against it and list the buckets that differ, with an example word.
//! Only the public API is hashed, so internal representations are free to change.

use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use decoderant_a64::{
    DecoderOptions, EncodeError, Feature, FeatureSet, Formatter, FormatterOptions, Instruction,
    InstructionInfoFactory, decode_word,
};

/// FxHash: fast and stable across runs and Rust releases.
struct Fx(u64);

impl Hasher for Fx {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write_u64(u64::from(i));
    }

    fn write_u16(&mut self, i: u16) {
        self.write_u64(u64::from(i));
    }

    fn write_u32(&mut self, i: u32) {
        self.write_u64(u64::from(i));
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }

    fn write_isize(&mut self, i: isize) {
        self.write_u64(i as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

fn hash_encode(r: Result<u32, EncodeError>, h: &mut Fx) {
    match r {
        Ok(w) => (0u8, w).hash(h),
        Err(e) => {
            1u8.hash(h);
            let (tag, v): (u8, usize) = match e {
                EncodeError::InvalidCode => (0, 0),
                EncodeError::NoOperandSupport(c) => (1, c as usize),
                EncodeError::OperandCount { expected, got } => (2, expected << 8 | got),
                EncodeError::WrongKind(i) => (3, i),
                EncodeError::Register(i) => (4, i),
                EncodeError::Arrangement(i) => (5, i),
                EncodeError::Range(i) => (6, i),
                EncodeError::Unrepresentable(i) => (7, i),
                EncodeError::Conflict(i) => (8, i),
                EncodeError::BufferFull => (9, 0),
            };
            (tag, v).hash(h);
        }
    }
}

struct Ctx {
    full: DecoderOptions,
    some: DecoderOptions,
    none: DecoderOptions,
    fmt: Formatter,
    plain: Formatter,
}

fn hash_insn(i: &Instruction, h: &mut Fx) {
    (
        i.code(),
        i.mnemonic(),
        i.pc(),
        i.next_pc(),
        i.decoded_word(),
    )
        .hash(h);
    (i.has_operands(), i.op_count(), i.should_be_bits()).hash(h);
    for (k, o) in i.operands().iter().enumerate() {
        (
            o.kind(),
            o.register(),
            o.arrangement(),
            o.index(),
            o.list_len(),
        )
            .hash(h);
        (o.immediate(), o.shift(), o.extend(), o.amount(), o.name()).hash(h);
        (i.op_kind(k), i.op_register(k), i.immediate(k), i.shift(k)).hash(h);
    }
    (i.memory_base(), i.memory_index(), i.memory_index_extend()).hash(h);
    (
        i.memory_displacement(),
        i.memory_mode(),
        i.condition_code(),
        i.label_target(),
    )
        .hash(h);
}

fn word_hash(w: u32, cx: &Ctx, info: &mut InstructionInfoFactory, s: &mut String, h: &mut Fx) {
    let i = decode_word(w, 0x4000_1000, &cx.full);
    hash_insn(&i, h);
    let oc = i.op_code();
    (oc.name(), oc.matches(w), i.is_invalid()).hash(h);
    if i.is_invalid() {
        // The feature-filtered decodes can only be invalid too.
        return;
    }
    hash_encode(i.encode(), h);
    hash_encode(i.without_kept_bits().encode(), h);
    hash_encode(decoderant_a64::encode(&i, 0x8000), h);
    (i.unpredictable_reason(), i.nondeterminism_reason()).hash(h);
    (i.flow_control(), i.min_el(), i.determinism()).hash(h);
    for f in i.features() {
        f.hash(h);
    }
    let r = info.info(&i);
    (r.used_registers(), r.used_memory()).hash(h);
    s.clear();
    cx.fmt.format(&i, s);
    s.push('|');
    cx.plain.format(&i, s);
    s.push('|');
    let _ = write!(s, "{i}");
    s.hash(h);
    let j = decode_word(w, 0, &cx.some);
    hash_insn(&j, h);
    s.clear();
    cx.fmt.format(&j, s);
    s.hash(h);
    hash_insn(&decode_word(w, 0, &cx.none), h);
}

#[test]
#[ignore]
fn fingerprint() {
    let path = std::env::var("DECODERANT_A64_FINGERPRINT")
        .expect("set DECODERANT_A64_FINGERPRINT to the fingerprint file");
    // A feature subset that removes some hint-space and base encodings.
    let some: FeatureSet = FeatureSet::ALL
        .without(Feature::PAuth)
        .without(Feature::BTI)
        .without(Feature::SVE)
        .without(Feature::LSE)
        .without(Feature::FP16);
    let cx = Ctx {
        full: DecoderOptions::default(),
        some: DecoderOptions::with_features(some),
        none: DecoderOptions::with_features(FeatureSet::EMPTY),
        fmt: Formatter::new(),
        plain: Formatter::with_options(FormatterOptions {
            preferred_aliases: false,
            hex_immediates: false,
            uppercase: true,
        }),
    };
    let hashes: Vec<AtomicU64> = (0..1 << 16).map(|_| AtomicU64::new(0)).collect();
    let next = AtomicU64::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                let mut info = InstructionInfoFactory::new();
                let mut s = String::new();
                loop {
                    let hi = next.fetch_add(1, Ordering::Relaxed);
                    if hi >= 1 << 16 {
                        break;
                    }
                    let mut h = Fx(hi);
                    for lo in 0..=0xffffu32 {
                        word_hash((hi as u32) << 16 | lo, &cx, &mut info, &mut s, &mut h);
                    }
                    hashes[hi as usize].store(h.finish(), Ordering::Relaxed);
                }
            });
        }
    });
    let got: Vec<u64> = hashes.iter().map(|h| h.load(Ordering::Relaxed)).collect();
    let text: String = got.iter().map(|h| format!("{h:016x}\n")).collect();
    let Ok(old) = std::fs::read_to_string(&path) else {
        std::fs::write(&path, text).unwrap();
        eprintln!("wrote {path}");
        return;
    };
    let old: Vec<u64> = old
        .lines()
        .map(|l| u64::from_str_radix(l, 16).unwrap())
        .collect();
    let bad: Vec<usize> = (0..1 << 16).filter(|&i| old[i] != got[i]).collect();
    assert!(
        bad.is_empty(),
        "{} buckets differ, e.g. top halves {:04x?}",
        bad.len(),
        &bad[..bad.len().min(32)]
    );
}
