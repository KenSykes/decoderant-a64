# aarch64-isa-gen report

@generated from AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09. Regenerate with aarch64-isa-gen; review the diff when the pin changes.

## Encodings

4429 A64 encodings, 5219 decode-tree nodes, 581 condition expressions, 408 features, 1689 mnemonics, 3218 system registers and operations.

Decode dispatch: 25892 slots, 5168 candidate entries; over all words, 1.59 tables and 0.82 candidates on average, at most 3 candidates.

| Group | Encodings | With operand support |
|---|---|---|
| control | 163 | 163 |
| dpimm | 44 | 44 |
| dpreg | 151 | 151 |
| ldst | 1015 | 1015 |
| reserved | 1 | 1 |
| simd_dp | 867 | 867 |
| sme | 842 | 0 |
| sve | 1346 | 846 |

129 encodings overlap a higher-priority encoding (listed in `OpCodeInfo::precedence()`).

## Operand mapping

3087 encodings with operand support, 1342 without. 2650 symbol value tables taken from the spec's rule names; 107 distinct hand rules (symbol meanings the spec does not give in machine-readable form), used 8824 times.

### Hand rules

| Rule | Uses |
|---|---|
| `<fbits> 64-scale` | 24 |
| `<fbits> immh:immb` | 8 |
| `<imm16> unsigned` | 1 |
| `<imm6> unsigned` | 1 |
| `<imm8> unsigned` | 9 |
| `<imm> signed` | 12 |
| `<imm> unsigned` | 53 |
| `<immr> unsigned` | 6 |
| `<imms> unsigned` | 6 |
| `<lsb> unsigned` | 8 |
| `<mask> unsigned` | 1 |
| `<nzcv> unsigned` | 14 |
| `<op1> unsigned` | 3 |
| `<op2> unsigned` | 3 |
| `<reason> unsigned` | 46 |
| `<shift> hw*16` | 6 |
| `<shift> immh:immb left` | 12 |
| `<shift> immh:immb right` | 32 |
| `<shift> unsigned` | 7 |
| `<simm> signed` | 4 |
| `<uimm4> unsigned` | 2 |
| `<uimm6> tag granules` | 2 |
| `<uimm> unsigned` | 4 |
| `AdvSIMD 64-bit byte-mask immediate` | 2 |
| `AdvSIMD FP immediate` | 3 |
| `AdvSIMD imm8` | 2 |
| `CRn/CRm` | 6 |
| `CTERM register width: X iff sz = 1` | 4 |
| `EXT index imm4` | 1 |
| `FCMLA by element index` | 1 |
| `FP immediate` | 3 |
| `INS/DUP register width: X iff imm5 = x1000` | 3 |
| `LDRAA offset *8` | 4 |
| `LSL-only shift` | 2 |
| `MSR (immediate) value in CRm` | 1 |
| `NB on the variable op1 bit` | 6 |
| `PSTATE field` | 1 |
| `Q table in alternative order` | 2 |
| `SMSTART/SMSTOP option` | 2 |
| `SVE FP constant pair` | 8 |
| `SVE FP immediate` | 4 |
| `SVE element size from tsz (one-hot) or imm13` | 113 |
| `SVE immediate, signed as the spec's number token says` | 39 |
| `SVE index above the lowest set bit of tsz` | 3 |
| `SVE index fields` | 105 |
| `SVE indexed register <Z..>` | 108 |
| `SVE logical immediate (one element)` | 8 |
| `SVE register <P..>` | 643 |
| `SVE register <PN..>` | 1 |
| `SVE register <Z..>` | 1818 |
| `SVE shift tsz:imm3 left` | 10 |
| `SVE shift tsz:imm3 right` | 29 |
| `SVE size table: element size code, others RESERVED` | 527 |
| `SVE table: alternatives in allowed-value order` | 685 |
| `alias -immr MOD datasize` | 8 |
| `alias LSL shift (immr and imms)` | 2 |
| `alias MOV wide immediate` | 4 |
| `alias immediate +1` | 8 |
| `alias immediate -1` | 8 |
| `alias inverted condition` | 10 |
| `alias width imms+1` | 8 |
| `alias width imms+1-immr` | 6 |
| `barrier options` | 3 |
| `barrier options nXS` | 1 |
| `bitmask immediate` | 12 |
| `byte register offset LSL #0 on S` | 12 |
| `cond suffix` | 3 |
| `condition` | 25 |
| `extend register width: X iff option<1:0> = 11` | 6 |
| `extended register` | 12 |
| `fixed register` | 1 |
| `fixed register size` | 4 |
| `fp register <letter><n>` | 84 |
| `hint name on the variable op2 bit` | 2 |
| `index H:L:M:Rm<3>` | 6 |
| `index by element size` | 31 |
| `index fixed fields` | 29 |
| `index imm4 by imm5` | 2 |
| `index imm5` | 13 |
| `index ldst single` | 98 |
| `label *4` | 77 |
| `label ADR/ADRP` | 2 |
| `label PC - imm16*4` | 4 |
| `ldsttags offset*16` | 13 |
| `memory writeback, no offset` | 228 |
| `optional operand default` | 18 |
| `pair offset scaled` | 62 |
| `pimm scaled by access size` | 24 |
| `post-index` | 147 |
| `prefetch operations` | 5 |
| `register <B..>` | 14 |
| `register <D..>` | 266 |
| `register <H..>` | 353 |
| `register <Q..>` | 56 |
| `register <S..>` | 268 |
| `register <W..>` | 877 |
| `register <WSp..>` | 46 |
| `register <X..>` | 938 |
| `register <XSp..>` | 96 |
| `register list` | 164 |
| `register offset W/X by option<0>` | 24 |
| `register pair from one field` | 2 |
| `register writeback` | 132 |
| `system operation name` | 10 |
| `system register` | 4 |
| `unscaled signed offset` | 106 |
| `vector element` | 82 |

### Encodings without operand support, by reason

- 136 × SVE/SME register list ⟪ <Zt>.D ⟫ (e.g. ld1rb_z_p_bi_u64, ld1rsw_z_p_bi_s64, ld1rh_z_p_bi_u64)
- 109 × SME ZA or ZT0 operand <ZAda>.S (e.g. fmop4a_za_zz_s1x1, fmop4s_za_zz_s1x1, fmop4a_za_zz_s1x2)
- 75 × SVE/SME register list ⟪ <Zt>.S ⟫ (e.g. ld1sb_z_p_bz_s_x32_unscaled, ld1sh_z_p_bz_s_x32_unscaled, ld1w_z_p_bz_s_x32_unscaled)
- 52 × SME ZA or ZT0 operand <ZAda>.D (e.g. fmopa_za_pp_zz_64, fmops_za_pp_zz_64, smopa_za_pp_zz_64)
- 30 × SME ZA or ZT0 operand <ZAda>.H (e.g. fmop4a_za16_z8z8_b1x1, fmop4a_za16_z8z8_b1x2, fmop4a_za16_z8z8_b2x1)
- 28 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs>{, VGx2}] (e.g. fmla_za_zzi_s2xi, fvdot_za_zzi_2xi, bfvdot_za_zzi_2xi)
- 28 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs>{, VGx4}] (e.g. fmla_za_zzi_s4xi, fdot_za32_z8z8i_4xi, svdot_za_zzi_s4xi)
- 28 × SVE prefetch operation (e.g. prfb_i_p_bz_s_x32_scaled, prfh_i_p_bz_s_x32_scaled, prfw_i_p_bz_s_x32_scaled)
- 27 × SVE/SME register list ⟪ <Zdn1>.<T>-<Zdn2>.<T> ⟫ (e.g. smax_mz_zzv_2x1, smin_mz_zzv_2x1, umax_mz_zzv_2x1)
- 27 × SVE/SME register list ⟪ <Zdn1>.<T>-<Zdn4>.<T> ⟫ (e.g. smax_mz_zzv_4x1, smin_mz_zzv_4x1, umax_mz_zzv_4x1)
- 24 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs2>{, VGx2}] (e.g. bfmlal_za_zzi_2xi, fmlal_za_zzi_2xi, bfmlsl_za_zzi_2xi)
- 24 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs2>{, VGx4}] (e.g. bfmlal_za_zzi_4xi, fmlal_za_zzi_4xi, bfmlsl_za_zzi_4xi)
- 24 × SVE pattern <Xdn>{, <pattern>{, MUL #<imm>}} (e.g. incb_r_rs_, decb_r_rs_, inch_r_rs_)
- 23 × SVE/SME register list ⟪ <Zt>.H ⟫ (e.g. ld1rb_z_p_bi_u16, ld1rh_z_p_bi_u16, ld1rsb_z_p_bi_s16)
- 22 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs>{, VGx2}] (e.g. fdot_za16_zz8i_2xi, fvdot_za16_zz8i_2xi, fmla_za_zzi_h2xi)
- 19 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs>{, VGx4}] (e.g. fmla_za_zzi_h4xi, bfmla_za_zzi_h4xi, fmls_za_zzi_h4xi)
- 18 × SVE/SME register list ⟪ <Zn1>.S-<Zn2>.S ⟫ (e.g. sqrshrn_z_mz2_, sqrshrun_z_mz2_, uqrshrn_z_mz2_)
- 16 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs>{, VGx2}] (e.g. sdot_za_zzv_2x1, udot_za_zzv_2x1, fmla_za_zzv_2x1)
- 16 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs>{, VGx4}] (e.g. sdot_za_zzv_4x1, udot_za_zzv_4x1, fmla_za_zzv_4x1)
- 16 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs2>] (e.g. bfmlal_za_zzi_1, fmlal_za_zzi_1, bfmlsl_za_zzi_1)
- 16 × SVE pattern <Wdn>{, <pattern>{, MUL #<imm>}} (e.g. sqincb_r_rs_sx, uqincb_r_rs_uw, sqdecb_r_rs_sx)
- 13 × SVE/SME register list ⟪ <Zd1>.H-<Zd2>.H ⟫ (e.g. mova_mz2_za_h1, movaz_mz2_za_h1, bfclamp_mz_zz_2)
- 13 × SVE/SME register list ⟪ <Zt>.B ⟫ (e.g. ld1rb_z_p_bi_u8, ld1rqb_z_p_br_contiguous, ld1rob_z_p_br_contiguous)
- 12 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzi_s2xi, usmlall_za_zzi_s2xi, smlsll_za_zzi_s2xi)
- 12 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzi_s4xi, usmlall_za_zzi_s4xi, smlsll_za_zzi_s4xi)
- 12 × SVE/SME register list ⟪ <Zd1>.<T>-<Zd2>.<T> ⟫ (e.g. luti2_mz2_ztz_1, luti4_mz2_ztz_1, sel_mz_p_zz_2)
- 12 × SVE/SME register list ⟪ <Zd1>.<T>-<Zd4>.<T> ⟫ (e.g. luti2_mz4_ztz_1, luti4_mz4_ztz_1, sel_mz_p_zz_4)
- 12 × SVE/SME register list ⟪ <Zd1>.S-<Zd2>.S ⟫ (e.g. mova_mz2_za_w1, movaz_mz2_za_w1, fcvtzs_mz_z_2)
- 12 × SVE/SME register list ⟪ <Zt1>.B, <Zt2>.B ⟫ (e.g. ld2b_z_p_br_contiguous, ld2b_z_p_bi_contiguous, st2b_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.B, <Zt2>.B, <Zt3>.B, <Zt4>.B ⟫ (e.g. ld4b_z_p_br_contiguous, ld4b_z_p_bi_contiguous, st4b_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.D, <Zt2>.D ⟫ (e.g. ld2d_z_p_br_contiguous, ld2d_z_p_bi_contiguous, st2d_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.D, <Zt2>.D, <Zt3>.D, <Zt4>.D ⟫ (e.g. ld4d_z_p_br_contiguous, ld4d_z_p_bi_contiguous, st4d_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.H, <Zt2>.H ⟫ (e.g. ld2h_z_p_br_contiguous, ld2h_z_p_bi_contiguous, st2h_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.H, <Zt2>.H, <Zt3>.H, <Zt4>.H ⟫ (e.g. ld4h_z_p_br_contiguous, ld4h_z_p_bi_contiguous, st4h_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.S, <Zt2>.S ⟫ (e.g. ld2w_z_p_br_contiguous, ld2w_z_p_bi_contiguous, st2w_z_p_br_contiguous)
- 12 × SVE/SME register list ⟪ <Zt1>.S, <Zt2>.S, <Zt3>.S, <Zt4>.S ⟫ (e.g. ld4w_z_p_br_contiguous, ld4w_z_p_bi_contiguous, st4w_z_p_br_contiguous)
- 10 × SVE/SME register list ⟪ <Zd1>.S-<Zd4>.S ⟫ (e.g. mova_mz4_za_w1, movaz_mz4_za_w1, fcvtzs_mz_z_4)
- 10 × SVE/SME register list ⟪ <Zdn1>.H-<Zdn2>.H ⟫ (e.g. bfmax_mz_zzv_2x1, bfmin_mz_zzv_2x1, bfmaxnm_mz_zzv_2x1)
- 10 × SVE/SME register list ⟪ <Zdn1>.H-<Zdn4>.H ⟫ (e.g. bfmax_mz_zzv_4x1, bfmin_mz_zzv_4x1, bfmaxnm_mz_zzv_4x1)
- 10 × SVE/SME register list ⟪ <Zt>.Q ⟫ (e.g. ld1w_z_p_bi_u128, ld1d_z_p_bi_u128, ld1w_z_p_br_u128)
- 9 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs4>] (e.g. smlall_za_zzi_s, usmlall_za_zzi_s, smlsll_za_zzi_s)
- 9 × SVE/SME register list ⟪ <Pd1>.<T>, <Pd2>.<T> ⟫ (e.g. whilege_pp_rr_, whilehs_pp_rr_, whilegt_pp_rr_)
- 9 × number table <vl>: "VLx2" (e.g. whilege_pn_rr_, whilehs_pn_rr_, whilegt_pn_rr_)
- 9 × table <T>: 2 alternatives for 4 allowed values (e.g. sqshrn_z_mz2_, sqshrun_z_mz2_, uqshrn_z_mz2_)
- 8 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzv_2x1, smlsll_za_zzv_2x1, umlall_za_zzv_2x1)
- 8 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzv_4x1, smlsll_za_zzv_4x1, umlall_za_zzv_4x1)
- 8 × SVE/SME register list ⟪ <Zt1>.B-<Zt2>.B ⟫ (e.g. ld1b_mz_p_br_2, ldnt1b_mz_p_br_2, st1b_mz_p_br_2)
- 8 × SVE/SME register list ⟪ <Zt1>.B-<Zt4>.B ⟫ (e.g. ld1b_mz_p_br_4, ldnt1b_mz_p_br_4, st1b_mz_p_br_4)
- 8 × SVE/SME register list ⟪ <Zt1>.D-<Zt2>.D ⟫ (e.g. ld1d_mz_p_br_2, ldnt1d_mz_p_br_2, st1d_mz_p_br_2)
- 8 × SVE/SME register list ⟪ <Zt1>.D-<Zt4>.D ⟫ (e.g. ld1d_mz_p_br_4, ldnt1d_mz_p_br_4, st1d_mz_p_br_4)
- 8 × SVE/SME register list ⟪ <Zt1>.H-<Zt2>.H ⟫ (e.g. ld1h_mz_p_br_2, ldnt1h_mz_p_br_2, st1h_mz_p_br_2)
- 8 × SVE/SME register list ⟪ <Zt1>.H-<Zt4>.H ⟫ (e.g. ld1h_mz_p_br_4, ldnt1h_mz_p_br_4, st1h_mz_p_br_4)
- 8 × SVE/SME register list ⟪ <Zt1>.S-<Zt2>.S ⟫ (e.g. ld1w_mz_p_br_2, ldnt1w_mz_p_br_2, st1w_mz_p_br_2)
- 8 × SVE/SME register list ⟪ <Zt1>.S-<Zt4>.S ⟫ (e.g. ld1w_mz_p_br_4, ldnt1w_mz_p_br_4, st1w_mz_p_br_4)
- 8 × table <R>: 2 alternatives for 4 allowed values (e.g. index_z_ri_, index_z_ir_, index_z_rr_)
- 7 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx4}] (e.g. mova_za_mz4_1, fmla_za_zzi_d4xi, sdot_za_zzi_d4xi)
- 7 × SVE/SME register list ⟪ <Zn1>.H-<Zn2>.H ⟫ (e.g. sqrshrn_z_mz2_b, sqrshrun_z_mz2_b, uqrshrn_z_mz2_b)
- 6 × SVE pattern <Zdn>.D{, <pattern>{, MUL #<imm>}} (e.g. sqincd_z_zs_, sqdecd_z_zs_, uqincd_z_zs_)
- 6 × SVE pattern <Zdn>.H{, <pattern>{, MUL #<imm>}} (e.g. sqinch_z_zs_, sqdech_z_zs_, uqinch_z_zs_)
- 6 × SVE pattern <Zdn>.S{, <pattern>{, MUL #<imm>}} (e.g. sqincw_z_zs_, sqdecw_z_zs_, uqincw_z_zs_)
- 6 × SVE/SME register list ⟪ <Zd1>.H-<Zd4>.H ⟫ (e.g. mova_mz4_za_h1, movaz_mz4_za_h1, bfclamp_mz_zz_4)
- 6 × SVE/SME register list ⟪ <Zn1>.<Tb>-<Zn4>.<Tb> ⟫ (e.g. sqcvt_z_mz4_, sqcvtu_z_mz4_, sqcvtn_z_mz4_)
- 6 × SVE/SME register list ⟪ <Zt>.<T> ⟫ (e.g. st1b_z_p_br_, st1h_z_p_br_, st1w_z_p_br_)
- 5 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>] (e.g. zero_za4_ri_1, smlall_za_zzi_d, smlsll_za_zzi_d)
- 5 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx2}] (e.g. mova_za_mz2_1, fmla_za_zzi_d2xi, sdot_za_zzi_d2xi)
- 5 × SME ZA or ZT0 operand ZT0 (e.g. luti2_z_ztz_, luti4_z_ztz_, luti6_z_ztz_)
- 4 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs1>:<offs4>] (e.g. smlall_za_zzv_1, smlsll_za_zzv_1, umlall_za_zzv_1)
- 4 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzi_d2xi, smlsll_za_zzi_d2xi, umlall_za_zzi_d2xi)
- 4 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzi_d4xi, smlsll_za_zzi_d4xi, umlall_za_zzi_d4xi)
- 4 × SVE pattern <Xd>{, <pattern>{, MUL #<imm>}} (e.g. cntb_r_s_, cnth_r_s_, cntw_r_s_)
- 4 × SVE/SME memory operand [<Xn|SP>{, #<imm>, MUL VL}] (e.g. ldr_p_bi_, ldr_z_bi_, str_p_bi_)
- 4 × SVE/SME register list ⟪ <Zd1>.B-<Zd4>.B ⟫ (e.g. mova_mz4_za_b1, movaz_mz4_za_b1, luti4_mz4_ztmz2_1)
- 4 × SVE/SME register list ⟪ <Zd1>.D-<Zd2>.D ⟫ (e.g. mova_mz2_za_d1, mova_mz_za2_1, movaz_mz2_za_d1)
- 4 × SVE/SME register list ⟪ <Zd1>.D-<Zd4>.D ⟫ (e.g. mova_mz4_za_d1, mova_mz_za4_1, movaz_mz4_za_d1)
- 4 × SVE/SME register list ⟪ <Zdn1>.B-<Zdn2>.B ⟫ (e.g. aese_mz_zzi_2x1, aesd_mz_zzi_2x1, aesemc_mz_zzi_2x1)
- 4 × SVE/SME register list ⟪ <Zdn1>.B-<Zdn4>.B ⟫ (e.g. aese_mz_zzi_4x1, aesd_mz_zzi_4x1, aesemc_mz_zzi_4x1)
- 4 × SVE/SME register list ⟪ <Zt1>.B, <Zt2>.B, <Zt3>.B ⟫ (e.g. ld3b_z_p_br_contiguous, ld3b_z_p_bi_contiguous, st3b_z_p_br_contiguous)
- 4 × SVE/SME register list ⟪ <Zt1>.D, <Zt2>.D, <Zt3>.D ⟫ (e.g. ld3d_z_p_br_contiguous, ld3d_z_p_bi_contiguous, st3d_z_p_br_contiguous)
- 4 × SVE/SME register list ⟪ <Zt1>.H, <Zt2>.H, <Zt3>.H ⟫ (e.g. ld3h_z_p_br_contiguous, ld3h_z_p_bi_contiguous, st3h_z_p_br_contiguous)
- 4 × SVE/SME register list ⟪ <Zt1>.Q, <Zt2>.Q ⟫ (e.g. ld2q_z_p_br_contiguous, ld2q_z_p_bi_contiguous, st2q_z_p_bi_contiguous)
- 4 × SVE/SME register list ⟪ <Zt1>.Q, <Zt2>.Q, <Zt3>.Q ⟫ (e.g. ld3q_z_p_br_contiguous, ld3q_z_p_bi_contiguous, st3q_z_p_bi_contiguous)
- 4 × SVE/SME register list ⟪ <Zt1>.Q, <Zt2>.Q, <Zt3>.Q, <Zt4>.Q ⟫ (e.g. ld4q_z_p_br_contiguous, ld4q_z_p_bi_contiguous, st4q_z_p_bi_contiguous)
- 4 × SVE/SME register list ⟪ <Zt1>.S, <Zt2>.S, <Zt3>.S ⟫ (e.g. ld3w_z_p_br_contiguous, ld3w_z_p_bi_contiguous, st3w_z_p_br_contiguous)
- 3 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs1>:<offs2>{, VGx2}] (e.g. fmlal_za_z8z8i_2xi, fmlal_za_z8z8v_2x1, fmlal_za_z8z8w_2x2)
- 3 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs1>:<offs2>{, VGx4}] (e.g. fmlal_za_z8z8i_4xi, fmlal_za_z8z8v_4x1, fmlal_za_z8z8w_4x4)
- 3 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs>] (e.g. mova_za_p_rz_b, mova_z_p_rza_b, movaz_z_rza_b)
- 3 × SVE/SME register list ⟪ <Zd1>.Q-<Zd2>.Q ⟫ (e.g. pmull_mz_zzw_1x2, zip_mz_zz_2q, uzp_mz_zz_2q)
- 3 × optional index on <Zd> (e.g. pmov_z_pi_h, pmov_z_pi_s, pmov_z_pi_d)
- 3 × optional index on <Zn> (e.g. pmov_p_zi_h, pmov_p_zi_s, pmov_p_zi_d)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs>] (e.g. mova_z_p_rza_d, movaz_z_rza_d)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs>] (e.g. mova_z_p_rza_h, movaz_z_rza_h)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.Q[<Ws>, <offs>] (e.g. mova_z_p_rza_q, movaz_z_rza_q)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs>] (e.g. mova_z_p_rza_w, movaz_z_rza_w)
- 2 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs1>:<offs2>] (e.g. fmlal_za_z8z8i_1, fmlal_za_z8z8v_1)
- 2 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs>, VGx4] (e.g. fvdotb_za32_z8z8i_2xi, fvdott_za32_z8z8i_2xi)
- 2 × SME ZA or ZT0 operand ZA[<Wv>, <offs>] (e.g. ldr_za_ri_, str_za_ri_)
- 2 × SME ZA or ZT0 operand ZT0[<offs>] (e.g. movt_r_zt_, movt_zt_r_)
- 2 × SVE pattern <Pd>.<T>{, <pattern>} (e.g. ptrue_p_s_, ptrues_p_s_)
- 2 × SVE/SME memory operand [<Xn|SP>{, #<imm>}] (e.g. ldapro_z_bi_, stlro_z_bi_)
- 2 × SVE/SME register list ⟪ <ZAt><HV>.D[<Ws>, <offs>] ⟫ (e.g. ld1d_za_p_rrr_, st1d_za_p_rrr_)
- 2 × SVE/SME register list ⟪ <ZAt><HV>.H[<Ws>, <offs>] ⟫ (e.g. ld1h_za_p_rrr_, st1h_za_p_rrr_)
- 2 × SVE/SME register list ⟪ <ZAt><HV>.Q[<Ws>, <offs>] ⟫ (e.g. ld1q_za_p_rrr_, st1q_za_p_rrr_)
- 2 × SVE/SME register list ⟪ <ZAt><HV>.S[<Ws>, <offs>] ⟫ (e.g. ld1w_za_p_rrr_, st1w_za_p_rrr_)
- 2 × SVE/SME register list ⟪ <Zd1>.<T>, <Zd2>.<T> ⟫ (e.g. luti2_mz2_ztz_8, luti4_mz2_ztz_8)
- 2 × SVE/SME register list ⟪ <Zd1>.B, <Zd2>.B, <Zd3>.B, <Zd4>.B ⟫ (e.g. luti4_mz4_ztmz2_4, luti6_mz4_ztmz3_4)
- 2 × SVE/SME register list ⟪ <Zd1>.B-<Zd2>.B ⟫ (e.g. mova_mz2_za_b1, movaz_mz2_za_b1)
- 2 × SVE/SME register list ⟪ <Zd1>.H, <Zd2>.H, <Zd3>.H, <Zd4>.H ⟫ (e.g. luti4_mz4_ztz_4, luti6_mz4_zmz2_4)
- 2 × SVE/SME register list ⟪ <Zd1>.Q-<Zd4>.Q ⟫ (e.g. zip_mz_z_4q, uzp_mz_z_4q)
- 2 × SVE/SME register list ⟪ <Zn1>.<T>, <Zn2>.<T> ⟫ (e.g. tbl_z_zz_2, splice_z_p_zz_con)
- 2 × SVE/SME register list ⟪ <Zn1>.<Tb>-<Zn2>.<Tb> ⟫ (e.g. fcvtzsn_z_mz2_, fcvtzun_z_mz2_)
- 2 × SVE/SME register list ⟪ <Zn1>.B, <Zn2>.B ⟫ (e.g. ext_z_zi_con, luti6_z_zzz_8)
- 2 × SVE/SME register list ⟪ <Zn1>.H, <Zn2>.H ⟫ (e.g. luti4_z_zz_2x16, luti6_z_zzz_16)
- 2 × SVE/SME register list ⟪ <Zn1>.S-<Zn4>.S ⟫ (e.g. fcvt_z8_mz4_, fcvtn_z8_mz4_)
- 2 × SVE/SME register list ⟪ <Zn>.<T> ⟫ (e.g. tbl_z_zz_1, tblq_z_zz_)
- 2 × SVE/SME register list ⟪ <Zn>.B ⟫ (e.g. luti2_z_zz_8, luti4_z_zz_8)
- 2 × SVE/SME register list ⟪ <Zn>.H ⟫ (e.g. luti2_z_zz_16, luti4_z_zz_1x16)
- 2 × SVE/SME register list ⟪ ZA0<HV>.B[<Ws>, <offs>] ⟫ (e.g. ld1b_za_p_rrr_, st1b_za_p_rrr_)
- 2 × table <T>: 2 alternatives for 3 allowed values (e.g. pmullb_z_zz_, pmullt_z_zz_)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.D[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_d1)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.D[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_d1)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.D[<Ws>, <offs>] (e.g. mova_za_p_rz_d)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.H[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_h1)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.H[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_h1)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.H[<Ws>, <offs>] (e.g. mova_za_p_rz_h)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.Q[<Ws>, <offs>] (e.g. mova_za_p_rz_q)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.S[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_w1)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.S[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_w1)
- 1 × SME ZA or ZT0 operand <ZAd><HV>.S[<Ws>, <offs>] (e.g. mova_za_p_rz_w)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs2>, VGx2] (e.g. zero_za2_ri_2)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs2>, VGx4] (e.g. zero_za2_ri_4)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs2>] (e.g. zero_za2_ri_1)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>, VGx2] (e.g. zero_za4_ri_2)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>, VGx4] (e.g. zero_za4_ri_4)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>, VGx2] (e.g. zero_za1_ri_2)
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>, VGx4] (e.g. zero_za1_ri_4)
- 1 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_b1)
- 1 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_b1)
- 1 × SME ZA or ZT0 operand ZT0{[<offs>, MUL VL]} (e.g. movt_zt_z_)
- 1 × SVE/SME memory operand [<Zn>.<T>, <Zm>.<T>{, <mod> <amount>}] (e.g. adr_z_az_sd_same_scaled)
- 1 × SVE/SME memory operand [<Zn>.D, <Zm>.D, SXTW<amount>] (e.g. adr_z_az_d_s32_scaled)
- 1 × SVE/SME memory operand [<Zn>.D, <Zm>.D, UXTW<amount>] (e.g. adr_z_az_d_u32_scaled)
- 1 × SVE/SME register list ⟪ <Zd1>.<T>, <Zd2>.<T>, <Zd3>.<T>, <Zd4>.<T> ⟫ (e.g. luti2_mz4_ztz_4)
- 1 × SVE/SME register list ⟪ <Zda1>.Q-<Zda2>.Q ⟫ (e.g. pmlal_mz_zzzw_1x2)
- 1 × SVE/SME register list ⟪ <mask> ⟫ (e.g. zero_za_i_)
- 1 × SVE/SME register list ⟪ ZT0 ⟫ (e.g. zero_zt_i_)
- 1 × indexed register <PNn> with an offset (e.g. pext_pn_rr_)
- 1 × operand shape <Pm>.<T>[<Wv>, <imm>] (e.g. psel_p_ppi_)

### Aliases without operand support, by reason

- 2 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_b), MOV (mova_z_p_rza_b))
- 2 × SVE/SME register list ⟪ <Zd1>.D-<Zd2>.D ⟫ (e.g. MOV (mova_mz2_za_d1), MOV (mova_mz_za2_1))
- 2 × SVE/SME register list ⟪ <Zd1>.D-<Zd4>.D ⟫ (e.g. MOV (mova_mz4_za_d1), MOV (mova_mz_za4_1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.D[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_d1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.D[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_d1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.D[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_d))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.H[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_h1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.H[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_h1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.H[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_h))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.Q[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_q))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.S[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_w1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.S[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_w1))
- 1 × SME ZA or ZT0 operand <ZAd><HV>.S[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_w))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_d))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_h))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.Q[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_q))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_w))
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx2}] (e.g. MOV (mova_za_mz2_1))
- 1 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx4}] (e.g. MOV (mova_za_mz4_1))
- 1 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_b1))
- 1 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_b1))
- 1 × SVE/SME register list ⟪ <Zd1>.B-<Zd2>.B ⟫ (e.g. MOV (mova_mz2_za_b1))
- 1 × SVE/SME register list ⟪ <Zd1>.B-<Zd4>.B ⟫ (e.g. MOV (mova_mz4_za_b1))
- 1 × SVE/SME register list ⟪ <Zd1>.H-<Zd2>.H ⟫ (e.g. MOV (mova_mz2_za_h1))
- 1 × SVE/SME register list ⟪ <Zd1>.H-<Zd4>.H ⟫ (e.g. MOV (mova_mz4_za_h1))
- 1 × SVE/SME register list ⟪ <Zd1>.S-<Zd2>.S ⟫ (e.g. MOV (mova_mz2_za_w1))
- 1 × SVE/SME register list ⟪ <Zd1>.S-<Zd4>.S ⟫ (e.g. MOV (mova_mz4_za_w1))

