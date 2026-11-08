//! Disassembles words given on the command line: `cargo run --example dis -- 8b020020`.
use decoderant_a64::{DecoderOptions, decode_word};

fn main() {
    for a in std::env::args().skip(1) {
        let w = u32::from_str_radix(a.trim_start_matches("0x"), 16).expect("hex word");
        let i = decode_word(w, 0x1000, &DecoderOptions::default());
        println!(
            "{w:08x}  {:<28} {}  {}",
            i.to_string(),
            i.op_code().name(),
            i.op_code().template()
        );
    }
}
