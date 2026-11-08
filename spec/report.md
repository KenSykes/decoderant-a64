# decoderant-a64-gen report

@generated from AARCHMRS_OPENSOURCE_A_profile_FAT-2026-09. Regenerate with decoderant-a64-gen; review the diff when the pin changes.

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
| sme | 842 | 842 |
| sve | 1346 | 1346 |

129 encodings overlap a higher-priority encoding (listed in `OpCodeInfo::precedence()`).

## Operand mapping

4429 encodings with operand support, 0 without. 3182 symbol value tables taken from the spec's rule names; 135 distinct hand rules (symbol meanings the spec does not give in machine-readable form), used 14147 times.

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
| `SME2 strided register list` | 71 |
| `SMSTART/SMSTOP option` | 2 |
| `SVE FP constant pair` | 8 |
| `SVE FP immediate` | 4 |
| `SVE MUL VL offset in multiples of the list length` | 94 |
| `SVE element size from tsz (one-hot) or imm13` | 132 |
| `SVE immediate, signed as the spec's number token says` | 100 |
| `SVE index above the lowest set bit of tsz` | 4 |
| `SVE index fields` | 261 |
| `SVE indexed register <PN..>` | 2 |
| `SVE indexed register <Z..>` | 247 |
| `SVE logical immediate (one element)` | 8 |
| `SVE memory offset scaled by the access size` | 65 |
| `SVE memory operand` | 502 |
| `SVE narrowing shift 2^width - imm` | 9 |
| `SVE optional index, left out at 0` | 6 |
| `SVE pattern and MUL #imm, left out at ALL and 1` | 62 |
| `SVE pattern, left out at ALL` | 2 |
| `SVE predicate pattern` | 64 |
| `SVE prefetch operations` | 28 |
| `SVE register <P..>` | 1122 |
| `SVE register <PN..>` | 140 |
| `SVE register <Z..>` | 2236 |
| `SVE register numbered by a map` | 153 |
| `SVE register width: X iff size = 11` | 11 |
| `SVE shift tsize:imm right` | 9 |
| `SVE shift tsz:imm3 left` | 10 |
| `SVE shift tsz:imm3 right` | 29 |
| `SVE size table: element size code, others RESERVED` | 568 |
| `SVE table: alternatives in allowed-value order` | 1157 |
| `ZA array vectors` | 298 |
| `ZA index register and offset` | 373 |
| `ZA tile` | 191 |
| `ZA tile slice` | 75 |
| `ZERO tile mask` | 1 |
| `ZT0 element, a byte offset in eights` | 2 |
| `ZT0 optional index in vector lengths` | 1 |
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
| `consecutive register list` | 408 |
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
| `multi-vector register list, a multiple of its length` | 865 |
| `one operand in braces` | 11 |
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
| `register <X..>` | 1009 |
| `register <XSp..>` | 96 |
| `register indexed by W12-W15 and an immediate` | 1 |
| `register list` | 164 |
| `register offset W/X by option<0>` | 24 |
| `register pair from one field` | 2 |
| `register writeback` | 132 |
| `system operation name` | 10 |
| `system register` | 4 |
| `unscaled signed offset` | 106 |
| `vector element` | 82 |

### Encodings without operand support, by reason


### Aliases without operand support, by reason


