//! Loading the pinned AARCHMRS release: REVISION parsing, hash and licence checks.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The `REVISION` pin.
pub struct Revision {
    pub release: String,
    pub files: BTreeMap<String, String>,
}

impl Revision {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut release = None;
        let mut files = BTreeMap::new();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts.as_slice() {
                [] => {}
                [c, ..] if c.starts_with('#') => {}
                ["release", name] => release = Some(name.to_string()),
                ["tarball", _, _] => {}
                ["file", name, hash] => {
                    files.insert(name.to_string(), hash.to_string());
                }
                _ => bail!("malformed REVISION line: {line}"),
            }
        }
        Ok(Self {
            release: release.context("REVISION has no release line")?,
            files,
        })
    }
}

/// The three spec files, parsed.
pub struct Spec {
    pub release: String,
    pub instructions: Value,
    pub features: Value,
    pub registers: Value,
}

fn sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Checks that a JSON file's `_meta` block states the BSD 3-clause licence and the
/// pinned release. A licence change in a later release must stop generation.
fn check_meta(meta: &Value, file: &str, release: &str) -> Result<()> {
    let info = meta["license"]["info"].as_str().unwrap_or("");
    if !info.contains("BSD 3-clause") {
        bail!("{file}: _meta licence is {info:?}, not BSD 3-clause; see docs/arch/aarch64.md");
    }
    let rel = meta["version"]["ref"].as_str().unwrap_or("");
    let short = rel.trim_end_matches("_rel");
    if !release.ends_with(short) {
        bail!("{file}: _meta version ref {rel:?} does not match release {release}");
    }
    Ok(())
}

fn load_json(dir: &Path, name: &str, rev: &Revision) -> Result<Value> {
    let path: PathBuf = dir.join(name);
    let want = rev
        .files
        .get(name)
        .with_context(|| format!("REVISION pins no hash for {name}"))?;
    let have = sha256(&path)?;
    if &have != want {
        bail!("{name}: SHA-256 {have}, REVISION pins {want}");
    }
    let file = fs::File::open(&path)?;
    let v: Value = serde_json::from_reader(std::io::BufReader::new(file))
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(v)
}

impl Spec {
    pub fn load(dir: &Path, rev: &Revision) -> Result<Self> {
        let notice = dir.join("docs/notice.html");
        let want = rev
            .files
            .get("docs/notice.html")
            .context("REVISION pins no hash for docs/notice.html")?;
        if &sha256(&notice)? != want {
            bail!("docs/notice.html differs from the pinned one: re-check the licence");
        }
        let instructions = load_json(dir, "Instructions.json", rev)?;
        check_meta(&instructions["_meta"], "Instructions.json", &rev.release)?;
        let features = load_json(dir, "Features.json", rev)?;
        check_meta(&features["_meta"], "Features.json", &rev.release)?;
        let registers = load_json(dir, "Registers.json", rev)?;
        let first = registers
            .as_array()
            .and_then(|a| a.first())
            .context("Registers.json is not a non-empty array")?;
        check_meta(&first["_meta"], "Registers.json", &rev.release)?;
        Ok(Self {
            release: rev.release.clone(),
            instructions,
            features,
            registers,
        })
    }
}
