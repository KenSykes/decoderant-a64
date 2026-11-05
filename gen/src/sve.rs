//! The hand-maintained operand mapping for SVE and SME (see `operands.rs`).
//!
//! The SVE/SME rule names carry no field values (`T_B__2`, not `T_00_8B`), so
//! every value table is a hand rule: in general the alternatives, in spec order,
//! are the values the encoding allows in ascending order (`Cx::allowed`); the
//! element sizes from `tsz` are one-hot (`onehot`). Register symbols are `Z`, `P`
//! and `PN` registers numbered by their field, with `PN` fields of three bits
//! naming `PN8`-`PN15`.

use crate::model::Field;
use crate::operands::{Cx, R, fs, imm, m};
use crate::template::SymAlt;

/// The element size in bytes, as a power of two, of an element-size name.
pub(crate) fn es_log2(s: &str) -> Option<u32> {
    Some(match s {
        "B" => 0,
        "H" => 1,
        "S" => 2,
        "D" => 3,
        "Q" => 4,
        _ => return None,
    })
}

/// Element-size tables on `tsz` (`tszh:tszl`, `tsz`) and `imm13`. `tsz` is one-hot:
/// when the alternatives ascend (`B, H, S, D`), by its highest set bit if an
/// immediate shares its low bits (`tsz:imm3` shift amounts; `with_imm`), else
/// exactly (`SQXTNB`: `011` is RESERVED); when they descend (`Q, D, S, H, B`:
/// `DUP (indexed)`, where `imm2:tsz` also holds the index), by its lowest set bit.
/// Entries per alternative, as (value, mask) over the concatenated fields.
pub(crate) fn onehot(
    tf: &[Field],
    alts: &[SymAlt],
    with_imm: bool,
) -> Option<Vec<Vec<(u32, u32)>>> {
    if let [f] = tf
        && f.name == "imm13"
    {
        // The element size of a logical immediate, from `N:imms` (bit 12 and bits
        // 5:0): D `1 xxxxxx`, S `0 0xxxxx`, H `0 10xxxx`, B `0 110xxx`, `0 1110xx`,
        // `0 11110x` (2- and 4-bit patterns are shown as bytes).
        let mut out = Vec::new();
        for a in alts {
            out.push(match a.text.as_str() {
                "D" => vec![(0x1000, 0x1000)],
                "S" => vec![(0x0000, 0x1020)],
                "H" => vec![(0x0020, 0x1030)],
                // `0 11xxxx`: the immediate rejects the RESERVED `0 11111x`.
                "B" => vec![(0x0030, 0x1030)],
                _ => return None,
            });
        }
        return Some(out);
    }
    if !tf.iter().all(|f| f.name.starts_with("tsz")) {
        return None;
    }
    let total: u32 = tf.iter().map(|f| f.width).sum();
    let full = (1u32 << total) - 1;
    let ks: Vec<u32> = alts
        .iter()
        .map(|a| es_log2(&a.text))
        .collect::<Option<_>>()?;
    let ascending = ks.windows(2).all(|w| w[0] < w[1]);
    let descending = ks.windows(2).all(|w| w[0] > w[1]);
    if ascending {
        // Alternative i: bit i is the highest set bit (or the only one).
        let low = |i: u32| if with_imm { (1 << i) - 1 } else { 0 };
        Some(
            (0..ks.len() as u32)
                .map(|i| vec![(1 << i, full & !low(i))])
                .collect(),
        )
    } else if descending {
        // Element size 2^k bytes: bit k is the lowest set bit.
        Some(ks.iter().map(|&k| vec![(1 << k, (2 << k) - 1)]).collect())
    } else {
        None
    }
}

/// `<Zd>`, `<Zm>`, `<Pg>`, `<PNg>`, `<Za>`: the register kind and the number added
/// to the field (8 for three-bit `PN` fields). `None` for other symbols, including
/// ZA tiles (`<ZAda>`) and list members (`<Zn1>`, `<Pd2>`).
pub(crate) fn zreg(cx: &Cx<'_>, d: &str) -> R<Option<(&'static str, u8)>> {
    let Some(inner) = d.strip_prefix('<').and_then(|x| x.strip_suffix('>')) else {
        return Ok(None);
    };
    let (kind, rest) = if let Some(r) = inner.strip_prefix("PN") {
        ("PN", r)
    } else if let Some(r) = inner.strip_prefix('P') {
        ("P", r)
    } else if let Some(r) = inner.strip_prefix('Z') {
        ("Z", r)
    } else {
        return Ok(None);
    };
    if rest.is_empty() || !rest.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        // `<ZAda>`, `<ZM>`, `<PNd>` is handled above; anything else is not a register.
        return Ok(None);
    }
    if rest.chars().any(|c| c.is_ascii_digit() || c == '+') {
        return Err(format!("multi-vector or pair register {d}"));
    }
    let width: u32 = cx.fields(d)?.iter().map(|f| f.width).sum();
    let add = if kind == "PN" && width == 3 { 8 } else { 0 };
    Ok(Some((kind, add)))
}

/// An element-size word or `<T>` for an SVE register (`NONE`: no size).
fn arr(cx: &mut Cx<'_>, t: &str) -> R<String> {
    cx.arr(t)
}

fn zr(cx: &mut Cx<'_>, d: &str, t: &str, pq: &str) -> R<String> {
    let Some((k, add)) = zreg(cx, d)? else {
        return Err(format!("register {d}"));
    };
    let a = arr(cx, t)?;
    let f = cx.field(d)?;
    cx.hand(&format!("SVE register <{k}..>"));
    Ok(format!("Op::Zr({f}, Rk::{k}, {a}, {add}, {pq})"))
}

/// The index of an indexed SVE register (`Zm.S[<imm>]`, `Zn[<index>]`).
fn zindex(cx: &mut Cx<'_>, idx: &str) -> R<String> {
    let fields = cx.fields(idx)?;
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    if names.last() == Some(&"tsz") {
        // `DUP <Zd>.<T>, <Zn>.<T>[<imm>]`: `imm2:tsz`, the bits above the lowest set
        // bit of `tsz`.
        cx.hand("SVE index above the lowest set bit of tsz");
        return Ok(format!("Idx::Imm5({})", fs(&fields)));
    }
    let v: Vec<Field> = fields
        .iter()
        .map(|f| cx.variable_part(f).unwrap_or_else(|| f.clone()))
        .collect();
    cx.hand("SVE index fields");
    Ok(format!("Idx::Bits({}, 0)", fs(&v)))
}

fn zelem(cx: &mut Cx<'_>, d: &str, t: &str, idx: &str) -> R<String> {
    let Some((k, add)) = zreg(cx, d)? else {
        return Err(format!("indexed register {d}"));
    };
    if add != 0 {
        return Err(format!("indexed register {d} with an offset"));
    }
    let a = arr(cx, t)?;
    let f = cx.field(d)?;
    let i = zindex(cx, idx)?;
    cx.hand(&format!("SVE indexed register <{k}..>"));
    Ok(format!("Op::ZElem({f}, Rk::{k}, {a}, {i})"))
}

/// Splits `imm13` into `N`, `immr`, `imms`.
fn imm13(f: &Field) -> [Field; 3] {
    let mk = |name: &str, lsb: u32, width: u32| Field {
        name: name.into(),
        lsb: f.lsb + lsb,
        width,
    };
    [mk("N", 12, 1), mk("immr", 6, 6), mk("imms", 0, 6)]
}

/// Shift instructions whose `tsz:imm3` amount counts up from the element size
/// (left shifts); the others count down from twice the size.
fn left_shift(mnemonic: &str) -> bool {
    mnemonic.contains("SHL") || mnemonic.starts_with("LSL") || mnemonic == "SLI"
}

/// `#<sym>` in an SVE/SME encoding.
fn hash_imm(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let fields = cx.fields(d).unwrap_or_default();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    if names == ["imm13"] {
        let [n, immr, imms] = imm13(&fields[0]);
        cx.hand("SVE logical immediate (one element)");
        return Ok(format!(
            "Op::BitImm(Imm::SveBitmask {{ n: {}, immr: {}, imms: {} }})",
            fs(&[n]),
            fs(&[immr]),
            fs(&[imms])
        ));
    }
    if names.first().is_some_and(|n| n.starts_with("tsz")) && names.last() == Some(&"imm3") {
        let f = fs(&fields);
        if left_shift(&cx.mnemonic) {
            cx.hand("SVE shift tsz:imm3 left");
            return Ok(format!("Op::Imm(Imm::ShlEs({f}), ImmShift::None)"));
        }
        cx.hand("SVE shift tsz:imm3 right");
        return Ok(format!("Op::Imm(Imm::ShrEs({f}), ImmShift::None)"));
    }
    if names == ["imm8"] && matches!(cx.mnemonic.as_str(), "FDUP" | "FCPY" | "FMOV") {
        cx.hand("SVE FP immediate");
        return Ok(format!("Op::FpImm(Imm::Fp8({}))", fs(&fields)));
    }
    if cx.is_table_sym(d) && fp_table(cx, d) {
        return fp_tab(cx, d);
    }
    imm(cx, d)
}

/// A table of FP constants (`#0.5`/`#1.0`).
fn fp_table(cx: &Cx<'_>, d: &str) -> bool {
    cx.sym(d).is_ok_and(|s| {
        s.alts
            .iter()
            .all(|a| a.text.trim_start_matches('#').contains('.'))
    })
}

fn fp_tab(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let s = cx.sym(d)?;
    let mut nums: Vec<i64> = Vec::new();
    for a in &s.alts {
        let t = a.text.trim_start_matches('#');
        let v: f64 = t.parse().map_err(|_| format!("FP constant {t:?}"))?;
        nums.push(v.to_bits() as i64);
    }
    let n2 = nums.clone();
    let t = cx.table_full(
        d,
        move |t| {
            let v: f64 = t.trim_start_matches('#').parse().ok()?;
            n2.iter()
                .position(|&x| x == v.to_bits() as i64)
                .map(|i| i as u8)
        },
        &[],
        &nums,
    )?;
    cx.hand("SVE FP constant pair");
    Ok(format!("Op::FpImm(Imm::Tab({t}))"))
}

/// One operand group of an SVE/SME encoding, or `None` to use the common rules
/// (general-purpose and scalar registers, `#<imm>{, <shift>}`, ...).
pub(crate) fn group(cx: &mut Cx<'_>, g: &str) -> Option<R<String>> {
    // Later stages.
    if g.starts_with('[') {
        return Some(Err(format!("SVE/SME memory operand {g}")));
    }
    if g.starts_with('⟪') {
        return Some(Err(format!("SVE/SME register list {g}")));
    }
    if g.starts_with("ZA") || g.starts_with("<ZA") || g.starts_with("ZT0") {
        return Some(Err(format!("SME ZA or ZT0 operand {g}")));
    }
    if g.contains("<pattern>") {
        return Some(Err(format!("SVE pattern {g}")));
    }
    if let Some(c) = m("%", g) {
        return match zreg(cx, &c[0]) {
            Ok(Some(_)) => Some(zr(cx, &c[0], "NONE", "Pq::None")),
            Ok(None) if c[0] == "<prfop>" => Some(Err("SVE prefetch operation".into())),
            Ok(None) if cx.is_table_sym(&c[0]) && fp_table(cx, &c[0]) => Some(fp_tab(cx, &c[0])),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        };
    }
    if let Some(c) = m("%/Z", g) {
        return Some(zr(cx, &c[0], "NONE", "Pq::Zeroing"));
    }
    if let Some(c) = m("%/M", g) {
        return Some(zr(cx, &c[0], "NONE", "Pq::Merging"));
    }
    if let Some(c) = m("%/%", g) {
        let r = (|| {
            let s = cx.sym(&c[1])?;
            let names: Vec<String> = s.alts.iter().map(|a| a.text.clone()).collect();
            if names != ["Z", "M"] {
                return Err(format!("predicate qualifier {}", c[1]));
            }
            let t = cx.table_full(
                &c[1],
                |x| ["Z", "M"].iter().position(|n| *n == x).map(|p| p as u8),
                &[],
                &[],
            )?;
            zr(cx, &c[0], "NONE", &format!("Pq::Tab({t})"))
        })();
        return Some(r);
    }
    if let Some(c) = m("%.%", g).or_else(|| m("%.%w", g)) {
        return match zreg(cx, &c[0]) {
            Ok(Some(_)) => Some(zr(cx, &c[0], &c[1], "Pq::None")),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        };
    }
    if let Some(c) = m("%.%[%]", g).or_else(|| m("%.%w[%]", g)) {
        return match zreg(cx, &c[0]) {
            Ok(Some(_)) => Some(zelem(cx, &c[0], &c[1], &c[2])),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        };
    }
    if let Some(c) = m("%[%]", g) {
        return match zreg(cx, &c[0]) {
            Ok(Some(_)) => Some(zelem(cx, &c[0], "NONE", &c[1])),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        };
    }
    if let Some(c) = m("%{[%]}", g) {
        return Some(Err(format!("optional index on {}", c[0])));
    }
    if let Some(c) = m("#%", g) {
        return Some(hash_imm(cx, &c[0]));
    }
    None
}
