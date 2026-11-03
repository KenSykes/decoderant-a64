//! `CodeAssembler`: assembling from operands gives the expected words, and every
//! word decodes back to the same text.

#![cfg(feature = "code_asm")]

use aarch64_isa::code_asm::{CodeAssembler, registers::*};
use aarch64_isa::{
    Arrangement as A, ConditionCode, DecoderOptions, EncodeError, ExtendKind, Operand as O,
    ShiftKind, decode_word, sys_encoding,
};

fn text(w: u32, pc: u64) -> String {
    decode_word(w, pc, &DecoderOptions::default()).to_string()
}

#[test]
fn handwritten_set() -> Result<(), EncodeError> {
    let mut a = CodeAssembler::new(0x1000);
    a.add(x0, x1, x2)?
        .adds(x0, x1, x2)?
        .subs(x0, x1, x2)?
        .adds(w0, w1, w2)?
        .subs(w0, w1, w2)?
        .ldr(x0, O::mem(x1))?
        .ldr(x0, O::mem_off(x1, 8))?
        .ldrb(w0, O::mem(x1))?
        .str(x0, O::mem(x1))?
        .str(w2, O::mem_off(x1, 4))?
        .fadd(d0, d1, d2)?
        .nop()?
        .udf(0u32)?;
    assert_eq!(
        a.words(),
        [
            0x8b02_0020,
            0xab02_0020,
            0xeb02_0020,
            0x2b02_0020,
            0x6b02_0020,
            0xf940_0020,
            0xf940_0420,
            0x3940_0020,
            0xf900_0020,
            0xb900_0422,
            0x1e62_2820,
            0xd503_201f,
            0x0000_0000,
        ]
    );
    // Branches take absolute targets.
    let mut b = CodeAssembler::new(0x1000);
    b.b(O::label(0x1004))?
        .b(O::label(0x1010))?
        .b(O::label(0x1008))?
        .brk(0xf003u32)?;
    assert_eq!(
        b.words(),
        [0x1400_0001, 0x1400_0003, 0x1400_0000, 0xd43e_0060]
    );
    Ok(())
}

#[test]
fn aliases_and_forms() -> Result<(), EncodeError> {
    let cases: Vec<(Result<u32, EncodeError>, &str)> = {
        let a = CodeAssembler::new(0x1000);
        use aarch64_isa::Mnemonic as M;
        vec![
            (a.encode(M::Mov, "", &[x0.into(), x1.into()]), "mov x0, x1"),
            (a.encode(M::Mov, "", &[sp.into(), x1.into()]), "mov sp, x1"),
            (
                a.encode(M::Mov, "", &[x3.into(), 0x1234u64.into()]),
                "mov x3, #0x1234",
            ),
            (
                a.encode(M::Mov, "", &[x3.into(), O::imm(0xffff_ffff_ffff_fffe)]),
                "mov x3, #0xfffffffffffffffe",
            ),
            (
                a.encode(M::Mov, "", &[x3.into(), O::imm(0x00ff_00ff_00ff_00ff)]),
                "mov x3, #0xff00ff00ff00ff",
            ),
            (
                a.encode(M::Cmp, "", &[x1.into(), 1u64.into()]),
                "cmp x1, #1",
            ),
            (a.encode(M::Cmp, "", &[w1.into(), w2.into()]), "cmp w1, w2"),
            (
                a.encode(M::Lsl, "", &[x0.into(), x1.into(), 3u64.into()]),
                "lsl x0, x1, #3",
            ),
            (
                a.encode(M::Lsr, "", &[w0.into(), w1.into(), 31u64.into()]),
                "lsr w0, w1, #0x1f",
            ),
            (
                a.encode(
                    M::Ubfx,
                    "",
                    &[x0.into(), x1.into(), 4u64.into(), 8u64.into()],
                ),
                "ubfx x0, x1, #4, #8",
            ),
            (
                a.encode(M::Add, "", &[x0.into(), sp.into(), x1.into()]),
                "add x0, sp, x1",
            ),
            (
                a.encode(
                    M::Add,
                    "",
                    &[x0.into(), x1.into(), O::shifted(x2, ShiftKind::Lsl, 3)],
                ),
                "add x0, x1, x2, lsl #3",
            ),
            (
                a.encode(
                    M::Add,
                    "",
                    &[x0.into(), x1.into(), O::extended(w2, ExtendKind::Sxtw, 2)],
                ),
                "add x0, x1, w2, sxtw #2",
            ),
            (
                a.encode(M::Add, "", &[x0.into(), x1.into(), O::imm_lsl(1, 12)]),
                "add x0, x1, #1, lsl #0xc",
            ),
            (
                a.encode(M::And, "", &[x0.into(), x1.into(), O::bitmask(0xff)]),
                "and x0, x1, #0xff",
            ),
            (
                a.encode(M::And, "", &[x0.into(), x1.into(), 0xffu64.into()]),
                "and x0, x1, #0xff",
            ),
            (
                a.encode(M::Ldr, "", &[x0.into(), O::mem_pre(sp, -16)]),
                "ldr x0, [sp, #-0x10]!",
            ),
            (
                a.encode(M::Ldr, "", &[x0.into(), O::mem_post(x1, 8)]),
                "ldr x0, [x1], #8",
            ),
            (
                a.encode(M::Ldur, "", &[x0.into(), O::mem_off(x1, -1)]),
                "ldur x0, [x1, #-1]",
            ),
            (
                a.encode(
                    M::Ldr,
                    "",
                    &[x0.into(), O::mem_index(x1, x2, ExtendKind::Lsl, Some(3))],
                ),
                "ldr x0, [x1, x2, lsl #3]",
            ),
            (
                a.encode(
                    M::Ldr,
                    "",
                    &[w0.into(), O::mem_index(x1, w2, ExtendKind::Uxtw, None)],
                ),
                "ldr w0, [x1, w2, uxtw]",
            ),
            (
                a.encode(M::Stp, "", &[x29.into(), x30.into(), O::mem_pre(sp, -16)]),
                "stp x29, x30, [sp, #-0x10]!",
            ),
            (
                a.encode(M::Ldp, "", &[x29.into(), x30.into(), O::mem_post(sp, 16)]),
                "ldp x29, x30, [sp], #0x10",
            ),
            (
                a.encode(M::Ldr, "", &[x0.into(), O::label(0x1100)]),
                "ldr x0, 0x1100",
            ),
            (
                a.encode(M::Adr, "", &[x0.into(), O::label(0x1010)]),
                "adr x0, 0x1010",
            ),
            (
                a.encode(M::Adrp, "", &[x0.into(), O::label(0x5000)]),
                "adrp x0, 0x5000",
            ),
            (
                a.encode(M::B, "", &[ConditionCode::Ne.into(), O::label(0x1008)]),
                "b.ne 0x1008",
            ),
            (
                a.encode(M::Cbz, "", &[x5.into(), O::label(0x0ffc)]),
                "cbz x5, 0xffc",
            ),
            (
                a.encode(M::Tbnz, "", &[w5.into(), 3u64.into(), O::label(0x1020)]),
                "tbnz w5, #3, 0x1020",
            ),
            (
                a.encode(
                    M::Csel,
                    "",
                    &[x0.into(), x1.into(), x2.into(), ConditionCode::Eq.into()],
                ),
                "csel x0, x1, x2, eq",
            ),
            (
                a.encode(M::Cset, "", &[w0.into(), ConditionCode::Lt.into()]),
                "cset w0, lt",
            ),
            (a.encode(M::Ret, "", &[x30.into()]), "ret"),
            (a.encode(M::Ret, "", &[x1.into()]), "ret x1"),
            (
                a.encode(
                    M::Mrs,
                    "",
                    &[x0.into(), O::sysreg(sys_encoding(3, 3, 4, 2, 0))],
                ),
                "mrs x0, nzcv",
            ),
            (
                a.encode(
                    M::Msr,
                    "",
                    &[O::sysreg(sys_encoding(3, 3, 4, 4, 0)), x1.into()],
                ),
                "msr fpcr, x1",
            ),
            (a.encode(M::Dmb, "", &[O::barrier(11)]), "dmb ish"),
            (
                a.encode(
                    M::Add,
                    "",
                    &[O::vec(v0, A::S4), O::vec(v1, A::S4), O::vec(v2, A::S4)],
                ),
                "add v0.4s, v1.4s, v2.4s",
            ),
            (
                a.encode(
                    M::Fmla,
                    "",
                    &[O::vec(v0, A::S4), O::vec(v1, A::S4), O::elem(v2, A::S, 3)],
                ),
                "fmla v0.4s, v1.4s, v2.s[3]",
            ),
            (
                a.encode(
                    M::Saddl,
                    "2",
                    &[O::vec(v0, A::H8), O::vec(v1, A::B16), O::vec(v2, A::B16)],
                ),
                "saddl2 v0.8h, v1.16b, v2.16b",
            ),
            (
                a.encode(M::Ld1, "", &[O::list(v0, 2, A::B16), O::mem(x0)]),
                "ld1 { v0.16b, v1.16b }, [x0]",
            ),
            (
                a.encode(M::Ld1, "", &[O::list(v0, 2, A::B16), O::mem_post(x0, 32)]),
                "ld1 { v0.16b, v1.16b }, [x0], #0x20",
            ),
            (
                a.encode(M::Ld1, "", &[O::list_elem(v3, 1, A::S, 2), O::mem(x0)]),
                "ld1 { v3.s }[2], [x0]",
            ),
            (
                a.encode(M::Movi, "", &[O::vec(v0, A::S4), O::imm_lsl(0xab, 8)]),
                "movi v0.4s, #0xab, lsl #8",
            ),
            (
                a.encode(M::Fmov, "", &[d0.into(), 1.0f64.into()]),
                "fmov d0, #1.0",
            ),
            (
                a.encode(
                    M::Shl,
                    "",
                    &[O::vec(v0, A::H8), O::vec(v1, A::H8), 3u64.into()],
                ),
                "shl v0.8h, v1.8h, #3",
            ),
            (
                a.encode(
                    M::Sshr,
                    "",
                    &[O::vec(v0, A::S2), O::vec(v1, A::S2), 32u64.into()],
                ),
                "sshr v0.2s, v1.2s, #0x20",
            ),
            (
                a.encode(M::Mov, "", &[O::elem(v4, A::H, 1), w26.into()]),
                "mov v4.h[1], w26",
            ),
            (
                a.encode(M::Umov, "", &[w0.into(), O::elem(v1, A::B, 7)]),
                "umov w0, v1.b[7]",
            ),
        ]
    };
    for (w, want) in cases {
        let w = w.unwrap_or_else(|e| panic!("{want}: {e}"));
        assert_eq!(text(w, 0x1000), want, "{w:08x}");
    }
    Ok(())
}

#[test]
fn errors() {
    let a = CodeAssembler::new(0x1000);
    use aarch64_isa::Mnemonic as M;
    // No such form.
    assert!(
        a.encode(M::Add, "", &[x0.into(), O::vec(v0, A::S4), x1.into()])
            .is_err()
    );
    // Out of range: ADD (immediate) takes 12 bits.
    assert!(
        a.encode(M::Add, "", &[x0.into(), x1.into(), 0x1001u64.into()])
            .is_err()
    );
    // A misaligned branch target.
    assert!(a.encode(M::B, "", &[O::label(0x1002)]).is_err());
    // Not a bitmask.
    assert!(
        a.encode(M::And, "", &[x0.into(), x1.into(), O::bitmask(0x1234)])
            .is_err()
    );
    // 32-bit shift amounts stop at 31.
    assert!(
        a.encode(
            M::Add,
            "",
            &[w0.into(), w1.into(), O::shifted(w2, ShiftKind::Lsl, 32)]
        )
        .is_err()
    );
}
