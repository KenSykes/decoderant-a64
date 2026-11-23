//! Binary for measuring what the formatter adds: build it with and without the
//! `formatter` feature and compare sizes (`scripts/formatter-size.ps1`).
use decoderant_a64::{DecoderOptions, decode_word};

fn main() {
    for a in std::env::args().skip(1) {
        let w = u32::from_str_radix(a.trim_start_matches("0x"), 16).expect("hex word");
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        #[cfg(feature = "formatter")]
        println!("{w:08x}  {i}");
        #[cfg(not(feature = "formatter"))]
        println!("{w:08x}  {}", i.code() as u32);
    }
}
