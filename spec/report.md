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
| sme | 842 | 284 |
| sve | 1346 | 1345 |

129 encodings overlap a higher-priority encoding (listed in `OpCodeInfo::precedence()`).

## Operand mapping

3870 encodings with operand support, 559 without. 3024 symbol value tables taken from the spec's rule names; 125 distinct hand rules (symbol meanings the spec does not give in machine-readable form), used 11718 times.

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
| `ADR fixed extend` | 2 |
| `ADR shift amount from msz` | 3 |
| `AdvSIMD 64-bit byte-mask immediate` | 2 |
| `AdvSIMD FP immediate` | 3 |
| `AdvSIMD imm8` | 2 |
| `CRn/CRm` | 6 |
| `CTERM register width: X iff sz = 1` | 4 |
| `EXT index imm4` | 1 |
| `FCMLA by element index` | 1 |
| `FP immediate` | 3 |
| `INS/DUP register width: X iff imm5 = x1000` | 3 |
| `LDAPRO/STLRO offset as the field value` | 2 |
| `LDRAA offset *8` | 4 |
| `LSL-only shift` | 2 |
| `MSR (immediate) value in CRm` | 1 |
| `NB on the variable op1 bit` | 6 |
| `PSTATE field` | 1 |
| `Q table in alternative order` | 2 |
| `SME2 strided register list` | 64 |
| `SMSTART/SMSTOP option` | 2 |
| `SVE FP constant pair` | 8 |
| `SVE FP immediate` | 4 |
| `SVE MUL VL offset in multiples of the list length` | 94 |
| `SVE element size from tsz (one-hot) or imm13` | 131 |
| `SVE immediate, signed as the spec's number token says` | 98 |
| `SVE index above the lowest set bit of tsz` | 3 |
| `SVE index fields` | 121 |
| `SVE indexed register <PN..>` | 2 |
| `SVE indexed register <Z..>` | 122 |
| `SVE logical immediate (one element)` | 8 |
| `SVE memory offset scaled by the access size` | 65 |
| `SVE memory operand` | 488 |
| `SVE narrowing shift 2^width - imm` | 9 |
| `SVE optional index, left out at 0` | 6 |
| `SVE pattern and MUL #imm, left out at ALL and 1` | 62 |
| `SVE pattern, left out at ALL` | 2 |
| `SVE predicate pattern` | 64 |
| `SVE prefetch operations` | 28 |
| `SVE register <P..>` | 1010 |
| `SVE register <PN..>` | 140 |
| `SVE register <Z..>` | 1997 |
| `SVE register width: X iff size = 11` | 11 |
| `SVE shift tsize:imm right` | 9 |
| `SVE shift tsz:imm3 left` | 10 |
| `SVE shift tsz:imm3 right` | 29 |
| `SVE size table: element size code, others RESERVED` | 559 |
| `SVE table: alternatives in allowed-value order` | 1009 |
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
| `consecutive register list` | 336 |
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
| `multi-vector register list, a multiple of its length` | 414 |
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
| `register <W..>` | 893 |
| `register <WSp..>` | 46 |
| `register <X..>` | 1007 |
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

- 109 × SME ZA or ZT0 operand <ZAda>.S (e.g. fmop4a_za_zz_s1x1, fmop4s_za_zz_s1x1, fmop4a_za_zz_s1x2)
- 52 × SME ZA or ZT0 operand <ZAda>.D (e.g. fmopa_za_pp_zz_64, fmops_za_pp_zz_64, smopa_za_pp_zz_64)
- 30 × SME ZA or ZT0 operand <ZAda>.H (e.g. fmop4a_za16_z8z8_b1x1, fmop4a_za16_z8z8_b1x2, fmop4a_za16_z8z8_b2x1)
- 28 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs>{, VGx2}] (e.g. fmla_za_zzi_s2xi, fvdot_za_zzi_2xi, bfvdot_za_zzi_2xi)
- 28 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs>{, VGx4}] (e.g. fmla_za_zzi_s4xi, fdot_za32_z8z8i_4xi, svdot_za_zzi_s4xi)
- 24 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs2>{, VGx2}] (e.g. bfmlal_za_zzi_2xi, fmlal_za_zzi_2xi, bfmlsl_za_zzi_2xi)
- 24 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs2>{, VGx4}] (e.g. bfmlal_za_zzi_4xi, fmlal_za_zzi_4xi, bfmlsl_za_zzi_4xi)
- 22 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs>{, VGx2}] (e.g. fdot_za16_zz8i_2xi, fvdot_za16_zz8i_2xi, fmla_za_zzi_h2xi)
- 19 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs>{, VGx4}] (e.g. fmla_za_zzi_h4xi, bfmla_za_zzi_h4xi, fmls_za_zzi_h4xi)
- 17 × SME ZA or ZT0 operand ZT0 (e.g. luti2_mz2_ztz_1, luti4_mz2_ztz_1, luti2_mz4_ztz_1)
- 16 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs>{, VGx2}] (e.g. sdot_za_zzv_2x1, udot_za_zzv_2x1, fmla_za_zzv_2x1)
- 16 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs>{, VGx4}] (e.g. sdot_za_zzv_4x1, udot_za_zzv_4x1, fmla_za_zzv_4x1)
- 16 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs2>] (e.g. bfmlal_za_zzi_1, fmlal_za_zzi_1, bfmlsl_za_zzi_1)
- 12 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzi_s2xi, usmlall_za_zzi_s2xi, smlsll_za_zzi_s2xi)
- 12 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzi_s4xi, usmlall_za_zzi_s4xi, smlsll_za_zzi_s4xi)
- 9 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx4}] (e.g. mova_za_mz4_1, mova_mz_za4_1, movaz_mz_za4_1)
- 9 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs1>:<offs4>] (e.g. smlall_za_zzi_s, usmlall_za_zzi_s, smlsll_za_zzi_s)
- 8 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzv_2x1, smlsll_za_zzv_2x1, umlall_za_zzv_2x1)
- 8 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzv_4x1, smlsll_za_zzv_4x1, umlall_za_zzv_4x1)
- 7 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx2}] (e.g. mova_za_mz2_1, mova_mz_za2_1, movaz_mz_za2_1)
- 5 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>] (e.g. zero_za4_ri_1, smlall_za_zzi_d, smlsll_za_zzi_d)
- 4 × SME ZA or ZT0 operand ZA.<T>[<Wv>, <offs1>:<offs4>] (e.g. smlall_za_zzv_1, smlsll_za_zzv_1, umlall_za_zzv_1)
- 4 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>{, VGx2}] (e.g. smlall_za_zzi_d2xi, smlsll_za_zzi_d2xi, umlall_za_zzi_d2xi)
- 4 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs1>:<offs4>{, VGx4}] (e.g. smlall_za_zzi_d4xi, smlsll_za_zzi_d4xi, umlall_za_zzi_d4xi)
- 3 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs1>:<offs2>{, VGx2}] (e.g. fmlal_za_z8z8i_2xi, fmlal_za_z8z8v_2x1, fmlal_za_z8z8w_2x2)
- 3 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs1>:<offs2>{, VGx4}] (e.g. fmlal_za_z8z8i_4xi, fmlal_za_z8z8v_4x1, fmlal_za_z8z8w_4x4)
- 3 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs2>] (e.g. mova_za2_z_b1, mova_mz2_za_b1, movaz_mz2_za_b1)
- 3 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs4>] (e.g. mova_za4_z_b1, mova_mz4_za_b1, movaz_mz4_za_b1)
- 3 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs>] (e.g. mova_za_p_rz_b, mova_z_p_rza_b, movaz_z_rza_b)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs1>:<offs2>] (e.g. mova_mz2_za_d1, movaz_mz2_za_d1)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs1>:<offs4>] (e.g. mova_mz4_za_d1, movaz_mz4_za_d1)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs>] (e.g. mova_z_p_rza_d, movaz_z_rza_d)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs1>:<offs2>] (e.g. mova_mz2_za_h1, movaz_mz2_za_h1)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs1>:<offs4>] (e.g. mova_mz4_za_h1, movaz_mz4_za_h1)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs>] (e.g. mova_z_p_rza_h, movaz_z_rza_h)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.Q[<Ws>, <offs>] (e.g. mova_z_p_rza_q, movaz_z_rza_q)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs1>:<offs2>] (e.g. mova_mz2_za_w1, movaz_mz2_za_w1)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs1>:<offs4>] (e.g. mova_mz4_za_w1, movaz_mz4_za_w1)
- 2 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs>] (e.g. mova_z_p_rza_w, movaz_z_rza_w)
- 2 × SME ZA or ZT0 operand ZA.H[<Wv>, <offs1>:<offs2>] (e.g. fmlal_za_z8z8i_1, fmlal_za_z8z8v_1)
- 2 × SME ZA or ZT0 operand ZA.S[<Wv>, <offs>, VGx4] (e.g. fvdotb_za32_z8z8i_2xi, fvdott_za32_z8z8i_2xi)
- 2 × SME ZA or ZT0 operand ZA[<Wv>, <offs>] (e.g. ldr_za_ri_, str_za_ri_)
- 2 × SME ZA or ZT0 operand ZT0[<offs>] (e.g. movt_r_zt_, movt_zt_r_)
- 2 × SVE/SME register list ⟪ <Zm1>-<Zm2> ⟫[<index>] (e.g. luti6_mz4_zmz2_1, luti6_mz4_zmz2_4)
- 2 × list element <ZAt><HV>.D[<Ws> (e.g. ld1d_za_p_rrr_, st1d_za_p_rrr_)
- 2 × list element <ZAt><HV>.H[<Ws> (e.g. ld1h_za_p_rrr_, st1h_za_p_rrr_)
- 2 × list element <ZAt><HV>.Q[<Ws> (e.g. ld1q_za_p_rrr_, st1q_za_p_rrr_)
- 2 × list element <ZAt><HV>.S[<Ws> (e.g. ld1w_za_p_rrr_, st1w_za_p_rrr_)
- 2 × list element ZA0<HV>.B[<Ws> (e.g. ld1b_za_p_rrr_, st1b_za_p_rrr_)
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
- 1 × SME ZA or ZT0 operand ZT0{[<offs>, MUL VL]} (e.g. movt_zt_z_)
- 1 × list element <mask> (e.g. zero_za_i_)
- 1 × list element ZT0 (e.g. zero_zt_i_)
- 1 × operand shape <Pm>.<T>[<Wv>, <imm>] (e.g. psel_p_ppi_)

### Aliases without operand support, by reason

- 2 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx2}] (e.g. MOV (mova_za_mz2_1), MOV (mova_mz_za2_1))
- 2 × SME ZA or ZT0 operand ZA.D[<Wv>, <offs>{, VGx4}] (e.g. MOV (mova_za_mz4_1), MOV (mova_mz_za4_1))
- 2 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_za2_z_b1), MOV (mova_mz2_za_b1))
- 2 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_za4_z_b1), MOV (mova_mz4_za_b1))
- 2 × SME ZA or ZT0 operand ZA0<HV>.B[<Ws>, <offs>] (e.g. MOV (mova_za_p_rz_b), MOV (mova_z_p_rza_b))
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
- 1 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_mz2_za_d1))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_mz4_za_d1))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.D[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_d))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_mz2_za_h1))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_mz4_za_h1))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.H[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_h))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.Q[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_q))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs1>:<offs2>] (e.g. MOV (mova_mz2_za_w1))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs1>:<offs4>] (e.g. MOV (mova_mz4_za_w1))
- 1 × SME ZA or ZT0 operand <ZAn><HV>.S[<Ws>, <offs>] (e.g. MOV (mova_z_p_rza_w))

