//! The hand-maintained UNDEFINED table: field values that the encoding's decode
//! pseudocode rejects. The open-source AARCHMRS package omits the ASL, so these are
//! not in the generated data; each entry cites the pseudocode it mirrors (Arm ARM,
//! the encoding's "Decode for this encoding"). Reviewed like the operand mapping and
//! checked by the differential tests (a missing entry shows up as an oracle
//! rejecting a word we accept).

use crate::tables::EncodingData;

/// Whether `word` is UNDEFINED for the encoding even though it matches the
/// encoding's pattern and conditions.
pub(crate) fn undefined(d: &EncodingData, w: u32) -> bool {
    let bit = |n: u32| w >> n & 1 == 1;
    let sf32 = !bit(31);
    match d.iclass {
        // `if sf == '0' && imm6<5> == '1' then UNDEFINED;`
        "log_shift" | "addsub_shift" => sf32 && bit(15),
        // `if sf == '0' && (N != '0' || immr<5> != '0' || imms<5> != '0') then
        // UNDEFINED;`
        "bitfield" => sf32 && (bit(22) || bit(21) || bit(15)),
        // `if sf == '0' && imms<5> == '1' then UNDEFINED;`
        "extract" => sf32 && bit(15),
        // `if sf == '0' && hw<1> == '1' then UNDEFINED;`
        "movewide" => sf32 && bit(22),
        // `if sf == '0' && scale<5> == '0' then UNDEFINED;`
        "float2fix" => sf32 && !bit(15),
        // CASP: `if Rs<0> == '1' then UNDEFINED; if Rt<0> == '1' then UNDEFINED;`
        "comswappr" | "comswappr_unpriv" | "rcwcomswappr" => bit(16) || bit(0),
        // CPY*/SET*: `if sz != '00' then UNDEFINED;` (sz is bits 31:30).
        "memcms" | "memset_go" => w >> 30 != 0,
        // MRRS/MSRR: `if t<0> == '1' then UNDEFINED;` (a register pair).
        "systemmovepr" => bit(0),
        // SDOT/UDOT (vector and by element): `if size != '10' then UNDEFINED;`
        "asimdsame2" | "asimdelem"
            if matches!(
                d.name,
                "SDOT_asimdsame2_D" | "UDOT_asimdsame2_D" | "SDOT_asimdelem_D" | "UDOT_asimdelem_D"
            ) =>
        {
            w >> 22 & 3 != 0b10
        }
        // LD64B/ST64B/ST64BV/ST64BV0: `if t > 22 || t<0> == '1' then UNDEFINED;`
        "memop" if d.name.starts_with("LD64B") || d.name.starts_with("ST64B") => {
            let t = w & 31;
            t > 22 || t & 1 == 1
        }
        // LDCLRP/LDSETP/SWPP and the RCW pair forms: `if t == 31 || t2 == 31 then
        // UNDEFINED;` (Rt and Rs hold the pair).
        "memop_128" => w & 31 == 31 || w >> 16 & 31 == 31,
        // EXT: `if Q == '0' && imm4<3> == '1' then UNDEFINED;`
        "asimdext" => !bit(30) && bit(14),
        // SYSP: `if t<0> == '1' && t != 31 then UNDEFINED;` (Rt names a pair).
        "syspairinstrs" => w & 1 == 1 && w & 31 != 31,
        _ => false,
    }
}

/// Fields the decode pseudocode pins to one value but no operand sets: the encoder
/// starts from these bits (`mask`, `value`).
pub(crate) const fn pinned(d: &EncodingData) -> (u32, u32) {
    // SDOT/UDOT: `size` must be 10.
    const SIZE_10: &[&str] = &[
        "SDOT_asimdsame2_D",
        "UDOT_asimdsame2_D",
        "SDOT_asimdelem_D",
        "UDOT_asimdelem_D",
    ];
    if any_eq(SIZE_10, d.name) {
        (3 << 22, 2 << 22)
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
