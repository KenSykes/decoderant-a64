//! Instruction info: the registers and memory an instance reads and writes, in the
//! manner of `iced_x86::InstructionInfoFactory`.
//!
//! The spec's open-source release has no operational pseudocode, so operand access
//! comes from a hand-maintained classification by iclass and mnemonic (below),
//! checked against `fARM64`'s `instruction_info()` by the differential test.

use crate::generated::{Mnemonic, Register};
use crate::instruction::{FlowControl, Instruction};
use crate::operand::{MemoryMode, OpKind, Operand};
use crate::regs::RegisterClass;

/// How a register or memory location is accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OpAccess {
    #[default]
    None,
    Read,
    Write,
    ReadWrite,
    /// Read only on some paths (the flags of `CCMP` when the condition fails, ...).
    CondRead,
    /// Written only on some paths.
    CondWrite,
}

impl OpAccess {
    fn merge(self, other: OpAccess) -> OpAccess {
        use OpAccess::*;
        match (self, other) {
            (None, a) | (a, None) => a,
            (a, b) if a == b => a,
            _ => ReadWrite,
        }
    }

    pub fn reads(self) -> bool {
        matches!(
            self,
            OpAccess::Read | OpAccess::ReadWrite | OpAccess::CondRead
        )
    }

    pub fn writes(self) -> bool {
        matches!(
            self,
            OpAccess::Write | OpAccess::ReadWrite | OpAccess::CondWrite
        )
    }
}

/// A register an instruction reads or writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct UsedRegister {
    pub register: Register,
    pub access: OpAccess,
}

/// A memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct UsedMemory {
    pub base: Register,
    pub index: Register,
    /// Offset from the base (0 for post-index forms, which access the base).
    pub displacement: i64,
    /// Bytes accessed (0 when unknown, such as `CPY*`/`SET*` and prefetches).
    pub size: u32,
    pub access: OpAccess,
}

const MAX_REGS: usize = 24;
const MAX_MEM: usize = 2;

/// The registers and memory an instruction uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstructionInfo {
    regs: [UsedRegister; MAX_REGS],
    nregs: u8,
    mem: [UsedMemory; MAX_MEM],
    nmem: u8,
}

impl InstructionInfo {
    pub fn used_registers(&self) -> &[UsedRegister] {
        &self.regs[..self.nregs as usize]
    }

    pub fn used_memory(&self) -> &[UsedMemory] {
        &self.mem[..self.nmem as usize]
    }

    /// The access to `r` (`OpAccess::None` when unused). Views (`W0`, `S0`) are
    /// reported as written, not merged into the full register.
    pub fn access(&self, r: Register) -> OpAccess {
        self.used_registers()
            .iter()
            .find(|u| u.register == r)
            .map_or(OpAccess::None, |u| u.access)
    }

    fn add_reg(&mut self, register: Register, access: OpAccess) {
        if register == Register::None || access == OpAccess::None {
            return;
        }
        // XZR/WZR are not state.
        if register.is_zero() {
            return;
        }
        for u in &mut self.regs[..self.nregs as usize] {
            if u.register == register {
                u.access = u.access.merge(access);
                return;
            }
        }
        if (self.nregs as usize) < MAX_REGS {
            self.regs[self.nregs as usize] = UsedRegister { register, access };
            self.nregs += 1;
        }
    }

    fn add_mem(&mut self, m: UsedMemory) {
        if (self.nmem as usize) < MAX_MEM {
            self.mem[self.nmem as usize] = m;
            self.nmem += 1;
        }
    }
}

/// Computes [`InstructionInfo`], reusing one buffer (as `iced-x86` does).
#[derive(Debug, Default)]
pub struct InstructionInfoFactory {
    info: InstructionInfo,
}

impl InstructionInfoFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn info(&mut self, insn: &Instruction) -> &InstructionInfo {
        self.info = compute(insn);
        &self.info
    }
}

impl Instruction {
    /// The registers this instance reads and writes (see [`InstructionInfoFactory`]).
    pub fn used_registers(&self) -> InstructionInfo {
        compute(self)
    }
}

/// How the explicit operands are accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Operand 0 written, the rest read.
    Dest,
    /// Operand 0 read and written (accumulate, insert, merge), the rest read.
    Accum,
    /// Every operand read.
    Read,
    /// Register operands before the memory operand written (loads).
    Load,
    /// Register operands before the memory operand read (stores).
    Store,
    /// Load that merges into the transfer registers (single-lane loads, `LDG`).
    LoadMerge,
    /// `STXR Ws, Xt, [Xn]`: the status register written, the data read.
    StoreExclusive,
    /// `LDADD Xs, Xt, [Xn]`, `SWP`: the source read, the old value written.
    Atomic,
    /// `CAS Xs, Xt, [Xn]`, `CASP`: the compare value read and replaced.
    CompareSwap,
    /// `LDCLRP Xt1, Xt2, [Xn]`, `SWPP`: the pair is both operand and result.
    AtomicPair,
    /// `MRS Xt, <sysreg>`, `SYSL`: the register operand written.
    SysRead,
}

fn mnemonic_str(m: Mnemonic) -> &'static str {
    m.name()
}

/// Destinations that are also read (the rest of the register is kept, or the
/// operation accumulates). Arm ARM operation pseudocode, by mnemonic.
fn accumulates(m: &str) -> bool {
    const EXACT: &[&str] = &[
        "MOVK",
        "BFM",
        "BFI",
        "BFXIL",
        "BFC",
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
        "SRA",
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
    if EXACT.contains(&m) {
        return true;
    }
    // Widening multiply-accumulate (SMLAL, UMLAL2, SQDMLSL, ...).
    let base = m.strip_suffix('2').unwrap_or(m);
    if matches!(
        base,
        "SMLAL" | "SMLSL" | "UMLAL" | "UMLSL" | "SQDMLAL" | "SQDMLSL"
    ) {
        return true;
    }
    // Narrowing to the upper half (`XTN2`, `SHRN2`, `FCVTN2`, `ADDHN2`, ...): the
    // lower half of the destination is kept. (`TRN2` is a permute, not one.)
    (m.ends_with("N2") && m != "TRN2") || m == "FCVTXN2"
}

fn role(insn: &Instruction) -> Role {
    let oc = insn.op_code();
    let m = mnemonic_str(insn.mnemonic());
    let ic = oc.iclass();
    let mem = insn.operands().iter().any(|o| o.kind == OpKind::Memory);
    let suffix2 = has_suffix_2(insn);
    if mem {
        if m.starts_with("PRFM") || m.starts_with("PRFUM") || m == "RPRFM" {
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
        return Role::Load;
    }
    match m {
        "MRS" | "MRRS" | "SYSL" => return Role::SysRead,
        "MSR" | "MSRR" | "SYS" | "SYSP" | "DC" | "IC" | "AT" | "TLBI" | "TLBIP" | "BR" | "BLR"
        | "RET" | "CBZ" | "CBNZ" | "TBZ" | "TBNZ" | "CCMP" | "CCMN" | "FCMP" | "FCMPE"
        | "FCCMP" | "FCCMPE" | "BRAA" | "BRAB" | "BRAAZ" | "BRABZ" | "BLRAA" | "BLRAB"
        | "BLRAAZ" | "BLRABZ" | "WFET" | "WFIT" | "AUTIASPPCR" | "AUTIBSPPCR" | "RETAASPPCR"
        | "RETABSPPCR" | "AXFLAG" | "XAFLAG" | "CFINV" | "RMIF" | "SETF8" | "SETF16" | "CMPP"
        | "PLBI" | "GCSPUSHM" | "GCSSS1" | "APAS" | "BRB" | "CFP" | "CPP" | "DVP" | "COSP"
        | "GIC" | "GICR" | "TRCIT" => return Role::Read,
        _ => {}
    }
    if ic.starts_with("compbranch") || ic.starts_with("testbranch") || ic.starts_with("compfault") {
        return Role::Read;
    }
    if accumulates(m) || (suffix2 && accumulates(&concat2(m))) {
        return Role::Accum;
    }
    Role::Dest
}

/// `LDAPR`, `LDAR`, `LDXR` and friends are plain loads in the `memop` class.
fn is_plain_load(m: &str) -> bool {
    m.starts_with("LDAPR") || m.starts_with("LDAR") || m == "LD64B" || m.starts_with("LDLAR")
}

fn has_suffix_2(insn: &Instruction) -> bool {
    insn.op_code()
        .data()
        .and_then(|d| d.ops)
        .is_some_and(|ops| crate::desc::suffix(ops, insn.word) == "2")
}

fn concat2(m: &str) -> heapless_name::Name {
    heapless_name::Name::new(m, "2")
}

/// A tiny fixed-size string for `<mnemonic>2` without allocation.
mod heapless_name {
    pub struct Name {
        b: [u8; 24],
        n: usize,
    }

    impl Name {
        pub fn new(a: &str, b: &str) -> Name {
            let mut s = Name { b: [0; 24], n: 0 };
            for &c in a.as_bytes().iter().chain(b.as_bytes()) {
                if s.n < s.b.len() {
                    s.b[s.n] = c;
                    s.n += 1;
                }
            }
            s
        }
    }

    impl core::ops::Deref for Name {
        type Target = str;
        fn deref(&self) -> &str {
            core::str::from_utf8(&self.b[..self.n]).unwrap_or("")
        }
    }
}

/// Registers a register-list operand names.
fn list_regs(o: &Operand) -> impl Iterator<Item = Register> + '_ {
    let n = o.reg.number();
    (0..u32::from(o.count)).map(move |k| match o.reg.class() {
        RegisterClass::Z => Register::z((n + k) % 32),
        _ => Register::v((n + k) % 32),
    })
}

/// The registers of a register-like operand (shifted and extended registers
/// included), for operands that are not memory.
fn operand_regs(o: &Operand, mut f: impl FnMut(Register)) {
    match o.kind {
        OpKind::Register | OpKind::ShiftedRegister | OpKind::ExtendedRegister => f(o.reg),
        OpKind::RegisterList => list_regs(o).for_each(f),
        _ => {}
    }
}

fn access_size(insn: &Instruction, transfers: &[Operand]) -> u32 {
    let _ = insn;
    transfers
        .iter()
        .map(|o| match o.kind {
            OpKind::RegisterList => {
                let per = if o.lane != Operand::NO_LANE {
                    o.arr.element_bytes()
                } else {
                    o.arr.total_bytes()
                };
                per * u32::from(o.count)
            }
            OpKind::Register => o.reg.size(),
            _ => 0,
        })
        .sum()
}

fn compute(insn: &Instruction) -> InstructionInfo {
    let mut info = InstructionInfo::default();
    if insn.is_invalid() || !insn.has_operands {
        return info;
    }
    let role = role(insn);
    let ops = insn.operands();
    let mem_index = ops.iter().position(|o| o.kind == OpKind::Memory);
    for (i, o) in ops.iter().enumerate() {
        if o.kind == OpKind::Memory {
            continue;
        }
        let before_mem = mem_index.is_none_or(|m| i < m);
        let access = match role {
            Role::Dest => {
                if i == 0 {
                    OpAccess::Write
                } else {
                    OpAccess::Read
                }
            }
            Role::Accum => {
                if i == 0 {
                    OpAccess::ReadWrite
                } else {
                    OpAccess::Read
                }
            }
            Role::Read => OpAccess::Read,
            Role::Load => {
                if before_mem {
                    OpAccess::Write
                } else {
                    OpAccess::Read
                }
            }
            Role::LoadMerge => {
                if before_mem {
                    OpAccess::ReadWrite
                } else {
                    OpAccess::Read
                }
            }
            Role::Store => {
                if o.writeback {
                    OpAccess::ReadWrite
                } else {
                    OpAccess::Read
                }
            }
            Role::StoreExclusive => {
                if i == 0 {
                    OpAccess::Write
                } else {
                    OpAccess::Read
                }
            }
            Role::Atomic => {
                // `LDADD Xs, Xt, [Xn]`: Xs read, Xt written; the `STADD Xs, [Xn]`
                // aliases have only the source.
                if i == 0 || mem_index == Some(1) {
                    OpAccess::Read
                } else {
                    OpAccess::Write
                }
            }
            Role::AtomicPair => OpAccess::ReadWrite,
            Role::CompareSwap => {
                // CAS Xs, Xt, [Xn]; CASP Xs, X(s+1), Xt, X(t+1), [Xn].
                let pair = mem_index == Some(4);
                let compare = if pair { i < 2 } else { i == 0 };
                if compare {
                    OpAccess::ReadWrite
                } else {
                    OpAccess::Read
                }
            }
            Role::SysRead => {
                if o.kind == OpKind::Register {
                    OpAccess::Write
                } else {
                    OpAccess::Read
                }
            }
        };
        // `X1!` in CPY*/SET*: updated.
        let access = if o.writeback {
            OpAccess::ReadWrite
        } else {
            access
        };
        operand_regs(o, |r| info.add_reg(r, access));
    }
    if let Some(mi) = mem_index {
        let m = ops[mi];
        let wb = matches!(m.mode, MemoryMode::PreIndex | MemoryMode::PostIndex) || m.writeback;
        info.add_reg(
            m.reg,
            if wb {
                OpAccess::ReadWrite
            } else {
                OpAccess::Read
            },
        );
        info.add_reg(m.index_reg, OpAccess::Read);
        let access = match role {
            Role::Load | Role::LoadMerge => OpAccess::Read,
            Role::Store | Role::StoreExclusive => OpAccess::Write,
            Role::Atomic | Role::CompareSwap | Role::AtomicPair => OpAccess::ReadWrite,
            _ => OpAccess::None,
        };
        let size = access_size(insn, &ops[..mi]);
        info.add_mem(UsedMemory {
            base: m.reg,
            index: m.index_reg,
            displacement: if m.mode == MemoryMode::PostIndex {
                0
            } else {
                m.imm as i64
            },
            size,
            access,
        });
        // CPY*: the source is a second memory operand.
        if let Some(m2) = ops.iter().skip(mi + 1).find(|o| o.kind == OpKind::Memory) {
            info.add_reg(m2.reg, OpAccess::ReadWrite);
            info.add_mem(UsedMemory {
                base: m2.reg,
                access: OpAccess::Read,
                ..UsedMemory::default()
            });
        }
    }
    implicit(insn, &mut info);
    info
}

/// Registers an instruction uses without naming them.
fn implicit(insn: &Instruction, info: &mut InstructionInfo) {
    use OpAccess::*;
    let m = insn.mnemonic().name();
    match insn.flow_control() {
        FlowControl::Call | FlowControl::IndirectCall => info.add_reg(Register::X30, Write),
        _ => {}
    }
    // Flags.
    let ic = insn.op_code().iclass();
    let sets_flags = (m.ends_with('S')
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
        || ic == "floatcmp";
    let cond_flags = matches!(m, "CCMP" | "CCMN" | "FCCMP" | "FCCMPE");
    let reads_flags = (insn.condition_code().is_some() && !is_compare_branch(ic))
        || matches!(
            m,
            "ADC"
                | "ADCS"
                | "SBC"
                | "SBCS"
                | "NGC"
                | "NGCS"
                | "AXFLAG"
                | "XAFLAG"
                | "CFINV"
                | "RMIF"
                | "SETF8"
                | "SETF16"
        )
        || cond_flags;
    if reads_flags && sets_flags || cond_flags {
        info.add_reg(Register::NZCV, ReadWrite);
    } else if sets_flags {
        info.add_reg(Register::NZCV, Write);
    } else if reads_flags {
        info.add_reg(Register::NZCV, Read);
    }
    // PC-relative addresses and literal loads read the PC.
    if insn.flow_control() == FlowControl::Next && insn.label_target().is_some() {
        info.add_reg(Register::PC, Read);
    }
    if matches!(
        m,
        "AUTIASPPC" | "AUTIBSPPC" | "PACIASPPC" | "PACIBSPPC" | "PACNBIASPPC" | "PACNBIBSPPC"
    ) {
        info.add_reg(Register::PC, Read);
    }
    // Pointer authentication with implicit operands.
    match m {
        "PACIASP" | "PACIBSP" | "AUTIASP" | "AUTIBSP" => {
            info.add_reg(Register::X30, ReadWrite);
            info.add_reg(Register::SP, Read);
        }
        "PACIAZ" | "PACIBZ" | "AUTIAZ" | "AUTIBZ" | "XPACLRI" => {
            info.add_reg(Register::X30, ReadWrite)
        }
        "PACIA1716" | "PACIB1716" | "AUTIA1716" | "AUTIB1716" => {
            info.add_reg(Register::X17, ReadWrite);
            info.add_reg(Register::X16, Read);
        }
        "RETAA" | "RETAB" | "RETAASPPC" | "RETABSPPC" | "RETAASPPCR" | "RETABSPPCR" => {
            info.add_reg(Register::X30, Read);
            info.add_reg(Register::SP, Read);
        }
        // ERETAA/ERETAB authenticate ELR (not a general register) with SP.
        "ERETAA" | "ERETAB" => info.add_reg(Register::SP, Read),
        "AUTIASPPC" | "AUTIBSPPC" | "AUTIASPPCR" | "AUTIBSPPCR" | "PACIASPPC" | "PACIBSPPC"
        | "PACNBIASPPC" | "PACNBIBSPPC" => {
            info.add_reg(Register::X30, ReadWrite);
            info.add_reg(Register::SP, Read);
        }
        // LD64B/ST64B*: Xt names eight consecutive registers.
        "LD64B" | "ST64B" | "ST64BV" | "ST64BV0" => {
            let ops = insn.operands();
            let t = ops
                .iter()
                .rev()
                .find(|o| o.kind == OpKind::Register)
                .map_or(0, |o| o.reg.number());
            let acc = if m == "LD64B" { Write } else { Read };
            for k in 1..8 {
                info.add_reg(Register::x((t + k) % 32), acc);
            }
        }
        "BRAAZ" | "BRABZ" | "BLRAAZ" | "BLRABZ" | "BRAA" | "BRAB" | "BLRAA" | "BLRAB" => {}
        "CHKFEAT" => info.add_reg(Register::X16, ReadWrite),
        _ => {}
    }
}

/// `CB<cc>` and the other compare-and-branch forms carry a condition but compare
/// registers, not flags.
fn is_compare_branch(iclass: &str) -> bool {
    iclass.starts_with("compbranch") || iclass.starts_with("compfault")
}
