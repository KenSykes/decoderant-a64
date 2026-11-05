//! Flattens the AARCHMRS assembly grammar (rules, choices, tokens) into a template per
//! encoding: literal text, operand symbols and optional parts.

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// One alternative of a symbol: the rule that produces it and its text, with
/// `{N}` standing for a number token (register number, immediate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymAlt {
    pub rule: String,
    pub text: String,
    /// A number in the text is signed (the `SInteger` token, not `UInteger`).
    pub signed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum T {
    Text(String),
    /// An operand symbol such as `<Xd>` or `<T>`, with the rule id that produced it.
    Sym {
        display: String,
        rule: String,
        alts: Vec<SymAlt>,
    },
    /// Present or absent as a whole (e.g. `{, <shift> #<amount>}`).
    Opt(Vec<T>),
    /// One of several forms (e.g. `<Wm>` or `<Xm>`).
    Alt(Vec<Vec<T>>),
}

pub struct Flattener<'a> {
    pub rules: &'a Value,
    pub stack: std::cell::RefCell<Vec<String>>,
    /// Set when [`Flattener::text`] meets an `SInteger` token.
    pub signed: std::cell::Cell<bool>,
}

fn is_number_token(id: &str) -> bool {
    !matches!(id, "SPACE" | "COMMA" | "OPT_SPACE")
}

impl Flattener<'_> {
    pub fn flatten(&self, assembly: &Value) -> Result<Vec<T>> {
        let mut out = Vec::new();
        for s in assembly["symbols"].as_array().into_iter().flatten() {
            self.symbol(s, &mut out)?;
        }
        Ok(merge_text(out))
    }

    fn symbol(&self, s: &Value, out: &mut Vec<T>) -> Result<()> {
        match s["_type"].as_str().unwrap_or("") {
            "Instruction.Symbols.Literal" => {
                out.push(T::Text(s["value"].as_str().unwrap_or("").to_string()));
            }
            "Instruction.Symbols.RuleReference" => {
                let id = s["rule_id"].as_str().context("rule_id")?;
                self.rule(id, out)?;
            }
            t => bail!("unexpected assembly symbol {t}"),
        }
        Ok(())
    }

    fn rule(&self, id: &str, out: &mut Vec<T>) -> Result<()> {
        if self.stack.borrow().iter().any(|s| s == id) {
            bail!("recursive rule {id} via {:?}", self.stack.borrow());
        }
        self.stack.borrow_mut().push(id.to_string());
        let r = self.rule_inner(id, out);
        self.stack.borrow_mut().pop();
        r
    }

    fn rule_inner(&self, id: &str, out: &mut Vec<T>) -> Result<()> {
        let r = &self.rules[id];
        let ty = r["_type"].as_str().unwrap_or("");
        if ty == "Instruction.Rules.Token" {
            match id {
                "SPACE" => out.push(T::Text(" ".into())),
                "COMMA" => out.push(T::Text(", ".into())),
                "OPT_SPACE" => {}
                _ => out.push(T::Text("{N}".into())),
            }
            return Ok(());
        }
        let display = r["display"].as_str().unwrap_or("");
        match display {
            "" => {}
            "#" => {
                out.push(T::Text("#".into()));
                return Ok(());
            }
            " " => {
                out.push(T::Text(" ".into()));
                return Ok(());
            }
            _ => {
                let alts = self.alternatives(id)?;
                out.push(T::Sym {
                    display: display.to_string(),
                    rule: id.to_string(),
                    alts,
                });
                return Ok(());
            }
        }
        match ty {
            "Instruction.Rules.Rule" => {
                for s in r["symbols"]["symbols"].as_array().into_iter().flatten() {
                    self.symbol(s, out)?;
                }
            }
            "Instruction.Rules.Choice" => {
                let mut alts = Vec::new();
                for c in r["choices"].as_array().into_iter().flatten() {
                    let mut a = Vec::new();
                    for s in c["symbols"].as_array().into_iter().flatten() {
                        self.symbol(s, &mut a)?;
                    }
                    alts.push(merge_text(a));
                }
                let non_empty: Vec<Vec<T>> =
                    alts.iter().filter(|a| !a.is_empty()).cloned().collect();
                if non_empty.is_empty() {
                } else if non_empty.len() < alts.len() {
                    if non_empty.len() == 1 {
                        out.push(T::Opt(non_empty.into_iter().next().unwrap()));
                    } else {
                        out.push(T::Opt(vec![T::Alt(non_empty)]));
                    }
                } else if non_empty.iter().all(|a| *a == non_empty[0]) {
                    out.extend(non_empty.into_iter().next().unwrap());
                } else {
                    out.push(T::Alt(non_empty));
                }
            }
            _ => bail!("rule {id} has type {ty}"),
        }
        Ok(())
    }

    /// The alternatives of a symbol rule, each as plain text with `{N}` for numbers.
    fn alternatives(&self, id: &str) -> Result<Vec<SymAlt>> {
        let r = &self.rules[id];
        let mut out = Vec::new();
        match r["_type"].as_str().unwrap_or("") {
            "Instruction.Rules.Choice" => {
                for c in r["choices"].as_array().into_iter().flatten() {
                    let syms = c["symbols"].as_array().cloned().unwrap_or_default();
                    let rule = match syms.as_slice() {
                        [one] if one["_type"] == "Instruction.Symbols.RuleReference" => {
                            one["rule_id"].as_str().unwrap_or("").to_string()
                        }
                        _ => String::new(),
                    };
                    let mut text = String::new();
                    self.signed.set(false);
                    for s in &syms {
                        self.text(s, &mut text)?;
                    }
                    out.push(SymAlt {
                        rule,
                        text,
                        signed: self.signed.get(),
                    });
                }
            }
            _ => {
                let mut text = String::new();
                self.signed.set(false);
                for s in r["symbols"]["symbols"].as_array().into_iter().flatten() {
                    self.text(s, &mut text)?;
                }
                out.push(SymAlt {
                    rule: id.to_string(),
                    text,
                    signed: self.signed.get(),
                });
            }
        }
        Ok(out)
    }

    /// The text a symbol produces, with `{N}` for number tokens and `|` between the
    /// alternatives of nested choices.
    fn text(&self, s: &Value, out: &mut String) -> Result<()> {
        match s["_type"].as_str().unwrap_or("") {
            "Instruction.Symbols.Literal" => out.push_str(s["value"].as_str().unwrap_or("")),
            "Instruction.Symbols.RuleReference" => {
                let id = s["rule_id"].as_str().unwrap_or("");
                let r = &self.rules[id];
                if self.stack.borrow().iter().any(|s| s == id) {
                    // A recursive rule (a repeated list element): show it once.
                    out.push_str("...");
                    return Ok(());
                }
                self.stack.borrow_mut().push(id.to_string());
                let res = self.text_rule(id, r, out);
                self.stack.borrow_mut().pop();
                res?;
            }
            t => bail!("unexpected symbol {t}"),
        }
        Ok(())
    }

    fn text_rule(&self, id: &str, r: &Value, out: &mut String) -> Result<()> {
        match r["_type"].as_str().unwrap_or("") {
            "Instruction.Rules.Token" => {
                if is_number_token(id) {
                    if id == "SInteger" {
                        self.signed.set(true);
                    }
                    out.push_str("{N}");
                } else if id == "COMMA" {
                    out.push_str(", ");
                } else if id == "SPACE" {
                    out.push(' ');
                }
            }
            "Instruction.Rules.Rule" => {
                for x in r["symbols"]["symbols"].as_array().into_iter().flatten() {
                    self.text(x, out)?;
                }
            }
            "Instruction.Rules.Choice" => {
                let mut parts = Vec::new();
                for c in r["choices"].as_array().into_iter().flatten() {
                    let mut p = String::new();
                    for x in c["symbols"].as_array().into_iter().flatten() {
                        self.text(x, &mut p)?;
                    }
                    parts.push(p);
                }
                if parts.len() == 2 && parts[1].is_empty() {
                    out.push_str(&parts[0]);
                } else {
                    out.push('(');
                    out.push_str(&parts.join("|"));
                    out.push(')');
                }
            }
            t => bail!("rule {id} has type {t}"),
        }
        Ok(())
    }
}

fn merge_text(v: Vec<T>) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for t in v {
        match (out.last_mut(), t) {
            (Some(T::Text(a)), T::Text(b)) => a.push_str(&b),
            (_, t) => out.push(t),
        }
    }
    out
}

/// Renders a template in the Arm ARM style, for reports and `OpCodeInfo`.
pub fn render(t: &[T]) -> String {
    let mut s = String::new();
    for x in t {
        match x {
            T::Text(a) => s.push_str(a),
            T::Sym { display, .. } => s.push_str(display),
            T::Opt(v) => {
                s.push('{');
                s.push_str(&render(v));
                s.push('}');
            }
            T::Alt(v) => {
                s.push('(');
                s.push_str(&v.iter().map(|a| render(a)).collect::<Vec<_>>().join("|"));
                s.push(')');
            }
        }
    }
    s
}
