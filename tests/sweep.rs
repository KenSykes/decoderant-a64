//! Exhaustive checks over all 2^32 words. Slow (minutes in release): run with
//! `cargo test -p decoderant-a64 --release --test sweep -- --ignored`.
//!
//! - every word decodes without panicking;
//! - every word that decodes to `C` satisfies `C`'s `matches()`;
//! - every word that satisfies `E`'s `matches()` decodes to `E` or to an encoding
//!   that `E`'s `OpCodeInfo` lists as taking precedence.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use decoderant_a64::{Code, DecoderOptions, decode_word};

/// For each value of the top 16 bits, the encodings whose fixed bits allow it.
fn buckets() -> Vec<Vec<Code>> {
    let mut b = vec![Vec::new(); 1 << 16];
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        let (m, v) = (oc.mask() >> 16, oc.value() >> 16);
        for (hi, list) in b.iter_mut().enumerate() {
            if hi as u32 & m == v {
                list.push(c);
            }
        }
    }
    b
}

#[derive(Default)]
struct Failures {
    count: u64,
    examples: Vec<String>,
}

impl Failures {
    fn add(&mut self, s: String) {
        self.count += 1;
        if self.examples.len() < 40 {
            self.examples.push(s);
        }
    }
}

fn sweep(check: impl Fn(u32, &[Code], &mut Failures) + Sync) {
    let b = buckets();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = AtomicU64::new(0);
    let fails = Mutex::new(Failures::default());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut local = Failures::default();
                loop {
                    let hi = next.fetch_add(1, Ordering::Relaxed);
                    if hi >= 1 << 16 {
                        break;
                    }
                    let list = &b[hi as usize];
                    for lo in 0..=0xffffu32 {
                        check((hi as u32) << 16 | lo, list, &mut local);
                    }
                }
                let mut f = fails.lock().unwrap();
                f.count += local.count;
                for e in local.examples {
                    if f.examples.len() < 40 {
                        f.examples.push(e);
                    }
                }
            });
        }
    });
    let f = fails.into_inner().unwrap();
    assert!(
        f.count == 0,
        "{} failures, e.g.:\n{}",
        f.count,
        f.examples.join("\n")
    );
}

#[test]
#[ignore]
fn every_word_classifies_consistently() {
    let opts = DecoderOptions::default();
    sweep(|w, list, f| {
        let insn = decode_word(w, 0, &opts);
        let c = insn.code();
        if c != Code::Invalid && !c.op_code().matches(w) {
            f.add(format!("{w:#010x} decodes to {c:?} but does not match it"));
        }
        for &e in list {
            if e != c && e.op_code().matches(w) && !e.op_code().precedence().contains(&c) {
                f.add(format!(
                    "{w:#010x} matches {e:?} but decodes to {c:?}, not listed as taking precedence"
                ));
            }
        }
    });
}

/// ISA-B: every valid word with operand support re-encodes to the identical word,
/// and, with the don't-care bits dropped, encodes from its operands to a word that
/// decodes to the same encoding and operands.
#[test]
#[ignore]
fn every_valid_word_re_encodes_bit_exactly() {
    let opts = DecoderOptions::default();
    sweep(|w, _, f| {
        let i = decode_word(w, 0x4000, &opts);
        if i.is_invalid() || !i.has_operands() {
            return;
        }
        match i.encode() {
            Ok(e) if e == w => {}
            Ok(e) => f.add(format!("{w:08x} {:?} re-encodes to {e:08x}", i.code())),
            Err(e) => f.add(format!("{w:08x} {:?}: {e}", i.code())),
        }
        match i.without_kept_bits().encode() {
            Ok(e) => {
                let j = decode_word(e, 0x4000, &opts);
                if j.code() != i.code() || j.operands() != i.operands() {
                    f.add(format!(
                        "{w:08x} {:?}: from operands gives {e:08x} ({:?})",
                        i.code(),
                        j.code()
                    ));
                }
            }
            Err(e) => f.add(format!("{w:08x} {:?}: from operands: {e}", i.code())),
        }
    });
}
