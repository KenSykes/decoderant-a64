//! The A64 decode hierarchy from `Instructions.json`: groups, encodings and aliases.

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// A named encoding field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub lsb: u32,
    pub width: u32,
}

/// Fixed bits and should-be bits of an encodeset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pattern {
    /// Bits that must have `value` for the node to match.
    pub mask: u32,
    pub value: u32,
    /// Should-be bits (`(0)`/`(1)` in the spec): matched loosely, stated value in
    /// `sb_value`.
    pub sb_mask: u32,
    pub sb_value: u32,
}

impl Pattern {
    pub fn intersects(&self, other: &Pattern) -> bool {
        let common = self.mask & other.mask;
        self.value & common == other.value & common
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Group,
    /// An encoding; the index into `Model::leaves`.
    Leaf(usize),
}

pub struct Node<'a> {
    pub name: String,
    pub kind: Kind,
    pub pattern: Pattern,
    pub fields: Vec<Field>,
    /// `None` when the condition is the constant `true`.
    pub cond: Option<&'a Value>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

pub struct Alias<'a> {
    pub json: &'a Value,
}

pub struct Leaf<'a> {
    pub name: String,
    pub node: usize,
    /// Name of the innermost group: the A64 iclass.
    pub iclass: String,
    pub json: &'a Value,
    pub aliases: Vec<Alias<'a>>,
}

pub struct Model<'a> {
    pub nodes: Vec<Node<'a>>,
    pub leaves: Vec<Leaf<'a>>,
    /// Index of the A64 instruction-set root in `nodes`.
    pub root: usize,
    pub rules: &'a Value,
}

fn bits_str(v: &Value) -> Result<&str> {
    let s = v["value"]
        .as_str()
        .with_context(|| format!("expected a Values.Value, got {v}"))?;
    Ok(s.trim_matches('\''))
}

/// Parses an `Instruction.Encodeset.Encodeset` into its pattern and named fields.
pub fn parse_encodeset(enc: &Value) -> Result<(Pattern, Vec<Field>)> {
    let mut p = Pattern::default();
    let mut fields = Vec::new();
    let Some(values) = enc["values"].as_array() else {
        return Ok((p, fields));
    };
    for f in values {
        let lsb = f["range"]["start"].as_u64().context("range.start")? as u32;
        let width = f["range"]["width"].as_u64().context("range.width")? as u32;
        let val = bits_str(&f["value"])?;
        let sb = match f.get("should_be_mask") {
            Some(m) if !m.is_null() => bits_str(m)?,
            _ => "",
        };
        if val.len() != width as usize || (!sb.is_empty() && sb.len() != width as usize) {
            bail!("field width mismatch in {f}");
        }
        for (i, c) in val.chars().enumerate() {
            let bit = 1u32 << (lsb + width - 1 - i as u32);
            let should_be = sb.as_bytes().get(i) == Some(&b'1');
            match (c, should_be) {
                ('x', _) => {}
                ('0', false) => p.mask |= bit,
                ('1', false) => {
                    p.mask |= bit;
                    p.value |= bit;
                }
                ('0', true) => p.sb_mask |= bit,
                ('1', true) => {
                    p.sb_mask |= bit;
                    p.sb_value |= bit;
                }
                _ => bail!("unexpected bit {c:?} in {val:?}"),
            }
        }
        if f["_type"] == "Instruction.Encodeset.Field" {
            let name = f["name"].as_str().context("field name")?.to_string();
            fields.push(Field { name, lsb, width });
        }
    }
    Ok((p, fields))
}

fn is_true(cond: &Value) -> bool {
    cond.is_null() || (cond["_type"] == "AST.Bool" && cond["value"] == Value::Bool(true))
}

impl<'a> Model<'a> {
    pub fn build(instructions: &'a Value) -> Result<Self> {
        let sets = instructions["instructions"]
            .as_array()
            .context("instructions[]")?;
        let a64 = sets
            .iter()
            .find(|s| s["name"] == "A64")
            .context("no A64 instruction set")?;
        let mut m = Model {
            nodes: Vec::new(),
            leaves: Vec::new(),
            root: 0,
            rules: &instructions["assembly_rules"],
        };
        m.root = m.add(a64, None, "")?;
        Ok(m)
    }

    fn add(&mut self, v: &'a Value, parent: Option<usize>, iclass: &str) -> Result<usize> {
        let name = v["name"].as_str().context("node name")?.to_string();
        let (pattern, fields) = parse_encodeset(&v["encoding"])?;
        let cond = (!is_true(&v["condition"])).then_some(&v["condition"]);
        let id = self.nodes.len();
        let ty = v["_type"].as_str().unwrap_or("");
        let kind = match ty {
            "Instruction.InstructionSet" | "Instruction.InstructionGroup" => Kind::Group,
            "Instruction.Instruction" => Kind::Leaf(self.leaves.len()),
            _ => bail!("unexpected node type {ty} for {name}"),
        };
        self.nodes.push(Node {
            name: name.clone(),
            kind,
            pattern,
            fields,
            cond,
            children: Vec::new(),
            parent,
        });
        match kind {
            Kind::Group => {
                for c in v["children"].as_array().into_iter().flatten() {
                    let child = self.add(c, Some(id), &name)?;
                    self.nodes[id].children.push(child);
                }
            }
            Kind::Leaf(_) => {
                let mut aliases = Vec::new();
                for c in v["children"].as_array().into_iter().flatten() {
                    if c["_type"] != "Instruction.InstructionAlias" {
                        bail!("{name}: an encoding has a non-alias child");
                    }
                    aliases.push(Alias { json: c });
                }
                self.leaves.push(Leaf {
                    name,
                    node: id,
                    iclass: iclass.to_string(),
                    json: v,
                    aliases,
                });
            }
        }
        Ok(id)
    }

    /// The node and its ancestors, innermost first.
    pub fn path(&self, mut node: usize) -> Vec<usize> {
        let mut out = vec![node];
        while let Some(p) = self.nodes[node].parent {
            out.push(p);
            node = p;
        }
        out
    }

    /// Looks a field name up in the node, then its ancestors.
    pub fn field(&self, node: usize, name: &str) -> Option<Field> {
        self.path(node).into_iter().find_map(|n| {
            self.nodes[n]
                .fields
                .iter()
                .find(|f| f.name == name)
                .cloned()
        })
    }

    /// The full pattern of a node: its own bits merged with every ancestor's.
    pub fn full_pattern(&self, node: usize) -> Pattern {
        let mut p = Pattern::default();
        for n in self.path(node) {
            let q = self.nodes[n].pattern;
            p.mask |= q.mask;
            p.value |= q.value;
            p.sb_mask |= q.sb_mask & !p.mask;
            p.sb_value |= q.sb_value & !p.mask;
        }
        p.sb_mask &= !p.mask;
        p.sb_value &= p.sb_mask;
        p
    }
}
