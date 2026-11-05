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
    if !tf
        .iter()
        .all(|f| f.name.starts_with("tsz") || f.name == "tsize")
    {
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
    if names.last().is_some_and(|n| n.starts_with("tsz")) {
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
    let a = arr(cx, t)?;
    let f = cx.field(d)?;
    let i = zindex(cx, idx)?;
    cx.hand(&format!("SVE indexed register <{k}..>"));
    Ok(format!("Op::ZElem({f}, Rk::{k}, {a}, {i}, {add})"))
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
    if names.first() == Some(&"tsize") && matches!(names.last(), Some(&"imm5" | &"imm3")) {
        // Multi-vector narrowing shifts (`SQRSHR <Zd>.B, { <Zn1>.S-<Zn4>.S }`): as
        // `immh:immb`, `2 * esize - tsize:imm`, the size from `tsize`'s top bit.
        let f = fs(&fields);
        if left_shift(&cx.mnemonic) {
            return Err(format!("left shift from {}", names.join(":")));
        }
        cx.hand("SVE shift tsize:imm right");
        return Ok(format!("Op::Imm(Imm::ShrEs({f}), ImmShift::None)"));
    }
    if matches!(names.as_slice(), ["imm4"] | ["imm3"]) && cx.mnemonic.contains("SHR") {
        // `SQRSHRN <Zd>.H, { <Zn1>.S-<Zn2>.S }, #<const>`: 16 - imm4 (and 8 - imm3
        // for byte results).
        cx.hand("SVE narrowing shift 2^width - imm");
        return Ok(format!(
            "Op::Imm(Imm::Rsub({}, {}), ImmShift::None)",
            1u32 << fields[0].width,
            fs(&fields)
        ));
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

/// A table of names (`<vl>`: `VLx2`, `VLx4`).
fn name_table(cx: &Cx<'_>, d: &str) -> bool {
    cx.sym(d).is_ok_and(|s| {
        s.alts.iter().all(|a| {
            a.text
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        })
    })
}

fn names(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let s = cx.sym(d)?;
    let names: Vec<String> = s.alts.iter().map(|a| a.text.clone()).collect();
    let n2 = names.clone();
    let t = cx.table_full(
        d,
        move |x| n2.iter().position(|n| n == x).map(|p| p as u8),
        &names,
        &[],
    )?;
    Ok(format!("Op::Name({t}, NameKind::Plain)"))
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
    if g.starts_with('[') {
        return Some(mem(cx, g));
    }
    if g.starts_with('⟪') {
        return Some(list(cx, g));
    }
    // Later stages.
    if g.starts_with("ZA") || g.starts_with("<ZA") || g.starts_with("ZT0") {
        return Some(Err(format!("SME ZA or ZT0 operand {g}")));
    }
    if let Some(c) = m("%", g) {
        return match zreg(cx, &c[0]) {
            Ok(Some(_)) => Some(zr(cx, &c[0], "NONE", "Pq::None")),
            Ok(None) if c[0] == "<prfop>" => Some(prfop(cx, &c[0])),
            Ok(None) if cx.is_table_sym(&c[0]) && fp_table(cx, &c[0]) => Some(fp_tab(cx, &c[0])),
            Ok(None) if cx.is_table_sym(&c[0]) && name_table(cx, &c[0]) => Some(names(cx, &c[0])),
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
    if let Some(c) = m("%.%[%, %]", g) {
        // `PSEL <Pd>, <Pn>, <Pm>.<T>[<Wv>, <imm>]`: `Wv` is W12-W15.
        let r = (|| {
            let Some((k, 0)) = zreg(cx, &c[0])? else {
                return Err(format!("indexed register {}", c[0]));
            };
            let a = cx.arr(&c[1])?;
            let f = cx.field(&c[0])?;
            let v = cx.field(&c[2])?;
            let i = zindex(cx, &c[3])?;
            cx.hand("register indexed by W12-W15 and an immediate");
            Ok(format!("Op::ZSel({f}, Rk::{k}, {a}, {v}, {i})"))
        })();
        return Some(r);
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
        // `PMOV <Zd>{[<imm>]}, <Pn>.H`: the index is left out when 0.
        let r = (|| {
            let Some((k, add)) = zreg(cx, &c[0])? else {
                return Err(format!("indexed register {}", c[0]));
            };
            let f = cx.field(&c[0])?;
            let i = cx.field(&c[1])?;
            cx.hand("SVE optional index, left out at 0");
            Ok(format!(
                "Op::ZElem({f}, Rk::{k}, Arr::Fixed(Arrangement::None), Idx::Opt({i}), {add})"
            ))
        })();
        return Some(r);
    }
    if let Some(c) = m("#%", g) {
        return Some(hash_imm(cx, &c[0]));
    }
    None
}

/// Groups that are several operands: `<Xd>{, <pattern>{, MUL #<imm>}}` (`CNTB`,
/// `INCH`, `SQINCB`) and `<Pd>.<T>{, <pattern>}` (`PTRUE`).
pub(crate) fn multi(cx: &mut Cx<'_>, g: &str) -> Option<R<Vec<String>>> {
    let (main, mul) = if let Some(m) = g.strip_suffix("{, <pattern>{, MUL #<imm>}}") {
        (m, true)
    } else {
        (g.strip_suffix("{, <pattern>}")?, false)
    };
    Some(pattern(cx, main, mul))
}

/// `POW2`, `VL1`-`VL8`, `VL16`-`VL256`, `MUL4`, `MUL3`, `ALL`; other values are
/// `#uimm5`. Left out when `ALL` (and `MUL #1`); `MUL #<imm>` left out when 1.
fn pattern(cx: &mut Cx<'_>, main: &str, mul: bool) -> R<Vec<String>> {
    let mut ops = vec![crate::operands::group(cx, main, None)?];
    let pf = cx.fields("<pattern>")?;
    let mut e: Vec<(u32, u32, u8)> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut named = |v: u32, n: String| {
        e.push((v, 0x1f, names.len() as u8));
        names.push(n);
    };
    named(0, "POW2".into());
    for (i, n) in [1, 2, 3, 4, 5, 6, 7, 8, 16, 32, 64, 128, 256]
        .iter()
        .enumerate()
    {
        named(1 + i as u32, format!("VL{n}"));
    }
    named(29, "MUL4".into());
    named(30, "MUL3".into());
    named(31, "ALL".into());
    let nr: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let t = cx.hand_table("SVE predicate pattern", &pf, &e, &nr);
    let name = format!("Op::Name({t}, NameKind::SvePattern)");
    if mul {
        let imm = cx.fields("<imm>")?;
        let mut both = pf.clone();
        both.extend(imm.iter().cloned());
        cx.hand("SVE pattern and MUL #imm, left out at ALL and 1");
        ops.push(format!(
            "Op::Opt(&{name}, {}, {:#x})",
            fs(&both),
            31 << imm[0].width
        ));
        ops.push(format!(
            "Op::Opt(&Op::MulImm(Imm::Add({f}, 1)), {f}, 0)",
            f = fs(&imm)
        ));
    } else {
        cx.hand("SVE pattern, left out at ALL");
        ops.push(format!("Op::Opt(&{name}, {}, 31)", fs(&pf)));
    }
    Ok(ops)
}

/// SVE prefetch operations (`prfop`, four bits): `PLDL1KEEP`..`PLDL3STRM` are 0-5,
/// `PSTL1KEEP`..`PSTL3STRM` 8-13; the rest are `#imm`.
fn prfop(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let f = cx.fields(d)?;
    let mut e = Vec::new();
    let mut names = Vec::new();
    for (ti, t) in ["PLD", "PST"].iter().enumerate() {
        for (gi, g) in ["L1", "L2", "L3"].iter().enumerate() {
            for (pi, p) in ["KEEP", "STRM"].iter().enumerate() {
                let v = (ti as u32) << 3 | (gi as u32) << 1 | pi as u32;
                e.push((v, 0xf, names.len() as u8));
                names.push(format!("{t}{g}{p}"));
            }
        }
    }
    let nr: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let t = cx.hand_table("SVE prefetch operations", &f, &e, &nr);
    Ok(format!("Op::Name({t}, NameKind::Prefetch)"))
}

/// The access size (log2 bytes) of an SVE load, store or prefetch, from the last
/// letter of its mnemonic (`LD1SB`, `LD1RQD`, `PRFW`, `ST1Q`).
fn msz(mnemonic: &str) -> R<u8> {
    Ok(match mnemonic.chars().last() {
        Some('B') => 0,
        Some('H') => 1,
        Some('W') => 2,
        Some('D') => 3,
        Some('Q') => 4,
        _ => return Err(format!("access size of {mnemonic}")),
    })
}

/// A base, index or element of an SVE memory operand: the register symbol and its
/// element size (`<Zn>.D`), if any.
fn split_es(s: &str) -> (&str, Option<&str>) {
    match s.split_once('.') {
        Some((r, e)) => (r, Some(e)),
        None => (s, None),
    }
}

/// The base of an SVE memory operand: `<Xn|SP>` or a vector `<Zn>.D`.
fn mem_base(cx: &mut Cx<'_>, b: &str) -> R<(String, &'static str, String)> {
    if let (r, Some(es)) = split_es(b) {
        if zreg(cx, r)? != Some(("Z", 0)) {
            return Err(format!("vector base {b}"));
        }
        let a = cx.arr(es)?;
        return Ok((cx.field(r)?, "Z", a));
    }
    let (k, _) = crate::operands::reg_sym(b).ok_or_else(|| format!("memory base {b}"))?;
    Ok((cx.field(b)?, k, "Arr::Fixed(Arrangement::None)".into()))
}

const EXTENDS: [&str; 9] = [
    "UXTB", "UXTH", "UXTW", "UXTX", "SXTB", "SXTH", "SXTW", "SXTX", "LSL",
];

/// The index of an SVE memory operand (`<Xm>`, `<Zm>.D`): the offset descriptor
/// and the element size of a vector index.
fn mem_index(
    cx: &mut Cx<'_>,
    d: &str,
    ext: Option<String>,
    amount: &str,
) -> R<(String, Option<String>)> {
    let (r, es) = split_es(d);
    let (k, arr) = match es {
        Some(es) => {
            if zreg(cx, r)? != Some(("Z", 0)) {
                return Err(format!("vector index {d}"));
            }
            ("Z", Some(cx.arr(es)?))
        }
        None => {
            let (k, _) = crate::operands::reg_sym(r).ok_or_else(|| format!("index {d}"))?;
            if k != "X" {
                return Err(format!("memory index {d}"));
            }
            ("X", None)
        }
    };
    let f = cx.field(r)?;
    let ext = ext.map_or("None".to_string(), |t| format!("Some({t})"));
    Ok((
        format!(
            "MemOff::Reg {{ m: {f}, k: RegSel::Fixed(Rk::{k}), ext: {ext}, amount: {amount} }}"
        ),
        arr,
    ))
}

/// Matches a memory shape whose `%` symbols may carry an element size: `.%w` after
/// a captured symbol is folded into it (`<Zm>.D`).
fn mm(pat: &str, s: &str) -> Option<Vec<String>> {
    // Fold `<sym>.<T>` and `<sym>.D` into one token for matching.
    let mut folded = String::new();
    let mut parts = Vec::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '<' {
            let st = i;
            while i < b.len() && b[i] != '>' {
                i += 1;
            }
            i += 1;
            if i < b.len() && b[i] == '.' {
                i += 1;
                if i < b.len() && b[i] == '<' {
                    while i < b.len() && b[i] != '>' {
                        i += 1;
                    }
                    i += 1;
                } else {
                    while i < b.len() && b[i].is_ascii_alphanumeric() {
                        i += 1;
                    }
                }
            }
            parts.push(b[st..i].iter().collect::<String>());
            folded.push_str(&format!("<{}>", parts.len() - 1));
        } else {
            folded.push(b[i]);
            i += 1;
        }
    }
    let caps = m(pat, &folded)?;
    Some(
        caps.into_iter()
            .map(|c| {
                c.strip_prefix('<')
                    .and_then(|x| x.strip_suffix('>'))
                    .and_then(|x| x.parse::<usize>().ok())
                    .and_then(|k| parts.get(k).cloned())
                    .unwrap_or(c)
            })
            .collect(),
    )
}

/// An SVE memory operand.
fn mem(cx: &mut Cx<'_>, g: &str) -> R<String> {
    let mk = |base: &(String, &str, String), arr: Option<String>, off: String, vl: bool| {
        format!(
            "Op::Mem(Mem {{ base: {}, base_k: Rk::{}, off: {off}, mode: MemoryMode::Offset, post: Post::None, writeback: false, arr: {}, mul_vl: {vl} }})",
            base.0,
            base.1,
            arr.unwrap_or_else(|| base.2.clone())
        )
    };
    cx.hand("SVE memory operand");
    if let Some(c) = mm("[%]", g) {
        let b = mem_base(cx, &c[0])?;
        return Ok(mk(&b, None, "MemOff::None".into(), false));
    }
    // `[<Xn|SP>{, #<imm>, MUL VL}]`: in vector lengths, signed as the spec says; for
    // a list of registers (`LD3B`, `ST1W { <Zt1>.S-<Zt4>.S }`) in multiples of its
    // length.
    if let Some(c) = mm("[%{, #%, MUL VL}]", g) {
        let b = mem_base(cx, &c[0])?;
        let regs = cx
            .order
            .iter()
            .filter_map(|s| s.strip_prefix("<Zt").and_then(|x| x.strip_suffix('>')))
            .filter_map(|n| n.parse::<u8>().ok())
            .max()
            .unwrap_or(1);
        let i = if regs > 1 {
            cx.hand("SVE MUL VL offset in multiples of the list length");
            format!("Imm::SMul({}, {regs})", cx.field(&c[1])?)
        } else {
            crate::operands::sym_imm(cx, &c[1])?
        };
        return Ok(mk(&b, None, format!("MemOff::Imm({i})"), true));
    }
    // `[<Xn|SP>{, #<imm>}]` (`LD1R*`, `LD1RQ*`, `LD1RO*`), `[<Zn>.D{, #<imm>}]`:
    // scaled by the access size, by 16 for `LD1RQ` and by 32 for `LD1RO`.
    if let Some(c) = mm("[%{, #%}]", g) {
        let b = mem_base(cx, &c[0])?;
        let mn = cx.mnemonic.clone();
        let scale = if mn.starts_with("LD1RQ") {
            4
        } else if mn.starts_with("LD1RO") {
            5
        } else if matches!(mn.as_str(), "LDAPRO" | "STLRO") {
            // The open-source spec gives these (2026) no scaling: the field value.
            cx.hand("LDAPRO/STLRO offset as the field value");
            0
        } else {
            msz(&mn)?
        };
        let f = cx.field(&c[1])?;
        let signed = cx.sym(&c[1])?.alts.iter().any(|a| a.signed);
        let i = format!("Imm::{}({f}, {scale})", if signed { "S" } else { "U" });
        cx.hand("SVE memory offset scaled by the access size");
        return Ok(mk(&b, None, format!("MemOff::Imm({i})"), false));
    }
    // `[<Xn|SP>{, <Xm>}]`, `[<Xn|SP>{, <Xm>, LSL #1}]`, `[<Zn>.D{, <Xm>}]`: the
    // index is left out when it is XZR.
    if let Some(c) = mm("[%{, %}]", g) {
        let b = mem_base(cx, &c[0])?;
        let f = cx.field(&c[1])?;
        return Ok(mk(&b, None, format!("MemOff::OptReg({f}, 0)"), false));
    }
    if let Some(c) = mm("[%{, %, LSL #%w}]", g) {
        let b = mem_base(cx, &c[0])?;
        let a: u8 = c[2].parse().map_err(|_| "lsl amount")?;
        let f = cx.field(&c[1])?;
        return Ok(mk(&b, None, format!("MemOff::OptReg({f}, {a})"), false));
    }
    // `[<Xn|SP>, <Xm>]`, `[<Xn|SP>, <Zm>.D]`.
    if let Some(c) = mm("[%, %]", g) {
        let b = mem_base(cx, &c[0])?;
        let (off, arr) = mem_index(cx, &c[1], None, "Amount::None")?;
        return Ok(mk(&b, arr, off, false));
    }
    // `[<Xn|SP>, <Xm>, LSL #3]`, `[<Xn|SP>, <Zm>.D, LSL #3]`.
    if let Some(c) = mm("[%, %, LSL #%w]", g) {
        let b = mem_base(cx, &c[0])?;
        let a: u8 = c[2].parse().map_err(|_| "lsl amount")?;
        let (off, arr) = mem_index(cx, &c[1], None, &format!("Amount::Fixed({a})"))?;
        return Ok(mk(&b, arr, off, false));
    }
    // `[<Xn|SP>, <Zm>.D, <mod>]`, `[<Xn|SP>, <Zm>.S, <mod> #2]`.
    if let Some(c) = mm("[%, %, % #%w]", g).or_else(|| mm("[%, %, %]", g)) {
        let b = mem_base(cx, &c[0])?;
        let t = cx.table(&c[2], |s| {
            EXTENDS.iter().position(|x| *x == s).map(|i| i as u8)
        })?;
        let amount = match c.get(3) {
            Some(a) => {
                let a: u8 = a.parse().map_err(|_| "extend amount")?;
                format!("Amount::Fixed({a})")
            }
            None => "Amount::None".into(),
        };
        let (off, arr) = mem_index(cx, &c[1], Some(t), &amount)?;
        return Ok(mk(&b, arr, off, false));
    }
    // `ADR <Zd>.D, [<Zn>.D, <Zm>.D, SXTW<amount>]` and `ADR <Zd>.<T>, [<Zn>.<T>,
    // <Zm>.<T>{, LSL <amount>}]`: `msz` is the amount (0: none printed).
    let adr = mm("[%, %, SXTW%]", g)
        .map(|c| (c, Some(6)))
        .or_else(|| mm("[%, %, UXTW%]", g).map(|c| (c, Some(2))))
        .or_else(|| mm("[%, %{, % %}]", g).map(|c| (c, None)));
    if let Some((c, ext)) = adr {
        let b = mem_base(cx, &c[0])?;
        let amount_sym = c.last().cloned().unwrap_or_default();
        let msz = cx.fields(&amount_sym)?;
        let ext = ext.map(|code| cx.hand_table("ADR fixed extend", &msz, &[(0, 0, code)], &[]));
        // The amount is `msz` itself, as a number table.
        cx.hand("ADR shift amount from msz");
        let body = format!(
            "Tab {{ f: {}, e: &[(0x0, 0x3, 0), (0x1, 0x3, 1), (0x2, 0x3, 2), (0x3, 0x3, 3)], names: &[], nums: &[0, 1, 2, 3] }}",
            fs(&msz)
        );
        let amt = cx.tab(body);
        let (off, arr) = mem_index(cx, &c[1], ext, &format!("Amount::Tab({amt})"))?;
        return Ok(mk(&b, arr, off, false));
    }
    Err(format!("SVE memory shape {g}"))
}

/// A list of SVE or SME registers (`⟪ <Zt>.D ⟫`, `⟪ <Zt1>.B, <Zt2>.B ⟫`,
/// `⟪ <Zdn1>.<T>-<Zdn4>.<T> ⟫`, `⟪ <Pd1>.<T>, <Pd2>.<T> ⟫`).
fn list(cx: &mut Cx<'_>, g: &str) -> R<String> {
    let (body, idx) = g
        .strip_prefix('⟪')
        .and_then(|x| x.rsplit_once('⟫'))
        .ok_or_else(|| format!("SVE/SME register list {g}"))?;
    let body = body.trim();
    // `⟪ <Zm1>-<Zm2> ⟫[<index>]`.
    let idx = if idx.is_empty() {
        "None".to_string()
    } else {
        let c = m("[%]", idx).ok_or_else(|| format!("list index {idx}"))?;
        format!("Some({})", zindex(cx, &c[0])?)
    };
    let (elems, range): (Vec<&str>, bool) = match body.split_once('-') {
        Some((a, b)) if !body.contains(", ") => (vec![a, b], true),
        _ => (body.split(", ").collect(), false),
    };
    let first = elems[0];
    // Elements without an element size (`LUTI6 ..., ⟪ <Zm1>-<Zm2> ⟫[<index>]`).
    let c = m("%.%", first)
        .or_else(|| m("%.%w", first))
        .or_else(|| {
            m("%", first).map(|mut c| {
                c.push("NONE".into());
                c
            })
        })
        .ok_or_else(|| format!("list element {first}"))?;
    let n: u8 = if range {
        // `<Zdn4>`: the last register's number is the length.
        elems[1]
            .split('>')
            .next()
            .and_then(|s| s.chars().last())
            .and_then(|ch| ch.to_digit(10))
            .ok_or_else(|| format!("list range {body}"))? as u8
    } else {
        elems.len() as u8
    };
    let sym = &c[0];
    let kind = if sym.starts_with("<PN") {
        "PN"
    } else if sym.starts_with("<P") {
        "P"
    } else if sym.starts_with("<Z") && !sym.starts_with("<ZA") {
        "Z"
    } else {
        return Err(format!("list element {first}"));
    };
    let fields = cx.fields(sym)?;
    let width: u32 = fields.iter().map(|f| f.width).sum();
    let a = cx.arr(&c[1])?;
    let f = fs(&fields);
    let regs: u32 = if kind == "Z" { 32 } else { 16 };
    // `T:Zt` or `D:Zd`: an SME2 strided list.
    let strided = fields.len() == 2 && fields[0].width == 1 && n > 1;
    let (stride, mul) = if strided {
        (16 / n, 1)
    } else {
        let mul = regs >> width;
        if mul == 0 || (mul != 1 && mul != u32::from(n)) {
            return Err(format!("list {body}: {width}-bit field for {n} registers"));
        }
        (1, mul as u8)
    };
    cx.hand(if strided {
        "SME2 strided register list"
    } else if mul > 1 {
        "multi-vector register list, a multiple of its length"
    } else {
        "consecutive register list"
    });
    Ok(format!(
        "Op::ZList({f}, Rk::{kind}, {n}, {stride}, {mul}, {a}, {idx})"
    ))
}
