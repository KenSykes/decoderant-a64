//! Shared helpers for the differential tests.

#![allow(dead_code)]

use std::collections::BTreeMap;

use aarch64_isa::{Code, DecoderOptions, Formatter, FormatterOptions, decode_word};
use fARM64::format::FmtFormatter;
use yaxpeax_arch::{Decoder as _, U8Reader};

pub const PC: u64 = 0x1000;

/// The groups the differential tests cover.
pub const GROUPS: &[&str] = &["control", "dpimm", "dpreg", "ldst", "simd_dp"];

/// The SVE and SME groups, which only fARM64 decodes (with its `sve` and `sme`
/// features; yaxpeax-arm 0.5 has neither).
pub const SVE_GROUPS: &[&str] = &["sve", "sme"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Oracle {
    Farm64,
    Yaxpeax,
}

impl Oracle {
    pub fn name(self) -> &'static str {
        match self {
            Oracle::Farm64 => "farm64",
            Oracle::Yaxpeax => "yaxpeax",
        }
    }

    /// Whether the oracle decodes SVE and SME.
    pub fn has_sve(self) -> bool {
        self == Oracle::Farm64
    }

    /// The oracle's text for `w`, or `None` when it rejects the word.
    pub fn text(self, w: u32) -> Option<String> {
        let b = w.to_le_bytes();
        match self {
            Oracle::Farm64 => {
                let i = fARM64::Decoder::new(&b, PC, fARM64::DecoderOptions::default()).decode();
                (!i.is_invalid())
                    .then(|| fARM64::format::format_to_string(&FmtFormatter::new(), &i))
            }
            Oracle::Yaxpeax => {
                let d = yaxpeax_arm::armv8::a64::InstDecoder::default();
                let mut r = U8Reader::new(&b);
                // yaxpeax-arm returns `invalid ...` text for some words it cannot name.
                d.decode(&mut r)
                    .ok()
                    .map(|i| i.to_string())
                    .filter(|s| !s.starts_with("invalid"))
            }
        }
    }
}

/// Our text for `w` (aliases on), or `None` when invalid.
pub fn ours(w: u32) -> Option<String> {
    let i = decode_word(w, PC, &DecoderOptions::default());
    if i.is_invalid() {
        return None;
    }
    let mut s = String::new();
    Formatter::with_options(FormatterOptions::default()).format(&i, &mut s);
    Some(s)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Text(String),
    Num(u64),
    Float(f64),
}

/// Normalises disassembly for comparison: lower case, single spaces, no `#`,
/// numbers parsed (hex or decimal, signed), yaxpeax's `$+off` labels made absolute.
pub fn tokens(s: &str) -> Vec<Tok> {
    let s = s
        .to_ascii_lowercase()
        .replace('\t', " ")
        .replace("{ ", "{")
        .replace(" }", "}")
        // Register ranges: `{ z0.s-z3.s }` and `{ z0.s - z3.s }`.
        .replace(" - ", "-");
    // Condition spellings: CS is HS, CC is LO.
    let s = replace_word(&replace_word(&s, "cs", "hs"), "cc", "lo");
    let b = s.as_bytes();
    let mut out: Vec<Tok> = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let flush = |text: &mut String, out: &mut Vec<Tok>| {
        let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            out.push(Tok::Text(t));
        }
        text.clear();
    };
    while i < b.len() {
        let c = b[i] as char;
        let prev_alnum = i > 0
            && ((b[i - 1] as char).is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.');
        let starts_num = c.is_ascii_digit()
            || ((c == '-' || c == '+' || c == '$')
                && b.get(i + 1)
                    .is_some_and(|d| d.is_ascii_digit() || *d == b'+' || *d == b'-'));
        if starts_num && !prev_alnum {
            // `$+0x10` / `$-0x10`: relative to the PC.
            let mut rel = false;
            if c == '$' {
                rel = true;
                i += 1;
            }
            let mut neg = false;
            if b[i] == b'-' || b[i] == b'+' {
                neg = b[i] == b'-';
                i += 1;
            }
            let st = i;
            let hex = b.get(i) == Some(&b'0') && b.get(i + 1) == Some(&b'x');
            if hex {
                i += 2;
                while i < b.len() && (b[i] as char).is_ascii_hexdigit() {
                    i += 1;
                }
            } else {
                while i < b.len()
                    && ((b[i] as char).is_ascii_digit() || b[i] == b'.' || b[i] == b'e')
                {
                    i += 1;
                }
            }
            let lit = &s[st..i];
            flush(&mut text, &mut out);
            if !hex && lit.contains('.') {
                let v: f64 = lit.parse().unwrap_or(f64::NAN);
                out.push(Tok::Float(if neg { -v } else { v }));
            } else {
                let v = if hex {
                    u64::from_str_radix(&lit[2..], 16).unwrap_or(0)
                } else {
                    lit.parse::<u64>().unwrap_or(0)
                };
                let mut v = if neg { v.wrapping_neg() } else { v };
                if rel {
                    v = v.wrapping_add(PC);
                }
                out.push(Tok::Num(v));
            }
            continue;
        }
        if c != '#' {
            text.push(c);
        }
        i += 1;
    }
    flush(&mut text, &mut out);
    out
}

/// Whether two disassemblies say the same thing.
pub fn same(a: &str, b: &str) -> bool {
    let (x, y) = (tokens(a), tokens(b));
    x.len() == y.len()
        && x.iter().zip(&y).all(|(p, q)| match (p, q) {
            (Tok::Text(s), Tok::Text(t)) => s == t,
            (Tok::Num(m), Tok::Num(n)) => m == n || (m & 0xffff_ffff) == (n & 0xffff_ffff),
            (Tok::Float(m), Tok::Float(n)) => m == n || (m.is_nan() && n.is_nan()),
            (Tok::Float(m), Tok::Num(n)) | (Tok::Num(n), Tok::Float(m)) => *m == *n as f64,
            _ => false,
        })
}

pub struct Group {
    pub count: usize,
    pub examples: Vec<String>,
    /// The oracle rejected every sample of this encoding: it does not implement it.
    pub oracle_unknown: bool,
    /// The other oracle agrees with us on every sample of the group.
    pub other_agrees: bool,
}

/// Disagreement class of one word.
fn class(mine: &Option<String>, theirs: &Option<String>) -> Option<&'static str> {
    match (mine, theirs) {
        (None, None) => None,
        (Some(_), None) => Some("valid-vs-invalid"),
        (None, Some(_)) => Some("invalid-vs-valid"),
        (Some(a), Some(b)) => {
            if same(a, b) {
                None
            } else {
                let ma = tokens(a);
                let mb = tokens(b);
                let mn = |t: &[Tok]| match t.first() {
                    Some(Tok::Text(s)) => s.split([' ', '.']).next().unwrap_or("").to_string(),
                    _ => String::new(),
                };
                if mn(&ma) != mn(&mb) {
                    Some("mnemonic")
                } else {
                    Some("operands")
                }
            }
        }
    }
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
}

/// Per SVE and SME encoding with operand support, `n` random fillings of the
/// variable bits plus all-zero and all-one. Each encoding has a generator of its
/// own, seeded from its name, so the samples of one encoding (and the base groups')
/// do not change as others gain operands.
pub fn sve_sample_words(n: u32) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        if !SVE_GROUPS.contains(&oc.group()) || !oc.has_operands() {
            continue;
        }
        let seed = oc.name().bytes().fold(0x5be5_0000_57e5_0001u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
        });
        let mut rng = Rng::new(seed);
        let var = !oc.mask();
        out.push((oc.value(), oc.name().to_string()));
        out.push((oc.value() | var, oc.name().to_string()));
        for _ in 0..n {
            out.push((oc.value() | (rng.next() & var), oc.name().to_string()));
        }
    }
    out
}

/// The words compared: per encoding in scope, `n` random fillings of the
/// variable bits plus all-zero and all-one; then random words.
pub fn sample_words(n: u32) -> Vec<(u32, String)> {
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
    let mut out = Vec::new();
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        if !GROUPS.contains(&oc.group()) {
            continue;
        }
        let var = !oc.mask();
        out.push((oc.value(), oc.name().to_string()));
        out.push((oc.value() | var, oc.name().to_string()));
        for _ in 0..n {
            out.push((oc.value() | (rng.next() & var), oc.name().to_string()));
        }
    }
    for _ in 0..n * 2000 {
        let w = rng.next();
        out.push((w, String::new()));
    }
    out
}

/// The encoding a word is filed under: ours when we decode it, else the one it was
/// sampled from; random words we reject are filed as `UNALLOCATED`. SVE and SME
/// words are in scope for an oracle that decodes them, when we decode their
/// operands.
fn file_under(oracle: Oracle, w: u32, sampled: &str) -> Option<String> {
    let i = decode_word(w, PC, &DecoderOptions::default());
    let name = i.op_code().name();
    if !i.is_invalid() {
        let g = i.op_code().group();
        let sve = oracle.has_sve() && SVE_GROUPS.contains(&g) && i.has_operands();
        return (GROUPS.contains(&g) || sve).then(|| name.to_string());
    }
    if !sampled.is_empty() {
        return Some(sampled.to_string());
    }
    // A random word we reject: only in scope when the oracle accepts it, and then
    // filed by the top-level group of its bits is unknown, so by `UNALLOCATED`.
    Some("UNALLOCATED".into())
}

pub fn compare_groups(oracle: Oracle, n: u32) -> BTreeMap<(String, String), Group> {
    let mut words = sample_words(n);
    if oracle.has_sve() {
        words.extend(sve_sample_words(n));
    }
    let (groups, accepted) = tally(oracle, words.into_iter());
    finish(groups, &accepted)
}

pub type Groups = BTreeMap<(String, String), Group>;

/// The disagreement groups of some words, and per encoding how many words the
/// oracle accepted.
pub fn tally(
    oracle: Oracle,
    words: impl Iterator<Item = (u32, String)>,
) -> (Groups, BTreeMap<String, usize>) {
    let mut groups: Groups = BTreeMap::new();
    let mut accepted: BTreeMap<String, usize> = BTreeMap::new();
    for (w, sampled) in words {
        let Some(enc) = file_under(oracle, w, &sampled) else {
            continue;
        };
        let mine = ours(w);
        let theirs = oracle.text(w);
        // Words we reject are grouped by what the oracle calls them.
        let enc = if enc == "UNALLOCATED" {
            let m = theirs
                .as_deref()
                .and_then(|t| t.split_whitespace().next())
                .unwrap_or("-")
                .to_ascii_lowercase();
            format!("UNALLOCATED:{m}")
        } else {
            enc
        };
        *accepted.entry(enc.clone()).or_default() += usize::from(theirs.is_some());
        if let Some(cl) = class(&mine, &theirs) {
            let unpred = decode_word(w, PC, &DecoderOptions::default()).is_unpredictable();
            let cl = if unpred && cl == "valid-vs-invalid" {
                "valid-vs-invalid:unpredictable"
            } else {
                cl
            };
            let g = groups.entry((enc, cl.to_string())).or_insert(Group {
                count: 0,
                examples: Vec::new(),
                oracle_unknown: false,
                other_agrees: true,
            });
            g.count += 1;
            let other = match oracle {
                Oracle::Farm64 => Oracle::Yaxpeax,
                Oracle::Yaxpeax => Oracle::Farm64,
            };
            if class(&mine, &other.text(w)).is_some() {
                g.other_agrees = false;
            }
            if g.examples.len() < 3 {
                g.examples.push(format!(
                    "{w:08x}: {} | {}",
                    mine.as_deref().unwrap_or("-"),
                    theirs.as_deref().unwrap_or("-")
                ));
            }
        }
    }
    (groups, accepted)
}

/// Merges tallies (from threads).
pub fn merge(
    into: &mut (Groups, BTreeMap<String, usize>),
    from: (Groups, BTreeMap<String, usize>),
) {
    for (k, g) in from.0 {
        let e = into.0.entry(k).or_insert(Group {
            count: 0,
            examples: Vec::new(),
            oracle_unknown: false,
            other_agrees: true,
        });
        e.count += g.count;
        e.other_agrees &= g.other_agrees;
        for x in g.examples {
            if e.examples.len() < 3 {
                e.examples.push(x);
            }
        }
    }
    for (k, n) in from.1 {
        *into.1.entry(k).or_default() += n;
    }
}

/// Marks the groups of encodings the oracle never accepted.
pub fn finish(mut groups: Groups, accepted: &BTreeMap<String, usize>) -> Groups {
    for ((enc, _), g) in groups.iter_mut() {
        g.oracle_unknown = accepted.get(enc) == Some(&0);
    }
    groups
}

/// `tests/triage/<oracle>.txt`: `<encoding> <class> <right side> <reason>`, one
/// group per line, `#` comments. The right side is `isa`, `oracle` or `both` (a
/// syntax difference that is not an error).
pub fn load_triage(oracle: Oracle) -> BTreeMap<(String, String), (String, String)> {
    load_triage_named(oracle.name())
}

/// A triage file by name (`tests/triage/<name>.txt`); empty when it is missing.
pub fn load_triage_named(name: &str) -> BTreeMap<(String, String), (String, String)> {
    let path = format!("{}/tests/triage/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.splitn(4, ' ');
        let (Some(enc), Some(cl), Some(side), reason) =
            (it.next(), it.next(), it.next(), it.next())
        else {
            panic!("{path}: malformed triage line: {line}");
        };
        assert!(
            matches!(side, "isa" | "oracle" | "both"),
            "{path}: side must be isa, oracle or both: {line}"
        );
        assert!(
            reason.is_some_and(|r| !r.trim().is_empty()),
            "{path}: no reason: {line}"
        );
        out.insert(
            (enc.to_string(), cl.to_string()),
            (side.to_string(), reason.unwrap_or("").to_string()),
        );
    }
    out
}

/// Replaces `from` where it is a whole word (also after `.`, as in `b.cs`).
fn replace_word(s: &str, from: &str, to: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let before_ok = i == 0 || !(b[i - 1] as char).is_ascii_alphanumeric();
        let after = i + from.len();
        if before_ok
            && s[i..].starts_with(from)
            && (after == b.len() || !(b[after] as char).is_ascii_alphanumeric())
        {
            out.push_str(to);
            i = after;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}
