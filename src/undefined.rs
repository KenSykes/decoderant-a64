//! The hand-maintained UNDEFINED table: field values that the encoding's decode
//! pseudocode rejects. The open-source AARCHMRS package omits the ASL, so these are
//! not in the generated data; each entry cites the pseudocode it mirrors (Arm ARM,
//! the encoding's "Decode for this encoding"). Reviewed like the operand mapping and
//! checked by the differential tests (a missing entry shows up as an oracle
//! rejecting a word we accept).

use crate::generated::CODE_COUNT;
use crate::generated::tables::ENCODINGS;
use crate::tables::EncodingData;

/// The UNDEFINED rule of an encoding, chosen by its iclass and name. Matching the
/// strings once, in a constant table, keeps the decoder from comparing them per word.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    None,
    Sf32Bit15,
    Bitfield,
    Sf32NotBit15,
    Sf32Bit22,
    Casp,
    Sz,
    Bit0,
    DotSize,
    Ld64b,
    Pair128,
    Ext,
    Sysp,
    SveShift,
    Pinned,
}

/// SDOT/UDOT with doubleword elements: `size` must be 10.
const SDOT_D: &[&str] = &[
    "SDOT_asimdsame2_D",
    "UDOT_asimdsame2_D",
    "SDOT_asimdelem_D",
    "UDOT_asimdelem_D",
];

const fn starts_with(s: &str, p: &str) -> bool {
    let (s, p) = (s.as_bytes(), p.as_bytes());
    if s.len() < p.len() {
        return false;
    }
    let mut i = 0;
    while i < p.len() {
        if s[i] != p[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn rule(d: &EncodingData) -> Rule {
    let (c, name) = (d.iclass, d.name);
    if any_eq(&["log_shift", "addsub_shift", "extract"], c) {
        // `if sf == '0' && imm6<5> == '1' then UNDEFINED;`
        // `if sf == '0' && imms<5> == '1' then UNDEFINED;`
        Rule::Sf32Bit15
    } else if any_eq(&["bitfield"], c) {
        // `if sf == '0' && (N != '0' || immr<5> != '0' || imms<5> != '0') then
        // UNDEFINED;`
        Rule::Bitfield
    } else if any_eq(&["movewide"], c) {
        // `if sf == '0' && hw<1> == '1' then UNDEFINED;`
        Rule::Sf32Bit22
    } else if any_eq(&["float2fix"], c) {
        // `if sf == '0' && scale<5> == '0' then UNDEFINED;`
        Rule::Sf32NotBit15
    } else if any_eq(&["comswappr", "comswappr_unpriv", "rcwcomswappr"], c) {
        // CASP: `if Rs<0> == '1' then UNDEFINED; if Rt<0> == '1' then UNDEFINED;`
        Rule::Casp
    } else if any_eq(&["memcms", "memset_go"], c) {
        // CPY*/SET*: `if sz != '00' then UNDEFINED;` (sz is bits 31:30).
        Rule::Sz
    } else if any_eq(&["systemmovepr"], c) {
        // MRRS/MSRR: `if t<0> == '1' then UNDEFINED;` (a register pair).
        Rule::Bit0
    } else if any_eq(&["asimdsame2", "asimdelem"], c) && any_eq(SDOT_D, name) {
        // SDOT/UDOT (vector and by element): `if size != '10' then UNDEFINED;`
        Rule::DotSize
    } else if any_eq(&["memop"], c) && (starts_with(name, "LD64B") || starts_with(name, "ST64B")) {
        // LD64B/ST64B/ST64BV/ST64BV0: `if t > 22 || t<0> == '1' then UNDEFINED;`
        Rule::Ld64b
    } else if any_eq(&["memop_128"], c) {
        // LDCLRP/LDSETP/SWPP and the RCW pair forms: `if t == 31 || t2 == 31 then
        // UNDEFINED;` (Rt and Rs hold the pair).
        Rule::Pair128
    } else if any_eq(&["asimdext"], c) {
        // EXT: `if Q == '0' && imm4<3> == '1' then UNDEFINED;`
        Rule::Ext
    } else if any_eq(&["syspairinstrs"], c) {
        // SYSP: `if t<0> == '1' && t != 31 then UNDEFINED;` (Rt names a pair).
        Rule::Sysp
    } else if any_eq(
        &[
            "sve_int_arith_imm0",
            "sve_int_dup_imm",
            "sve_int_dup_imm_pred",
        ],
        c,
    ) {
        // SVE unsigned and signed immediates with an optional `LSL #8`, `DUP` and
        // `CPY` (immediate): `if size:sh == '001' then UNDEFINED;` (no byte shift).
        Rule::SveShift
    } else if any_eq(&["sve", "sme"], d.group) {
        Rule::Pinned
    } else {
        Rule::None
    }
}

/// `(rule, pinned)` per encoding, indexed by `Code as usize - 1`.
static RULES: [(Rule, (u32, u32)); CODE_COUNT - 1] = {
    let mut t = [(Rule::None, (0, 0)); CODE_COUNT - 1];
    let mut i = 0;
    while i < t.len() {
        t[i] = (rule(&ENCODINGS[i]), pinned(&ENCODINGS[i]));
        i += 1;
    }
    t
};

fn check(r: Rule, pin: (u32, u32), w: u32) -> bool {
    let bit = |n: u32| w >> n & 1 == 1;
    let sf32 = !bit(31);
    match r {
        Rule::None => false,
        Rule::Sf32Bit15 => sf32 && bit(15),
        Rule::Bitfield => sf32 && (bit(22) || bit(21) || bit(15)),
        Rule::Sf32Bit22 => sf32 && bit(22),
        Rule::Sf32NotBit15 => sf32 && !bit(15),
        Rule::Casp => bit(16) || bit(0),
        Rule::Sz => w >> 30 != 0,
        Rule::Bit0 => bit(0),
        Rule::DotSize => w >> 22 & 3 != 0b10,
        Rule::Ld64b => {
            let t = w & 31;
            t > 22 || t & 1 == 1
        }
        Rule::Pair128 => w & 31 == 31 || w >> 16 & 31 == 31,
        Rule::Ext => !bit(30) && bit(14),
        Rule::Sysp => w & 1 == 1 && w & 31 != 31,
        Rule::SveShift => w >> 22 & 3 == 0 && bit(13),
        Rule::Pinned => pin.0 != 0 && w & pin.0 != pin.1,
    }
}

/// Whether `word` is UNDEFINED for the encoding even though it matches the
/// encoding's pattern and conditions.
pub(crate) fn undefined(d: &EncodingData, w: u32) -> bool {
    check(rule(d), pinned(d), w)
}

/// [`undefined`] for the encoding at index `i` of `ENCODINGS`.
#[inline]
pub(crate) fn undefined_at(i: usize, w: u32) -> bool {
    let (r, pin) = RULES[i];
    check(r, pin, w)
}

/// [`pinned`] for the encoding at index `i` of `ENCODINGS`.
#[cfg(feature = "encoder")]
#[inline]
pub(crate) fn pinned_at(i: usize) -> (u32, u32) {
    RULES[i].1
}

/// Fields the decode pseudocode pins to one value but no operand sets: the encoder
/// starts from these bits (`mask`, `value`). Outside the AdvSIMD classes that
/// check it themselves above, another value is UNDEFINED.
pub(crate) const fn pinned(d: &EncodingData) -> (u32, u32) {
    // SVE encodings whose operands have a fixed element size and whose `size`
    // field no operand reads: `if size != '11' then UNDEFINED;` (SXTW, UXTW, REVW:
    // doublewords), `if size != '10' ...` (URECPE, URSQRTE: words), `if size !=
    // '00' ...` (HISTSEG: bytes).
    const SVE_SIZE_11: &[&str] = &[
        "sxtw_z_p_z_m",
        "sxtw_z_p_z_z",
        "uxtw_z_p_z_m",
        "uxtw_z_p_z_z",
        "revw_z_z_m",
        "revw_z_z_z",
    ];
    const SVE_SIZE_10: &[&str] = &[
        "urecpe_z_p_z_m",
        "urecpe_z_p_z_z",
        "ursqrte_z_p_z_m",
        "ursqrte_z_p_z_z",
    ];
    const SVE_SIZE_00: &[&str] = &["histseg_z_zz_"];
    // SME2 LUTI4 with a fixed element size: `size` (bits 13:12) must be 00 for
    // bytes, 01 for halfwords.
    const LUTI_SIZE_00: &[&str] = &["luti4_mz4_ztmz2_1", "luti4_mz4_ztmz2_4"];
    const LUTI_SIZE_01: &[&str] = &["luti4_mz4_ztz_4"];
    if any_eq(SDOT_D, d.name) || any_eq(SVE_SIZE_10, d.name) {
        (3 << 22, 2 << 22)
    } else if any_eq(SVE_SIZE_11, d.name) {
        (3 << 22, 3 << 22)
    } else if any_eq(SVE_SIZE_00, d.name) {
        (3 << 22, 0)
    } else if any_eq(LUTI_SIZE_00, d.name) {
        (3 << 12, 0)
    } else if any_eq(LUTI_SIZE_01, d.name) {
        (3 << 12, 1 << 12)
    } else {
        (0, 0)
    }
}

/// Whether `s` is in `list` (usable in constants).
const fn any_eq(list: &[&str], s: &str) -> bool {
    let mut i = 0;
    while i < list.len() {
        let (a, b) = (list[i].as_bytes(), s.as_bytes());
        if a.len() == b.len() {
            let mut j = 0;
            while j < a.len() && a[j] == b[j] {
                j += 1;
            }
            if j == a.len() {
                return true;
            }
        }
        i += 1;
    }
    false
}
