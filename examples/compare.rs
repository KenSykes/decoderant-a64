//! Prints sample disagreements with fARM64 and yaxpeax-arm, for development.
//! `cargo run --release --example compare -- [samples-per-code] [filter]`
use decoderant_a64::{Code, DecoderOptions, decode_word};
use fARM64::format::FmtFormatter;
use yaxpeax_arch::{Decoder as _, U8Reader};

fn farm(w: u32) -> Option<String> {
    let b = w.to_le_bytes();
    let i = fARM64::Decoder::new(&b, 0x1000, fARM64::DecoderOptions::default()).decode();
    if i.is_invalid() {
        return None;
    }
    Some(fARM64::format::format_to_string(&FmtFormatter::new(), &i))
}

fn yax(w: u32) -> Option<String> {
    let b = w.to_le_bytes();
    let d = yaxpeax_arm::armv8::a64::InstDecoder::default();
    let mut r = U8Reader::new(&b);
    d.decode(&mut r).ok().map(|i| i.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let filter = args.get(2).cloned().unwrap_or_default();
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as u32
    };
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        if !matches!(
            oc.group(),
            "control" | "dpimm" | "dpreg" | "ldst" | "simd_dp"
        ) {
            continue;
        }
        if !oc.name().contains(&filter) {
            continue;
        }
        for _ in 0..n {
            let w = oc.value() | (rnd() & !oc.mask());
            let i = decode_word(w, 0x1000, &DecoderOptions::default());
            let mine = (!i.is_invalid()).then(|| i.to_string());
            let f = farm(w);
            let y = yax(w);
            println!(
                "{w:08x} {:<28} | {:<40} | {:<40} | {}",
                oc.name(),
                mine.unwrap_or("-".into()),
                f.unwrap_or("-".into()),
                y.unwrap_or("-".into())
            );
        }
    }
}
