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

2247 encodings with operand support, 2182 without. 1325 symbol value tables taken from the spec's rule names; 91 distinct hand rules (symbol meanings the spec does not give in machine-readable form), used 4700 times.

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
| `register <X..>` | 930 |
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

- 2119 × SVE/SME operands (not supported yet) (e.g. add_z_p_zz_, sub_z_p_zz_, subr_z_p_zz_)
- 24 × operand shape <Xdn>{, <pattern>{, MUL #<imm>}} (e.g. incb_r_rs_, decb_r_rs_, inch_r_rs_)
- 16 × operand shape <Wdn>{, <pattern>{, MUL #<imm>}} (e.g. sqincb_r_rs_sx, uqincb_r_rs_uw, sqdecb_r_rs_sx)
- 4 × operand shape <Xd>{, <pattern>{, MUL #<imm>}} (e.g. cntb_r_s_, cnth_r_s_, cntw_r_s_)
- 2 × literal ZT0 (e.g. ldr_zt_br_, str_zt_br_)
- 2 × operand shape ZA[<Wv>, <offs>] (e.g. ldr_za_ri_, str_za_ri_)
- 2 × operand shape ZT0[<offs>] (e.g. movt_r_zt_, movt_zt_r_)
- 2 × table <R>: rule R_X__4 carries no field values (e.g. ctermeq_rr_, ctermne_rr_)
- 1 × list element <mask> (e.g. zero_za_i_)
- 1 × list element ZT0 (e.g. zero_zt_i_)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs2>, VGx2] (e.g. zero_za2_ri_2)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs2>, VGx4] (e.g. zero_za2_ri_4)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs2>] (e.g. zero_za2_ri_1)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs4>, VGx2] (e.g. zero_za4_ri_2)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs4>, VGx4] (e.g. zero_za4_ri_4)
- 1 × operand shape ZA.D[<Wv>, <offs1>:<offs4>] (e.g. zero_za4_ri_1)
- 1 × operand shape ZA.D[<Wv>, <offs>, VGx2] (e.g. zero_za1_ri_2)
- 1 × operand shape ZA.D[<Wv>, <offs>, VGx4] (e.g. zero_za1_ri_4)
- 1 × optional group { <nb>} (e.g. TEXIT_te_branch_reg)

### Aliases without operand support, by reason

- 65 × SVE/SME operands (not supported yet) (e.g. MOV (orr_z_zz_), ORN (orr_z_zi_), EON (eor_z_zi_))
- 1 × optional group { <Xt>} (e.g. GCSPOPM (SYSL_RC_systeminstrs))

## Alias conditions not compiled

- 1 × unknown function SysLOp

