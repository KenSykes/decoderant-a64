//! Instruction info: the registers and memory an instance reads and writes, in the
//! manner of `iced_x86::InstructionInfoFactory`.
//!
//! The spec's open-source release has no operational pseudocode, so operand access
//! comes from a hand-maintained classification by iclass and mnemonic, checked
//! against `fARM64`'s `instruction_info()` by the differential test. The generator
//! applies it once per encoding (`gen/src/info.rs`), as `iced-x86` generates its
//! tables: at run time only the facts that differ between instances of an encoding
//! are looked at.

use crate::generated::Register;
use crate::generated::info::ACCESS_CLASSES;
use crate::instruction::{FlowControl, Instruction};
use crate::operand::{MemoryMode, OpKind, Operand};

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
pub(crate) enum Role {
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

/// Registers an encoding uses without naming them (besides the flags, the PC, the
/// first-fault register and the link register of calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Implicit {
    None,
    /// `PACIASP`: X30 is the pointer, SP the modifier.
    LrSp,
    /// `PACIAZ`, `XPACLRI`.
    Lr,
    /// `PACIA1716`: X17 is the pointer, X16 the modifier.
    X17X16,
    /// `RETAA`: X30 authenticated with SP.
    RetSp,
    /// `ERETAA`/`ERETAB` authenticate ELR (not a general register) with SP.
    Sp,
    /// `LD64B`/`ST64B*`: Xt names eight consecutive registers, written or read.
    Ld64b,
    St64b,
    /// `CHKFEAT X16`.
    Chkfeat,
}

/// What instruction info needs to know about an encoding (`gen/src/info.rs`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct AccessClass {
    /// The role when the instance has a memory operand.
    pub mem: Role,
    /// The role otherwise; `Dest` may still be `Accum` for the instance (see
    /// [`role`]).
    pub reg: Role,
    pub flags: u16,
    pub implicit: Implicit,
}

/// An SVE or SME encoding.
pub(crate) const SVE: u16 = 1 << 0;
/// SVE `ADR <Zd>.D, [<Zn>.D, <Zm>.D, ...]` computes addresses; it does not access
/// memory.
pub(crate) const SVE_ADR: u16 = 1 << 1;
/// Accumulates when the instance has the mnemonic suffix `2`.
pub(crate) const ACCUM_2: u16 = 1 << 2;
/// SME: accumulates into the ZA array vectors or tile slice of operand 0.
pub(crate) const ZA_ACCUM: u16 = 1 << 3;
/// SME `MOVT`: accumulates into `ZT0` (it writes part of it).
pub(crate) const MOVT: u16 = 1 << 4;
/// `MOVAZ` reads a tile slice and zeroes it.
pub(crate) const MOVAZ: u16 = 1 << 5;
pub(crate) const SETS_FLAGS: u16 = 1 << 6;
/// Reads the flags whatever its operands (`ADC`, `CCMP`, ...).
pub(crate) const READS_FLAGS: u16 = 1 << 7;
/// `CCMP`, `FCCMP`: the flags are read and written.
pub(crate) const COND_FLAGS: u16 = 1 << 8;
/// `CB<cc>` and the other compare-and-branch forms carry a condition but compare
/// registers, not flags.
pub(crate) const COMPARE_BRANCH: u16 = 1 << 9;
/// Reads the PC without a label operand (`PACIASPPC`).
pub(crate) const READS_PC: u16 = 1 << 10;
pub(crate) const FFR_READ: u16 = 1 << 11;
pub(crate) const FFR_WRITE: u16 = 1 << 12;

/// The role of the instance, and whether it accesses memory.
fn role(insn: &Instruction, c: &AccessClass) -> (Role, bool) {
    let mem = c.flags & SVE_ADR == 0 && insn.operands().iter().any(|o| o.kind == OpKind::Memory);
    if mem {
        return (c.mem, true);
    }
    let accum = c.reg == Role::Dest
        && (c.flags & ACCUM_2 != 0 && has_suffix_2(insn)
            || c.flags & SVE != 0 && (sme_accumulates(insn, c) || merging(insn)));
    (if accum { Role::Accum } else { c.reg }, false)
}

/// SME instructions that accumulate into ZA array vectors or tile slices (`FMLA
/// ZA.S[W8, 0, VGx2], ...`, `ADD ZA.D[...]`, `SMLAL`, `FDOT`, ...): every one but
/// the moves and `ZERO`, which only write; and `MOVT` into part of `ZT0`.
fn sme_accumulates(insn: &Instruction, c: &AccessClass) -> bool {
    let Some(d) = insn.operands().first() else {
        return false;
    };
    match d.kind {
        OpKind::ZaArray | OpKind::ZaSlice => c.flags & ZA_ACCUM != 0,
        OpKind::Register => c.flags & MOVT != 0 && d.reg == Register::ZT0,
        _ => false,
    }
}

/// Merging predication (`/M`): inactive elements of the destination are kept, so it
/// is read as well.
fn merging(insn: &Instruction) -> bool {
    insn.operands()
        .iter()
        .any(|o| o.predicate() == crate::operand::PredQual::Merging)
}

fn has_suffix_2(insn: &Instruction) -> bool {
    insn.op_code()
        .data()
        .and_then(|d| d.ops)
        .is_some_and(|ops| crate::desc::suffix(ops, insn.word) == "2")
}

/// The registers of a register-like operand (shifted and extended registers
/// included), for operands that are not memory.
fn operand_regs(o: &Operand, mut f: impl FnMut(Register)) {
    match o.kind {
        OpKind::Register | OpKind::ShiftedRegister | OpKind::ExtendedRegister => {
            f(o.reg);
            // `PSEL ..., <Pm>.<T>[<Wv>, <imm>]`: the index register (only read).
            if o.kind == OpKind::Register && o.index_reg != Register::None {
                f(o.index_reg);
            }
        }
        OpKind::RegisterList => (0..u32::from(o.count)).for_each(|k| f(o.list_register(k))),
        // The ZA storage an SME operand names: a tile, or ZA for array vectors.
        OpKind::ZaTile | OpKind::ZaSlice => f(o.reg),
        OpKind::ZaArray => f(Register::ZA),
        OpKind::ZaMask => {
            for t in 0..8 {
                if o.imm >> t & 1 == 1 {
                    f(Register::za_tile(3, t).unwrap_or(Register::None));
                }
            }
        }
        _ => {}
    }
}

fn access_size(transfers: &[Operand]) -> u32 {
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
    let c = &ACCESS_CLASSES[insn.code as usize];
    let (role, accesses_memory) = role(insn, c);
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
        // A governing predicate is read, also before the memory operand of a load.
        let access = if o.pred != crate::operand::PredQual::None {
            OpAccess::Read
        } else {
            access
        };
        // `MOVAZ` reads a tile slice and zeroes it.
        let access = if matches!(o.kind, OpKind::ZaSlice | OpKind::ZaArray) && c.flags & MOVAZ != 0
        {
            OpAccess::ReadWrite
        } else {
            access
        };
        operand_regs(o, |r| info.add_reg(r, access));
        // The index register of a tile slice or array vectors is only read.
        if matches!(o.kind, OpKind::ZaSlice | OpKind::ZaArray) {
            info.add_reg(o.index_reg, OpAccess::Read);
        }
    }
    if let Some(mi) = mem_index.filter(|_| !accesses_memory) {
        // SVE `ADR`.
        let m = ops[mi];
        info.add_reg(m.reg, OpAccess::Read);
        info.add_reg(m.index_reg, OpAccess::Read);
    } else if let Some(mi) = mem_index {
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
        // SVE and SME accesses depend on the vector length (and the predicate): their
        // size is unknown (0).
        let size = if c.flags & SVE != 0 {
            0
        } else {
            access_size(&ops[..mi])
        };
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
    implicit(insn, c, &mut info);
    info
}

/// Registers an instruction uses without naming them.
fn implicit(insn: &Instruction, c: &AccessClass, info: &mut InstructionInfo) {
    use OpAccess::*;
    let flow = insn.flow_control();
    if matches!(flow, FlowControl::Call | FlowControl::IndirectCall) {
        info.add_reg(Register::X30, Write);
    }
    // Flags.
    let sets_flags = c.flags & SETS_FLAGS != 0;
    let reads_flags = c.flags & READS_FLAGS != 0
        || (c.flags & COMPARE_BRANCH == 0 && insn.condition_code().is_some());
    if reads_flags && sets_flags || c.flags & COND_FLAGS != 0 {
        info.add_reg(Register::NZCV, ReadWrite);
    } else if sets_flags {
        info.add_reg(Register::NZCV, Write);
    } else if reads_flags {
        info.add_reg(Register::NZCV, Read);
    }
    // The first-fault register.
    match (c.flags & FFR_READ != 0, c.flags & FFR_WRITE != 0) {
        (true, true) => info.add_reg(Register::FFR, ReadWrite),
        (true, false) => info.add_reg(Register::FFR, Read),
        (false, true) => info.add_reg(Register::FFR, Write),
        (false, false) => {}
    }
    // PC-relative addresses and literal loads read the PC.
    if flow == FlowControl::Next && insn.label_target().is_some() {
        info.add_reg(Register::PC, Read);
    }
    if c.flags & READS_PC != 0 {
        info.add_reg(Register::PC, Read);
    }
    match c.implicit {
        Implicit::None => {}
        Implicit::LrSp => {
            info.add_reg(Register::X30, ReadWrite);
            info.add_reg(Register::SP, Read);
        }
        Implicit::Lr => info.add_reg(Register::X30, ReadWrite),
        Implicit::X17X16 => {
            info.add_reg(Register::X17, ReadWrite);
            info.add_reg(Register::X16, Read);
        }
        Implicit::RetSp => {
            info.add_reg(Register::X30, Read);
            info.add_reg(Register::SP, Read);
        }
        Implicit::Sp => info.add_reg(Register::SP, Read),
        Implicit::Ld64b | Implicit::St64b => {
            let t = insn
                .operands()
                .iter()
                .rev()
                .find(|o| o.kind == OpKind::Register)
                .map_or(0, |o| o.reg.number());
            let acc = if c.implicit == Implicit::Ld64b {
                Write
            } else {
                Read
            };
            for k in 1..8 {
                info.add_reg(Register::x((t + k) % 32), acc);
            }
        }
        Implicit::Chkfeat => info.add_reg(Register::X16, ReadWrite),
    }
}
