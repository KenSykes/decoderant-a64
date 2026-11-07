//! The per-encoding classification tables for `info` and `instr_info`
//! (`src/generated/info.rs`).
//!
//! The spec's open-source release has no operational pseudocode, so how an
//! instruction accesses its operands is a hand-maintained classification by iclass
//! and mnemonic. It is applied here, once per encoding, as `iced-x86` generates its
//! instruction-info tables, so that the library compares no strings at run time.
//! What differs between instances of one encoding (a memory operand, the mnemonic
//! suffix `2`, merging predication, a condition) the library checks itself, from
//! the flags below. The library's differential test against `fARM64` checks the
//! result.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use anyhow::{Result, bail};

/// What the classification works from.
pub struct Row<'a> {
    pub name: &'a str,
    pub iclass: &'a str,
    /// The top-level decode group (`sve`, `sme`, `ldst`, ...).
    pub group: &'a str,
    pub mnemonic: &'a str,
    /// The rendered assembler template.
    pub template: &'a str,
}

/// The library's `instr_info::Role`: how the explicit operands are accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Dest,
    Accum,
    Read,
    Load,
    Store,
    LoadMerge,
    StoreExclusive,
    Atomic,
    CompareSwap,
    AtomicPair,
    SysRead,
}

/// The library's `instr_info` flags.
const FLAGS: &[&str] = &[
    "SVE",
    "SVE_ADR",
    "ACCUM_2",
    "ZA_ACCUM",
    "MOVT",
    "MOVAZ",
    "SETS_FLAGS",
    "READS_FLAGS",
    "COND_FLAGS",
    "COMPARE_BRANCH",
    "READS_PC",
    "FFR_READ",
    "FFR_WRITE",
];

pub fn generate(rows: &[Row<'_>]) -> Result<String> {
    check_lists(rows)?;
    let mut out = String::new();
    let n = rows.len() + 1;
    let _ = writeln!(
        out,
        "use crate::info::Iclass;\n\n\
         /// Per `Code`, `Code::Invalid` first.\n\
         pub(crate) static ICLASSES: [Iclass; {n}] = [\n    Iclass::Other,"
    );
    for r in rows {
        let _ = writeln!(out, "    Iclass::{}, // {}", iclass(r), r.name);
    }
    let _ = writeln!(out, "];\n");
    let _ = writeln!(
        out,
        "#[cfg(feature = \"instr_info\")]\n\
         use crate::instr_info::{{AccessClass as A, Implicit as I, Role as R, *}};\n\n\
         /// Per `Code`, `Code::Invalid` first (its entry is never used).\n\
         #[cfg(feature = \"instr_info\")]\n\
         pub(crate) static ACCESS_CLASSES: [A; {n}] = [\n    \
         A {{ mem: R::Read, reg: R::Read, flags: 0, implicit: I::None }},"
    );
    for r in rows {
        let sve = r.group == "sve" || r.group == "sme";
        let flags = flags(r, sve);
        let flags = if flags.is_empty() {
            "0".to_string()
        } else {
            flags.join(" | ")
        };
        let _ = writeln!(
            out,
            "    A {{ mem: R::{:?}, reg: R::{:?}, flags: {flags}, implicit: I::{} }}, // {}",
            mem_role(r.mnemonic, r.iclass, sve),
            reg_role(r.mnemonic, r.iclass, r.template, sve),
            implicit(r.mnemonic),
            r.name
        );
    }
    let _ = writeln!(out, "];");
    Ok(out)
}

/// The library's `info::Iclass`: the iclasses `info` needs to tell apart.
fn iclass(r: &Row<'_>) -> &'static str {
    match r.iclass {
        "systemmove" => "SystemMove",
        "systemmovepr" => "SystemMovePair",
        "systeminstrs" => "SystemInstrs",
        "syspairinstrs" => "SysPair",
        // CFINV, XAFLAG and AXFLAG share the iclass but name no PSTATE field.
        "pstate" if r.mnemonic == "MSR" => "MsrImm",
        ic if ic.starts_with("compbranch") => "CompBranch",
        _ => "Other",
    }
}

fn flags(r: &Row<'_>, sve: bool) -> Vec<&'static str> {
    let m = r.mnemonic;
    let ic = r.iclass;
    let cond_flags = matches!(m, "CCMP" | "CCMN" | "FCCMP" | "FCCMPE");
    let first_fault = sve && (m.starts_with("LDFF1") || m.starts_with("LDNF1"));
    let on = [
        sve,
        // SVE `ADR <Zd>.D, [<Zn>.D, <Zm>.D, ...]` computes addresses; it does not
        // access memory.
        sve && m == "ADR",
        accumulates(m, true) && !accumulates(m, false),
        // SME instructions that accumulate into ZA array vectors or tile slices
        // (`FMLA ZA.S[W8, 0, VGx2], ...`, `ADD ZA.D[...]`, `SMLAL`, `FDOT`, ...):
        // every one but the moves and `ZERO`, which only write.
        sve && !matches!(m, "MOVA" | "MOV" | "ZERO"),
        // `MOVT` into part of `ZT0`.
        sve && m == "MOVT",
        m == "MOVAZ",
        sets_flags(m, ic, sve),
        cond_flags || READS_FLAGS.contains(&m),
        cond_flags,
        // `CB<cc>` and the other compare-and-branch forms carry a condition but
        // compare registers, not flags.
        ic.starts_with("compbranch") || ic.starts_with("compfault"),
        READS_PC.contains(&m),
        sve && matches!(m, "RDFFR" | "RDFFRS") || first_fault,
        sve && matches!(m, "SETFFR" | "WRFFR") || first_fault,
    ];
    FLAGS
        .iter()
        .zip(on)
        .filter_map(|(f, on)| on.then_some(*f))
        .collect()
}

/// The role of an encoding with a memory operand.
fn mem_role(m: &str, ic: &str, sve: bool) -> Role {
    if m.starts_with("PRFM")
        || m.starts_with("PRFUM")
        || m == "RPRFM"
        || sve && m.starts_with("PRF")
    {
        return Role::Read;
    }
    if m.starts_with("CAS") || m.starts_with("RCWCAS") || m.starts_with("RCWSCAS") {
        return Role::CompareSwap;
    }
    if ic == "memop_128" {
        return Role::AtomicPair;
    }
    if m.starts_with("SWP")
        || (ic.starts_with("memop") && m.starts_with("LD") && !is_plain_load(m))
        || m.starts_with("RCW")
    {
        return Role::Atomic;
    }
    if ic.starts_with("ldstexcl") && m.starts_with("ST") {
        return Role::StoreExclusive;
    }
    if m == "ST64BV" || m == "ST64BV0" {
        return Role::StoreExclusive;
    }
    if m.starts_with("ST") || m.starts_with("CPY") || m.starts_with("SET") {
        return Role::Store;
    }
    if m == "LDG" || (ic.starts_with("asisdlso") && !m.ends_with('R')) {
        return Role::LoadMerge;
    }
    Role::Load
}

/// Encodings without a memory operand whose register operands are all read.
///
/// These lists name encoding mnemonics, not aliases: `DC`, `TLBI` and the other
/// `SYS` aliases are `SYS`, `BFI` is `BFM`, `NGC` is `SBC` ([`check_lists`]).
const READ: &[&str] = &[
    "MSR",
    "MSRR",
    "SYS",
    "SYSP",
    "BR",
    "BLR",
    "RET",
    "CBZ",
    "CBNZ",
    "TBZ",
    "TBNZ",
    "CCMP",
    "CCMN",
    "FCMP",
    "FCMPE",
    "FCCMP",
    "FCCMPE",
    "BRAA",
    "BRAB",
    "BRAAZ",
    "BRABZ",
    "BLRAA",
    "BLRAB",
    "BLRAAZ",
    "BLRABZ",
    "WFET",
    "WFIT",
    "AUTIASPPCR",
    "AUTIBSPPCR",
    "RETAASPPCR",
    "RETABSPPCR",
    "AXFLAG",
    "XAFLAG",
    "CFINV",
    "RMIF",
    "SETF8",
    "SETF16",
    "CTERMEQ",
    "CTERMNE",
    "PTEST",
    "WRFFR",
];

/// The role of an encoding without a memory operand. The library makes a `Dest`
/// an `Accum` for the instances that need it (the `ACCUM_2`, `ZA_ACCUM` and `MOVT`
/// flags; merging predication).
fn reg_role(m: &str, ic: &str, template: &str, sve: bool) -> Role {
    if matches!(m, "MRS" | "MRRS" | "SYSL") {
        return Role::SysRead;
    }
    if READ.contains(&m) {
        return Role::Read;
    }
    if ic.starts_with("compbranch") || ic.starts_with("testbranch") || ic.starts_with("compfault") {
        return Role::Read;
    }
    if accumulates(m, false) {
        return Role::Accum;
    }
    // SVE2 narrowing into the top halves (`SQXTNT`, `SHRNT`, `ADDHNT`, `FCVTNT`, ...)
    // keeps the bottom halves, as `XTN2` keeps the lower half.
    let narrow_top = m.ends_with("NT") && m != "CNT";
    if sve && (sve_rw_destination(template) || narrow_top) {
        return Role::Accum;
    }
    Role::Dest
}

/// Destinations that are also read (the rest of the register is kept, or the
/// operation accumulates). Arm ARM operation pseudocode, by mnemonic.
const ACCUMULATES: &[&str] = &[
    "MOVK",
    "BFM",
    "INS",
    "MLA",
    "MLS",
    "FMLA",
    "FMLS",
    "FMLAL",
    "FMLAL2",
    "FMLSL",
    "FMLSL2",
    "FCMLA",
    "SDOT",
    "UDOT",
    "USDOT",
    "SUDOT",
    "BFDOT",
    "FDOT",
    "BFMLALB",
    "BFMLALT",
    "BFMMLA",
    "SMMLA",
    "UMMLA",
    "USMMLA",
    "FMMLA",
    "BSL",
    "BIT",
    "BIF",
    "SSRA",
    "USRA",
    "SRSRA",
    "URSRA",
    "SLI",
    "SRI",
    "SADALP",
    "UADALP",
    "SABA",
    "UABA",
    "SABAL",
    "SABAL2",
    "UABAL",
    "UABAL2",
    "SQRDMLAH",
    "SQRDMLSH",
    "TBX",
    "AESE",
    "AESD",
    "SHA1C",
    "SHA1M",
    "SHA1P",
    "SHA1SU0",
    "SHA1SU1",
    "SHA256H",
    "SHA256H2",
    "SHA256SU0",
    "SHA256SU1",
    "SHA512H",
    "SHA512H2",
    "SHA512SU0",
    "SHA512SU1",
    "SM3TT1A",
    "SM3TT1B",
    "SM3TT2A",
    "SM3TT2B",
    "SM3PARTW1",
    "SM3PARTW2",
    "SM4E",
    "FMLALB",
    "FMLALT",
    "FMLALLBB",
    "FMLALLBT",
    "FMLALLTB",
    "FMLALLTT",
    "LDG",
    // Pointer authentication: the pointer register is the input and the result.
    "PACIA",
    "PACIB",
    "PACDA",
    "PACDB",
    "PACIZA",
    "PACIZB",
    "PACDZA",
    "PACDZB",
    "AUTIA",
    "AUTIB",
    "AUTDA",
    "AUTDB",
    "AUTIZA",
    "AUTIZB",
    "AUTDZA",
    "AUTDZB",
    "XPACI",
    "XPACD",
];

/// Widening multiply-accumulate (SMLAL, UMLAL2, SQDMLSL, ...).
const WIDENING: &[&str] = &["SMLAL", "SMLSL", "UMLAL", "UMLSL", "SQDMLAL", "SQDMLSL"];

/// Whether the destination is also read. `suffix2`: the instance has the mnemonic
/// suffix `2`, which `m` does not include (`SMLAL` for `SMLAL2`).
fn accumulates(m: &str, suffix2: bool) -> bool {
    // With the suffix, the full mnemonic is `<m>2`.
    if ACCUMULATES
        .iter()
        .any(|e| *e == m || (suffix2 && e.strip_suffix('2') == Some(m)))
    {
        return true;
    }
    if WIDENING.contains(&m.strip_suffix('2').unwrap_or(m)) || (suffix2 && WIDENING.contains(&m)) {
        return true;
    }
    // Narrowing to the upper half (`XTN2`, `SHRN2`, `FCVTN2`, `ADDHN2`, ...): the
    // lower half of the destination is kept. (`TRN2` is a permute, not one.)
    let narrowing = |base: &str| base.ends_with('N') && base != "TRN";
    m.strip_suffix('2').is_some_and(narrowing) || (suffix2 && narrowing(m))
}

/// SVE and SME templates name a destination that is also a source `<Zda>`, `<Zdn>`,
/// `<Pdn>`, `<Pdm>`, `<ZAda>`, `<Xdn>` (accumulate, destructive and insert forms:
/// `FMLA <Zda>.S, ...`, `MAD <Zdn>.S, ...`, `INSR <Zdn>.S, <Xm>`).
fn sve_rw_destination(template: &str) -> bool {
    let Some(first) = template.split_once(' ').map(|(_, rest)| rest.trim_start()) else {
        return false;
    };
    let first = first.trim_start_matches(['{', ' ']);
    let Some(sym) = first.strip_prefix('<').and_then(|s| s.split('>').next()) else {
        return false;
    };
    let name = sym.trim_start_matches(|c: char| c.is_ascii_uppercase());
    let name = name.trim_end_matches(|c: char| c.is_ascii_digit());
    matches!(name, "da" | "dn" | "dm")
}

/// `LDAPR`, `LDAR`, `LDXR` and friends are plain loads in the `memop` class.
fn is_plain_load(m: &str) -> bool {
    m.starts_with("LDAPR") || m.starts_with("LDAR") || m == "LD64B" || m.starts_with("LDLAR")
}

/// Mnemonics that read the flags whatever their operands (besides `CCMP` and
/// friends).
const READS_FLAGS: &[&str] = &[
    "ADC", "ADCS", "SBC", "SBCS", "AXFLAG", "XAFLAG", "CFINV", "RMIF", "SETF8", "SETF16",
];

fn sets_flags(m: &str, ic: &str, sve: bool) -> bool {
    (m.ends_with('S')
        && matches!(
            ic,
            "addsub_imm"
                | "addsub_shift"
                | "addsub_ext"
                | "addsub_carry"
                | "log_imm"
                | "log_shift"
                | "dp_2src"
        ))
        || matches!(
            m,
            "FCMP" | "FCMPE" | "AXFLAG" | "XAFLAG" | "CFINV" | "RMIF" | "SETF8" | "SETF16"
        )
        || ic == "floatcmp"
        // SVE: the predicate-generating instructions that set the flags from the
        // result.
        || sve && sve_sets_flags(m)
}

/// SVE instructions that set NZCV from a predicate result (the Arm ARM's
/// `PredTest`): the integer compares into predicates, the `S` forms of the
/// predicate logical and break instructions, `PTRUES`, `RDFFRS`, `PTEST`, `PFIRST`,
/// `PNEXT`, `WHILE*`, `CTERM*` and `MATCH`/`NMATCH`.
fn sve_sets_flags(m: &str) -> bool {
    (m.starts_with("CMP") && m.len() == 5)
        || m.starts_with("WHILE")
        || matches!(
            m,
            "ANDS"
                | "BICS"
                | "EORS"
                | "NANDS"
                | "NORS"
                | "ORNS"
                | "ORRS"
                | "BRKAS"
                | "BRKBS"
                | "BRKNS"
                | "BRKPAS"
                | "BRKPBS"
                | "PTRUES"
                | "RDFFRS"
                | "PTEST"
                | "PFIRST"
                | "PNEXT"
                | "CTERMEQ"
                | "CTERMNE"
                | "MATCH"
                | "NMATCH"
        )
}

/// Pointer authentication with the PC as a modifier.
const READS_PC: &[&str] = &[
    "AUTIASPPC",
    "AUTIBSPPC",
    "PACIASPPC",
    "PACIBSPPC",
    "PACNBIASPPC",
    "PACNBIBSPPC",
];

/// The library's `instr_info::Implicit`: registers used without being named.
fn implicit(m: &str) -> &'static str {
    match m {
        "PACIASP" | "PACIBSP" | "AUTIASP" | "AUTIBSP" => "LrSp",
        "PACIAZ" | "PACIBZ" | "AUTIAZ" | "AUTIBZ" | "XPACLRI" => "Lr",
        "PACIA1716" | "PACIB1716" | "AUTIA1716" | "AUTIB1716" => "X17X16",
        "RETAA" | "RETAB" | "RETAASPPC" | "RETABSPPC" | "RETAASPPCR" | "RETABSPPCR" => "RetSp",
        "ERETAA" | "ERETAB" => "Sp",
        "AUTIASPPC" | "AUTIBSPPC" | "AUTIASPPCR" | "AUTIBSPPCR" | "PACIASPPC" | "PACIBSPPC"
        | "PACNBIASPPC" | "PACNBIBSPPC" => "LrSp",
        "LD64B" => "Ld64b",
        "ST64B" | "ST64BV" | "ST64BV0" => "St64b",
        "CHKFEAT" => "Chkfeat",
        _ => "None",
    }
}

/// Every name in the exact-match lists is the mnemonic of some encoding (or, in the
/// accumulate lists, one with the suffix `2`): a misspelt or retired name would
/// silently match nothing.
fn check_lists(rows: &[Row<'_>]) -> Result<()> {
    let names: BTreeSet<&str> = rows.iter().map(|r| r.mnemonic).collect();
    let suffixed = |n: &str| n.strip_suffix('2').is_some_and(|b| names.contains(b));
    let missing: Vec<&str> = READ
        .iter()
        .chain(READS_FLAGS)
        .chain(READS_PC)
        .filter(|n| !names.contains(*n))
        .chain(
            ACCUMULATES
                .iter()
                .chain(WIDENING)
                .filter(|n| !names.contains(*n) && !suffixed(n)),
        )
        .copied()
        .collect();
    if !missing.is_empty() {
        bail!("instruction-info lists name no encoding's mnemonic: {missing:?}");
    }
    Ok(())
}
