//! The system register and system operation table from `Registers.json`: names,
//! encodings, required features and the lowest exception level that can access each.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Context, Result};
use serde_json::Value;

pub struct SysRegs {
    pub code: String,
    pub count: usize,
}

/// Three-valued logic for access conditions evaluated at a given EL.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    fn not(self) -> Tri {
        match self {
            Tri::True => Tri::False,
            Tri::False => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }
}

fn el_of(v: &Value) -> Option<u8> {
    let s = v["value"].as_str()?;
    s.strip_prefix("EL")?.parse().ok()
}

fn is_pstate_el(v: &Value) -> bool {
    v["_type"] == "AST.DotAtom"
        && v["values"]
            .as_array()
            .is_some_and(|a| a.len() == 2 && a[0]["value"] == "PSTATE" && a[1]["value"] == "EL")
}

/// Evaluates an access condition at `el`, treating everything but the current EL as
/// unknown (configuration-dependent).
fn tri(c: &Value, el: u8) -> Tri {
    if c.is_null() {
        return Tri::True;
    }
    match c["_type"].as_str().unwrap_or("") {
        "AST.Bool" => {
            if c["value"] == true {
                Tri::True
            } else {
                Tri::False
            }
        }
        "AST.UnaryOp" if c["op"] == "!" => tri(&c["expr"], el).not(),
        "AST.BinaryOp" => {
            let op = c["op"].as_str().unwrap_or("");
            match op {
                "&&" => match (tri(&c["left"], el), tri(&c["right"], el)) {
                    (Tri::False, _) | (_, Tri::False) => Tri::False,
                    (Tri::True, Tri::True) => Tri::True,
                    _ => Tri::Unknown,
                },
                "||" => match (tri(&c["left"], el), tri(&c["right"], el)) {
                    (Tri::True, _) | (_, Tri::True) => Tri::True,
                    (Tri::False, Tri::False) => Tri::False,
                    _ => Tri::Unknown,
                },
                "==" | "!=" if is_pstate_el(&c["left"]) => {
                    let Some(want) = el_of(&c["right"]) else {
                        return Tri::Unknown;
                    };
                    let eq = if want == el { Tri::True } else { Tri::False };
                    if op == "==" { eq } else { eq.not() }
                }
                "IN" if is_pstate_el(&c["left"]) => {
                    let set = c["right"]["values"].as_array().cloned().unwrap_or_default();
                    if set.iter().any(|v| el_of(v) == Some(el)) {
                        Tri::True
                    } else {
                        Tri::False
                    }
                }
                _ => Tri::Unknown,
            }
        }
        "AST.Function" => {
            let name = c["name"].as_str().unwrap_or("");
            match name {
                "EffectivelyAtEL0NotInHost" | "EffectivelyAtEL0InHost" | "IsInHost" => {
                    if el == 0 || (name == "IsInHost" && el == 2) {
                        Tri::Unknown
                    } else {
                        Tri::False
                    }
                }
                _ => Tri::Unknown,
            }
        }
        _ => Tri::Unknown,
    }
}

/// Whether an access leaf performs the access (rather than UNDEFINED or a trap).
fn leaf_accesses(v: &Value) -> bool {
    if v["_type"] == "AST.Function" {
        let name = v["name"].as_str().unwrap_or("");
        return !(name == "Undefined"
            || name.contains("Trap")
            || name == "UnimplementedIDRegister"
            || name.starts_with("Unpredictable"));
    }
    true
}

/// Whether some path through an access body performs the access at `el`.
fn reachable(access: &Value, el: u8) -> bool {
    match access {
        Value::Array(chain) => {
            for node in chain {
                let c = tri(&node["condition"], el);
                if c == Tri::False {
                    continue;
                }
                if reachable(&node["access"], el) {
                    return true;
                }
                if c == Tri::True {
                    return false;
                }
            }
            false
        }
        Value::Object(_) if access["_type"] == "Accessors.Permission.SystemAccess" => {
            match tri(&access["condition"], el) {
                Tri::False => false,
                _ => reachable(&access["access"], el),
            }
        }
        Value::Null => false,
        _ => leaf_accesses(access),
    }
}

fn min_el(access: &Value) -> u8 {
    (0..=3).find(|&el| reachable(access, el)).unwrap_or(4)
}

fn feature_names(c: &Value, out: &mut Vec<String>) {
    match c {
        Value::Object(m) => {
            if m.get("name").and_then(|n| n.as_str()) == Some("IsFeatureImplemented") {
                for a in m["arguments"].as_array().into_iter().flatten() {
                    if let Some(s) = a["value"].as_str()
                        && !out.iter().any(|x| x == s)
                    {
                        out.push(s.to_string());
                    }
                }
            }
            for v in m.values() {
                feature_names(v, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|v| feature_names(v, out)),
        _ => {}
    }
}

/// Positive feature requirements of a register condition: `IsFeatureImplemented(X)`
/// terms joined by `&&` (a disjunction yields no single requirement).
fn required_features(c: &Value, out: &mut Vec<String>) {
    match c["_type"].as_str().unwrap_or("") {
        "AST.Function" if c["name"] == "IsFeatureImplemented" => feature_names(c, out),
        "AST.BinaryOp" if c["op"] == "&&" => {
            required_features(&c["left"], out);
            required_features(&c["right"], out);
        }
        _ => {}
    }
}

const FIELDS: [(&str, u32, u32); 5] = [
    ("op0", 14, 2),
    ("op1", 11, 3),
    ("CRn", 7, 4),
    ("CRm", 3, 4),
    ("op2", 0, 3),
];

struct Row {
    kind: &'static str,
    insn: String,
    name: String,
    value: u32,
    mask: u32,
    min_el: u8,
    features: Vec<String>,
}

fn kind_of(acc: &str) -> (&'static str, String) {
    let insn = acc.strip_prefix("A64.").unwrap_or(acc);
    let kind = match insn {
        "MRS" => "Mrs",
        "MSRregister" => "Msr",
        "MSRimmediate" => "MsrImm",
        "MRRS" => "Mrrs",
        "MSRRregister" => "Msrr",
        "SYSL" => "Sysl",
        "SYSP" | "TLBIP" => "Sysp",
        _ => "Sys",
    };
    let insn = match insn {
        "MSRregister" | "MSRimmediate" => "MSR",
        "MSRRregister" => "MSRR",
        s => s,
    };
    (kind, insn.to_string())
}

/// Field values of one encoding entry for index `idx` of an array (`None` for a
/// plain register).
fn encode(enc: &Value, var: Option<&str>, idx: u32) -> Result<(u32, u32)> {
    let (mut value, mut mask) = (0u32, 0u32);
    let map = enc["encodings"].as_object().context("encodings")?;
    for (name, lsb, width) in FIELDS {
        let Some(v) = map.get(name) else { continue };
        match v["_type"].as_str().unwrap_or("") {
            "Values.Value" => {
                let s = v["value"].as_str().unwrap_or("").trim_matches('\'');
                for (i, c) in s.chars().enumerate() {
                    let bit = 1u32 << (lsb + width - 1 - i as u32);
                    match c {
                        '0' => mask |= bit,
                        '1' => {
                            mask |= bit;
                            value |= bit;
                        }
                        _ => {}
                    }
                }
            }
            "Values.EquationValue" => {
                if var.is_none() || v["value"].as_str() != var {
                    // A value computed from another operand: any value matches.
                    continue;
                }
                // Concatenation of slices of the index, most significant first.
                let mut f = 0u32;
                for s in v["slice"].as_array().into_iter().flatten() {
                    let st = s["start"].as_u64().unwrap_or(0) as u32;
                    let w = s["width"].as_u64().unwrap_or(0) as u32;
                    f = f << w | (idx >> st) & ((1 << w) - 1);
                }
                let m = ((1u32 << width) - 1) << lsb;
                mask |= m;
                value |= (f << lsb) & m;
            }
            "Values.Group" => {
                // A concatenation such as `'1':m[1:0]`.
                let s = v["value"].as_str().unwrap_or("");
                let mut parts = Vec::new();
                let (mut depth, mut cur) = (0, String::new());
                for c in s.chars() {
                    match c {
                        '[' => depth += 1,
                        ']' => depth -= 1,
                        ':' if depth == 0 => {
                            parts.push(std::mem::take(&mut cur));
                            continue;
                        }
                        _ => {}
                    }
                    cur.push(c);
                }
                parts.push(cur);
                let (mut f, mut fw) = (0u32, 0u32);
                for p in &parts {
                    if let Some(bits) = p.strip_prefix('\'') {
                        for c in bits.trim_end_matches('\'').chars() {
                            f = f << 1 | u32::from(c == '1');
                            fw += 1;
                        }
                        continue;
                    }
                    let (vname, range) = match p.split_once('[') {
                        Some((n, r)) => (n, Some(r.trim_end_matches(']'))),
                        None => (p.as_str(), None),
                    };
                    if Some(vname) != var {
                        anyhow::bail!("group value {s} names {vname}");
                    }
                    let (hi, lo) = match range {
                        Some(r) => match r.split_once(':') {
                            Some((h, l)) => (h.parse::<u32>()?, l.parse::<u32>()?),
                            None => (r.parse::<u32>()?, r.parse::<u32>()?),
                        },
                        None => (width - 1, 0),
                    };
                    let w = hi - lo + 1;
                    f = f << w | (idx >> lo) & ((1 << w) - 1);
                    fw += w;
                }
                if fw != width {
                    anyhow::bail!("group value {s} is {fw} bits, field {name} is {width}");
                }
                let m = ((1u32 << width) - 1) << lsb;
                mask |= m;
                value |= (f << lsb) & m;
            }
            t => anyhow::bail!("{name}: unsupported encoding value {t}"),
        }
    }
    Ok((value, mask))
}

pub fn generate(regs: &Value, fvar: &BTreeMap<String, String>) -> Result<SysRegs> {
    let mut rows = Vec::new();
    for r in regs.as_array().context("Registers.json")? {
        if r["state"] != "AArch64" {
            continue;
        }
        let mut feats = Vec::new();
        required_features(&r["condition"], &mut feats);
        for acc in r["accessors"].as_array().into_iter().flatten() {
            let ty = acc["_type"].as_str().unwrap_or("");
            if ty != "Accessors.SystemAccessor" && ty != "Accessors.SystemAccessorArray" {
                continue;
            }
            let (kind, insn) = kind_of(acc["name"].as_str().unwrap_or(""));
            let mut el = min_el(&acc["access"]);
            let mut afeats = feats.clone();
            required_features(&acc["condition"], &mut afeats);
            afeats.retain(|f| fvar.contains_key(f));
            let var = acc["index_variable"].as_str();
            let indexes: Vec<u32> = match var {
                None => vec![0],
                Some(_) => {
                    let mut v = Vec::new();
                    for rg in acc["indexes"].as_array().into_iter().flatten() {
                        let st = rg["start"].as_u64().unwrap_or(0) as u32;
                        let w = rg["width"].as_u64().unwrap_or(0) as u32;
                        v.extend(st..st + w);
                    }
                    v
                }
            };
            for enc in acc["encoding"].as_array().into_iter().flatten() {
                let asm = enc["asmvalue"].as_str().unwrap_or("").to_string();
                if kind == "MsrImm" && acc["access"].is_null() {
                    el = pstate_min_el(&asm);
                }
                for &i in &indexes {
                    let (value, mask) = encode(enc, var, i)?;
                    let name = match var {
                        Some(v) => asm.replace(&format!("<{v}>"), &i.to_string()),
                        None => asm.clone(),
                    };
                    rows.push(Row {
                        kind,
                        insn: insn.clone(),
                        name,
                        value,
                        mask,
                        min_el: el,
                        features: afeats.clone(),
                    });
                }
            }
        }
    }
    // In `SysKind` order: the library binary-searches by kind.
    const ORDER: [&str; 8] = [
        "Mrs", "Msr", "MsrImm", "Mrrs", "Msrr", "Sys", "Sysl", "Sysp",
    ];
    let rank = |k: &str| ORDER.iter().position(|o| *o == k).unwrap_or(ORDER.len());
    rows.sort_by(|a, b| {
        (rank(a.kind), a.value, &a.name, a.mask).cmp(&(rank(b.kind), b.value, &b.name, b.mask))
    });
    rows.dedup_by(|a, b| {
        a.kind == b.kind && a.value == b.value && a.name == b.name && a.mask == b.mask
    });
    let mut code = String::new();
    let _ = writeln!(
        code,
        "use crate::generated::Feature;\nuse crate::sysreg::{{SysEntry, SysKind}};\n\n\
         /// Sorted by kind, then encoding (`op0:op1:CRn:CRm:op2`), then name.\n\
         pub(crate) static SYSREGS: [SysEntry; {}] = [",
        rows.len()
    );
    for r in &rows {
        let f: Vec<String> = r
            .features
            .iter()
            .map(|f| format!("Feature::{}", fvar[f]))
            .collect();
        let _ = writeln!(
            code,
            "    SysEntry {{ kind: SysKind::{}, insn: {:?}, name: {:?}, value: {:#06x}, mask: {:#06x}, min_el: {}, features: &[{}] }},",
            r.kind,
            r.insn,
            r.name,
            r.value,
            r.mask,
            r.min_el,
            f.join(", ")
        );
    }
    let _ = writeln!(code, "];");
    Ok(SysRegs {
        code,
        count: rows.len(),
    })
}

/// The `MSR (immediate)` accessors carry no access description in the open-source
/// release, so the lowest EL per PSTATE field is a hand-maintained table (Arm ARM
/// C5.2, "MSR (immediate)": the fields writable at EL0, some only under
/// configuration such as SCTLR_EL1.UMA for DAIF).
fn pstate_min_el(field: &str) -> u8 {
    match field {
        "SSBS" | "DIT" | "TCO" | "SVCRSM" | "SVCRZA" | "SVCRSMZA" | "DAIFSet" | "DAIFClr" => 0,
        _ => 1,
    }
}
