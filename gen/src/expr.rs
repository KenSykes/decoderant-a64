//! Compiles AARCHMRS condition ASTs (encoding conditions, alias conditions) into a
//! shared expression arena that the library evaluates against a word.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde_json::Value;

use crate::model::Field;

/// One compiled expression node. Children are arena indices.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum E {
    Bool(bool),
    /// An encoding field's bits.
    Field {
        lsb: u32,
        width: u32,
    },
    /// A bit-string constant, possibly with `x` bits (`mask` has the fixed ones).
    Bits {
        value: u32,
        mask: u32,
        width: u32,
    },
    Int(i64),
    /// `IsFeatureImplemented(FEAT_*)`.
    Feature(String),
    Not(u32),
    BitNot(u32),
    /// Binary operator by name (`==`, `!=`, `&&`, `||`, `<`, `>`, `>=`, `<=`, `+`,
    /// `-`, `MOD`, `XOR`, `::`).
    Bin(String, u32, u32),
    In(u32, Vec<u32>),
    /// `x<hi:lo>`.
    Slice(u32, u32, u32),
    UInt(u32),
    /// A helper function from the pseudocode (alias preferences), implemented by
    /// hand in the library.
    Call(String, Vec<u32>),
}

/// System instruction aliases named by `Sys_*` in alias conditions, by index
/// (emitted for the library as `generated::conds::SYSOP_NAMES`).
pub const SYSOP_NAMES: &[&str] = &[
    "SYS", "DC", "IC", "AT", "TLBI", "TLBIP", "BRB", "CFP", "CPP", "DVP", "COSP", "APAS", "GIC",
    "GICR", "GSB", "PLBI", "TRCIT", "GCSPUSHM", "GCSPOPM", "GCSPUSHX", "GCSPOPX", "GCSPOPCX",
    "GCSSS1", "GCSSS2",
];

fn is_sysop_test(v: &Value) -> bool {
    let op = v["op"].as_str().unwrap_or("");
    (op == "==" || op == "!=")
        && v["left"]["_type"] == "AST.Function"
        && matches!(
            v["left"]["name"].as_str(),
            Some("SysOp" | "SysLOp" | "SysOp128")
        )
        && v["right"]["value"]
            .as_str()
            .is_some_and(|s| s.starts_with("Sys_") || s.starts_with("Sysl_"))
}

/// Helper functions the library implements (`decoderant_a64::cond::Func`).
pub const KNOWN_FUNCS: &[&str] = &[
    "SysOpIs",
    "BFXPreferred",
    "MoveWidePreferred",
    "SVEMoveMaskPreferred",
    "DecodeBitMasksCheckUndefined",
    "IsZero",
    "IsOnes",
    "BitCount",
    "ZeroExtend",
];

#[derive(Default)]
pub struct Arena {
    pub nodes: Vec<E>,
    index: BTreeMap<E, u32>,
}

/// Field lookup for identifiers: the node's fields, then its ancestors'.
pub trait Scope {
    fn field(&self, name: &str) -> Option<Field>;
}

impl Arena {
    pub fn intern(&mut self, e: E) -> u32 {
        if let Some(&i) = self.index.get(&e) {
            return i;
        }
        let i = self.nodes.len() as u32;
        self.nodes.push(e.clone());
        self.index.insert(e, i);
        i
    }

    pub fn compile(&mut self, v: &Value, scope: &dyn Scope) -> Result<u32> {
        let ty = v["_type"].as_str().unwrap_or("");
        let e = match ty {
            "AST.Bool" => E::Bool(v["value"].as_bool().unwrap_or(false)),
            "AST.Integer" => E::Int(v["value"].as_i64().unwrap_or(0)),
            "Values.Value" => {
                let s = v["value"].as_str().unwrap_or("").trim_matches('\'');
                let width = s.len() as u32;
                let (mut value, mut mask) = (0u32, 0u32);
                for (i, c) in s.chars().enumerate() {
                    let bit = 1u32 << (width - 1 - i as u32);
                    match c {
                        '0' => mask |= bit,
                        '1' => {
                            mask |= bit;
                            value |= bit;
                        }
                        'x' => {}
                        _ => bail!("bad bit string {s:?}"),
                    }
                }
                E::Bits { value, mask, width }
            }
            "AST.Identifier" => {
                let name = v["value"].as_str().unwrap_or("");
                match scope.field(name) {
                    Some(f) => E::Field {
                        lsb: f.lsb,
                        width: f.width,
                    },
                    None => bail!("unknown identifier {name}"),
                }
            }
            "AST.UnaryOp" => {
                let a = self.compile(&v["expr"], scope)?;
                match v["op"].as_str().unwrap_or("") {
                    "!" => E::Not(a),
                    "NOT" => E::BitNot(a),
                    op => bail!("unary operator {op}"),
                }
            }
            "AST.BinaryOp" if is_sysop_test(v) => {
                // `SysOp(op1, CRn, CRm, op2) == Sys_DC`: which system instruction alias
                // the operands name.
                let f = &v["left"];
                let kind = match f["name"].as_str().unwrap_or("") {
                    "SysOp" => 0,
                    "SysLOp" => 1,
                    _ => 2,
                };
                // `Sys_DC`, or `Sysl_GICR` (the one `SysLOp` test spells it so).
                let name = v["right"]["value"].as_str().unwrap_or("");
                let name = name
                    .strip_prefix("Sysl_")
                    .or_else(|| name.strip_prefix("Sys_"))
                    .unwrap_or(name);
                let Some(k) = SYSOP_NAMES.iter().position(|n| *n == name) else {
                    bail!("unknown system operation Sys_{name}");
                };
                let mut ids = Vec::new();
                for a in f["arguments"].as_array().into_iter().flatten() {
                    ids.push(self.compile(a, scope)?);
                }
                ids.push(self.intern(E::Int(kind)));
                ids.push(self.intern(E::Int(k as i64)));
                let call = self.intern(E::Call("SysOpIs".into(), ids));
                if v["op"] == "!=" {
                    E::Not(call)
                } else {
                    return Ok(call);
                }
            }
            "AST.BinaryOp" => {
                let op = v["op"].as_str().unwrap_or("").to_string();
                let a = self.compile(&v["left"], scope)?;
                if op == "IN" {
                    let set = &v["right"];
                    if set["_type"] != "AST.Set" {
                        bail!("IN needs a set");
                    }
                    let mut members = Vec::new();
                    for m in set["values"].as_array().into_iter().flatten() {
                        members.push(self.compile(m, scope)?);
                    }
                    E::In(a, members)
                } else {
                    let b = self.compile(&v["right"], scope)?;
                    match op.as_str() {
                        "==" | "!=" | "&&" | "||" | "<" | ">" | ">=" | "<=" | "+" | "-" | "MOD"
                        | "XOR" | "::" => E::Bin(op, a, b),
                        _ => bail!("binary operator {op}"),
                    }
                }
            }
            "AST.SquareOp" => {
                let var = self.compile(&v["var"], scope)?;
                let args = v["arguments"].as_array().cloned().unwrap_or_default();
                let [arg] = args.as_slice() else {
                    bail!("multi-index slice");
                };
                match arg["_type"].as_str().unwrap_or("") {
                    "AST.Integer" => {
                        let b = arg["value"].as_u64().unwrap_or(0) as u32;
                        E::Slice(var, b, b)
                    }
                    "AST.Slice" => {
                        let hi = arg["left"]["value"].as_u64().unwrap_or(0) as u32;
                        let lo = arg["right"]["value"].as_u64().unwrap_or(0) as u32;
                        E::Slice(var, hi, lo)
                    }
                    t => bail!("slice index {t}"),
                }
            }
            "AST.Function" => {
                let name = v["name"].as_str().unwrap_or("");
                let args = v["arguments"].as_array().cloned().unwrap_or_default();
                match name {
                    "IsFeatureImplemented" => {
                        let [a] = args.as_slice() else {
                            bail!("IsFeatureImplemented arity");
                        };
                        E::Feature(a["value"].as_str().unwrap_or("").to_string())
                    }
                    "UInt" => {
                        let [a] = args.as_slice() else {
                            bail!("UInt arity");
                        };
                        E::UInt(self.compile(a, scope)?)
                    }
                    _ if KNOWN_FUNCS.contains(&name) => {
                        let mut ids = Vec::new();
                        for a in &args {
                            ids.push(self.compile(a, scope)?);
                        }
                        E::Call(name.to_string(), ids)
                    }
                    _ => bail!("unknown function {name}"),
                }
            }
            _ => bail!("unsupported AST node {ty}"),
        };
        Ok(self.intern(e))
    }

    /// Every feature name the expression names.
    pub fn features(&self, id: u32, out: &mut Vec<String>) {
        match &self.nodes[id as usize] {
            E::Feature(f) => {
                if !out.contains(f) {
                    out.push(f.clone());
                }
            }
            E::Not(a) | E::BitNot(a) | E::UInt(a) | E::Slice(a, _, _) => self.features(*a, out),
            E::Bin(_, a, b) => {
                self.features(*a, out);
                self.features(*b, out);
            }
            E::In(a, m) => {
                self.features(*a, out);
                for x in m {
                    self.features(*x, out);
                }
            }
            E::Call(_, args) => {
                for x in args {
                    self.features(*x, out);
                }
            }
            _ => {}
        }
    }

    /// Whether the expression mentions only features (and constants): such a
    /// condition says which features an encoding needs, not which field values.
    pub fn is_feature_only(&self, id: u32) -> bool {
        match &self.nodes[id as usize] {
            E::Feature(_) | E::Bool(_) => true,
            E::Not(a) => self.is_feature_only(*a),
            E::Bin(op, a, b) if op == "&&" || op == "||" => {
                self.is_feature_only(*a) && self.is_feature_only(*b)
            }
            _ => false,
        }
    }
}

impl Arena {
    /// The field values a condition pins: the `field == 'bits'` terms of its
    /// top-level conjunction, as (mask, value) over the word.
    pub fn fixed_bits(&self, id: u32) -> (u32, u32) {
        match &self.nodes[id as usize] {
            E::Bin(op, a, b) if op == "&&" => {
                let (m1, v1) = self.fixed_bits(*a);
                let (m2, v2) = self.fixed_bits(*b);
                (m1 | m2, v1 | v2)
            }
            E::Bin(op, a, b) if op == "==" => {
                match (&self.nodes[*a as usize], &self.nodes[*b as usize]) {
                    (
                        E::Field { lsb, width },
                        E::Bits {
                            value,
                            mask,
                            width: w2,
                        },
                    ) if width == w2 => (mask << lsb, value << lsb),
                    _ => (0, 0),
                }
            }
            _ => (0, 0),
        }
    }
}

/// Two fields as (lsb, width) each.
pub type FieldPair = ((u32, u32), (u32, u32));

impl Arena {
    /// The `field == field` terms of a condition's top-level conjunction (`Rm == Rn`
    /// in `ROR`): the assembler makes the fields equal.
    pub fn copies(&self, id: u32, out: &mut Vec<FieldPair>) {
        match &self.nodes[id as usize] {
            E::Bin(op, a, b) if op == "&&" => {
                self.copies(*a, out);
                self.copies(*b, out);
            }
            E::Bin(op, a, b) if op == "==" => {
                if let (E::Field { lsb: l1, width: w1 }, E::Field { lsb: l2, width: w2 }) =
                    (&self.nodes[*a as usize], &self.nodes[*b as usize])
                    && w1 == w2
                {
                    out.push(((*l1, *w1), (*l2, *w2)));
                }
            }
            _ => {}
        }
    }
}

/// A value while evaluating an expression in the generator (as `cond::Val` does in
/// the library).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum V {
    Bool(bool),
    Bits { value: u64, mask: u64, width: u32 },
    Int(i64),
}

impl V {
    fn bits(value: u64, width: u32) -> V {
        let mask = if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        V::Bits {
            value: value & mask,
            mask,
            width,
        }
    }

    fn truthy(self) -> bool {
        match self {
            V::Bool(b) => b,
            V::Bits { value, .. } => value != 0,
            V::Int(i) => i != 0,
        }
    }

    fn int(self) -> i64 {
        match self {
            V::Bool(b) => b as i64,
            V::Bits { value, .. } => value as i64,
            V::Int(i) => i,
        }
    }

    fn raw(self) -> (u64, u32) {
        match self {
            V::Bits { value, width, .. } => (value, width),
            V::Bool(b) => (b as u64, 1),
            V::Int(i) => (i as u64, 64),
        }
    }
}

fn equal(a: V, b: V) -> bool {
    match (a, b) {
        (
            V::Bits {
                value: av,
                mask: am,
                ..
            },
            V::Bits {
                value: bv,
                mask: bm,
                ..
            },
        ) => {
            let m = am & bm;
            av & m == bv & m
        }
        (V::Bool(x), V::Bool(y)) => x == y,
        _ => a.int() == b.int(),
    }
}

impl Arena {
    /// Evaluates an expression on a word, with every feature implemented. Helper
    /// functions (alias preferences) count as true: this is for path conditions,
    /// which name fields and features only.
    fn eval(&self, id: u32, word: u32) -> V {
        match &self.nodes[id as usize] {
            E::Bool(b) => V::Bool(*b),
            E::Field { lsb, width } => V::bits(u64::from(word >> lsb), *width),
            E::Bits { value, mask, width } => V::Bits {
                value: u64::from(*value),
                mask: u64::from(*mask),
                width: *width,
            },
            E::Int(i) => V::Int(*i),
            E::Feature(_) | E::Call(..) => V::Bool(true),
            E::Not(a) => V::Bool(!self.eval(*a, word).truthy()),
            E::BitNot(a) => {
                let (v, w) = self.eval(*a, word).raw();
                V::bits(!v, w)
            }
            E::Bin(op, a, b) => {
                let x = self.eval(*a, word);
                match op.as_str() {
                    "&&" => return V::Bool(x.truthy() && self.eval(*b, word).truthy()),
                    "||" => return V::Bool(x.truthy() || self.eval(*b, word).truthy()),
                    _ => {}
                }
                let y = self.eval(*b, word);
                match op.as_str() {
                    "==" => V::Bool(equal(x, y)),
                    "!=" => V::Bool(!equal(x, y)),
                    "<" => V::Bool(x.int() < y.int()),
                    ">" => V::Bool(x.int() > y.int()),
                    ">=" => V::Bool(x.int() >= y.int()),
                    "<=" => V::Bool(x.int() <= y.int()),
                    "+" => V::Int(x.int().wrapping_add(y.int())),
                    "-" => V::Int(x.int().wrapping_sub(y.int())),
                    "MOD" => V::Int(x.int().checked_rem_euclid(y.int()).unwrap_or(0)),
                    "XOR" => {
                        let ((a, w), (b, _)) = (x.raw(), y.raw());
                        V::bits(a ^ b, w)
                    }
                    _ => {
                        // `::`.
                        let ((a, aw), (b, bw)) = (x.raw(), y.raw());
                        V::bits(a << bw | b, aw + bw)
                    }
                }
            }
            E::In(a, m) => {
                let x = self.eval(*a, word);
                V::Bool(m.iter().any(|&m| equal(x, self.eval(m, word))))
            }
            E::Slice(a, hi, lo) => {
                let (v, _) = self.eval(*a, word).raw();
                V::bits(v >> lo, hi - lo + 1)
            }
            E::UInt(a) => V::Int(self.eval(*a, word).raw().0 as i64),
        }
    }

    /// Whether every condition holds for the word.
    pub fn holds(&self, conds: &[u32], word: u32) -> bool {
        conds.iter().all(|&c| self.eval(c, word).truthy())
    }
}
