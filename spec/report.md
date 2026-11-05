# aarch64-isa-gen report

@generated from AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09. Regenerate with aarch64-isa-gen; review the diff when the pin changes.

## Encodings

4429 A64 encodings, 5219 decode-tree nodes, 579 condition expressions, 408 features, 1688 mnemonics, 3218 system registers and operations.

Decode dispatch: 25892 slots, 5168 candidate entries; over all words, 1.59 tables and 0.82 candidates on average, at most 3 candidates.

| Group | Encodings | With operand support |
|---|---|---|
| control | 163 | 162 |
| dpimm | 44 | 44 |
| dpreg | 151 | 151 |
| ldst | 1015 | 1015 |
| reserved | 1 | 1 |
| simd_dp | 867 | 867 |
| sme | 842 | 0 |
| sve | 1346 | 7 |

129 encodings overlap a higher-priority encoding (listed in `OpCodeInfo::precedence()`).

## Operand mapping

2247 encodings with operand support, 2182 without. 1325 symbol value tables taken from the spec's rule names; 91 distinct hand rules (symbol meanings the spec does not give in machine-readable form), used 4691 times.

### Hand rules

| Rule | Uses |
|---|---|
| `<fbits> 64-scale` | 24 |
| `<fbits> immh:immb` | 8 |
| `<imm16> unsigned` | 1 |
| `<imm6> unsigned` | 1 |
| `<imm8> unsigned` | 9 |
| `<imm> signed` | 12 |
| `<imm> unsigned` | 59 |
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
| `EXT index imm4` | 1 |
| `FCMLA by element index` | 1 |
| `FP immediate` | 3 |
| `INS/DUP register width: X iff imm5 = x1000` | 3 |
| `LDRAA offset *8` | 4 |
| `LSL-only shift` | 2 |
| `MSR (immediate) value in CRm` | 1 |
| `NB on the variable op1 bit` | 5 |
| `PSTATE field` | 1 |
| `Q table in alternative order` | 2 |
| `SMSTART/SMSTOP option` | 2 |
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
| `optional operand default` | 17 |
| `pair offset scaled` | 62 |
| `pimm scaled by access size` | 24 |
| `post-index` | 147 |
| `prefetch operations` | 5 |
| `register <B..>` | 14 |
| `register <D..>` | 264 |
| `register <H..>` | 353 |
| `register <Q..>` | 56 |
| `register <S..>` | 268 |
| `register <W..>` | 873 |
| `register <WSp..>` | 42 |
| `register <X..>` | 921 |
| `register <XSp..>` | 92 |
| `register list` | 164 |
| `register offset W/X by option<0>` | 24 |
| `register pair from one field` | 2 |
| `register writeback` | 132 |
| `system operation name` | 9 |
| `system register` | 4 |
| `unscaled signed offset` | 106 |
| `vector element` | 82 |

### Encodings without operand support, by reason

- 573 × vector register (e.g. addpt_z_p_zz_, subpt_z_p_zz_, asr_z_p_zw_)
- 239 × operand shape <Pg>/Z (e.g. ld1sb_z_p_bz_s_x32_unscaled, ld1sh_z_p_bz_s_x32_unscaled, ld1w_z_p_bz_s_x32_unscaled)
- 84 × operand <Pg> (e.g. saddv_r_p_z_, uaddv_r_p_z_, ptest__p_p_)
- 60 × table <T>: rule T_H__2 carries no field values (e.g. add_z_p_zz_, sub_z_p_zz_, subr_z_p_zz_)
- 32 × operand <PNg> (e.g. st1b_mzx_p_br_2x8, stnt1b_mzx_p_br_2x8, st1h_mzx_p_br_2x8)
- 32 × operand shape <PNg>/Z (e.g. ld1b_mzx_p_br_2x8, ldnt1b_mzx_p_br_2x8, ld1h_mzx_p_br_2x8)
- 32 × table <T>: rule T_H carries no field values (e.g. movprfx_z_p_z_, abs_z_p_z_m, abs_z_p_z_z)
- 28 × operand <prfop> (e.g. prfb_i_p_bz_s_x32_scaled, prfh_i_p_bz_s_x32_scaled, prfw_i_p_bz_s_x32_scaled)
- 28 × operand shape ZA.S[<Wv>, <offs>{, VGx2}] (e.g. fmla_za_zzi_s2xi, fvdot_za_zzi_2xi, bfvdot_za_zzi_2xi)
- 28 × operand shape ZA.S[<Wv>, <offs>{, VGx4}] (e.g. fmla_za_zzi_s4xi, fdot_za32_z8z8i_4xi, svdot_za_zzi_s4xi)
- 28 × table <T>: rule T_H__4 carries no field values (e.g. add_z_zz_, sub_z_zz_, sqadd_z_zz_)
- 27 × list element <Zdn1>.<T>-<Zdn2>.<T> (e.g. smax_mz_zzv_2x1, smin_mz_zzv_2x1, umax_mz_zzv_2x1)
- 27 × list element <Zdn1>.<T>-<Zdn4>.<T> (e.g. smax_mz_zzv_4x1, smin_mz_zzv_4x1, umax_mz_zzv_4x1)
- 27 × table <T>: rule T_S__29 carries no field values (e.g. fcadd_z_p_zz_, fmaxinm_z_p_zz_, fmininm_z_p_zz_)
- 26 × table <T>: rule T_S__30 carries no field values (e.g. fabs_z_p_z_m, fabs_z_p_z_z, fneg_z_p_z_m)
- 24 × operand shape <Xdn>{, <pattern>{, MUL #<imm>}} (e.g. incb_r_rs_, decb_r_rs_, inch_r_rs_)
- 24 × operand shape ZA.S[<Wv>, <offs1>:<offs2>{, VGx2}] (e.g. bfmlal_za_zzi_2xi, fmlal_za_zzi_2xi, bfmlsl_za_zzi_2xi)
- 24 × operand shape ZA.S[<Wv>, <offs1>:<offs2>{, VGx4}] (e.g. bfmlal_za_zzi_4xi, fmlal_za_zzi_4xi, bfmlsl_za_zzi_4xi)
- 22 × operand shape ZA.H[<Wv>, <offs>{, VGx2}] (e.g. fdot_za16_zz8i_2xi, fvdot_za16_zz8i_2xi, fmla_za_zzi_h2xi)
- 21 × table <T>: rule T_S__64 carries no field values (e.g. saddlb_z_zz_, ssublb_z_zz_, sabdlb_z_zz_)
- 20 × table <T>: rule T_H__15 carries no field values (e.g. cmpeq_p_p_zw_, cmpne_p_p_zw_, cmpge_p_p_zw_)
- 20 × table <T>: rule T_S__63 carries no field values (e.g. sqdmlalbt_z_zzz_, sqdmlslbt_z_zzz_, smlalb_z_zzz_)
- 19 × operand shape ZA.H[<Wv>, <offs>{, VGx4}] (e.g. fmla_za_zzi_h4xi, bfmla_za_zzi_h4xi, fmls_za_zzi_h4xi)
- 18 × table <T>: rule T_H__54 carries no field values (e.g. whilege_pp_rr_, whilehs_pp_rr_, whilegt_pp_rr_)
- 16 × operand shape <Wdn>{, <pattern>{, MUL #<imm>}} (e.g. sqincb_r_rs_sx, uqincb_r_rs_uw, sqdecb_r_rs_sx)
- 16 × operand shape ZA.<T>[<Wv>, <offs>{, VGx2}] (e.g. sdot_za_zzv_2x1, udot_za_zzv_2x1, fmla_za_zzv_2x1)
- 16 × operand shape ZA.<T>[<Wv>, <offs>{, VGx4}] (e.g. sdot_za_zzv_4x1, udot_za_zzv_4x1, fmla_za_zzv_4x1)
- 16 × operand shape ZA.S[<Wv>, <offs1>:<offs2>] (e.g. bfmlal_za_zzi_1, fmlal_za_zzi_1, bfmlsl_za_zzi_1)
- 13 × list element <Zd1>.H-<Zd2>.H (e.g. mova_mz2_za_h1, movaz_mz2_za_h1, bfclamp_mz_zz_2)
- 12 × list element <Zd1>.<T>-<Zd2>.<T> (e.g. luti2_mz2_ztz_1, luti4_mz2_ztz_1, sel_mz_p_zz_2)
- 12 × list element <Zd1>.<T>-<Zd4>.<T> (e.g. luti2_mz4_ztz_1, luti4_mz4_ztz_1, sel_mz_p_zz_4)
- 12 × list element <Zd1>.S-<Zd2>.S (e.g. mova_mz2_za_w1, movaz_mz2_za_w1, fcvtzs_mz_z_2)
- 12 × operand shape ZA.S[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzi_s2xi, usmlall_za_zzi_s2xi, smlsll_za_zzi_s2xi)
- 12 × operand shape ZA.S[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzi_s4xi, usmlall_za_zzi_s4xi, smlsll_za_zzi_s4xi)
- 12 × table <T>: rule T_H__3 carries no field values (e.g. add_z_zi_, sub_z_zi_, subr_z_zi_)
- 11 × table <T>: rule T_H__57 carries no field values (e.g. sqshrunb_z_zi_, sqrshrunb_z_zi_, shrnb_z_zi_)
- 11 × table <V>: rule V_S__5 carries no field values (e.g. fmaxinmv_v_p_z_, fmininmv_v_p_z_, fmaxmginmv_v_p_z_)
- 10 × list element <Zd1>.S-<Zd4>.S (e.g. mova_mz4_za_w1, movaz_mz4_za_w1, fcvtzs_mz_z_4)
- 10 × list element <Zdn1>.H-<Zdn2>.H (e.g. bfmax_mz_zzv_2x1, bfmin_mz_zzv_2x1, bfmaxnm_mz_zzv_2x1)
- 10 × list element <Zdn1>.H-<Zdn4>.H (e.g. bfmax_mz_zzv_4x1, bfmin_mz_zzv_4x1, bfmaxnm_mz_zzv_4x1)
- 10 × table <T>: rule T_H__22 carries no field values (e.g. sqincp_r_p_r_sx, uqincp_r_p_r_uw, sqdecp_r_p_r_sx)
- 9 × operand shape ZA.S[<Wv>, <offs1>:<offs4>] (e.g. smlall_za_zzi_s, usmlall_za_zzi_s, smlsll_za_zzi_s)
- 9 × table <T>: rule T_H__55 carries no field values (e.g. whilege_pn_rr_, whilehs_pn_rr_, whilegt_pn_rr_)
- 9 × table <T>: rule T_H__9 carries no field values (e.g. asr_z_p_zi_, lsl_z_p_zi_, asrd_z_p_zi_)
- 9 × table <T>: rule T_S__31 carries no field values (e.g. fcmtl_p_zz_, fcmtlmg_p_zz_, fcmge_p_p_zz_)
- 8 × list element <Zt1>.B-<Zt2>.B (e.g. ld1b_mz_p_br_2, ldnt1b_mz_p_br_2, st1b_mz_p_br_2)
- 8 × list element <Zt1>.B-<Zt4>.B (e.g. ld1b_mz_p_br_4, ldnt1b_mz_p_br_4, st1b_mz_p_br_4)
- 8 × list element <Zt1>.D-<Zt2>.D (e.g. ld1d_mz_p_br_2, ldnt1d_mz_p_br_2, st1d_mz_p_br_2)
- 8 × list element <Zt1>.D-<Zt4>.D (e.g. ld1d_mz_p_br_4, ldnt1d_mz_p_br_4, st1d_mz_p_br_4)
- 8 × list element <Zt1>.H-<Zt2>.H (e.g. ld1h_mz_p_br_2, ldnt1h_mz_p_br_2, st1h_mz_p_br_2)
- 8 × list element <Zt1>.H-<Zt4>.H (e.g. ld1h_mz_p_br_4, ldnt1h_mz_p_br_4, st1h_mz_p_br_4)
- 8 × list element <Zt1>.S-<Zt2>.S (e.g. ld1w_mz_p_br_2, ldnt1w_mz_p_br_2, st1w_mz_p_br_2)
- 8 × list element <Zt1>.S-<Zt4>.S (e.g. ld1w_mz_p_br_4, ldnt1w_mz_p_br_4, st1w_mz_p_br_4)
- 8 × operand shape ZA.<T>[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzv_2x1, smlsll_za_zzv_2x1, umlall_za_zzv_2x1)
- 8 × operand shape ZA.<T>[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzv_4x1, smlsll_za_zzv_4x1, umlall_za_zzv_4x1)
- 8 × table <T>: rule T_8H carries no field values (e.g. addqv_z_p_z_, smaxqv_z_p_z_, sminqv_z_p_z_)
- 8 × table <T>: rule T_D__FRINT carries no field values (e.g. frint32z_z_p_z_z, frint32x_z_p_z_z, frint64z_z_p_z_z)
- 8 × table <T>: rule T_H__14 carries no field values (e.g. mla_z_p_zzz_, mls_z_p_zzz_, cmla_z_zzz_)
- 8 × table <T>: rule T_H__16 carries no field values (e.g. cmphs_p_p_zz_, cmpge_p_p_zz_, cmpeq_p_p_zz_)
- 8 × table <T>: rule T_H__58 carries no field values (e.g. sqshrunt_z_zi_, sqrshrunt_z_zi_, shrnt_z_zi_)
- 8 × table <T>: rule T_S__32 carries no field values (e.g. fadd_z_p_zs_, fsub_z_p_zs_, fmul_z_p_zs_)
- 8 × table <T>: rule T_S__67 carries no field values (e.g. saddwb_z_zz_, ssubwb_z_zz_, saddwt_z_zz_)
- 7 × operand shape ZA.D[<Wv>, <offs>{, VGx4}] (e.g. mova_za_mz4_1, fmla_za_zzi_d4xi, sdot_za_zzi_d4xi)
- 7 × table <T>: rule T_H__25 carries no field values (e.g. addsubp_z_zz_, tbx_z_zz_, tbxq_z_zz_)
- 7 × table <V>: rule V_H carries no field values (e.g. smaxv_r_p_z_, sminv_r_p_z_, umaxv_r_p_z_)
- 6 × list element <Zd1>.H-<Zd4>.H (e.g. mova_mz4_za_h1, movaz_mz4_za_h1, bfclamp_mz_zz_4)
- 6 × operand shape <Zdn>.D{, <pattern>{, MUL #<imm>}} (e.g. sqincd_z_zs_, sqdecd_z_zs_, uqincd_z_zs_)
- 6 × operand shape <Zdn>.H{, <pattern>{, MUL #<imm>}} (e.g. sqinch_z_zs_, sqdech_z_zs_, uqinch_z_zs_)
- 6 × operand shape <Zdn>.S{, <pattern>{, MUL #<imm>}} (e.g. sqincw_z_zs_, sqdecw_z_zs_, uqincw_z_zs_)
- 6 × table <T>: rule T_H__67 carries no field values (e.g. sqcvt_z_mz4_, sqcvtu_z_mz4_, sqcvtn_z_mz4_)
- 6 × table <T>: rule T_H__68 carries no field values (e.g. sqrshr_z_mz4_, sqrshru_z_mz4_, sqrshrn_z_mz4_)
- 6 × table <T>: rule T_H__76 carries no field values (e.g. zip1_p_pp_, uzp1_p_pp_, trn1_p_pp_)
- 6 × table <T>: rule T_S__26 carries no field values (e.g. sqincp_z_p_z_, sqdecp_z_p_z_, uqincp_z_p_z_)
- 6 × table <T>: rule T_S__38 carries no field values (e.g. fcmge_p_p_z0_, fcmeq_p_p_z0_, fcmgt_p_p_z0_)
- 6 × table <T>: rule T_fmul_S carries no field values (e.g. fadd_z_zz_, fsub_z_zz_, fmul_z_zz_)
- 5 × literal ZT0 (e.g. luti4_mz4_ztz_4, luti4_mz4_ztmz2_4, luti6_mz4_ztmz3_4)
- 5 × operand shape ZA.D[<Wv>, <offs1>:<offs4>] (e.g. zero_za4_ri_1, smlall_za_zzi_d, smlsll_za_zzi_d)
- 5 × operand shape ZA.D[<Wv>, <offs>{, VGx2}] (e.g. mova_za_mz2_1, fmla_za_zzi_d2xi, sdot_za_zzi_d2xi)
- 5 × table <T>: rule T_4S__2 carries no field values (e.g. faddqv_z_p_z_, fmaxnmqv_z_p_z_, fminnmqv_z_p_z_)
- 5 × table <T>: rule T_S__39 carries no field values (e.g. fcmla_z_p_zzz_, fmla_z_p_zzz_, fmls_z_p_zzz_)
- 4 × list element <Zd1>.B-<Zd4>.B (e.g. mova_mz4_za_b1, movaz_mz4_za_b1, luti4_mz4_ztmz2_1)
- 4 × list element <Zd1>.D-<Zd2>.D (e.g. mova_mz2_za_d1, mova_mz_za2_1, movaz_mz2_za_d1)
- 4 × list element <Zd1>.D-<Zd4>.D (e.g. mova_mz4_za_d1, mova_mz_za4_1, movaz_mz4_za_d1)
- 4 × list element <Zdn1>.B-<Zdn2>.B (e.g. aese_mz_zzi_2x1, aesd_mz_zzi_2x1, aesemc_mz_zzi_2x1)
- 4 × list element <Zdn1>.B-<Zdn4>.B (e.g. aese_mz_zzi_4x1, aesd_mz_zzi_4x1, aesemc_mz_zzi_4x1)
- 4 × operand <Zt> (e.g. ldr_z_bi_, ldapro_z_bi_, str_z_bi_)
- 4 × operand shape <Xd>{, <pattern>{, MUL #<imm>}} (e.g. cntb_r_s_, cnth_r_s_, cntw_r_s_)
- 4 × operand shape ZA.<T>[<Wv>, <offs1>:<offs4>] (e.g. smlall_za_zzv_1, smlsll_za_zzv_1, umlall_za_zzv_1)
- 4 × operand shape ZA.D[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzi_d2xi, smlsll_za_zzi_d2xi, umlall_za_zzi_d2xi)
- 4 × operand shape ZA.D[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzi_d4xi, smlsll_za_zzi_d4xi, umlall_za_zzi_d4xi)
- 4 × table <T>: rule T_D carries no field values (e.g. dup_z_zi_, compact_z_p_z_, revh_z_z_m)
- 4 × table <T>: rule T_D__2 carries no field values (e.g. adclb_z_zzz_, sbclb_z_zzz_, adclt_z_zzz_)
- 4 × table <T>: rule T_D__3 carries no field values (e.g. sdiv_z_p_zz_, sdivr_z_p_zz_, udiv_z_p_zz_)
- 4 × table <T>: rule T_H__5 carries no field values (e.g. addhnb_z_zz_, raddhnb_z_zz_, subhnb_z_zz_)
- 4 × table <T>: rule T_H__6 carries no field values (e.g. addhnt_z_zz_, raddhnt_z_zz_, subhnt_z_zz_)
- 4 × table <T>: rule T_H__71 carries no field values (e.g. ssra_z_zi_, srsra_z_zi_, usra_z_zi_)
- 4 × table <T>: rule T_S__42 carries no field values (e.g. fmad_z_p_zzz_, fmsb_z_p_zzz_, fnmad_z_p_zzz_)
- 4 × table <T>: rule T_S__76 carries no field values (e.g. sshllb_z_zi_, sshllt_z_zi_, ushllb_z_zi_)
- 4 × table <T>: rule T_S__78 carries no field values (e.g. sunpklo_z_z_, sunpkhi_z_z_, uunpklo_z_z_)
- 4 × table <T>: rule T_S__84 carries no field values (e.g. scvtf_z_z_, scvtflt_z_z_, ucvtf_z_z_)
- 4 × table <T>: rule T_xt_D__2 carries no field values (e.g. sxth_z_p_z_m, sxth_z_p_z_z, uxth_z_p_z_m)
- 4 × table <T>: rule T_xt_S carries no field values (e.g. sxtb_z_p_z_m, sxtb_z_p_z_z, uxtb_z_p_z_m)
- 3 × list element <Zd1>.Q-<Zd2>.Q (e.g. pmull_mz_zzw_1x2, zip_mz_zz_2q, uzp_mz_zz_2q)
- 3 × operand shape <Zd>{[<imm>]} (e.g. pmov_z_pi_h, pmov_z_pi_s, pmov_z_pi_d)
- 3 × operand shape ZA.H[<Wv>, <offs1>:<offs2>{, VGx2}] (e.g. fmlal_za_z8z8i_2xi, fmlal_za_z8z8v_2x1, fmlal_za_z8z8w_2x2)
- 3 × operand shape ZA.H[<Wv>, <offs1>:<offs2>{, VGx4}] (e.g. fmlal_za_z8z8i_4xi, fmlal_za_z8z8v_4x1, fmlal_za_z8z8w_4x4)
- 3 × table <T>: rule T_D__12 carries no field values (e.g. sdot_z_zzz_, udot_z_zzz_, cdot_z_zzz_)
- 3 × table <T>: rule T_H__11 carries no field values (e.g. asr_z_zi_, lsl_z_zi_, lsr_z_zi_)
- 3 × table <T>: rule T_H__19 carries no field values (e.g. cpy_z_o_i_, cpy_z_p_i_, dup_z_i_)
- 3 × table <T>: rule T_H__24 carries no field values (e.g. dupm_z_i_, dup_z_r_bhs_, cpy_z_p_r_bhs_)
- 3 × table <T>: rule T_H__69 carries no field values (e.g. sqxtnb_z_zz_, sqxtunb_z_zz_, uqxtnb_z_zz_)
- 3 × table <T>: rule T_H__7 carries no field values (e.g. orr_z_zi_, eor_z_zi_, and_z_zi_)
- 3 × table <T>: rule T_H__70 carries no field values (e.g. sqxtnt_z_zz_, sqxtunt_z_zz_, uqxtnt_z_zz_)
- 3 × table <T>: rule T_S carries no field values (e.g. dupq_z_zi_, revb_z_z_m, revb_z_z_z)
- 2 × list element <ZAt><HV>.D[<Ws> (e.g. ld1d_za_p_rrr_, st1d_za_p_rrr_)
- 2 × list element <ZAt><HV>.H[<Ws> (e.g. ld1h_za_p_rrr_, st1h_za_p_rrr_)
- 2 × list element <ZAt><HV>.Q[<Ws> (e.g. ld1q_za_p_rrr_, st1q_za_p_rrr_)
- 2 × list element <ZAt><HV>.S[<Ws> (e.g. ld1w_za_p_rrr_, st1w_za_p_rrr_)
- 2 × list element <Zd1>.B-<Zd2>.B (e.g. mova_mz2_za_b1, movaz_mz2_za_b1)
- 2 × list element <Zd1>.Q-<Zd4>.Q (e.g. zip_mz_z_4q, uzp_mz_z_4q)
- 2 × list element ZA0<HV>.B[<Ws> (e.g. ld1b_za_p_rrr_, st1b_za_p_rrr_)
- 2 × operand <Pt> (e.g. ldr_p_bi_, str_p_bi_)
- 2 × operand <Zd> (e.g. movprfx_z_z_, pmov_z_pi_b)
- 2 × operand shape <Pd>.<T>{, <pattern>} (e.g. ptrue_p_s_, ptrues_p_s_)
- 2 × operand shape ZA.H[<Wv>, <offs1>:<offs2>] (e.g. fmlal_za_z8z8i_1, fmlal_za_z8z8v_1)
- 2 × operand shape ZA.S[<Wv>, <offs>, VGx4] (e.g. fvdotb_za32_z8z8i_2xi, fvdott_za32_z8z8i_2xi)
- 2 × operand shape ZA[<Wv>, <offs>] (e.g. ldr_za_ri_, str_za_ri_)
- 2 × operand shape ZT0[<offs>] (e.g. movt_r_zt_, movt_zt_r_)
- 2 × table <R>: rule R_X__2 carries no field values (e.g. clasta_r_p_z_, clastb_r_p_z_)
- 2 × table <R>: rule R_X__4 carries no field values (e.g. ctermeq_rr_, ctermne_rr_)
- 2 × table <R>: rule R_X__9 carries no field values (e.g. lasta_r_p_z_, lastb_r_p_z_)
- 2 × table <T>: rule T_D__5 carries no field values (e.g. adr_z_az_sd_same_scaled, histcnt_z_p_zz_)
- 2 × table <T>: rule T_D__53 carries no field values (e.g. pmullb_z_zz_, pmullt_z_zz_)
- 2 × table <T>: rule T_D__69 carries no field values (e.g. st1w_z_p_br_, st1w_z_p_bi_)
- 2 × table <T>: rule T_H__49 carries no field values (e.g. mad_z_p_zzz_, msb_z_p_zzz_)
- 2 × table <T>: rule T_H__50 carries no field values (e.g. pext_pn_rr_, pext_pp_rr_)
- 2 × table <T>: rule T_H__64 carries no field values (e.g. sri_z_zzi_, sli_z_zzi_)
- 2 × table <T>: rule T_H__73 carries no field values (e.g. st1b_z_p_br_, st1b_z_p_bi_)
- 2 × table <T>: rule T_H__80 carries no field values (e.g. fcvtzsn_z_mz2_, fcvtzun_z_mz2_)
- 2 × table <T>: rule T_S__40 carries no field values (e.g. fcpy_z_p_i_, fdup_z_i_)
- 2 × table <T>: rule T_S__65 carries no field values (e.g. sadalp_z_p_z_, uadalp_z_p_z_)
- 2 × table <T>: rule T_S__77 carries no field values (e.g. st1h_z_p_br_, st1h_z_p_bi_)
- 2 × table <V>: rule V_H__2 carries no field values (e.g. clasta_v_p_z_, clastb_v_p_z_)
- 2 × table <V>: rule V_H__7 carries no field values (e.g. lasta_v_p_z_, lastb_v_p_z_)
- 1 × list element <Zda1>.Q-<Zda2>.Q (e.g. pmlal_mz_zzzw_1x2)
- 1 × list element <Zm1>-<Zm2> (e.g. luti6_mz4_zmz2_4)
- 1 × list element <mask> (e.g. zero_za_i_)
- 1 × list element ZT0 (e.g. zero_zt_i_)
- 1 × operand <Pd> (e.g. psel_p_ppi_)
- 1 × operand shape <ZAd><HV>.D[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_d1)
- 1 × operand shape <ZAd><HV>.D[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_d1)
- 1 × operand shape <ZAd><HV>.D[<Ws>, <offs>] (e.g. mova_za_p_rz_d)
- 1 × operand shape <ZAd><HV>.H[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_h1)
- 1 × operand shape <ZAd><HV>.H[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_h1)
- 1 × operand shape <ZAd><HV>.H[<Ws>, <offs>] (e.g. mova_za_p_rz_h)
- 1 × operand shape <ZAd><HV>.Q[<Ws>, <offs>] (e.g. mova_za_p_rz_q)
- 1 × operand shape <ZAd><HV>.S[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_w1)
- 1 × operand shape <ZAd><HV>.S[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_w1)
- 1 × operand shape <ZAd><HV>.S[<Ws>, <offs>] (e.g. mova_za_p_rz_w)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs2>, VGx2] (e.g. zero_za2_ri_2)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs2>, VGx4] (e.g. zero_za2_ri_4)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs2>] (e.g. zero_za2_ri_1)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs4>, VGx2] (e.g. zero_za4_ri_2)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs4>, VGx4] (e.g. zero_za4_ri_4)
- 1 × operand shape ZA.D[<Wv>, <offs>, VGx2] (e.g. zero_za1_ri_2)
- 1 × operand shape ZA.D[<Wv>, <offs>, VGx4] (e.g. zero_za1_ri_4)
- 1 × operand shape ZA0<HV>.B[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_b1)
- 1 × operand shape ZA0<HV>.B[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_b1)
- 1 × operand shape ZA0<HV>.B[<Ws>, <offs>] (e.g. mova_za_p_rz_b)
- 1 × operand shape ZT0{[<offs>, MUL VL]} (e.g. movt_zt_z_)
- 1 × optional group { <nb>} (e.g. TEXIT_te_branch_reg)
- 1 × table <T>: rule T_H__18 carries no field values (e.g. cntp_r_pn_)
- 1 × table <T>: rule T_H__21 carries no field values (e.g. cpy_z_p_v_)
- 1 × table <T>: rule T_H__41 carries no field values (e.g. index_z_ii_)
- 1 × table <T>: rule T_H__42 carries no field values (e.g. index_z_ir_)
- 1 × table <T>: rule T_H__43 carries no field values (e.g. index_z_ri_)
- 1 × table <T>: rule T_H__44 carries no field values (e.g. index_z_rr_)
- 1 × table <T>: rule T_H__45 carries no field values (e.g. insr_z_r_)
- 1 × table <T>: rule T_H__46 carries no field values (e.g. insr_z_v_)
- 1 × table <T>: rule T_H__52 carries no field values (e.g. pnext_p_p_p_)
- 1 × table <T>: rule T_H__56 carries no field values (e.g. rev_p_p_)
- 1 × table <T>: rule T_H__63 carries no field values (e.g. sel_z_p_zz_)
- 1 × table <T>: rule T_H__65 carries no field values (e.g. splice_z_p_zz_con)
- 1 × table <T>: rule T_H__66 carries no field values (e.g. splice_z_p_zz_des)
- 1 × table <T>: rule T_H__78 carries no field values (e.g. xar_z_zzi_)
- 1 × table <T>: rule T_S__37 carries no field values (e.g. fclamp_z_zz_)
- 1 × table <T>: rule T_S__41 carries no field values (e.g. fexpa_z_z_)
- 1 × table <T>: rule T_S__44 carries no field values (e.g. ftmad_z_zzi_)
- 1 × table <T>: rule T_S__45 carries no field values (e.g. ftssel_z_zz_)
- 1 × table <V>: rule V_S__4 carries no field values (e.g. fadda_v_p_z_)

### Aliases without operand support, by reason

- 15 × vector register (e.g. MOV (orr_z_zz_), MOV (dup_z_r_d_), MOV (cpy_z_p_r_d_))
- 4 × table <T>: rule T_H__16 carries no field values (e.g. CMPLS (cmphs_p_p_zz_), CMPLE (cmpge_p_p_zz_), CMPLO (cmphi_p_p_zz_))
- 4 × table <T>: rule T_S__31 carries no field values (e.g. FCMLE (fcmge_p_p_zz_), FACLE (facge_p_p_zz_), FACLT (facgt_p_p_zz_))
- 3 × table <T>: rule T_H__19 carries no field values (e.g. MOV (cpy_z_o_i_), MOV (cpy_z_p_i_), MOV (dup_z_i_))
- 3 × table <T>: rule T_H__24 carries no field values (e.g. MOV (dupm_z_i_), MOV (dup_z_r_bhs_), MOV (cpy_z_p_r_bhs_))
- 3 × table <T>: rule T_H__7 carries no field values (e.g. ORN (orr_z_zi_), EON (eor_z_zi_), BIC (and_z_zi_))
- 2 × list element <Zd1>.D-<Zd2>.D (e.g. MOV (mova_mz2_za_d1), MOV (mova_mz_za2_1))
- 2 × list element <Zd1>.D-<Zd4>.D (e.g. MOV (mova_mz4_za_d1), MOV (mova_mz_za4_1))
- 2 × table <T>: rule T_D carries no field values (e.g. MOV (dup_z_zi_), MOV (dup_z_zi_))
- 2 × table <T>: rule T_S__22 carries no field values (e.g. FMOV (cpy_z_p_i_), FMOV (dup_z_i_))
- 2 × table <T>: rule T_S__40 carries no field values (e.g. FMOV (fcpy_z_p_i_), FMOV (fdup_z_i_))
- 1 × list element <Zd1>.B-<Zd2>.B (e.g. MOV (mova_mz2_za_b1))
- 1 × list element <Zd1>.B-<Zd4>.B (e.g. MOV (mova_mz4_za_b1))
- 1 × list element <Zd1>.H-<Zd2>.H (e.g. MOV (mova_mz2_za_h1))
- 1 × list element <Zd1>.H-<Zd4>.H (e.g. MOV (mova_mz4_za_h1))
- 1 × list element <Zd1>.S-<Zd2>.S (e.g. MOV (mova_mz2_za_w1))
- 1 × list element <Zd1>.S-<Zd4>.S (e.g. MOV (mova_mz4_za_w1))
- 1 × operand shape <ZAd><HV>.D[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_d1))
- 1 × operand shape <ZAd><HV>.D[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_d1))
- 1 × operand shape <ZAd><HV>.D[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_d))
- 1 × operand shape <ZAd><HV>.H[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_h1))
- 1 × operand shape <ZAd><HV>.H[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_h1))
- 1 × operand shape <ZAd><HV>.H[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_h))
- 1 × operand shape <ZAd><HV>.Q[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_q))
- 1 × operand shape <ZAd><HV>.S[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_w1))
- 1 × operand shape <ZAd><HV>.S[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_w1))
- 1 × operand shape <ZAd><HV>.S[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_w))
- 1 × operand shape ZA.D[<Wv>, <offs>{, VGx2}] (e.g. MOV (mova_za_mz2_1))
- 1 × operand shape ZA.D[<Wv>, <offs>{, VGx4}] (e.g. MOV (mova_za_mz4_1))
- 1 × operand shape ZA0<HV>.B[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_b1))
- 1 × operand shape ZA0<HV>.B[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_b1))
- 1 × operand shape ZA0<HV>.B[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_b))
- 1 × optional group { <Xt>} (e.g. GCSPOPM (SYSL_RC_systeminstrs))
- 1 × table <T>: rule T_H__21 carries no field values (e.g. MOV (cpy_z_p_v_))
- 1 × table <T>: rule T_H__63 carries no field values (e.g. MOV (sel_z_p_zz_))

## Alias conditions not compiled

- 1 × unknown function SysLOp

