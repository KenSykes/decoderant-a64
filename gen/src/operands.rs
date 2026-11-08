//! The hand-maintained operand mapping: assembly symbols to operand descriptors
//! (`decoderant_a64::desc::Op`).
//!
//! The spec gives each encoding's assembler template and, for every symbol, the
//! fields it is encoded in. Value tables (arrangements, shifts, extends, register
//! sizes, small number sets) come from the spec too: the rule names of a symbol's
//! alternatives carry the field values (`T_01_1_8H`: size 01, Q 1). What the spec
//! does not say in machine-readable form is the meaning of the other symbols:
//! register 31 as SP or ZR, offset scaling, PC-relative targets, bitmask and FP
//! immediates, shift amounts from `immh:immb`, element indices. That is this file:
//! operand shapes (`[<Xn|SP>{, #<pimm>}]`) and symbol rules, reviewed like the
//! constraints table. Anything it does not recognise is reported and left without
//! operand support, never guessed.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::Value;

use crate::expr::Arena;
use crate::model::{Field, Leaf, Model};
use crate::template::{SymAlt, T};

/// Tables shared between encodings, emitted as `static TAB_<n>: Tab`.
#[derive(Default)]
pub struct Tabs {
    pub bodies: Vec<String>,
    index: BTreeMap<String, usize>,
}

impl Tabs {
    fn add(&mut self, body: String) -> String {
        let n = self.bodies.len();
        let i = *self.index.entry(body.clone()).or_insert_with(|| {
            self.bodies.push(body);
            n
        });
        format!("&TAB_{i}")
    }
}

#[derive(Default)]
pub struct Report {
    pub mapped: usize,
    pub unmapped: usize,
    /// Failure reason → (count, example encodings).
    pub failures: BTreeMap<String, (usize, Vec<String>)>,
    /// Symbols whose value table came from the spec's rule names.
    pub data_tables: usize,
    /// Symbols interpreted by a hand rule.
    pub hand_rules: BTreeMap<String, usize>,
    pub tabs: Tabs,
    /// Alias templates without operand support, by reason.
    pub alias_failures: BTreeMap<String, (usize, Vec<String>)>,
    /// Of the last successful mapping: the operand count and the mnemonic suffix
    /// names (empty when there is no suffix).
    pub last_arity: usize,
    pub last_suffixes: Vec<String>,
    /// The last mapping has a `.<cond>` mnemonic suffix (`B.cond`).
    pub last_cond: bool,
}

impl Report {
    pub fn render(&self) -> String {
        let mut r = String::new();
        let _ = writeln!(r, "## Operand mapping\n");
        let _ = writeln!(
            r,
            "{} encodings with operand support, {} without. {} symbol value tables taken \
             from the spec's rule names; {} distinct hand rules (symbol meanings the spec \
             does not give in machine-readable form), used {} times.\n",
            self.mapped,
            self.unmapped,
            self.data_tables,
            self.hand_rules.len(),
            self.hand_rules.values().sum::<usize>()
        );
        let _ = writeln!(r, "### Hand rules\n\n| Rule | Uses |\n|---|---|");
        for (k, n) in &self.hand_rules {
            let _ = writeln!(r, "| `{k}` | {n} |");
        }
        let _ = writeln!(r, "\n### Encodings without operand support, by reason\n");
        let mut v: Vec<_> = self.failures.iter().collect();
        v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
        for (k, (n, ex)) in v {
            let _ = writeln!(r, "- {n} × {k} (e.g. {})", ex.join(", "));
        }
        let _ = writeln!(
            r,
            "
### Aliases without operand support, by reason
"
        );
        let mut v: Vec<_> = self.alias_failures.iter().collect();
        v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
        for (k, (n, ex)) in v {
            let _ = writeln!(r, "- {n} × {k} (e.g. {})", ex.join(", "));
        }
        r
    }
}

/// What the mapping may evaluate about the encoding: its path conditions.
pub struct Env<'a> {
    pub arena: &'a Arena,
    /// The conditions on the encoding's decode path.
    pub conds: Vec<u32>,
}

pub(crate) struct Sym {
    pub(crate) fields: Vec<Field>,
    pub(crate) alts: Vec<SymAlt>,
}

pub(crate) struct Cx<'r> {
    name: String,
    /// The encoding's named fields.
    leaf_fields: Vec<Field>,
    /// Mapping an alias template rather than the encoding's own.
    alias: bool,
    /// Fixed bits of the encoding.
    pat_mask: u32,
    pat_value: u32,
    env: &'r Env<'r>,
    /// Symbols in template order.
    pub(crate) order: Vec<String>,
    iclass: String,
    /// The top-level group (`sve`, `sme`, `ldst`, ...).
    group: String,
    pub(crate) mnemonic: String,
    syms: BTreeMap<String, Sym>,
    rep: &'r mut Report,
}

pub(crate) type R<T> = Result<T, String>;

fn f1(f: &Field) -> String {
    format!("F::new({}, {})", f.lsb, f.width)
}

/// Adjacent fields (`a:b:c`, `o0:op1:CRn:CRm:op2`) merged into runs.
fn runs(fields: &[Field]) -> Vec<Field> {
    let mut out: Vec<Field> = Vec::new();
    for f in fields {
        match out.last_mut() {
            Some(l) if l.lsb == f.lsb + f.width => {
                l.lsb = f.lsb;
                l.width += f.width;
            }
            _ => out.push(f.clone()),
        }
    }
    out
}

pub(crate) fn fs(fields: &[Field]) -> String {
    let mut parts: Vec<String> = runs(fields).iter().map(f1).collect();
    while parts.len() < 4 {
        parts.push("F::NONE".into());
    }
    if parts.len() > 4 {
        return "Fs::NONE".into();
    }
    format!("Fs([{}])", parts.join(", "))
}

const ARRS: [&str; 18] = [
    "", "8B", "16B", "4H", "8H", "2S", "4S", "1D", "2D", "1Q", "2H", "4B", "2B", "B", "H", "S",
    "D", "Q",
];
/// Extend names by table code (`ExtendKind as u8`).
pub const EXTENDS: [&str; 9] = [
    "UXTB", "UXTH", "UXTW", "UXTX", "SXTB", "SXTH", "SXTW", "SXTX", "LSL",
];
const RKS: [&str; 13] = [
    "X", "W", "XSp", "WSp", "B", "H", "S", "D", "Q", "V", "Z", "P", "PN",
];

fn arr_variant(s: &str) -> Option<&'static str> {
    Some(match s {
        "8B" => "B8",
        "16B" => "B16",
        "4H" => "H4",
        "8H" => "H8",
        "2S" => "S2",
        "4S" => "S4",
        "1D" => "D1",
        "2D" => "D2",
        "1Q" => "Q1",
        "2H" => "H2",
        "4B" => "B4",
        "2B" => "B2",
        "B" => "B",
        "H" => "H",
        "S" => "S",
        "D" => "D",
        "Q" => "Q",
        _ => return None,
    })
}

fn rk_of_letter(s: &str) -> Option<u8> {
    Some(match s {
        "W" => 1,
        "X" => 0,
        "B" => 4,
        "H" => 5,
        "S" => 6,
        "D" => 7,
        "Q" => 8,
        _ => return None,
    })
}

/// Field values from a rule id: the `_`-separated bit-string parts matching the
/// fields' widths, in order. `None` when the id carries no values.
fn rule_bits(rule: &str, widths: &[u32]) -> Option<(u32, u32)> {
    let parts: Vec<&str> = rule.split('_').collect();
    let total: u32 = widths.iter().sum();
    for start in 0..parts.len() {
        if start + widths.len() > parts.len() {
            break;
        }
        let ok = widths.iter().enumerate().all(|(i, &w)| {
            let p = parts[start + i];
            p.len() == w as usize && p.chars().all(|c| matches!(c, '0' | '1' | 'x'))
        });
        if ok {
            let s: String = parts[start..start + widths.len()].concat();
            let (mut v, mut m) = (0u32, 0u32);
            for (i, c) in s.chars().enumerate() {
                let bit = 1u32 << (total - 1 - i as u32);
                match c {
                    '0' => m |= bit,
                    '1' => {
                        m |= bit;
                        v |= bit;
                    }
                    _ => {}
                }
            }
            return Some((v, m));
        }
    }
    None
}

impl Cx<'_> {
    pub(crate) fn hand(&mut self, rule: &str) {
        *self.rep.hand_rules.entry(rule.to_string()).or_default() += 1;
    }

    pub(crate) fn sym(&self, d: &str) -> R<&Sym> {
        self.syms
            .get(d)
            .ok_or_else(|| format!("symbol {d} not encoded"))
    }

    pub(crate) fn fields(&self, d: &str) -> R<Vec<Field>> {
        let s = self.sym(d)?;
        if s.fields.is_empty() || runs(&s.fields).len() > 4 {
            return Err(format!("symbol {d} has {} fields", s.fields.len()));
        }
        Ok(s.fields.clone())
    }

    pub(crate) fn field(&self, d: &str) -> R<String> {
        Ok(fs(&self.fields(d)?))
    }

    /// A value table for a symbol, mapping each alternative's text to a code.
    pub(crate) fn table(&mut self, d: &str, code: impl Fn(&str) -> Option<u8>) -> R<String> {
        self.table_full(d, code, &[], &[])
    }

    pub(crate) fn table_full(
        &mut self,
        d: &str,
        code: impl Fn(&str) -> Option<u8>,
        names: &[String],
        nums: &[i64],
    ) -> R<String> {
        let s = self.sym(d)?;
        if s.fields.is_empty() {
            return Err(format!("table {d} has no fields"));
        }
        let full_widths: Vec<u32> = s.fields.iter().map(|f| f.width).collect();
        // Fields partly fixed by the encoding (BTI `op2` = `xx0`, `size` = `0x`): the
        // rule names may carry only the variable bits.
        let var_fields: Option<Vec<Field>> =
            s.fields.iter().map(|f| self.variable_part(f)).collect();
        let none_full = s
            .alts
            .iter()
            .all(|a| rule_bits(&a.rule, &full_widths).is_none());
        let tf = match var_fields {
            Some(v) if none_full => v,
            _ => s.fields.clone(),
        };
        let widths: Vec<u32> = tf.iter().map(|f| f.width).collect();
        let mut alts = s.alts.clone();
        let mut entries = Vec::new();
        let mut catch_all = None;
        let no_bits = alts.iter().all(|a| rule_bits(&a.rule, &widths).is_none());
        // SVE/SME rule names carry no values, though their numeric suffixes can look
        // like bits (`T_B__10`).
        if self.sve() {
            let full = ((1u64 << widths.iter().sum::<u32>()) - 1) as u32;
            // An immediate that shares the fields' low bits (`tsz:imm3`, `tsize:imm5`):
            // a symbol over these fields and more.
            let with_imm = self.syms.values().any(|s| {
                s.fields.len() > tf.len()
                    && s.fields.iter().any(|f| tf.iter().any(|t| t.lsb == f.lsb))
            });
            let pairs: Vec<Vec<(u32, u32)>> = match crate::sve::onehot(&tf, &alts, with_imm) {
                Some(p) => {
                    self.hand("SVE element size from tsz (one-hot) or imm13");
                    p
                }
                None => self
                    .allowed_entries(d, &tf, alts.len())?
                    .into_iter()
                    .map(|v| vec![(v, full)])
                    .collect(),
            };
            for (a, p) in alts.iter().zip(pairs) {
                let c = code(&a.text)
                    .ok_or_else(|| format!("table {d}: no meaning for {:?}", a.text))?;
                for (v, m) in p {
                    entries.push(format!("({v:#x}, {m:#x}, {c})"));
                }
            }
            alts.clear();
        }
        for (i, a) in alts.iter().enumerate() {
            let Some(c) = code(&a.text) else {
                return Err(format!("table {d}: no meaning for {:?}", a.text));
            };
            let fnames: Vec<&str> = tf.iter().map(|f| f.name.as_str()).collect();
            let std = if no_bits {
                immh_bits(d, &fnames, &a.text)
            } else {
                None
            };
            match rule_bits(&a.rule, &widths).or(std) {
                Some((v, m)) => entries.push(format!("({v:#x}, {m:#x}, {c})")),
                // A one-bit `Q` table whose rule names carry no values
                // (`Tb_2S__1bit`, `Tb_4S__1bit`): the alternatives are in Q order.
                None if no_bits && tf.len() == 1 && tf[0].name == "Q" && alts.len() == 2 => {
                    self.rep
                        .hand_rules
                        .entry("Q table in alternative order".into())
                        .and_modify(|n| *n += 1)
                        .or_insert(1);
                    entries.push(format!("({i:#x}, 0x1, {c})"));
                }
                // One alternative without values: the default for the rest.
                None if catch_all.is_none() && alts.len() > 1 => catch_all = Some(c),
                None => {
                    return Err(format!(
                        "table {d}: rule {} carries no field values",
                        a.rule
                    ));
                }
            }
        }
        if let Some(c) = catch_all {
            entries.push(format!("(0x0, 0x0, {c})"));
        }
        let body = format!(
            "Tab {{ f: {}, e: &[{}], names: &[{}], nums: &[{}] }}",
            fs(&tf),
            entries.join(", "),
            names
                .iter()
                .map(|n| format!("{n:?}"))
                .collect::<Vec<_>>()
                .join(", "),
            nums.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.rep.data_tables += 1;
        Ok(self.rep.tabs.add(body))
    }

    /// A table body of the generator's own (`Tab { ... }`), shared like the others.
    pub(crate) fn tab(&mut self, body: String) -> String {
        self.rep.tabs.add(body)
    }

    /// An SVE or SME encoding.
    pub(crate) fn sve(&self) -> bool {
        matches!(self.group.as_str(), "sve" | "sme")
    }

    /// The values of the concatenated fields `f` (first field most significant)
    /// that some word of the encoding has: words that match its fixed bits and
    /// satisfy its path conditions, with the other variable bits sampled.
    pub(crate) fn allowed(&self, f: &[Field]) -> Vec<u32> {
        let total: u32 = f.iter().map(|x| x.width).sum();
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut fills = vec![0u32, u32::MAX];
        for _ in 0..62 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            fills.push((seed >> 16) as u32);
        }
        let mut out = Vec::new();
        for v in 0..1u32 << total {
            let ok = fills.iter().any(|&fill| {
                let mut w = self.pat_value | (fill & !self.pat_mask);
                let mut shift = total;
                for x in f {
                    shift -= x.width;
                    let m = ((1u64 << x.width) - 1) as u32;
                    w = (w & !(m << x.lsb)) | ((v >> shift & m) << x.lsb);
                }
                w & self.pat_mask == self.pat_value && self.env.arena.holds(&self.env.conds, w)
            });
            if ok {
                out.push(v);
            }
        }
        out
    }

    /// SVE/SME tables, whose rule names carry no field values: the alternatives, in
    /// spec order, are the field values the encoding allows, in ascending order.
    fn allowed_entries(&mut self, d: &str, tf: &[Field], n: usize) -> R<Vec<u32>> {
        let total: u32 = tf.iter().map(|x| x.width).sum();
        if total > 8 {
            return Err(format!("table {d}: {total} bits of fields, no hand rule"));
        }
        let a = self.allowed(tf);
        if a.len() == n {
            self.hand("SVE table: alternatives in allowed-value order");
            return Ok(a);
        }
        // More values than names: the rest are RESERVED by the decode pseudocode
        // (`size` = 11 for byte to word reductions, ...), which the open-source spec
        // omits. Element sizes on a two-bit `size` then take their size code (B 00,
        // H 01, S 10, D 11), counted from the widest table on the same field so that
        // `<Tb>` lines up with `<T>` (`H, S, D` with `B, H, S` is 01, 10, 11).
        if let [f] = tf
            && f.name == "size"
            && f.width == 2
        {
            let codes = |s: &Sym| -> Option<Vec<u32>> {
                (s.fields.len() == 1 && s.fields[0].lsb == f.lsb && s.fields[0].width == 2)
                    .then_some(())?;
                s.alts
                    .iter()
                    .map(|a| crate::sve::es_log2(&a.text))
                    .collect()
            };
            let anchor = self
                .syms
                .values()
                .filter_map(codes)
                .filter(|c| c.len() == n)
                .max_by_key(|c| c.last().copied());
            if let Some(c) = anchor
                && c.iter().all(|v| a.contains(v))
            {
                self.hand("SVE size table: element size code, others RESERVED");
                return Ok(c);
            }
        }
        Err(format!(
            "table {d}: {n} alternatives for {} allowed values",
            a.len()
        ))
    }

    /// The bits of a field the encoding leaves variable, when they are contiguous.
    pub(crate) fn variable_part(&self, f: &Field) -> Option<Field> {
        let full: u32 = ((1u64 << f.width) - 1) as u32;
        let var = !(self.pat_mask >> f.lsb) & full;
        if var == 0 {
            return None;
        }
        let low = var.trailing_zeros();
        if (var >> low).trailing_ones() != var.count_ones() {
            return None;
        }
        Some(Field {
            name: f.name.clone(),
            lsb: f.lsb + low,
            width: var.count_ones(),
        })
    }

    /// A hand-written table over explicit fields.
    pub(crate) fn hand_table(
        &mut self,
        rule: &str,
        f: &[Field],
        e: &[(u32, u32, u8)],
        names: &[&str],
    ) -> String {
        self.hand(rule);
        let body = format!(
            "Tab {{ f: {}, e: &[{}], names: &[{}], nums: &[] }}",
            fs(f),
            e.iter()
                .map(|(v, m, c)| format!("({v:#x}, {m:#x}, {c})"))
                .collect::<Vec<_>>()
                .join(", "),
            names
                .iter()
                .map(|n| format!("{n:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.rep.tabs.add(body)
    }

    fn arr_table(&mut self, d: &str) -> R<String> {
        let t = self.table(d, |s| ARRS.iter().position(|a| *a == s).map(|i| i as u8))?;
        Ok(format!("Arr::Tab({t})"))
    }

    /// `<T>` or a fixed arrangement word.
    pub(crate) fn arr(&mut self, t: &str) -> R<String> {
        if t == "NONE" {
            return Ok("Arr::Fixed(Arrangement::None)".into());
        }
        if t.starts_with('<') {
            self.arr_table(t)
        } else {
            arr_variant(t)
                .map(|a| format!("Arr::Fixed(Arrangement::{a})"))
                .ok_or_else(|| format!("arrangement {t}"))
        }
    }

    /// A number table (`#<imm>` from `Q`, `<amount>` from `S` or `cmode`).
    pub(crate) fn num_table(&mut self, d: &str) -> R<String> {
        let s = self.sym(d)?;
        let mut nums: Vec<i64> = Vec::new();
        for a in &s.alts {
            let t = a.text.trim_start_matches('#');
            let n = t
                .parse::<i64>()
                .map_err(|_| format!("number table {d}: {t:?}"))?;
            if !nums.contains(&n) {
                nums.push(n);
            }
        }
        let n2 = nums.clone();
        self.table_full(
            d,
            move |t| {
                let n = t.trim_start_matches('#').parse::<i64>().ok()?;
                n2.iter().position(|&x| x == n).map(|i| i as u8)
            },
            &[],
            &nums,
        )
    }

    pub(crate) fn is_table_sym(&self, d: &str) -> bool {
        self.syms
            .get(d)
            .is_some_and(|s| !s.alts.is_empty() && s.alts.iter().all(|a| !a.text.contains("{N}")))
    }

    /// The access size (log2 bytes) of a load/store, from its transfer register.
    fn access_scale(&self, reg: &str) -> R<u8> {
        let letter = reg.trim_start_matches('<').chars().next().unwrap_or(' ');
        Ok(match letter {
            'B' => 0,
            'H' => 1,
            'S' => 2,
            'D' => 3,
            'Q' => 4,
            'W' | 'X' => {
                // From the encoding's fixed `size` bits.
                match self.name.split('_').nth(1) {
                    _ if self.mnemonic.ends_with('B') && !self.mnemonic.ends_with("AB") => 0,
                    _ if self.mnemonic.ends_with('H') => 1,
                    _ if self.mnemonic.ends_with("SW") => 2,
                    _ => {
                        if letter == 'X' {
                            3
                        } else {
                            2
                        }
                    }
                }
            }
            _ => return Err(format!("access size of {reg}")),
        })
    }
}

/// Parses a register symbol: `(kind letter, add)` for `<Xd>`, `<Wn|WSP>`, `<Hd>`,
/// `<X(s+1)>`, `<Xt+1>`, `<Vn+2>`, `<Vt3>` (list member: add 2).
pub(crate) fn reg_sym(d: &str) -> Option<(&'static str, u8)> {
    let inner = d.strip_prefix('<')?.strip_suffix('>')?;
    let (body, sp) = match inner.split_once('|') {
        Some((b, "SP")) | Some((b, "WSP")) => (b, true),
        Some(_) => return None,
        None => (inner, false),
    };
    let mut chars = body.chars();
    let letter = chars.next()?;
    let rest: String = chars.collect();
    if rest.is_empty() {
        return None;
    }
    let mut add = 0u8;
    if let Some(r) = rest.strip_prefix('(') {
        // `X(s+1)`.
        let r = r.strip_suffix(')')?;
        let (_, n) = r.split_once('+')?;
        add = n.parse().ok()?;
    } else if let Some((name, n)) = rest.split_once('+') {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase()) {
            return None;
        }
        add = n.parse().ok()?;
    } else {
        // `t`, `t1`, `t2`, `n`, `d`, `a` ...; `Vt2`..`Vt4` are list members.
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase())
            .collect();
        let num: String = rest.chars().skip(name.len()).collect();
        if name.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if letter == 'V' && !num.is_empty() && name == "t" {
            add = num.parse::<u8>().ok()? - 1;
        }
    }
    let kind = match (letter, sp) {
        ('X', false) => "X",
        ('X', true) => "XSp",
        ('W', false) => "W",
        ('W', true) => "WSp",
        ('B', false) => "B",
        ('H', false) => "H",
        ('S', false) => "S",
        ('D', false) => "D",
        ('Q', false) => "Q",
        ('V', false) => "V",
        _ => return None,
    };
    Some((kind, add))
}

/// Matches `s` against `pat`: `%` captures a `<...>` symbol, `%w` an alphanumeric
/// word, anything else is literal.
pub(crate) fn m(pat: &str, s: &str) -> Option<Vec<String>> {
    let p = pat.as_bytes();
    let b = s.as_bytes();
    let (mut i, mut j) = (0, 0);
    let mut caps = Vec::new();
    while i < p.len() {
        if p[i] == b'%' {
            if p.get(i + 1) == Some(&b'w') {
                let st = j;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-') {
                    j += 1;
                }
                if st == j {
                    return None;
                }
                caps.push(s[st..j].to_string());
                i += 2;
            } else {
                if b.get(j) != Some(&b'<') {
                    return None;
                }
                let st = j;
                while j < b.len() && b[j] != b'>' {
                    j += 1;
                }
                if j == b.len() {
                    return None;
                }
                j += 1;
                caps.push(s[st..j].to_string());
                i += 1;
            }
        } else {
            if b.get(j) != Some(&p[i]) {
                return None;
            }
            i += 1;
            j += 1;
        }
    }
    (j == b.len()).then_some(caps)
}

/// Renders a template for shape matching: `{...}` optional, `(a|b)` alternatives,
/// `⟪ ⟫` the literal braces of register lists.
pub(crate) fn shape(t: &[T]) -> String {
    let mut s = String::new();
    for x in t {
        match x {
            T::Text(a) => s.push_str(&a.replace('{', "⟪").replace('}', "⟫")),
            T::Sym { display, .. } => s.push_str(display),
            T::Opt(v) => {
                s.push('{');
                s.push_str(&shape(v));
                s.push('}');
            }
            T::Alt(v) => {
                s.push('(');
                s.push_str(&v.iter().map(|a| shape(a)).collect::<Vec<_>>().join("|"));
                s.push(')');
            }
        }
    }
    s
}

/// Splits operands at top-level `, `.
fn split_groups(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '[' | '{' | '(' | '⟪' | '<' => depth += 1,
            ']' | '}' | ')' | '⟫' | '>' => depth -= 1,
            _ => {}
        }
        if depth == 0 && c == ',' && chars.get(i + 1) == Some(&' ') {
            out.push(std::mem::take(&mut cur));
            i += 2;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn collect_syms(
    t: &[T],
    enc: &Value,
    model: &Model<'_>,
    node: usize,
    out: &mut BTreeMap<String, Sym>,
    order: &mut Vec<String>,
) {
    for x in t {
        match x {
            T::Sym { display, alts, .. } => {
                if out.contains_key(display) {
                    continue;
                }
                order.push(display.clone());
                let mut fields = Vec::new();
                let mut ok = true;
                for v in enc[display.as_str()].as_array().into_iter().flatten() {
                    match v["value"].as_str().and_then(|n| model.field(node, n)) {
                        Some(f) if v["_type"] == "AST.Identifier" => fields.push(f),
                        _ => ok = false,
                    }
                }
                if !ok {
                    fields.clear();
                }
                out.insert(
                    display.clone(),
                    Sym {
                        fields,
                        alts: alts.clone(),
                    },
                );
            }
            T::Opt(v) => collect_syms(v, enc, model, node, out, order),
            T::Alt(v) => v
                .iter()
                .for_each(|a| collect_syms(a, enc, model, node, out, order)),
            T::Text(_) => {}
        }
    }
}

pub fn map(
    model: &Model<'_>,
    leaf: &Leaf<'_>,
    t: &[T],
    mnemonic: &str,
    env: &Env<'_>,
    rep: &mut Report,
) -> Option<String> {
    let res = map_atomic(model, leaf, leaf.json, t, mnemonic, env, rep);
    match res {
        Ok(s) => {
            rep.mapped += 1;
            Some(s)
        }
        Err(e) => {
            rep.unmapped += 1;
            let entry = rep.failures.entry(e).or_default();
            entry.0 += 1;
            if entry.1.len() < 3 {
                entry.1.push(leaf.name.clone());
            }
            None
        }
    }
}

pub fn map_alias(
    model: &Model<'_>,
    leaf: &Leaf<'_>,
    alias: &Value,
    t: &[T],
    mnemonic: &str,
    env: &Env<'_>,
    rep: &mut Report,
) -> Option<String> {
    // Alias failures are not counted against the encoding.
    let (mapped, unmapped, failures) = (rep.mapped, rep.unmapped, rep.failures.clone());
    let r = map_atomic(model, leaf, alias, t, mnemonic, env, rep);
    rep.mapped = mapped;
    rep.unmapped = unmapped;
    rep.failures = failures;
    match r {
        Ok(s) => Some(s),
        Err(e) => {
            let entry = rep.alias_failures.entry(e).or_default();
            entry.0 += 1;
            if entry.1.len() < 3 {
                entry.1.push(format!("{mnemonic} ({})", leaf.name));
            }
            None
        }
    }
}

/// [`map_inner`], leaving no tables or hand-rule counts behind when it fails.
fn map_atomic(
    model: &Model<'_>,
    leaf: &Leaf<'_>,
    json: &Value,
    t: &[T],
    mnemonic: &str,
    env: &Env<'_>,
    rep: &mut Report,
) -> R<String> {
    let (tabs, data, hand) = (
        rep.tabs.bodies.len(),
        rep.data_tables,
        rep.hand_rules.clone(),
    );
    let r = map_inner(model, leaf, json, t, mnemonic, env, rep);
    if r.is_err() {
        rep.tabs.bodies.truncate(tabs);
        rep.tabs.index.retain(|_, i| *i < tabs);
        rep.data_tables = data;
        rep.hand_rules = hand;
    }
    r
}

fn map_inner(
    model: &Model<'_>,
    leaf: &Leaf<'_>,
    json: &Value,
    t: &[T],
    mnemonic: &str,
    env: &Env<'_>,
    rep: &mut Report,
) -> R<String> {
    rep.last_suffixes.clear();
    rep.last_arity = 0;
    rep.last_cond = false;
    let mut syms = BTreeMap::new();
    let mut order = Vec::new();
    collect_syms(
        t,
        &json["_meta"]["encoded_in"],
        model,
        leaf.node,
        &mut syms,
        &mut order,
    );
    let mut cx = Cx {
        name: leaf.name.clone(),
        leaf_fields: model.nodes[leaf.node].fields.clone(),
        alias: !std::ptr::eq(json, leaf.json),
        pat_mask: model.full_pattern(leaf.node).mask,
        pat_value: model.full_pattern(leaf.node).value,
        env,
        order,
        iclass: leaf.iclass.clone(),
        group: {
            let p = model.path(leaf.node);
            model.nodes[p[p.len() - 2]].name.clone()
        },
        mnemonic: mnemonic.to_string(),
        syms,
        rep,
    };
    dump(&cx, t);
    let s = shape(t);
    // The mnemonic part runs to the first space outside brackets and symbols.
    let split = {
        let mut depth = 0i32;
        let mut at = None;
        for (i, c) in s.char_indices() {
            match c {
                '[' | '{' | '(' | '⟪' | '<' => depth += 1,
                ']' | '}' | ')' | '⟫' | '>' => depth -= 1,
                ' ' if depth == 0 => {
                    at = Some(i);
                    break;
                }
                _ => {}
            }
        }
        at
    };
    let (head, rest) = match split {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s.as_str(), ""),
    };
    let mut ops: Vec<String> = Vec::new();
    // Mnemonic suffixes: `.<cond>`, `{2}`, `<bt>`.
    let suffix = &head[mnemonic.len().min(head.len())..];
    let mut head_rest = String::new();
    match suffix {
        "" => {}
        ".<cond>" => {
            cx.hand("cond suffix");
            cx.rep.last_cond = true;
            ops.push(format!("Op::Cond({}, true, false)", cx.field("<cond>")?));
        }
        "2" | "<bt>" => {
            let s = cx.sym(suffix)?;
            let names: Vec<String> = s.alts.iter().map(|a| a.text.clone()).collect();
            let n2 = names.clone();
            let t = cx.table_full(
                suffix,
                move |x| n2.iter().position(|n| n == x).map(|p| p as u8),
                &names,
                &[],
            )?;
            ops.push(format!("Op::Suffix({t})"));
            cx.rep.last_suffixes = names;
        }
        _ => {
            // `RET{ <Xn>}`, `ISB{( <option>| #<imm>)}`: an optional operand.
            if let Some(o) = suffix.strip_prefix("{ ").and_then(|x| x.strip_suffix('}')) {
                head_rest = o.to_string();
            } else if let Some(o) = suffix.strip_prefix("{( ") {
                head_rest = format!("({}", o.strip_suffix('}').unwrap_or(o).replace("| ", "|"));
            } else {
                return Err(format!("mnemonic suffix {suffix}"));
            }
        }
    }
    if !head_rest.is_empty() {
        let inner = group(&mut cx, &head_rest, None)?;
        ops.push(optional(&mut cx, &inner, &head_rest)?);
    }
    let groups = split_groups(rest);
    let mut i = 0;
    while i < groups.len() {
        let g = groups[i].as_str();
        // A memory operand followed by a post-index part.
        if !cx.sve()
            && g.starts_with('[')
            && g.ends_with(']')
            && i + 1 < groups.len()
            && let Some(op) = post_index(&mut cx, g, &groups[i + 1])?
        {
            ops.push(op);
            i += 2;
            continue;
        }
        // `MOVI <Vd>.<T>, #<imm8>, MSL #<amount>`: the MSL belongs to the immediate.
        if let Some(c) = m("MSL #%", g) {
            let last = ops.pop().ok_or("MSL without an immediate")?;
            let a = sym_imm(&mut cx, &c[0])?;
            let last = last
                .strip_suffix(", ImmShift::None)")
                .ok_or("MSL after a non-immediate")?
                .to_string();
            ops.push(format!("{last}, ImmShift::Msl({a}))"));
            i += 1;
            continue;
        }
        // A trailing optional operand: `SYS ..., #<op2>{, <Xt>}`, `IRG <Xd|SP>,
        // <Xn|SP>{, <Xm>}`, `SYSP ..., #<op2>{, <Xt1>, <Xt2>}`.
        let trailing = ["{, <Xt>}", "{, <Xm>}", "{, <Xt1>, <Xt2>}"]
            .iter()
            .find(|t| g.ends_with(**t) && g.len() > t.len());
        if let Some(t) = trailing {
            let main = &g[..g.len() - t.len()];
            ops.push(group(&mut cx, main, None)?);
            let inner = &t[3..t.len() - 1];
            for part in inner.split(", ") {
                let o = group(&mut cx, part, None)?;
                ops.push(optional(&mut cx, &o, part)?);
            }
            i += 1;
            continue;
        }
        // `<Xn>{, <nb>}`: NB is itself optional.
        if let Some(main) = g.strip_suffix("{, <nb>}") {
            ops.push(group(&mut cx, main, None)?);
            ops.push(single(&mut cx, "<nb>")?);
            i += 1;
            continue;
        }
        // `TEXIT { <nb>}`, `GCSPOPM { <Xt>}`: an optional last operand.
        if let Some(inner) = g.strip_prefix("{ ").and_then(|x| x.strip_suffix('}'))
            && i + 1 == groups.len()
            && m("%", inner).is_some()
        {
            let o = single(&mut cx, inner)?;
            ops.push(optional(&mut cx, &o, inner)?);
            i += 1;
            continue;
        }
        if cx.sve()
            && let Some(r) = crate::sve::multi(&mut cx, g)
        {
            ops.extend(r?);
            i += 1;
            continue;
        }
        if g.starts_with('{') {
            return Err(format!("optional group {g}"));
        }
        let op = group(&mut cx, g, groups.get(i + 1).map(|x| x.as_str()))?;
        ops.push(op);
        i += 1;
    }
    if ops.len() > 8 {
        return Err("more than 8 operands".into());
    }
    cx.rep.last_arity = ops.iter().filter(|o| !o.starts_with("Op::Suffix")).count();
    Ok(ops.join(", "))
}

/// Wraps an operand that may be left out, with its default field value.
fn optional(cx: &mut Cx<'_>, inner: &str, shape: &str) -> R<String> {
    if inner.starts_with("Op::Opt(") {
        return Ok(inner.to_string());
    }
    let (f, default): (Vec<Field>, u32) = if let Some(c) = m("<Xn>", shape) {
        let _ = c;
        (cx.fields("<Xn>")?, 30)
    } else if m("#<imm>", shape).is_some() {
        let f = cx.fields("<imm>")?;
        // CLREX #15, DCPS #0.
        let d = if cx.mnemonic == "CLREX" { 15 } else { 0 };
        (f, d)
    } else if m("<targets>", shape).is_some() {
        (cx.fields("<targets>")?, 0)
    } else if shape.starts_with("(<option>") {
        (cx.fields("<imm>")?, 15)
    } else if let Some(c) = ["<Xt>", "<Xm>", "<Xt1>", "<Xt2>"]
        .iter()
        .find(|s| **s == shape)
    {
        (cx.fields(c)?, 31)
    } else if m("<nb>", shape).is_some() {
        (cx.fields("<nb>")?, 0)
    } else {
        return Err(format!("optional operand {shape}"));
    };
    cx.hand("optional operand default");
    let inner = inner.to_string();
    Ok(format!("Op::Opt(&{inner}, {}, {default})", fs(&f)))
}

fn reg(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let (k, add) = reg_sym(d).ok_or_else(|| format!("register symbol {d}"))?;
    let f = cx.field(d)?;
    if matches!(d, "<Xt2>" | "<Wt2>") && cx.fields(d)?.first().is_some_and(|f| f.name == "Rt") {
        // `SYSP ..., <Xt1>, <Xt2>`: one field names the pair.
        cx.hand("register pair from one field");
        return Ok(format!("Op::RegPair({f}, Rk::{k})"));
    }
    cx.hand(&format!("register <{k}..>"));
    Ok(if k == "V" {
        format!("Op::Vec({f}, Arr::Fixed(Arrangement::None), {add})")
    } else {
        format!("Op::Reg({f}, RegSel::Fixed(Rk::{k}), {add})")
    })
}

/// `<V><d>`, `<R><n>`, `<Vb><m>`: a register whose size comes from a table.
fn sized_reg(cx: &mut Cx<'_>, size: &str, num: &str) -> R<String> {
    let f = cx.field(num)?;
    if let Ok(s) = cx.sym(size)
        && s.fields.is_empty()
        && let [a] = s.alts.as_slice()
        && let Some(k) = rk_of_letter(&a.text)
    {
        cx.hand("fixed register size");
        return Ok(format!(
            "Op::Reg({f}, RegSel::Fixed(Rk::{}), 0)",
            RKS[k as usize]
        ));
    }
    let t = r_table(cx, size)?;
    Ok(format!("Op::Reg({f}, RegSel::Tab({t}), 0)"))
}

/// The register-size table of `<R>`, `<V>`, `<Va>`, ...
fn r_table(cx: &mut Cx<'_>, size: &str) -> R<String> {
    if size == "<R>"
        && cx
            .fields(size)
            .is_ok_and(|v| v.len() == 1 && v[0].name == "imm5")
    {
        // INS/DUP (general): X for D elements (imm5 = x1000), else W.
        let o = cx.fields(size)?;
        return Ok(cx.hand_table(
            "INS/DUP register width: X iff imm5 = x1000",
            &o,
            &[(0b01000, 0b01111, 0), (0, 0, 1)],
            &[],
        ));
    }
    if size == "<R>"
        && cx
            .fields(size)
            .is_ok_and(|v| v.len() == 1 && v[0].name == "option")
    {
        // The rule names list only `00x` and `x11`; W is every value but `x11`.
        let o = cx.fields(size)?;
        return Ok(cx.hand_table(
            "extend register width: X iff option<1:0> = 11",
            &o,
            &[(0b011, 0b011, 0), (0, 0, 1)],
            &[],
        ));
    }
    if size == "<R>"
        && cx.sve()
        && cx
            .fields(size)
            .is_ok_and(|v| v.len() == 1 && v[0].name == "size")
    {
        // INDEX, LASTA, CLASTA, ...: X for doubleword elements, else W.
        let o = cx.fields(size)?;
        return Ok(cx.hand_table(
            "SVE register width: X iff size = 11",
            &o,
            &[(0b11, 0b11, 0), (0, 0, 1)],
            &[],
        ));
    }
    if size == "<R>"
        && cx
            .fields(size)
            .is_ok_and(|v| v.len() == 1 && v[0].name == "sz")
    {
        // CTERMEQ/CTERMNE: the rule names carry no values; `sz` 1 is X.
        let o = cx.fields(size)?;
        return Ok(cx.hand_table(
            "CTERM register width: X iff sz = 1",
            &o,
            &[(1, 1, 0), (0, 1, 1)],
            &[],
        ));
    }
    cx.table(size, rk_of_letter)
}

/// The value of an immediate symbol.
pub(crate) fn sym_imm(cx: &mut Cx<'_>, d: &str) -> R<String> {
    if let Some(r) = alias_imm(cx, d)? {
        return Ok(r);
    }
    if cx.is_table_sym(d) {
        let t = cx.num_table(d)?;
        return Ok(format!("Imm::Tab({t})"));
    }
    if cx.sve() {
        // The spec says whether the number is signed (`SInteger`) or not.
        let f = cx.field(d)?;
        let signed = cx.sym(d)?.alts.iter().any(|a| a.signed);
        cx.hand("SVE immediate, signed as the spec's number token says");
        return Ok(format!("Imm::{}({f}, 0)", if signed { "S" } else { "U" }));
    }
    let fields = cx.fields(d)?;
    let f = fs(&fields);
    let names: Vec<&str> = fields.iter().map(|x| x.name.as_str()).collect();
    let mn = cx.mnemonic.clone();
    let rule = match (d, names.as_slice()) {
        ("<shift>", ["immh", "immb"]) => {
            let left = mn.contains("SHL") || mn.contains("SLI") || mn == "SSHLL" || mn == "USHLL";
            if left {
                cx.hand("<shift> immh:immb left");
                return Ok(format!("Imm::ShlEs({f})"));
            }
            cx.hand("<shift> immh:immb right");
            return Ok(format!("Imm::ShrEs({f})"));
        }
        ("<fbits>", ["immh", "immb"]) => {
            cx.hand("<fbits> immh:immb");
            return Ok(format!("Imm::ShrEs({f})"));
        }
        ("<index>", ["Q", "imm4"]) => {
            // EXT: the byte index is imm4 (imm4<3> must be zero when Q is 0).
            cx.hand("EXT index imm4");
            return Ok(format!("Imm::U({}, 0)", fs(&fields[1..])));
        }
        ("<fbits>", ["scale"]) => {
            cx.hand("<fbits> 64-scale");
            return Ok(format!("Imm::Rsub(64, {f})"));
        }
        ("<shift>", ["hw"]) => {
            cx.hand("<shift> hw*16");
            return Ok(format!("Imm::U({f}, 4)"));
        }
        ("<uimm6>", _) => {
            cx.hand("<uimm6> tag granules");
            return Ok(format!("Imm::U({f}, 4)"));
        }
        ("<imm>", ["imm9h", "imm9l"]) | ("<imm>", ["simm7"]) | ("<simm>", _) => "signed",
        ("<imm>", ["imm9"]) if cx.iclass == "ldsttags" => {
            cx.hand("ldsttags offset*16");
            return Ok(format!("Imm::S({f}, 4)"));
        }
        _ => "unsigned",
    };
    cx.hand(&format!("{d} {rule}"));
    Ok(if rule == "signed" {
        format!("Imm::S({f}, 0)")
    } else {
        format!("Imm::U({f}, 0)")
    })
}

fn bitmask(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let fields = cx.fields(d)?;
    let get = |n: &str| fields.iter().find(|f| f.name == n).cloned();
    let immr = get("immr").ok_or("bitmask without immr")?;
    let imms = get("imms").ok_or("bitmask without imms")?;
    let n = get("N");
    let size = if cx.name.contains("_32") { 32 } else { 64 };
    cx.hand("bitmask immediate");
    let nf = n.map_or("Fs::NONE".to_string(), |n| fs(&[n]));
    Ok(format!(
        "Op::BitImm(Imm::Bitmask {{ n: {nf}, immr: {}, imms: {}, size: {size} }})",
        fs(&[immr]),
        fs(&[imms])
    ))
}

/// One operand group.
pub(crate) fn group(cx: &mut Cx<'_>, g: &str, _next: Option<&str>) -> R<String> {
    if cx.sve()
        && let Some(r) = crate::sve::group(cx, g)
    {
        return r;
    }
    // Registers.
    if let Some(c) = m("%", g) {
        return single(cx, &c[0]);
    }
    if let Some(c) = m("%!", g) {
        let (k, _) = reg_sym(&c[0]).ok_or("writeback register")?;
        cx.hand("register writeback");
        return Ok(format!("Op::RegWb({}, Rk::{k})", cx.field(&c[0])?));
    }
    if let Some(c) = m("%.%", g) {
        let a = cx.arr(&c[1])?;
        let f = cx.field(&c[0])?;
        let (_, add) = reg_sym(&c[0]).ok_or("vector register")?;
        return Ok(format!("Op::Vec({f}, {a}, {add})"));
    }
    if let Some(c) = m("%.%w", g) {
        let a = cx.arr(&c[1])?;
        let f = cx.field(&c[0])?;
        let (_, add) = reg_sym(&c[0]).ok_or("vector register")?;
        return Ok(format!("Op::Vec({f}, {a}, {add})"));
    }
    if let Some(c) = m("%.%[%]", g).or_else(|| m("%.%w[%]", g)) {
        return elem(cx, &c[0], &c[1], &c[2]);
    }
    if let Some(c) = m("%.%w[%w]", g) {
        let a = cx.arr(&c[1])?;
        let f = cx.field(&c[0])?;
        let i: u8 = c[2].parse().map_err(|_| format!("index {}", c[2]))?;
        return Ok(format!("Op::Elem({f}, {a}, Idx::Const({i}), false)"));
    }
    if let Some(c) = m("V%.%[%]", g) {
        // `V<m>.<Ts>[<index>]`: register from `<m>` (M:Rm, or Rm for H elements).
        return elem(cx, &c[0], &c[1], &c[2]);
    }
    if let Some(c) = m("%[%]", g) {
        // `<Vm>[<index>]` (LUTI2/LUTI4): a vector segment index.
        return elem(cx, &c[0], "NONE", &c[1]);
    }
    if let Some(c) = m("%%", g) {
        // `<V><d>`, `<R><n>`.
        return sized_reg(cx, &c[0], &c[1]);
    }
    if let Some(c) = m("%w%", g) {
        // `D<n>`, `S<d>`, `H<d>`.
        let k = match c[0].as_str() {
            "B" | "H" | "S" | "D" | "Q" => c[0].clone(),
            _ => return Err(format!("register prefix {}", c[0])),
        };
        cx.hand("fp register <letter><n>");
        return Ok(format!(
            "Op::Reg({}, RegSel::Fixed(Rk::{k}), 0)",
            cx.field(&c[1])?
        ));
    }
    // Register lists.
    if g.starts_with('⟪') {
        return list(cx, g);
    }
    // Immediates.
    if let Some(c) = m("#%", g) {
        return imm(cx, &c[0]);
    }
    if let Some(c) = m("#%{, %}", g) {
        // `#<imm>{, <shift>}` with `<shift>` = LSL #0 / LSL #12.
        let i = sym_imm(cx, &c[0])?;
        let s = cx.sym(&c[1])?;
        let nums: Vec<i64> = s
            .alts
            .iter()
            .map(|a| {
                a.text
                    .trim_start_matches("LSL #")
                    .parse::<i64>()
                    .unwrap_or(-1)
            })
            .collect();
        if nums.contains(&-1) {
            return Err(format!("immediate shift {}", c[1]));
        }
        let n2 = nums.clone();
        let t = cx.table_full(
            &c[1],
            move |t| {
                let n = t.trim_start_matches("LSL #").parse::<i64>().ok()?;
                n2.iter().position(|&x| x == n).map(|p| p as u8)
            },
            &[],
            &nums,
        )?;
        return Ok(format!("Op::Imm({i}, ImmShift::Lsl(Imm::Tab({t})))"));
    }
    if let Some(c) = m("#%{, LSL #0}", g) {
        let i = sym_imm(cx, &c[0])?;
        return Ok(format!("Op::Imm({i}, ImmShift::None)"));
    }
    if let Some(c) = m("#%{, LSL #%}", g) {
        let i = sym_imm(cx, &c[0])?;
        let a = sym_imm(cx, &c[1])?;
        return Ok(format!("Op::Imm({i}, ImmShift::Lsl({a}))"));
    }
    if let Some(c) = m("#%w", g) {
        if c[0] == "0.0" {
            return Ok("Op::FpImm(Imm::Const(0))".into());
        }
        let v: i64 = c[0].parse().map_err(|_| format!("constant #{}", c[0]))?;
        return Ok(format!("Op::Imm(Imm::Const({v}), ImmShift::None)"));
    }
    if g == "#0.0" {
        return Ok("Op::FpImm(Imm::Const(0))".into());
    }
    if let Some(c) = m("%w", g) {
        return literal(cx, &c[0]);
    }
    // Shifted and extended registers.
    if let Some(c) = m("%{, % #%}", g) {
        let (k, _) = reg_sym(&c[0]).ok_or("shifted register")?;
        let t = cx.table(&c[1], |s| {
            ["LSL", "LSR", "ASR", "ROR"]
                .iter()
                .position(|x| *x == s)
                .map(|i| i as u8)
        })?;
        let a = cx.field(&c[2])?;
        return Ok(format!(
            "Op::Shifted({}, RegSel::Fixed(Rk::{k}), {t}, {a})",
            cx.field(&c[0])?
        ));
    }
    if let Some(c) = m("%{, LSL #%}", g) {
        let (k, _) = reg_sym(&c[0]).ok_or("shifted register")?;
        let a = cx.field(&c[1])?;
        let t = cx.hand_table("LSL-only shift", &[], &[(0, 0, 0)], &[]);
        return Ok(format!(
            "Op::Shifted({}, RegSel::Fixed(Rk::{k}), {t}, {a})",
            cx.field(&c[0])?
        ));
    }
    if let Some(c) = m("%%{, %{ #%}}", g) {
        // `<R><m>{, <extend>{ #<amount>}}`.
        let f = cx.field(&c[1])?;
        let t = r_table(cx, &c[0])?;
        let opt = cx.field(&c[2])?;
        let a = cx.field(&c[3])?;
        cx.hand("extended register");
        return Ok(format!("Op::Extended({f}, RegSel::Tab({t}), {opt}, {a})"));
    }
    if let Some(c) = m("%{, %{ #%}}", g) {
        let (k, _) = reg_sym(&c[0]).ok_or("extended register")?;
        let opt = cx.field(&c[1])?;
        let a = cx.field(&c[2])?;
        cx.hand("extended register");
        return Ok(format!(
            "Op::Extended({}, RegSel::Fixed(Rk::{k}), {opt}, {a})",
            cx.field(&c[0])?
        ));
    }
    // Memory.
    if g.starts_with('[') {
        return mem(cx, g, "MemoryMode::Offset");
    }
    // Alternatives.
    if let Some(c) = m("(%|S%_%_%_%_%)", g) {
        let _ = c;
        let f = cx.field("<systemreg>")?;
        cx.hand("system register");
        return Ok(format!("Op::SysReg({f})"));
    }
    if let Some(c) = m("(%|#%)", g) {
        return named(cx, &c[0], &c[1]);
    }
    if let Some(c) = m("%nXS", g) {
        let f = cx.fields(&c[0])?;
        let opts = [(0u32, "OSHnXS"), (1, "NSHnXS"), (2, "ISHnXS"), (3, "SYnXS")];
        let e: Vec<(u32, u32, u8)> = opts
            .iter()
            .enumerate()
            .map(|(i, (v, _))| (*v, 3, i as u8))
            .collect();
        let names: Vec<&str> = opts.iter().map(|(_, n)| *n).collect();
        let t = cx.hand_table("barrier options nXS", &f, &e, &names);
        return Ok(format!("Op::Name({t}, NameKind::Barrier)"));
    }
    Err(format!("operand shape {g}"))
}

pub(crate) fn single(cx: &mut Cx<'_>, d: &str) -> R<String> {
    match d {
        "<invcond>" => {
            cx.hand("alias inverted condition");
            return Ok(format!("Op::Cond({}, false, true)", cx.field(d)?));
        }
        _ if d.ends_with("_op>") => {
            cx.hand("system operation name");
            return Ok("Op::SysOperation(Fs([F::new(5, 14), F::NONE, F::NONE, F::NONE]))".into());
        }
        "<option>" if cx.mnemonic.starts_with("SMST") => {
            let f = [Field {
                name: "CRm21".into(),
                lsb: 9,
                width: 2,
            }];
            let t = cx.hand_table(
                "SMSTART/SMSTOP option",
                &f,
                &[(1, 3, 0), (2, 3, 1)],
                &["SM", "ZA"],
            );
            return Ok(format!(
                "Op::Opt(&Op::Name({t}, NameKind::Plain), {}, 3)",
                fs(&f)
            ));
        }
        "<label>" => return label(cx),
        "<cond>" => {
            cx.hand("condition");
            return Ok(format!("Op::Cond({}, false, false)", cx.field(d)?));
        }
        "<Cn>" | "<Cm>" => {
            cx.hand("CRn/CRm");
            return Ok(format!("Op::Cr({})", cx.field(d)?));
        }
        "<pstatefield>" => {
            cx.hand("PSTATE field");
            // op1:CRm:op2 regardless of the symbol's field order.
            let fields = cx.fields(d)?;
            let get = |n: &str| fields.iter().find(|f| f.name == n).cloned();
            let (Some(op1), Some(crm), Some(op2)) = (get("op1"), get("CRm"), get("op2")) else {
                return Err("pstatefield fields".into());
            };
            return Ok(format!("Op::PState({})", fs(&[op1, crm, op2])));
        }
        "<nb>" => {
            // `NB` when the field's variable bit is set (TCHANGEB/TCHANGEF, TSTART).
            let f = cx.fields(d)?;
            let v = cx.variable_part(&f[0]).ok_or("nb without a variable bit")?;
            let t = cx.hand_table(
                "NB on the variable op1 bit",
                std::slice::from_ref(&v),
                &[(1, 1, 0)],
                &["NB"],
            );
            return Ok(format!(
                "Op::Opt(&Op::Name({t}, NameKind::Plain), {}, 0)",
                fs(&[v])
            ));
        }
        "<targets>" | "<policy>" => {
            let s = cx.sym(d)?;
            let names: Vec<String> = s.alts.iter().map(|a| a.text.clone()).collect();
            let n2 = names.clone();
            let t = cx.table_full(
                d,
                move |x| n2.iter().position(|n| n == x).map(|p| p as u8),
                &names,
                &[],
            )?;
            return Ok(format!("Op::Name({t}, NameKind::Plain)"));
        }
        _ => {}
    }
    if d.starts_with('<') && reg_sym(d).is_some() {
        return reg(cx, d);
    }
    if cx.is_table_sym(d) {
        // `<imm>` from `Q` in post-index lists etc.
        let t = cx.num_table(d)?;
        return Ok(format!("Op::Imm(Imm::Tab({t}), ImmShift::None)"));
    }
    Err(format!("operand {d}"))
}

fn literal(cx: &mut Cx<'_>, w: &str) -> R<String> {
    if let Some(n) = w.strip_prefix('X').and_then(|n| n.parse::<u32>().ok()) {
        cx.hand("fixed register");
        return Ok(format!("Op::RegFixed(Register::X{n})"));
    }
    if matches!(w, "PH" | "STSHSTRM") && cx.syms.contains_key(w) {
        // Present when the variable `op2` bit is set (SHUH, SRLS).
        let f = cx.fields(w)?;
        let v = cx.variable_part(&f[0]).ok_or("no variable bit")?;
        let t = cx.hand_table(
            "hint name on the variable op2 bit",
            std::slice::from_ref(&v),
            &[(1, 1, 0)],
            &[w],
        );
        return Ok(format!(
            "Op::Opt(&Op::Name({t}, NameKind::Plain), {}, 0)",
            fs(&[v])
        ));
    }
    if w.chars().all(|c| c.is_ascii_uppercase()) {
        return Ok(format!("Op::Lit({w:?})"));
    }
    if cx.syms.contains_key(w) {
        // `PH`, `STSHSTRM`: a named table symbol without angle brackets.
        let s = cx.sym(w)?;
        let names: Vec<String> = s.alts.iter().map(|a| a.text.clone()).collect();
        let n2 = names.clone();
        let t = cx.table_full(
            w,
            move |x| n2.iter().position(|n| n == x).map(|p| p as u8),
            &names,
            &[],
        )?;
        return Ok(format!("Op::Name({t}, NameKind::Plain)"));
    }
    Err(format!("literal {w}"))
}

fn label(cx: &mut Cx<'_>) -> R<String> {
    let fields = cx.fields("<label>")?;
    let f = fs(&fields);
    let names: Vec<&str> = fields.iter().map(|x| x.name.as_str()).collect();
    if cx.name.contains("SPPC") {
        cx.hand("label PC - imm16*4");
        return Ok(format!("Op::Label(Imm::NegU({f}, 2), false)"));
    }
    if names == ["immhi", "immlo"] {
        let page = cx.mnemonic == "ADRP";
        cx.hand("label ADR/ADRP");
        return Ok(format!(
            "Op::Label(Imm::S({f}, {}), {page})",
            if page { 12 } else { 0 }
        ));
    }
    cx.hand("label *4");
    Ok(format!("Op::Label(Imm::S({f}, 2), false)"))
}

pub(crate) fn imm(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let fields = cx.fields(d).unwrap_or_default();
    let names: Vec<&str> = fields.iter().map(|x| x.name.as_str()).collect();
    if names.contains(&"immr") && names.contains(&"imms") && d == "<imm>" {
        return bitmask(cx, d);
    }
    let is_fp = cx.name.contains("floatimm") || cx.mnemonic == "FMOV";
    if names.len() == 8 && names[0] == "a" {
        // a:b:c:d:e:f:g:h.
        let f = fs(&fields);
        if is_fp {
            cx.hand("AdvSIMD FP immediate");
            return Ok(format!("Op::FpImm(Imm::Fp8({}))", fs8(&fields)));
        }
        if cx.name.starts_with("MOVI_asimdimm_D") {
            cx.hand("AdvSIMD 64-bit byte-mask immediate");
            return Ok(format!(
                "Op::Imm(Imm::ByteMask({}), ImmShift::None)",
                fs8(&fields)
            ));
        }
        let _ = f;
        cx.hand("AdvSIMD imm8");
        return Ok(format!(
            "Op::Imm(Imm::U({}, 0), ImmShift::None)",
            fs8(&fields)
        ));
    }
    if is_fp && names == ["imm8"] {
        cx.hand("FP immediate");
        return Ok(format!("Op::FpImm(Imm::Fp8({}))", fs(&fields)));
    }
    let i = sym_imm(cx, d)?;
    Ok(format!("Op::Imm({i}, ImmShift::None)"))
}

/// `a:b:c:d:e:f:g:h` is eight one-bit fields; they are contiguous in every
/// encoding (`a:b:c` at 18:16, `d:e:f:g:h` at 9:5), so use two ranges.
fn fs8(fields: &[Field]) -> String {
    let hi = Field {
        name: "abc".into(),
        lsb: fields[2].lsb,
        width: 3,
    };
    let lo = Field {
        name: "defgh".into(),
        lsb: fields[7].lsb,
        width: 5,
    };
    fs(&[hi, lo])
}

fn elem(cx: &mut Cx<'_>, r: &str, t: &str, idx: &str) -> R<String> {
    let a = cx.arr(t)?;
    let rf = cx.fields(r)?;
    let names: Vec<String> = rf.iter().map(|f| f.name.clone()).collect();
    // `<m>` in size:M:Rm: drop `size`; H elements use Rm<3:0>.
    let (reg_fields, mask_h): (Vec<Field>, bool) =
        if names.first().map(|s| s.as_str()) == Some("size") {
            (rf[1..].to_vec(), cx.mnemonic != "FCMLA")
        } else {
            (rf.clone(), false)
        };
    let i = index(cx, idx, t)?;
    // When the index uses Rm<3>, the register is Rm<2:0>.
    let idx_names: Vec<String> = cx.fields(idx)?.iter().map(|f| f.name.clone()).collect();
    let reg_fields = if idx_names.iter().any(|n| n == "Rm") {
        reg_fields
            .iter()
            .map(|f| {
                if f.name == "Rm" {
                    Field {
                        name: "Rm20".into(),
                        lsb: f.lsb,
                        width: 3,
                    }
                } else {
                    f.clone()
                }
            })
            .collect()
    } else {
        reg_fields
    };
    cx.hand("vector element");
    Ok(format!("Op::Elem({}, {a}, {i}, {mask_h})", fs(&reg_fields)))
}

fn index(cx: &mut Cx<'_>, idx: &str, es: &str) -> R<String> {
    let fields = cx.fields(idx)?;
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let get = |n: &str| fields.iter().find(|f| f.name == n).cloned();
    if cx.mnemonic == "FCMLA" {
        // Complex pairs: H elements index H:L, S elements index H (L zero).
        let (Some(h), Some(l)) = (get("H"), get("L")) else {
            return Err("FCMLA index fields".into());
        };
        cx.hand("FCMLA by element index");
        return Ok(format!(
            "Idx::ByEs {{ b: Fs::NONE, h: {}, s: {}, d: Fs::NONE, zs: {}, dz: Fs::NONE }}",
            fs(&[h.clone(), l.clone()]),
            fs(&[h]),
            fs(&[l])
        ));
    }
    let es_bytes = match es {
        "B" => Some(1),
        "H" => Some(2),
        "S" => Some(4),
        "D" => Some(8),
        "4B" | "2H" => Some(4),
        "2B" => Some(2),
        _ => None,
    };
    let r = match names.as_slice() {
        ["imm5"] => {
            cx.hand("index imm5");
            format!("Idx::Imm5({})", fs(&fields))
        }
        ["imm5", "imm4"] => {
            cx.hand("index imm4 by imm5");
            format!(
                "Idx::Imm4({}, {})",
                fs(&[fields[1].clone()]),
                fs(&[fields[0].clone()])
            )
        }
        ["Q", "S", "size"] | ["Q", "S"] | ["Q"] => {
            let shr = match (names.len(), es_bytes) {
                (3, Some(1)) => 0,
                (3, Some(2)) => 1,
                _ => 0,
            };
            cx.hand("index ldst single");
            format!("Idx::Bits({}, {shr})", fs(&fields))
        }
        ["size", ..] | ["sz", ..] => {
            // By element size: H → H:L:M, S → H:L, D → H (sz:L:H: S → H:L, D → H
            // with L zero).
            let (h, l, mm) = (get("H"), get("L"), get("M"));
            let (Some(h), Some(l)) = (h, l) else {
                return Err(format!("index {idx} fields"));
            };
            let hlm = mm.map_or("Fs::NONE".to_string(), |m| fs(&[h.clone(), l.clone(), m]));
            let dz = if names[0] == "sz" {
                fs(std::slice::from_ref(&l))
            } else {
                "Fs::NONE".into()
            };
            cx.hand("index by element size");
            format!(
                "Idx::ByEs {{ b: Fs::NONE, h: {hlm}, s: {}, d: {}, zs: Fs::NONE, dz: {dz} }}",
                fs(&[h.clone(), l.clone()]),
                fs(&[h])
            )
        }
        ["H", "L", "M", "Rm"] => {
            // FP8 by element: index H:L:M:Rm<3>, register Rm<2:0>.
            let rm = get("Rm").ok_or("index Rm")?;
            let rm3 = Field {
                name: "Rm3".into(),
                lsb: rm.lsb + 3,
                width: 1,
            };
            cx.hand("index H:L:M:Rm<3>");
            format!(
                "Idx::Bits({}, 0)",
                fs(&[get("H").unwrap(), get("L").unwrap(), get("M").unwrap(), rm3])
            )
        }
        _ => {
            // The variable bits of the fields (LUTI4 `len` = `x1`: index len<1>).
            let v: Vec<Field> = fields
                .iter()
                .map(|f| cx.variable_part(f).unwrap_or_else(|| f.clone()))
                .collect();
            cx.hand("index fixed fields");
            format!("Idx::Bits({}, 0)", fs(&v))
        }
    };
    Ok(r)
}

fn list(cx: &mut Cx<'_>, g: &str) -> R<String> {
    // `⟪ <Vt>.<T>, <Vt2>.<T> ⟫` or `⟪ <Vt>.B ⟫[<index>]` or `⟪ <Vn>.16B, <Vn+1>.16B ⟫`.
    let (body, idx) = match g.rsplit_once('⟫') {
        Some((b, rest)) => (b.trim_start_matches('⟪').trim(), rest),
        None => return Err(format!("list {g}")),
    };
    let elems: Vec<&str> = body.split(", ").collect();
    let n = elems.len() as u8;
    let first = elems[0];
    let c = m("%.%", first)
        .or_else(|| m("%.%w", first))
        .ok_or_else(|| format!("list element {first}"))?;
    let a = cx.arr(&c[1])?;
    let f = cx.field(&c[0])?;
    let i = if idx.is_empty() {
        "None".to_string()
    } else {
        let ic = m("[%]", idx).ok_or_else(|| format!("list index {idx}"))?;
        format!("Some({})", index(cx, &ic[0], &c[1])?)
    };
    cx.hand("register list");
    Ok(format!("Op::List({f}, {n}, {a}, {i})"))
}

fn named(cx: &mut Cx<'_>, name: &str, imm: &str) -> R<String> {
    let f = cx.fields(imm)?;
    match name {
        "<prfop>" | "<rprfop>" => {
            // RPRFM: option<2>:option<0>:S:Rt<2:0>.
            let f = if name == "<rprfop>" {
                vec![
                    Field {
                        name: "option2".into(),
                        lsb: 15,
                        width: 1,
                    },
                    Field {
                        name: "option0S".into(),
                        lsb: 12,
                        width: 2,
                    },
                    Field {
                        name: "Rt20".into(),
                        lsb: 0,
                        width: 3,
                    },
                ]
            } else {
                f
            };
            let mut e = Vec::new();
            let mut names = Vec::new();
            if name == "<prfop>" {
                // type:target:policy (Rt<4:3>, Rt<2:1>, Rt<0>).
                let types = ["PLD", "PLI", "PST"];
                let targets = ["L1", "L2", "L3", "SLC"];
                for (ti, t) in types.iter().enumerate() {
                    for (gi, g) in targets.iter().enumerate() {
                        for (pi, p) in ["KEEP", "STRM"].iter().enumerate() {
                            let v = (ti as u32) << 3 | (gi as u32) << 1 | pi as u32;
                            e.push((v, 0x1f, names.len() as u8));
                            names.push(format!("{t}{g}{p}"));
                        }
                    }
                }
            } else {
                // RPRFM: option<1>:option<0>... only four named operations.
                for (v, n) in [
                    (0u32, "PLDKEEP"),
                    (1, "PSTKEEP"),
                    (4, "PLDSTRM"),
                    (5, "PSTSTRM"),
                ] {
                    e.push((v, 0x3f, names.len() as u8));
                    names.push(n.to_string());
                }
            }
            let nr: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let t = cx.hand_table("prefetch operations", &f, &e, &nr);
            Ok(format!("Op::Name({t}, NameKind::Prefetch)"))
        }
        "<option>" => {
            // Barrier options by CRm (DMB/DSB), or SY only (ISB).
            let opts: &[(u32, &str)] = if cx.mnemonic == "ISB" {
                &[(15, "SY")]
            } else if cx.name.contains("nXS") || f.len() == 1 && f[0].name == "imm2" {
                &[(0, "OSH"), (1, "NSH"), (2, "ISH"), (3, "SY")]
            } else {
                &[
                    (1, "OSHLD"),
                    (2, "OSHST"),
                    (3, "OSH"),
                    (5, "NSHLD"),
                    (6, "NSHST"),
                    (7, "NSH"),
                    (9, "ISHLD"),
                    (10, "ISHST"),
                    (11, "ISH"),
                    (13, "LD"),
                    (14, "ST"),
                    (15, "SY"),
                ]
            };
            let e: Vec<(u32, u32, u8)> = opts
                .iter()
                .enumerate()
                .map(|(i, (v, _))| (*v, (1 << f[0].width) - 1, i as u8))
                .collect();
            let names: Vec<&str> = opts.iter().map(|(_, n)| *n).collect();
            let t = cx.hand_table("barrier options", &f, &e, &names);
            Ok(format!("Op::Name({t}, NameKind::Barrier)"))
        }
        _ => Err(format!("named operand ({name}|#{imm})")),
    }
}

fn mem(cx: &mut Cx<'_>, g: &str, mode: &str) -> R<String> {
    let mk = |base: String, k: &str, off: String, mode: &str, wb: bool| {
        format!(
            "Op::Mem(Mem {{ base: {base}, base_k: Rk::{k}, off: {off}, mode: {mode}, post: Post::None, writeback: {wb}, arr: Arr::Fixed(Arrangement::None), mul_vl: false }})"
        )
    };
    let base_of = |cx: &mut Cx<'_>, d: &str| -> R<(String, &'static str)> {
        let (k, _) = reg_sym(d).ok_or_else(|| format!("memory base {d}"))?;
        Ok((cx.field(d)?, k))
    };
    if let Some(c) = m("[%]", g) {
        let (b, k) = base_of(cx, &c[0])?;
        return Ok(mk(b, k, "MemOff::None".into(), mode, false));
    }
    if let Some(c) = m("[%]!", g) {
        let (b, k) = base_of(cx, &c[0])?;
        cx.hand("memory writeback, no offset");
        return Ok(mk(b, k, "MemOff::None".into(), mode, true));
    }
    if let Some(c) = m("[%{, #0}]", g).or_else(|| m("[%{ , #0}]", g)) {
        let (b, k) = base_of(cx, &c[0])?;
        return Ok(mk(b, k, "MemOff::None".into(), mode, false));
    }
    if let Some(c) = m("[%{, #%}]", g) {
        let (b, k) = base_of(cx, &c[0])?;
        let off = mem_imm(cx, &c[1])?;
        return Ok(mk(b, k, format!("MemOff::Imm({off})"), mode, false));
    }
    if let Some(c) = m("[%, #%]!", g).or_else(|| m("[%{, #%}]!", g)) {
        let (b, k) = base_of(cx, &c[0])?;
        let off = mem_imm(cx, &c[1])?;
        return Ok(mk(
            b,
            k,
            format!("MemOff::Imm({off})"),
            "MemoryMode::PreIndex",
            false,
        ));
    }
    if let Some(c) = m("[%, #%w]!", g) {
        let (b, k) = base_of(cx, &c[0])?;
        let v: i64 = c[1].parse().map_err(|_| format!("pre-index #{}", c[1]))?;
        return Ok(mk(
            b,
            k,
            format!("MemOff::Imm(Imm::Const({v}))"),
            "MemoryMode::PreIndex",
            false,
        ));
    }
    if let Some(c) = m("[%, (%|%){, %{ %}}]", g).or_else(|| m("[%, (%|%), %{ %}]", g)) {
        // Register offset, W or X by option<0>.
        let (b, k) = base_of(cx, &c[0])?;
        let mf = cx.field(&c[2])?;
        let opt = cx.fields(&c[3])?;
        let bit0 = Field {
            name: "option0".into(),
            lsb: opt[0].lsb,
            width: 1,
        };
        let wx = cx.hand_table(
            "register offset W/X by option<0>",
            &[bit0],
            &[(0, 1, 1), (1, 1, 0)],
            &[],
        );
        let ext = cx.table(&c[3], |s| {
            EXTENDS.iter().position(|x| *x == s).map(|i| i as u8)
        })?;
        let amount = match cx.num_table(&c[4]) {
            Ok(t) => format!("Amount::Tab({t})"),
            Err(_) => {
                cx.hand("byte register offset LSL #0 on S");
                format!("Amount::OnBit({}, 0)", cx.field(&c[4])?)
            }
        };
        return Ok(mk(
            b,
            k,
            format!(
                "MemOff::Reg {{ m: {mf}, k: RegSel::Tab({wx}), ext: Some({ext}), amount: {amount} }}"
            ),
            mode,
            false,
        ));
    }
    if let Some(c) = m("[%, %{, LSL %}]", g) {
        let (b, k) = base_of(cx, &c[0])?;
        let mf = cx.field(&c[1])?;
        let amount = match cx.num_table(&c[2]) {
            Ok(t) => format!("Amount::Tab({t})"),
            Err(_) => {
                // Byte accesses: `LSL #0` present when S is set.
                cx.hand("byte register offset LSL #0 on S");
                format!("Amount::OnBit({}, 0)", cx.field(&c[2])?)
            }
        };
        return Ok(mk(
            b,
            k,
            format!(
                "MemOff::Reg {{ m: {mf}, k: RegSel::Fixed(Rk::X), ext: None, amount: {amount} }}"
            ),
            mode,
            false,
        ));
    }
    if let Some(c) = m("[%, %]", g) {
        let (b, k) = base_of(cx, &c[0])?;
        let (mk2, _) = reg_sym(&c[1]).ok_or("index register")?;
        let mf = cx.field(&c[1])?;
        return Ok(mk(
            b,
            k,
            format!(
                "MemOff::Reg {{ m: {mf}, k: RegSel::Fixed(Rk::{mk2}), ext: None, amount: Amount::None }}"
            ),
            mode,
            false,
        ));
    }
    if let Some(c) = m("[%, %, LSL #%w]", g) {
        let (b, k) = base_of(cx, &c[0])?;
        let mf = cx.field(&c[1])?;
        let a: u8 = c[2].parse().map_err(|_| "lsl amount")?;
        return Ok(mk(
            b,
            k,
            format!(
                "MemOff::Reg {{ m: {mf}, k: RegSel::Fixed(Rk::X), ext: None, amount: Amount::Fixed({a}) }}"
            ),
            mode,
            false,
        ));
    }
    Err(format!("memory shape {g}"))
}

/// A memory offset immediate, scaled by the access size where the spec scales it.
fn mem_imm(cx: &mut Cx<'_>, d: &str) -> R<String> {
    let fields = cx.fields(d)?;
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let f = fs(&fields);
    // The transfer register is the first register symbol of the template.
    let reg = cx.order.iter().find(|k| reg_sym(k).is_some()).cloned();
    if cx.iclass == "ldsttags" && names == ["imm9"] {
        cx.hand("ldsttags offset*16");
        return Ok(format!("Imm::S({f}, 4)"));
    }
    Ok(match (d, names.as_slice()) {
        ("<pimm>", _) => {
            let s = if cx.mnemonic == "PRFM" {
                3
            } else {
                let reg = reg.ok_or("pimm without a transfer register")?;
                cx.access_scale(&reg)?
            };
            cx.hand("pimm scaled by access size");
            format!("Imm::U({f}, {s})")
        }
        ("<imm>", ["imm7"]) | ("<imm>", ["simm7"]) => {
            let reg = reg.ok_or("pair offset without a transfer register")?;
            let s = if cx.mnemonic == "STGP" {
                4
            } else {
                cx.access_scale(&reg)?
            };
            cx.hand("pair offset scaled");
            format!("Imm::S({f}, {s})")
        }
        ("<simm>", ["S", "imm9"]) => {
            cx.hand("LDRAA offset *8");
            format!("Imm::S({f}, 3)")
        }
        ("<imm>", ["imm9"]) if cx.iclass == "ldsttags" => {
            cx.hand("ldsttags offset*16");
            format!("Imm::S({f}, 4)")
        }
        ("<simm>", _) | ("<imm>", ["imm9"]) => {
            cx.hand("unscaled signed offset");
            format!("Imm::S({f}, 0)")
        }
        ("<imm>", ["imm12"]) => {
            let reg = reg.ok_or("imm12 without a transfer register")?;
            let s = cx.access_scale(&reg)?;
            cx.hand("imm12 scaled");
            format!("Imm::U({f}, {s})")
        }
        _ => return Err(format!("memory offset {d} in {}", names.join(":"))),
    })
}

/// `[<Xn|SP>], #<simm>` and friends.
fn post_index(cx: &mut Cx<'_>, g: &str, next: &str) -> R<Option<String>> {
    let Some(c) = m("[%]", g) else {
        return Ok(None);
    };
    let (k, _) = reg_sym(&c[0]).ok_or("post-index base")?;
    let base = cx.field(&c[0])?;
    let post = if let Some(p) = m("#%", next) {
        let i = if cx.is_table_sym(&p[0]) {
            let t = cx.num_table(&p[0])?;
            format!("Imm::Tab({t})")
        } else {
            mem_imm(cx, &p[0])?
        };
        format!("Post::Imm({i})")
    } else if let Some(p) = m("#%w", next) {
        let v: i64 = p[0].parse().map_err(|_| format!("post-index #{}", p[0]))?;
        format!("Post::Imm(Imm::Const({v}))")
    } else if let Some(p) = m("%", next) {
        if cx.is_table_sym(&p[0]) {
            let t = cx.num_table(&p[0])?;
            format!("Post::Imm(Imm::Tab({t}))")
        } else if reg_sym(&p[0]).is_some() {
            format!("Post::Reg({})", cx.field(&p[0])?)
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };
    cx.hand("post-index");
    Ok(Some(format!(
        "Op::Mem(Mem {{ base: {base}, base_k: Rk::{k}, off: MemOff::None, mode: MemoryMode::PostIndex, post: {post}, writeback: false, arr: Arr::Fixed(Arrangement::None), mul_vl: false }})"
    )))
}

#[allow(dead_code)]
fn unused() -> (&'static [&'static str; 13], &'static [&'static str; 18]) {
    (&RKS, &ARRS)
}

/// Alias symbols whose value is a function of the encoding's fields.
fn alias_imm(cx: &mut Cx<'_>, d: &str) -> R<Option<String>> {
    let size = if cx.name.contains("_32") { 32 } else { 64 };
    let mn = cx.mnemonic.clone();
    if cx.iclass == "pstate" && d == "<imm>" {
        let f = cx.fields(d)?;
        if let Some(crm) = f.iter().find(|x| x.name == "CRm") {
            cx.hand("MSR (immediate) value in CRm");
            return Ok(Some(format!(
                "Imm::U({}, 0)",
                fs(std::slice::from_ref(crm))
            )));
        }
    }
    if !cx.alias {
        return Ok(None);
    }
    let fields = cx.fields(d).unwrap_or_default();
    let get = |n: &str| fields.iter().find(|f| f.name == n).cloned();
    let r = match (mn.as_str(), d) {
        ("LSL", "<shift>") => {
            // LSL #n is UBFM #(-n MOD size), #(size - 1 - n).
            let immr = get("immr").ok_or("alias LSL without immr")?;
            let imms = cx_field(cx, "imms").ok_or("alias LSL without imms")?;
            cx.hand("alias LSL shift (immr and imms)");
            format!(
                "Imm::LslShift {{ immr: {}, imms: {}, size: {size} }}",
                fs(&[immr]),
                fs(&[imms])
            )
        }
        ("BFI" | "SBFIZ" | "UBFIZ" | "BFC", "<lsb>") => {
            let immr = get("immr").ok_or("alias lsb without immr")?;
            cx.hand("alias -immr MOD datasize");
            format!("Imm::NegMod({}, {size})", fs(&[immr]))
        }
        ("BFI" | "SBFIZ" | "UBFIZ" | "BFC", "<width>") => {
            let imms = get("imms").ok_or("alias width without imms")?;
            cx.hand("alias width imms+1");
            format!("Imm::Add({}, 1)", fs(&[imms]))
        }
        ("BFXIL" | "SBFX" | "UBFX", "<width>") => {
            let imms = get("imms").ok_or("alias width without imms")?;
            let immr = get("immr").ok_or("alias width without immr")?;
            cx.hand("alias width imms+1-immr");
            format!("Imm::Width({}, {})", fs(&[imms]), fs(&[immr]))
        }
        ("MOV", "<imm>") if get("imm16").is_some() => {
            let inv = cx.name.starts_with("MOVN");
            cx.hand("alias MOV wide immediate");
            format!(
                "Imm::WideMov {{ imm16: {}, hw: {}, inv: {inv}, size: {size} }}",
                fs(&[get("imm16").unwrap()]),
                fs(&[get("hw").ok_or("MOV without hw")?])
            )
        }
        (_, "<immp1>") => {
            cx.hand("alias immediate +1");
            format!("Imm::Add({}, 1)", fs(&fields))
        }
        (_, "<imms1>") => {
            cx.hand("alias immediate -1");
            format!("Imm::Add({}, -1)", fs(&fields))
        }
        _ => return Ok(None),
    };
    Ok(Some(r))
}

/// `immh` (and `immh:Q`) arrangement tables whose rule names carry no values
/// (`Ta_8H`, `Tb_8B`): the highest set bit of `immh` gives the element size
/// (`0001` B, `001x` H, `01xx` S, `1xxx` D), from the source elements for `<Ta>`.
fn immh_bits(d: &str, fields: &[&str], text: &str) -> Option<(u32, u32)> {
    let (es, q) = match text {
        "8B" => (0, 0),
        "16B" => (0, 1),
        "4H" => (1, 0),
        "8H" => (1, 1),
        "2S" => (2, 0),
        "4S" => (2, 1),
        "1D" => (3, 0),
        "2D" => (3, 1),
        _ => return None,
    };
    // `<Ta>` is the double-width destination: its elements are one size up.
    let es = if d == "<Ta>" { es - 1 } else { es };
    let (v, m) = match es {
        0 => (0b0001, 0b1111),
        1 => (0b0010, 0b1110),
        2 => (0b0100, 0b1100),
        3 => (0b1000, 0b1000),
        _ => return None,
    };
    match fields {
        ["immh"] => Some((v, m)),
        ["immh", "Q"] => Some((v << 1 | q, m << 1 | 1)),
        _ => None,
    }
}

/// A field of the encoding by name (for alias rules that set fields no alias symbol
/// names).
fn cx_field(cx: &Cx<'_>, name: &str) -> Option<Field> {
    cx.leaf_fields.iter().find(|f| f.name == name).cloned()
}

/// With `DECODERANT_A64_GEN_DUMP=<file>`, appends each template mapped and its symbols
/// (fields, rule names and texts) to the file: the raw material for new hand rules.
fn dump(cx: &Cx<'_>, t: &[T]) {
    use std::io::Write as _;
    let Ok(p) = std::env::var("DECODERANT_A64_GEN_DUMP") else {
        return;
    };
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
    else {
        return;
    };
    let kind = if cx.alias { "alias" } else { "encoding" };
    let _ = writeln!(f, "{} {kind} | {}", cx.name, shape(t));
    for k in &cx.order {
        let s = &cx.syms[k];
        let fl: Vec<String> = s
            .fields
            .iter()
            .map(|f| format!("{}:{}+{}", f.name, f.lsb, f.width))
            .collect();
        let al: Vec<String> = s
            .alts
            .iter()
            .map(|a| format!("{}={}", a.rule, a.text))
            .collect();
        let _ = writeln!(f, "    {k} [{}] {}", fl.join(" "), al.join(" ; "));
    }
}
