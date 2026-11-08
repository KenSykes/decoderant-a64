//! Encoder round trips, sampled per `Code` (the exhaustive version is in
//! `sweep.rs`):
//!
//! - bit-exact: every valid word re-encodes to itself (operands, should-be bits and
//!   don't-care bits);
//! - from operands: with the don't-care bits dropped, the encoded word decodes to
//!   the same encoding and operands, so the encoder does not lean on kept bits.

mod common;

use common::Rng;
use decoderant_a64::{Code, DecoderOptions, decode_word};

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
    use decoderant_a64::{Instruction, Operand, Register as R};
    // ADD X0, X1, X2 from operands.
    let mut i = Instruction::with(
        Code::Add_64_addsub_shift,
        &[
            Operand::reg(R::X0),
            Operand::reg(R::X1),
            Operand::shifted(R::X2, decoderant_a64::ShiftKind::Lsl, 0),
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

/// Operand values the encoding cannot hold are errors, never truncated or taken
/// for another register class.
#[test]
fn rejects_what_does_not_fit() {
    use decoderant_a64::{Arrangement as A, Instruction, Operand, Register as R};
    let opts = DecoderOptions::default();
    // AND W0, W1, #imm: a 32-bit logical immediate with higher bits set.
    let and = |imm| {
        Instruction::with(
            Code::And_32_log_imm,
            &[
                Operand::reg(R::W0),
                Operand::reg(R::W1),
                Operand::bitmask(imm),
            ],
            0,
        )
    };
    assert_eq!(and(0xff).unwrap().to_string(), "and w0, w1, #0xff");
    assert!(and(0x1_0000_00ff).is_err());
    // LD1 { V3.16B }, [X1]: a list of general registers is not a vector list.
    let ld1 = |first| {
        Instruction::with(
            Code::Ld1_asisdlse_R1_1v,
            &[Operand::list(first, 1, A::B16), Operand::mem(R::X1)],
            0,
        )
    };
    assert_eq!(ld1(R::V3).unwrap().to_string(), "ld1 { v3.16b }, [x1]");
    assert!(ld1(R::X3).is_err());
    // PTRUE P0.B, #imm: a pattern number above 32 bits.
    let mut p = decode_word(0x2518_e3e0, 0, &opts);
    assert_eq!(p.to_string(), "ptrue p0.b");
    p.set_op(1, Operand::imm(14)).unwrap();
    assert_eq!(p.to_string(), "ptrue p0.b, #0xe");
    assert!(p.set_op(1, Operand::imm(1 << 32 | 14)).is_err());
    assert_eq!(p.to_string(), "ptrue p0.b, #0xe");
}

/// Moving an instruction keeps its absolute label target in the text (the
/// alias forms are a unit test of the formatter: none is preferred in this
/// release).
#[test]
fn set_pc_keeps_label_targets() {
    let opts = DecoderOptions::default();
    let mut rng = Rng::new(0x1abe1);
    let mut n = 0;
    for c in Code::values().skip(1) {
        let oc = c.op_code();
        for _ in 0..16 {
            let mut i = decode_word(oc.value() | rng.next() & !oc.mask(), 0x1000, &opts);
            if i.code() != c || i.label_target().is_none() {
                continue;
            }
            let text = i.to_string();
            i.set_pc(0x80_0000);
            assert_eq!(i.to_string(), text, "{}", oc.name());
            n += 1;
        }
    }
    assert!(n > 100, "{n} instructions");
}
