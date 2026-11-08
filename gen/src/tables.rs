//! Builds every generated table and the report.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::emit;
use crate::expr::{Arena, Scope};
use crate::info;
use crate::model::{Field, Kind, Model, Pattern};
use crate::names;
use crate::operands;
use crate::spec::Spec;
use crate::sysreg;
use crate::template::{self, Flattener, T};

struct NodeScope<'m, 'a> {
    model: &'m Model<'a>,
    node: usize,
}

impl Scope for NodeScope<'_, '_> {
    fn field(&self, name: &str) -> Option<Field> {
        self.model.field(self.node, name)
    }
}

/// The leading mnemonic of a template: `ADD`, `B` for `B.<cond>`, `BFMLAL` for
/// `BFMLAL<bt>`.
pub fn mnemonic_of(t: &[T]) -> Result<String> {
    let Some(T::Text(s)) = t.first() else {
        bail!(
            "template does not start with a mnemonic: {}",
            template::render(t)
        );
    };
    let m: String = s
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if m.is_empty() {
        bail!("empty mnemonic in {}", template::render(t));
    }
    Ok(m)
}

fn cond_list(ids: &[u32]) -> String {
    let v: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    format!("&[{}]", v.join(", "))
}

pub struct Leaf {
    pub name: String,
    pub template: Vec<T>,
    pub mnemonic: String,
}

pub fn generate(
    spec: &Spec,
    model: &Model<'_>,
    out_dir: &Path,
    previous: &str,
) -> Result<(String, String)> {
    let fl = Flattener {
        rules: model.rules,
        stack: Default::default(),
        signed: Default::default(),
    };
    let mut arena = Arena::default();
    // Expression 0 is `true`, so an empty slot never reads garbage.
    arena.intern(crate::expr::E::Bool(true));

    // Node conditions.
    let mut node_cond: Vec<Option<u32>> = vec![None; model.nodes.len()];
    for (i, n) in model.nodes.iter().enumerate() {
        if let Some(c) = n.cond {
            let scope = NodeScope { model, node: i };
            let id = arena
                .compile(c, &scope)
                .with_context(|| format!("condition of {}", n.name))?;
            node_cond[i] = Some(id);
        }
    }

    // Decode order: children by decreasing specificity, then spec order.
    let full: Vec<Pattern> = (0..model.nodes.len())
        .map(|i| model.full_pattern(i))
        .collect();
    let mut sorted_children: Vec<Vec<usize>> = Vec::with_capacity(model.nodes.len());
    for n in &model.nodes {
        let mut c = n.children.clone();
        c.sort_by_key(|&x| std::cmp::Reverse(full[x].mask.count_ones()));
        sorted_children.push(c);
    }
    // Each leaf's position key along its path, for precedence.
    let mut key: Vec<Vec<usize>> = vec![Vec::new(); model.leaves.len()];
    {
        fn walk(
            n: usize,
            k: &mut Vec<usize>,
            model: &Model<'_>,
            sorted: &[Vec<usize>],
            key: &mut [Vec<usize>],
        ) {
            if let Kind::Leaf(l) = model.nodes[n].kind {
                key[l] = k.clone();
            }
            for (i, &c) in sorted[n].iter().enumerate() {
                k.push(i);
                walk(c, k, model, sorted, key);
                k.pop();
            }
        }
        walk(
            model.root,
            &mut Vec::new(),
            model,
            &sorted_children,
            &mut key,
        );
    }

    // Leaves.
    let mut leaves = Vec::new();
    for leaf in &model.leaves {
        let t = fl
            .flatten(&leaf.json["assembly"])
            .with_context(|| format!("template of {}", leaf.name))?;
        let mnemonic = mnemonic_of(&t).with_context(|| leaf.name.clone())?;
        leaves.push(Leaf {
            name: leaf.name.clone(),
            template: t,
            mnemonic,
        });
    }

    // Aliases.
    struct AliasOut {
        mnemonic: String,
        template: String,
        cond: Option<u32>,
        preferred: Option<u32>,
        ops: Option<String>,
    }
    let mut aliases: Vec<Vec<AliasOut>> = Vec::new();
    let mut unsupported_alias_conds = BTreeMap::<String, usize>::new();
    let mut mapping = operands::Report::default();
    // Assembler methods: (mnemonic, suffix) to the operand counts, each with whether
    // an encoding outside SVE/SME has it.
    let mut asm: BTreeMap<(String, String), BTreeMap<usize, bool>> = BTreeMap::new();
    for leaf in &model.leaves {
        let scope = NodeScope {
            model,
            node: leaf.node,
        };
        let mut v = Vec::new();
        for a in &leaf.aliases {
            let t = fl.flatten(&a.json["assembly"])?;
            let compile = |arena: &mut Arena, key: &str| -> Result<Option<u32>> {
                let c = &a.json[key];
                if c.is_null() || (c["_type"] == "AST.Bool" && c["value"] == true) {
                    Ok(None)
                } else {
                    arena.compile(c, &scope).map(Some)
                }
            };
            let cond = compile(&mut arena, "condition");
            let preferred = compile(&mut arena, "preferred");
            match (cond, preferred) {
                (Ok(cond), Ok(preferred)) => {
                    let am = mnemonic_of(&t)?;
                    let env = operands::Env {
                        arena: &arena,
                        conds: path_conds(model, leaf.node, &node_cond),
                    };
                    let ops = operands::map_alias(model, leaf, a.json, &t, &am, &env, &mut mapping);
                    if ops.is_some() {
                        asm_add(&mut asm, &am, &mapping, !sve_sme_leaf(model, leaf.node));
                    }
                    v.push(AliasOut {
                        mnemonic: am,
                        template: template::render(&t),
                        cond,
                        preferred,
                        ops,
                    });
                }
                (Err(e), _) | (_, Err(e)) => {
                    *unsupported_alias_conds.entry(e.to_string()).or_default() += 1;
                }
            }
        }
        // Aliases with their own condition (LSL, LSR, ASR, SXTW ...) before those
        // with only a preference (UBFIZ, UBFX): where both prefer themselves, the
        // conditioned one is the more specific (Arm ARM alias precedence).
        v.sort_by_key(|a: &AliasOut| a.cond.is_none());
        aliases.push(v);
    }

    // Features: Features.json order, then any others the conditions name.
    let mut feature_names: Vec<String> = Vec::new();
    for p in spec.features["parameters"].as_array().into_iter().flatten() {
        if let Some(n) = p["name"].as_str()
            && n.starts_with("FEAT_")
            && !feature_names.iter().any(|f| f == n)
        {
            feature_names.push(n.to_string());
        }
    }
    let mut extra = BTreeSet::new();
    for e in &arena.nodes {
        if let crate::expr::E::Feature(f) = e
            && !feature_names.contains(f)
        {
            extra.insert(f.clone());
        }
    }
    feature_names.extend(extra);
    let fvar = emit::feature_variants(&feature_names);

    // Mnemonics: every encoding's and alias's, sorted.
    let mut mnemonics: BTreeSet<String> = leaves.iter().map(|l| l.mnemonic.clone()).collect();
    for v in &aliases {
        for a in v {
            mnemonics.insert(a.mnemonic.clone());
        }
    }

    // Operands.
    let mut op_text: Vec<Option<String>> = Vec::new();
    for (i, leaf) in model.leaves.iter().enumerate() {
        let env = operands::Env {
            arena: &arena,
            conds: path_conds(model, leaf.node, &node_cond),
        };
        let o = operands::map(
            model,
            leaf,
            &leaves[i].template,
            &leaves[i].mnemonic,
            &env,
            &mut mapping,
        );
        if o.is_some() {
            asm_add(
                &mut asm,
                &leaves[i].mnemonic,
                &mapping,
                !sve_sme_leaf(model, leaf.node),
            );
        }
        op_text.push(o);
    }

    // Precedence.
    let leaf_full: Vec<Pattern> = model.leaves.iter().map(|l| full[l.node]).collect();
    let mut precedence: Vec<Vec<usize>> = vec![Vec::new(); model.leaves.len()];
    for c in 0..model.leaves.len() {
        for l in 0..model.leaves.len() {
            if l != c && leaf_full[l].intersects(&leaf_full[c]) && key[l] < key[c] {
                precedence[c].push(l);
            }
        }
    }

    // Decode dispatch: the leaves in decode order (depth first, children most
    // specific first: the order of their position keys), each with the conjunction
    // of its path conditions.
    let mut order: Vec<usize> = (0..model.leaves.len()).collect();
    order.sort_by(|&a, &b| key[a].cmp(&key[b]));
    let leaf_cond: Vec<Option<u32>> = model
        .leaves
        .iter()
        .map(|leaf| {
            let mut all: Option<u32> = None;
            for &n in model.path(leaf.node).iter().rev() {
                if let Some(c) = node_cond[n] {
                    all = Some(match all {
                        None => c,
                        Some(a) => arena.intern(crate::expr::E::Bin("&&".into(), a, c)),
                    });
                }
            }
            all
        })
        .collect();
    let pats: Vec<(u32, u32)> = order
        .iter()
        .map(|&l| (leaf_full[l].mask, leaf_full[l].value))
        .collect();
    let dispatch = crate::dispatch::build(&pats);

    // --- code.rs: Code, Mnemonic, Feature.
    let mut code = emit::header(&spec.release);
    let mut cv: Vec<(String, String, String)> =
        vec![("Invalid".into(), "INVALID".into(), String::new())];
    let mut seen = BTreeSet::new();
    for l in &leaves {
        let v = names::code_variant(&l.name);
        if !seen.insert(v.clone()) {
            bail!("two encodings map to Code::{v}");
        }
        cv.push((
            v,
            l.name.clone(),
            format!("`{}`", template::render(&l.template)),
        ));
    }
    emit::simple_enum(
        &mut code,
        "/// An A64 encoding: one variant per AARCHMRS encoding, named after its encoding\n\
         /// name. Discriminants are not stable across spec releases: persist the name.\n\
         #[non_exhaustive]",
        "Code",
        &cv,
        Some("Not a valid A64 encoding (unallocated, or a feature is disabled)."),
    );
    let mut mv: Vec<(String, String, String)> =
        vec![("INVALID".into(), "INVALID".into(), String::new())];
    for m in &mnemonics {
        mv.push((names::mnemonic_variant(m), m.clone(), String::new()));
    }
    // `INVALID` is not a camel-case name; keep the variant `Invalid`.
    mv[0].0 = "Invalid".into();
    emit::simple_enum(
        &mut code,
        "/// An architectural mnemonic, including the preferred aliases.\n#[non_exhaustive]",
        "Mnemonic",
        &mv,
        Some("No mnemonic (`Code::Invalid`)."),
    );
    let fv: Vec<(String, String, String)> = feature_names
        .iter()
        .map(|n| (fvar[n].clone(), n.clone(), String::new()))
        .collect();
    emit::simple_enum(
        &mut code,
        "/// An architecture feature (`FEAT_*`), as named in `Features.json`.\n#[non_exhaustive]",
        "Feature",
        &fv,
        None,
    );
    emit::write(&out_dir.join("code.rs"), &code)?;

    let mut reg = emit::header(&spec.release);
    reg.push_str(&emit::registers());
    emit::write(&out_dir.join("register.rs"), &reg)?;

    // --- tables.rs.
    let mut t = emit::header(&spec.release);
    t.push_str(
        "#![allow(clippy::all)]\n\n\
         use crate::bits::{F, Fs};\n\
         #[cfg(test)]\n\
         use crate::cond::{BinOp, Expr, Func};\n\
         #[allow(unused_imports)]\n\
         use crate::desc::*;\n\
         use crate::generated::{Code, Feature, Mnemonic, Register};\n\
         use crate::operand::{Arrangement, MemoryMode};\n\
         use crate::tables::{AliasData, Cand, EncodingData, FieldInfo, NO_COND};\n\
         #[cfg(test)]\n\
         use crate::tables::Node;\n\n",
    );
    t.push_str(&emit::exprs(&arena, &fvar));
    let mut conds = emit::header(&spec.release);
    conds.push_str(&emit::cond_fns(&arena, &fvar));
    emit::write(&out_dir.join("conds.rs"), &conds)?;

    let mut children: Vec<usize> = Vec::new();
    let mut nodes = String::new();
    for (i, n) in model.nodes.iter().enumerate() {
        let p = full[i];
        let first = children.len();
        children.extend(&sorted_children[i]);
        let code = match n.kind {
            Kind::Leaf(l) => format!("Code::{}", names::code_variant(&model.leaves[l].name)),
            Kind::Group => "Code::Invalid".into(),
        };
        let _ = writeln!(
            nodes,
            "    Node {{ mask: {:#010x}, value: {:#010x}, cond: {}, first: {first}, count: {}, code: {code} }},",
            p.mask,
            p.value,
            node_cond[i].map_or("NO_COND".into(), |c| c.to_string()),
            sorted_children[i].len()
        );
    }
    let _ = writeln!(
        t,
        "// The decode tree: the reference the dispatch is tested against.\n\
         #[cfg(test)]\n\
         pub(crate) const ROOT: usize = {};\n\n\
         #[cfg(test)]\n\
         pub(crate) static NODES: [Node; {}] = [\n{nodes}];\n",
        model.root,
        model.nodes.len()
    );
    let _ = writeln!(
        t,
        "#[cfg(test)]\npub(crate) static CHILDREN: [u16; {}] = [",
        children.len()
    );
    for chunk in children.chunks(16) {
        let line: Vec<String> = chunk.iter().map(|c| c.to_string()).collect();
        let _ = writeln!(t, "    {},", line.join(", "));
    }
    let _ = writeln!(t, "];\n");

    // The dispatch: slots, then the candidate lists.
    let _ = writeln!(
        t,
        "pub(crate) const DISPATCH_ROOT: u32 = {:#010x};\n\n\
         pub(crate) static DISPATCH: [u32; {}] = [",
        dispatch.root,
        dispatch.slots.len()
    );
    for chunk in dispatch.slots.chunks(8) {
        let line: Vec<String> = chunk.iter().map(|s| format!("{s:#010x}")).collect();
        let _ = writeln!(t, "    {},", line.join(", "));
    }
    let _ = writeln!(t, "];\n");
    let _ = writeln!(
        t,
        "pub(crate) static CANDIDATES: [Cand; {}] = [",
        dispatch.lists.len()
    );
    for &i in &dispatch.lists {
        let l = order[i];
        let _ = writeln!(
            t,
            "    Cand {{ mask: {:#010x}, value: {:#010x}, cond: {}, code: Code::{} }},",
            leaf_full[l].mask,
            leaf_full[l].value,
            leaf_cond[l].map_or("NO_COND".into(), |c| c.to_string()),
            names::code_variant(&model.leaves[l].name)
        );
    }
    let _ = writeln!(t, "];\n");

    for (i, b) in mapping.tabs.bodies.iter().enumerate() {
        let _ = writeln!(t, "static TAB_{i}: Tab = {b};");
    }
    t.push('\n');
    let _ = writeln!(
        t,
        "pub(crate) static ENCODINGS: [EncodingData; {}] = [",
        model.leaves.len()
    );
    let templates: Vec<String> = leaves
        .iter()
        .map(|l| template::render(&l.template))
        .collect();
    let mut info_rows = Vec::with_capacity(model.leaves.len());
    for (i, leaf) in model.leaves.iter().enumerate() {
        let path = model.path(leaf.node);
        let top = &model.nodes[path[path.len() - 2]].name;
        info_rows.push(info::Row {
            name: &leaf.name,
            iclass: &leaf.iclass,
            group: top,
            mnemonic: &leaves[i].mnemonic,
            template: &templates[i],
        });
        let mut conds = Vec::new();
        let mut fconds = Vec::new();
        let mut feats = Vec::new();
        for &n in path.iter().rev() {
            if let Some(c) = node_cond[n] {
                conds.push(c);
                if arena.is_feature_only(c) {
                    fconds.push(c);
                }
                arena.features(c, &mut feats);
            }
        }
        let p = leaf_full[i];
        let fields: Vec<String> = model.nodes[leaf.node]
            .fields
            .iter()
            .map(|f| {
                format!(
                    "FieldInfo {{ name: {:?}, lsb: {}, width: {} }}",
                    f.name, f.lsb, f.width
                )
            })
            .collect();
        let prec: Vec<String> = precedence[i]
            .iter()
            .map(|&l| format!("Code::{}", names::code_variant(&model.leaves[l].name)))
            .collect();
        let fl: Vec<String> = feats
            .iter()
            .map(|f| format!("Feature::{}", fvar[f]))
            .collect();
        let ops = op_text[i]
            .as_ref()
            .map_or("None".to_string(), |s| format!("Some(&[{s}])"));
        let mut al = String::new();
        for a in &aliases[i] {
            let _ = write!(
                al,
                "AliasData {{ mnemonic: Mnemonic::{}, template: {:?}, cond: {}, preferred: {}, fixed: ({:#010x}, {:#010x}), copies: &[{}], ops: {} }}, ",
                names::mnemonic_variant(&a.mnemonic),
                a.template,
                a.cond.map_or("NO_COND".into(), |c| c.to_string()),
                a.preferred.map_or("NO_COND".into(), |c| c.to_string()),
                a.cond.map_or(0, |c| arena.fixed_bits(c).0),
                a.cond.map_or(0, |c| arena.fixed_bits(c).1),
                {
                    let mut v = Vec::new();
                    if let Some(c) = a.cond {
                        arena.copies(c, &mut v);
                    }
                    v.iter()
                        .map(|((l1, w1), (l2, w2))| {
                            format!("(F::new({l1}, {w1}), F::new({l2}, {w2}))")
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                a.ops
                    .as_ref()
                    .map_or("None".to_string(), |s| format!("Some(&[{s}])")),
            );
        }
        let _ = writeln!(
            t,
            "    EncodingData {{\n        name: {:?}, iclass: {:?}, group: {:?}, mnemonic: Mnemonic::{},\n        \
             mask: {:#010x}, value: {:#010x}, sb_mask: {:#010x}, sb_value: {:#010x},\n        \
             conds: {}, feature_conds: {}, features: &[{}],\n        fields: &[{}],\n        \
             template: {:?},\n        precedence: &[{}],\n        ops: {ops},\n        aliases: &[{al}],\n    }},",
            leaf.name,
            leaf.iclass,
            top,
            names::mnemonic_variant(&leaves[i].mnemonic),
            p.mask,
            p.value,
            p.sb_mask,
            p.sb_value,
            cond_list(&conds),
            cond_list(&fconds),
            fl.join(", "),
            fields.join(", "),
            templates[i],
            prec.join(", "),
        );
    }
    let _ = writeln!(t, "];");
    emit::write(&out_dir.join("tables.rs"), &t)?;

    // --- info.rs.
    let mut inf = emit::header(&spec.release);
    inf.push_str(&info::generate(&info_rows)?);
    emit::write(&out_dir.join("info.rs"), &inf)?;

    // --- mod.rs.
    let mut m = emit::header(&spec.release);
    m.push_str(emit::MOD_RS);
    emit::write(&out_dir.join("mod.rs"), &m)?;

    // --- code_asm.rs.
    let mut ca = emit::header(&spec.release);
    ca.push_str(&emit::code_asm(&asm));
    emit::write(&out_dir.join("code_asm.rs"), &ca)?;

    // --- sysreg.rs.
    let mut s = emit::header(&spec.release);
    let sys = sysreg::generate(&spec.registers, &fvar)?;
    s.push_str(&sys.code);
    emit::write(&out_dir.join("sysreg.rs"), &s)?;

    // --- Report.
    let mut r = String::new();
    let _ = writeln!(
        r,
        "# decoderant-a64-gen report\n\n@generated from {}. Regenerate with decoderant-a64-gen; \
         review the diff when the pin changes.\n",
        spec.release
    );
    let mut by_group: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (i, leaf) in model.leaves.iter().enumerate() {
        let path = model.path(leaf.node);
        let top = model.nodes[path[path.len() - 2]].name.clone();
        let e = by_group.entry(top).or_default();
        e.0 += 1;
        if op_text[i].is_some() {
            e.1 += 1;
        }
    }
    let _ = writeln!(r, "## Encodings\n");
    let _ = writeln!(
        r,
        "{} A64 encodings, {} decode-tree nodes, {} condition expressions, {} features, \
         {} mnemonics, {} system registers and operations.\n",
        model.leaves.len(),
        model.nodes.len(),
        arena.nodes.len(),
        feature_names.len(),
        mnemonics.len(),
        sys.count
    );
    let _ = writeln!(
        r,
        "Decode dispatch: {} slots, {} candidate entries; over all words, {:.2} tables \
         and {:.2} candidates on average, at most {} candidates.\n",
        dispatch.slots.len(),
        dispatch.lists.len(),
        dispatch.mean_depth,
        dispatch.mean_list,
        dispatch.max_list
    );
    let _ = writeln!(
        r,
        "| Group | Encodings | With operand support |\n|---|---|---|"
    );
    for (g, (n, s)) in &by_group {
        let _ = writeln!(r, "| {g} | {n} | {s} |");
    }
    let overlaps = precedence.iter().filter(|p| !p.is_empty()).count();
    let _ = writeln!(
        r,
        "\n{overlaps} encodings overlap a higher-priority encoding (listed in \
         `OpCodeInfo::precedence()`).\n"
    );
    let _ = writeln!(r, "{}", mapping.render());
    if !unsupported_alias_conds.is_empty() {
        let _ = writeln!(r, "## Alias conditions not compiled\n");
        for (e, n) in &unsupported_alias_conds {
            let _ = writeln!(r, "- {n} × {e}");
        }
        r.push('\n');
    }
    // The encoding list (`spec/encodings.txt`): one line per encoding, so a new
    // release shows up as a reviewable diff.
    let mut list = String::new();
    for (i, leaf) in model.leaves.iter().enumerate() {
        let p = leaf_full[i];
        let fields: Vec<String> = model.nodes[leaf.node]
            .fields
            .iter()
            .map(|f| format!("{}:{}+{}", f.name, f.lsb, f.width))
            .collect();
        let _ = writeln!(
            list,
            "{} {} {:08x}/{:08x} sb={:08x}/{:08x} [{}] {}",
            leaf.name,
            leaf.iclass,
            p.mask,
            p.value,
            p.sb_mask,
            p.sb_value,
            fields.join(" "),
            template::render(&leaves[i].template)
        );
    }
    change_report(previous, &list, &spec.release)?;
    Ok((r, list))
}

/// The conditions on the decode path of a node.
fn path_conds(model: &Model<'_>, node: usize, node_cond: &[Option<u32>]) -> Vec<u32> {
    model
        .path(node)
        .iter()
        .filter_map(|&n| node_cond[n])
        .collect()
}

/// Whether a node is in the SVE or SME groups.
fn sve_sme_leaf(model: &Model<'_>, node: usize) -> bool {
    let p = model.path(node);
    matches!(model.nodes[p[p.len() - 2]].name.as_str(), "sve" | "sme")
}

/// Records an assembler method for a mapped operand list (`base`: from an encoding
/// outside SVE/SME).
fn asm_add(
    asm: &mut BTreeMap<(String, String), BTreeMap<usize, bool>>,
    mnemonic: &str,
    rep: &operands::Report,
    base: bool,
) {
    let suffixes = if rep.last_cond {
        vec!["_cond".to_string()]
    } else if rep.last_suffixes.is_empty() {
        vec![String::new()]
    } else {
        rep.last_suffixes.clone()
    };
    for s in suffixes {
        *asm.entry((mnemonic.to_string(), s))
            .or_default()
            .entry(rep.last_arity)
            .or_default() |= base;
    }
}

/// Writes `spec/changes.md` when the encoding list changed: encodings added,
/// removed, renamed (same pattern and fields, new name) and changed (pattern, fields
/// or template). Not part of `--check`; review it with the regenerated tables.
fn change_report(previous: &str, list: &str, release: &str) -> Result<()> {
    if previous.is_empty() || previous == list {
        return Ok(());
    }
    let parse = |s: &str| -> BTreeMap<String, String> {
        s.lines()
            .filter_map(|l| {
                l.split_once(' ')
                    .map(|(n, rest)| (n.to_string(), rest.to_string()))
            })
            .collect()
    };
    let (old, new) = (parse(previous), parse(list));
    // The pattern and fields (`<mask/value> sb=<...> [<fields>`), without the iclass
    // and template, identify a rename.
    let shape = |rest: &str| {
        let tail = rest.split_once(' ').map_or("", |(_, t)| t);
        tail.split(']').next().unwrap_or("").to_string()
    };
    let mut removed: Vec<&String> = old.keys().filter(|k| !new.contains_key(*k)).collect();
    let mut added: Vec<&String> = new.keys().filter(|k| !old.contains_key(*k)).collect();
    let mut renamed = Vec::new();
    removed.retain(|r| {
        if let Some(pos) = added
            .iter()
            .position(|a| shape(&new[*a]) == shape(&old[*r]))
        {
            renamed.push(((*r).clone(), added.remove(pos).clone()));
            false
        } else {
            true
        }
    });
    let changed: Vec<&String> = new
        .keys()
        .filter(|k| old.get(*k).is_some_and(|o| o != &new[*k]))
        .collect();
    let mut s = format!("# Encoding changes for {release}\n\n");
    let mut section = |title: &str, items: Vec<String>| {
        if !items.is_empty() {
            let _ = writeln!(s, "## {title} ({})\n", items.len());
            for i in items {
                let _ = writeln!(s, "- {i}");
            }
            s.push('\n');
        }
    };
    section(
        "Added",
        added.iter().map(|a| format!("`{a}` {}", new[*a])).collect(),
    );
    section(
        "Removed",
        removed.iter().map(|r| format!("`{r}`")).collect(),
    );
    section(
        "Renamed",
        renamed
            .iter()
            .map(|(o, n)| format!("`{o}` → `{n}`"))
            .collect(),
    );
    section(
        "Changed",
        changed
            .iter()
            .map(|c| format!("`{c}`: {} → {}", old[*c], new[*c]))
            .collect(),
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/changes.md");
    if emit::CHECK.lock().expect("check lock").is_none() {
        std::fs::write(path, s)?;
    }
    Ok(())
}
