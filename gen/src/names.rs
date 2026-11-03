//! Rust identifiers for spec names.

/// `ADD_64_addsub_shift` → `Add_64_addsub_shift`; `ld1b_z_p_bi_u8` → `Ld1b_z_p_bi_u8`.
pub fn code_variant(enc: &str) -> String {
    let mut parts = enc.splitn(2, '_');
    let head = parts.next().unwrap_or("");
    let rest = parts.next();
    let mut s = capitalize(head);
    if let Some(r) = rest {
        s.push('_');
        s.push_str(r);
    }
    sanitize(&s)
}

/// `LDADDAL` → `Ldaddal`.
pub fn mnemonic_variant(m: &str) -> String {
    sanitize(&capitalize(m))
}

/// `FEAT_PAuth_LR` → `PAuth_LR`.
pub fn feature_variant(f: &str) -> String {
    let s = f.strip_prefix("FEAT_").unwrap_or(f);
    let s = sanitize(s);
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("F{s}")
    } else {
        s
    }
}

fn capitalize(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut c = lower.chars();
    match c.next() {
        Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
