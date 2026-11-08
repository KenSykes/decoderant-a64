//! `CodeAssembler`: assembling from operands gives the expected words, and every
//! word decodes back to the same text.

use decoderant_a64::code_asm::{CodeAssembler, registers::*};
use decoderant_a64::{
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
        use decoderant_a64::Mnemonic as M;
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

/// SVE: registers with element sizes, governing predicates, indexed elements and
/// the immediate forms, built from operands.
#[test]
fn sve() -> Result<(), EncodeError> {
    use decoderant_a64::PredQual as Q;
    let z = O::vec;
    let mut a = CodeAssembler::new(0x1000);
    a.add_4(
        z(z23, A::B),
        O::pred(p1, Q::Merging),
        z(z23, A::B),
        z(z12, A::B),
    )?
    .movprfx_3(z(z21, A::D), O::pred(p6, Q::Zeroing), z(z22, A::D))?
    .sdot(z(z11, A::S), z(z20, A::B), O::elem(z0, A::B, 2))?
    .cmpge(z(p12, A::D), O::pred(p2, Q::Zeroing), z(z10, A::D), -7i64)?
    .subr(z(z11, A::H), z(z11, A::H), O::imm_lsl(6, 8))?
    .dupm(z(z0, A::S), O::bitmask(1))?
    .and_4(
        z(p12, A::B),
        O::pred(p10, Q::Zeroing),
        z(p6, A::B),
        z(p8, A::B),
    )?
    .ptrue(z(pn8, A::B))?;
    assert_eq!(
        a.words(),
        [
            0x0400_0597,
            0x04d0_3ad5,
            0x44b0_028b,
            0x25d9_094c,
            0x2563_e0cb,
            0x05c0_0000,
            0x2508_68cc,
            0x2520_7810,
        ]
    );
    for (k, &w) in a.words().iter().enumerate() {
        assert!(
            !text(w, 0x1000 + 4 * k as u64).starts_with(".inst"),
            "{w:08x}"
        );
    }
    // The `MOV` alias of `DUPM` when no `DUP` gives the pattern; a merging `MOV`
    // (`SEL` with the destination as the second source).
    let mut b = CodeAssembler::new(0);
    b.mov(z(z22, A::D), O::bitmask(0xffff_f00f_ffff_ffff))?;
    assert_eq!(text(b.words()[0], 0), "mov z22.d, #0xfffff00fffffffff");
    // A predicate where a vector is expected, or the wrong qualifier, is rejected.
    let mut c = CodeAssembler::new(0);
    assert!(
        c.movprfx_3(z(z0, A::D), O::pred(p0, Q::Merging), z(p1, A::D))
            .is_err()
    );
    assert!(
        c.cmpge(z(p1, A::D), O::pred(p2, Q::Merging), z(z1, A::D), 1i64)
            .is_err()
    );
    Ok(())
}

/// SVE memory operands, patterns and register lists, built from operands.
#[test]
fn sve_memory_and_patterns() -> Result<(), EncodeError> {
    use decoderant_a64::PredQual as Q;
    let z = O::vec;
    let zl = |r, a| O::list(r, 1, a);
    let mut a = CodeAssembler::new(0x1000);
    a.ld1w(zl(z0, A::S), O::pred(p0, Q::Zeroing), O::mem(x0))?
        .ld1w(
            zl(z21, A::S),
            O::pred(p2, Q::Zeroing),
            O::mem_vec(O::mem_index(x6, z17, ExtendKind::Uxtw, Some(2)), A::S),
        )?
        .ld4w(O::list(z26, 4, A::S), O::pred(p5, Q::Zeroing), O::mem(x19))?
        .ldr(z12, O::mem_mul_vl(sp, -0x6e))?
        .ldff1b(zl(z31, A::B), O::pred(p7, Q::Zeroing), O::mem(sp))?
        .prfb(O::prefetch(0), p0, O::mem_vec(O::mem_off(z22, 1), A::S))?
        .cntb(x13, O::named(16), O::imm(15))?
        .cntb(x0, O::named(31), O::imm(1))?
        .ptrue_2(z(p0, A::S), O::named(31))?;
    assert_eq!(
        a.words(),
        [
            0xa540_a000,
            0x8531_48d5,
            0xa560_f67a,
            0x85b2_4bec,
            0xa41f_7fff,
            0x8401_e2c0,
            0x042e_e20d,
            0x0420_e3e0,
            0x2598_e3e0,
        ]
    );
    assert_eq!(text(a.words()[7], 0), "cntb x0");
    // `MUL VL` offsets of a list are multiples of its length; a vector index needs
    // its element size.
    let mut b = CodeAssembler::new(0);
    assert!(
        b.ld4w(
            O::list(z0, 4, A::S),
            O::pred(p0, Q::Zeroing),
            O::mem_mul_vl(x0, 2)
        )
        .is_err()
    );
    assert!(
        b.ld1w(
            zl(z0, A::S),
            O::pred(p0, Q::Zeroing),
            O::mem_index(x0, z1, ExtendKind::Uxtw, Some(2))
        )
        .is_err()
    );
    Ok(())
}

/// Multi-vector, strided and predicate-pair lists, and `PSEL`.
#[test]
fn sve_lists() -> Result<(), EncodeError> {
    use decoderant_a64::PredQual as Q;
    let mut a = CodeAssembler::new(0);
    let sel = O::elem_sel(p11, A::S, w13, 1);
    a.add(
        O::list(z20, 4, A::S),
        O::list(z20, 4, A::S),
        O::vec(z12, A::S),
    )?
    .ld1b(
        O::list_strided(z2, 4, 4, A::B),
        O::pred(pn12, Q::Zeroing),
        O::mem_index(x26, x17, ExtendKind::Lsl, None),
    )?
    .whilege(O::list(p0, 2, A::H), x0, x27)?
    .psel(p2, p4, sel)?;
    assert_eq!(
        a.words(),
        [0xc1ac_ab14, 0xa111_9342, 0x257b_5010, 0x2571_5162]
    );
    // A strided list cannot start at a register its stride does not allow.
    let mut b = CodeAssembler::new(0);
    assert!(
        b.ld1b(
            O::list_strided(z5, 4, 4, A::B),
            O::pred(pn12, Q::Zeroing),
            O::mem_index(x26, x17, ExtendKind::Lsl, None),
        )
        .is_err()
    );
    Ok(())
}

/// SME operands built from operands: tiles, slices, array vectors, ZT0, the mask.
#[test]
fn sme() -> Result<(), EncodeError> {
    use decoderant_a64::{PredQual as Q, Register as R};
    let mut a = CodeAssembler::new(0);
    a.fmopa(
        O::za_tile(R::ZAS0),
        O::pred(p1, Q::Merging),
        O::pred(p5, Q::Merging),
        O::vec(z18, A::S),
        O::vec(z0, A::S),
    )?
    .mov_3(
        O::za_slice(R::ZAB0, false, w12, 7, 1),
        O::pred(p3, Q::Merging),
        O::vec(z25, A::B),
    )?
    .ld1d(
        O::za_slice(R::ZAD2, true, w13, 1, 1).braced(),
        O::pred(p6, Q::Zeroing),
        O::mem_index(x18, x2, ExtendKind::Lsl, Some(3)),
    )?
    .ldr(O::za_array(A::None, w15, 3, 1, 0), O::mem_mul_vl(x8, 3))?
    .fmla(
        O::za_array(A::S, w9, 7, 1, 2),
        O::list(z8, 2, A::S),
        O::elem(z10, A::S, 3),
    )?
    .zero(O::za_mask(0xc6))?
    .movt(xzr, O::elem(R::ZT0, A::None, 56))?;
    assert_eq!(
        a.words(),
        [
            0x8080_a640,
            0xc000_0f27,
            0xe0c2_ba45,
            0xe100_6103,
            0xc15a_2d07,
            0xc008_00c6,
            0xc04c_73ff
        ]
    );
    // The two offsets of `LDR ZA` share a field: they must agree.
    let mut b = CodeAssembler::new(0);
    assert!(
        b.ldr(O::za_array(A::None, w15, 3, 1, 0), O::mem_mul_vl(x8, 2))
            .is_err()
    );
    // A ZT0 byte offset is a multiple of 8.
    assert!(b.movt(xzr, O::elem(R::ZT0, A::None, 5)).is_err());
    Ok(())
}

#[test]
fn errors() {
    let a = CodeAssembler::new(0x1000);
    use decoderant_a64::Mnemonic as M;
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

/// A 32-bit `MOV` immediate with higher bits set is an error, not truncated to
/// the low word (`mov w3, #0x1_0000_0001` is not `mov w3, #1`).
#[test]
fn mov_w_immediates_are_not_truncated() {
    use decoderant_a64::Mnemonic as M;
    let a = CodeAssembler::new(0x1000);
    let mov = |v: u64| a.encode(M::Mov, "", &[w3.into(), O::imm(v)]);
    for (v, want) in [
        (0x1234, "mov w3, #0x1234"),
        (0xffff_0000, "mov w3, #0xffff0000"),
        (0xffff_fffe, "mov w3, #0xfffffffe"),
        (0x00ff_00ff, "mov w3, #0xff00ff"),
    ] {
        assert_eq!(text(mov(v).unwrap(), 0x1000), want);
    }
    for v in [0x1_0000_0001, 0xffff_ffff_ffff_0000, 0x1_00ff_00ff] {
        assert!(
            mov(v).is_err(),
            "{v:#x}: {:?}",
            mov(v).map(|w| text(w, 0x1000))
        );
    }
}
