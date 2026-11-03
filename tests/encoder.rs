//! Encoder round trips, sampled per `Code` (the exhaustive version is in
//! `sweep.rs`):
//!
//! - bit-exact: every valid word re-encodes to itself (operands, should-be bits and
//!   don't-care bits);
//! - from operands: with the don't-care bits dropped, the encoded word decodes to
//!   the same encoding and operands, so the encoder does not lean on kept bits.

mod common;

use aarch64_isa::{Code, DecoderOptions, decode_word};
use common::Rng;

/// Checks one word; `Err` describes the failure.
pub fn check(w: u32) -> Result<(), String> {
    let opts = DecoderOptions::default();
    let i = decode_word(w, 0x4000, &opts);
    if i.is_invalid() || !i.has_operands() {
        return Ok(());
    }
    let e = i
        .encode()
        .map_err(|e| format!("{w:08x} {}: {e} ({i})", i.op_code().name()))?;
    if e != w {
        return Err(format!(
            "{w:08x} {}: re-encodes to {e:08x} ({i})",
            i.op_code().name()
        ));
    }
    let fresh = i.without_kept_bits();
    let e2 = fresh
        .encode()
        .map_err(|e| format!("{w:08x} {}: from operands: {e}", i.op_code().name()))?;
    let j = decode_word(e2, 0x4000, &opts);
    if j.code() != i.code() || j.operands() != i.operands() {
        return Err(format!(
            "{w:08x} {}: from operands gives {e2:08x} = {} ({j}), not {i}",
            i.op_code().name(),
            j.op_code().name()
        ));
    }
    Ok(())
}

#[test]
fn round_trip_per_code() {
    let mut rng = Rng::new(0x5eed);
    let mut fails = Vec::new();
    let mut n = 0;
    let mut total = 0;
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        if !oc.has_operands() {
            continue;
        }
        let var = !oc.mask();
        for k in 0..66 {
            let fill = match k {
                0 => 0,
                1 => var,
                _ => rng.next() & var,
            };
            n += 1;
            if let Err(e) = check(oc.value() | fill) {
                total += 1;
                if fails.len() < 40 {
                    fails.push(e);
                }
            }
        }
    }
    assert!(
        fails.is_empty(),
        "{total} failures over {n} words, e.g.:\n{}",
        fails.join("\n")
    );
}

#[test]
fn build_and_edit() {
    use aarch64_isa::{Instruction, Operand, Register as R};
    // ADD X0, X1, X2 from operands.
    let mut i = Instruction::with(
        Code::Add_64_addsub_shift,
        &[
            Operand::reg(R::X0),
            Operand::reg(R::X1),
            Operand::shifted(R::X2, aarch64_isa::ShiftKind::Lsl, 0),
        ],
        0x1000,
    )
    .unwrap();
    assert_eq!(i.encode().unwrap(), 0x8b02_0020);
    assert_eq!(i.to_string(), "add x0, x1, x2");
    // Edit a register; the alias choice follows (ADD X0, XZR... is still ADD).
    i.set_op(1, Operand::reg(R::X5)).unwrap();
    assert_eq!(i.to_string(), "add x0, x5, x2");
    assert_eq!(
        decode_word(i.encode().unwrap(), 0x1000, &DecoderOptions::default()).to_string(),
        "add x0, x5, x2"
    );
    // An unencodable edit leaves it unchanged.
    assert!(i.set_op(0, Operand::reg(R::SP)).is_err());
    assert_eq!(i.to_string(), "add x0, x5, x2");
    // A branch moved to a new PC keeps its absolute target.
    let mut b = decode_word(0x1400_0004, 0x1000, &DecoderOptions::default());
    assert_eq!(b.label_target(), Some(0x1010));
    b.set_pc(0x2000);
    assert_eq!(b.encode().unwrap(), 0x17ff_fc04);
}
