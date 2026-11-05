//! Operand kinds and the small enums operands carry.

use crate::generated::Register;

/// The kind of an instruction operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum OpKind {
    #[default]
    None,
    /// A register: general-purpose, scalar SIMD&FP (`B`..`Q`), or a vector with an
    /// [`Arrangement`] (`V0.4S`), or a vector element (`V0.S[1]`, see
    /// [`Operand::index`]).
    Register,
    /// A list of consecutive vector registers (`{ V0.16B, V1.16B }`), optionally
    /// with an element index.
    RegisterList,
    /// An integer immediate, possibly shifted (`#1, LSL #12`).
    Immediate,
    /// A logical (bitmask) immediate, already expanded.
    BitmaskImmediate,
    /// A floating-point immediate; the value is the `f64` bit pattern.
    FpImmediate,
    /// A register with a shift (`X2, LSL #3`).
    ShiftedRegister,
    /// A register with an extend (`W2, UXTW #2`).
    ExtendedRegister,
    /// A memory reference; see the memory accessors on
    /// [`Instruction`](crate::Instruction).
    Memory,
    /// A PC-relative target address.
    Label,
    /// A condition code.
    Condition,
    /// A system register (`MRS`/`MSR`), by `op0:op1:CRn:CRm:op2`.
    SystemRegister,
    /// A PSTATE field (`MSR <pstatefield>, #imm`).
    PState,
    /// A system operation (`DC`, `IC`, `AT`, `TLBI`, ... and `SYS` operands).
    SystemOp,
    /// A `CRn`/`CRm` operand of `SYS`/`SYSL`.
    ControlRegister,
    /// A barrier option (`DMB ISH`).
    Barrier,
    /// A prefetch operation (`PRFM PLDL1KEEP`).
    Prefetch,
    /// A fixed name that is part of the syntax (`CSYNC`, BTI targets, `PH`, ...).
    Name,
    /// An SME ZA tile (`ZA3.S`): the tile register.
    ZaTile,
    /// A horizontal or vertical slice of a ZA tile (`ZA1H.S[W12, 3]`, `ZA0V.D[W13,
    /// 0:1]`): the tile register, [`Operand::is_vertical`], the index register
    /// ([`Operand::index_register`]), the first offset ([`Operand::immediate`]) and
    /// the number of slices ([`Operand::list_len`]).
    ZaSlice,
    /// ZA array vectors (`ZA.D[W8, 1, VGx4]`, `ZA[W12, 0]`): as a slice, with
    /// `ZA` as the register, the element size if any, and the vector group
    /// ([`Operand::vector_group`]).
    ZaArray,
    /// The `ZERO` tile mask: bit `n` names the 64-bit tile `ZA<n>.D`
    /// ([`Operand::immediate`]).
    ZaMask,
}

/// A vector arrangement or element size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Arrangement {
    #[default]
    None,
    B8,
    B16,
    H4,
    H8,
    S2,
    S4,
    D1,
    D2,
    Q1,
    H2,
    B4,
    B2,
    /// Element sizes, for lanes (`V0.S[1]`) and lists with an index.
    B,
    H,
    S,
    D,
    Q,
}

impl Arrangement {
    /// The assembler suffix without the dot (`4S`).
    pub fn name(self) -> &'static str {
        match self {
            Arrangement::None => "",
            Arrangement::B8 => "8B",
            Arrangement::B16 => "16B",
            Arrangement::H4 => "4H",
            Arrangement::H8 => "8H",
            Arrangement::S2 => "2S",
            Arrangement::S4 => "4S",
            Arrangement::D1 => "1D",
            Arrangement::D2 => "2D",
            Arrangement::Q1 => "1Q",
            Arrangement::H2 => "2H",
            Arrangement::B4 => "4B",
            Arrangement::B2 => "2B",
            Arrangement::B => "B",
            Arrangement::H => "H",
            Arrangement::S => "S",
            Arrangement::D => "D",
            Arrangement::Q => "Q",
        }
    }

    pub fn from_name(s: &str) -> Option<Arrangement> {
        Some(match s {
            "8B" => Arrangement::B8,
            "16B" => Arrangement::B16,
            "4H" => Arrangement::H4,
            "8H" => Arrangement::H8,
            "2S" => Arrangement::S2,
            "4S" => Arrangement::S4,
            "1D" => Arrangement::D1,
            "2D" => Arrangement::D2,
            "1Q" => Arrangement::Q1,
            "2H" => Arrangement::H2,
            "4B" => Arrangement::B4,
            "2B" => Arrangement::B2,
            "B" => Arrangement::B,
            "H" => Arrangement::H,
            "S" => Arrangement::S,
            "D" => Arrangement::D,
            "Q" => Arrangement::Q,
            _ => return None,
        })
    }

    /// Element size in bytes (0 for `None`).
    pub fn element_bytes(self) -> u32 {
        match self {
            Arrangement::None => 0,
            Arrangement::B8
            | Arrangement::B16
            | Arrangement::B4
            | Arrangement::B2
            | Arrangement::B => 1,
            Arrangement::H4 | Arrangement::H8 | Arrangement::H2 | Arrangement::H => 2,
            Arrangement::S2 | Arrangement::S4 | Arrangement::S => 4,
            Arrangement::D1 | Arrangement::D2 | Arrangement::D => 8,
            Arrangement::Q1 | Arrangement::Q => 16,
        }
    }

    /// Total size in bytes (element size for the element forms).
    pub fn total_bytes(self) -> u32 {
        match self {
            Arrangement::B8 | Arrangement::H4 | Arrangement::S2 | Arrangement::D1 => 8,
            Arrangement::B16
            | Arrangement::H8
            | Arrangement::S4
            | Arrangement::D2
            | Arrangement::Q1 => 16,
            Arrangement::H2 | Arrangement::B4 => 4,
            Arrangement::B2 => 2,
            a => a.element_bytes(),
        }
    }
}

/// The element size `Arrangement`s by log2 bytes.
pub(crate) const ARR_ES: [Arrangement; 5] = [
    Arrangement::B,
    Arrangement::H,
    Arrangement::S,
    Arrangement::D,
    Arrangement::Q,
];

/// A shift applied to a register or immediate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShiftKind {
    #[default]
    Lsl,
    Lsr,
    Asr,
    Ror,
    /// `MSL` (AdvSIMD modified immediate: shift in ones).
    Msl,
}

impl ShiftKind {
    pub fn name(self) -> &'static str {
        match self {
            ShiftKind::Lsl => "LSL",
            ShiftKind::Lsr => "LSR",
            ShiftKind::Asr => "ASR",
            ShiftKind::Ror => "ROR",
            ShiftKind::Msl => "MSL",
        }
    }
}

/// An extend applied to a register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExtendKind {
    #[default]
    Uxtb,
    Uxth,
    Uxtw,
    Uxtx,
    Sxtb,
    Sxth,
    Sxtw,
    Sxtx,
    /// `LSL`, the preferred spelling of `UXTX` (64-bit) or `UXTW` (32-bit) where
    /// the stack pointer is involved, and of `UXTX` in register-offset addressing.
    Lsl,
}

impl ExtendKind {
    pub fn name(self) -> &'static str {
        match self {
            ExtendKind::Uxtb => "UXTB",
            ExtendKind::Uxth => "UXTH",
            ExtendKind::Uxtw => "UXTW",
            ExtendKind::Uxtx => "UXTX",
            ExtendKind::Sxtb => "SXTB",
            ExtendKind::Sxth => "SXTH",
            ExtendKind::Sxtw => "SXTW",
            ExtendKind::Sxtx => "SXTX",
            ExtendKind::Lsl => "LSL",
        }
    }

    /// From the `option` field (`extend_type`).
    pub fn from_option(option: u32) -> ExtendKind {
        match option & 7 {
            0 => ExtendKind::Uxtb,
            1 => ExtendKind::Uxth,
            2 => ExtendKind::Uxtw,
            3 => ExtendKind::Uxtx,
            4 => ExtendKind::Sxtb,
            5 => ExtendKind::Sxth,
            6 => ExtendKind::Sxtw,
            _ => ExtendKind::Sxtx,
        }
    }
}

/// A condition code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConditionCode {
    #[default]
    Eq,
    Ne,
    Cs,
    Cc,
    Mi,
    Pl,
    Vs,
    Vc,
    Hi,
    Ls,
    Ge,
    Lt,
    Gt,
    Le,
    Al,
    Nv,
}

impl ConditionCode {
    pub fn from_bits(b: u32) -> ConditionCode {
        const ALL: [ConditionCode; 16] = [
            ConditionCode::Eq,
            ConditionCode::Ne,
            ConditionCode::Cs,
            ConditionCode::Cc,
            ConditionCode::Mi,
            ConditionCode::Pl,
            ConditionCode::Vs,
            ConditionCode::Vc,
            ConditionCode::Hi,
            ConditionCode::Ls,
            ConditionCode::Ge,
            ConditionCode::Lt,
            ConditionCode::Gt,
            ConditionCode::Le,
            ConditionCode::Al,
            ConditionCode::Nv,
        ];
        ALL[(b & 15) as usize]
    }

    pub fn bits(self) -> u32 {
        self as u32
    }

    pub fn name(self) -> &'static str {
        [
            "EQ", "NE", "CS", "CC", "MI", "PL", "VS", "VC", "HI", "LS", "GE", "LT", "GT", "LE",
            "AL", "NV",
        ][self as usize]
    }

    #[must_use]
    pub fn invert(self) -> ConditionCode {
        ConditionCode::from_bits(self.bits() ^ 1)
    }
}

/// The qualifier of a governing predicate (`P0/Z`, `P0/M`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PredQual {
    #[default]
    None,
    /// `/Z`: inactive elements of the destination are set to zero.
    Zeroing,
    /// `/M`: inactive elements of the destination keep their value.
    Merging,
}

impl PredQual {
    /// The assembler suffix without the slash (`Z`, `M`; empty for none).
    pub fn name(self) -> &'static str {
        match self {
            PredQual::None => "",
            PredQual::Zeroing => "Z",
            PredQual::Merging => "M",
        }
    }
}

/// Addressing mode of a memory operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MemoryMode {
    /// `[base{, offset}]`.
    #[default]
    Offset,
    /// `[base, offset]!`: the base is updated before the access.
    PreIndex,
    /// `[base], offset`: the base is updated after the access (offset may be a
    /// register, see [`Operand::index`]).
    PostIndex,
    /// A PC-relative literal; the operand is a [`OpKind::Label`].
    Literal,
}

/// One decoded operand. Which fields are meaningful depends on [`OpKind`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Operand {
    pub(crate) kind: OpKind,
    /// The register, the list's first register, or a memory base.
    pub(crate) reg: Register,
    /// A memory index register (offset or post-index register).
    pub(crate) index_reg: Register,
    pub(crate) arr: Arrangement,
    /// Register-list length.
    pub(crate) count: u8,
    /// Element index for lanes and indexed lists (`u8::MAX`: none).
    pub(crate) lane: u8,
    pub(crate) shift: ShiftKind,
    pub(crate) extend: ExtendKind,
    /// Shift or extend amount (also for shifted immediates).
    pub(crate) amount: u8,
    /// Whether an amount was written (`UXTW #0` vs `UXTW`).
    pub(crate) has_amount: bool,
    pub(crate) mode: MemoryMode,
    /// Register writeback without an offset (`[X0]!`, `X1!` in `CPY*`/`SET*`).
    pub(crate) writeback: bool,
    /// Immediate, memory offset, label target, condition, system register or
    /// operation encoding, depending on the kind.
    pub(crate) imm: u64,
    /// Static text for [`OpKind::Name`] operands.
    pub(crate) name: &'static str,
    /// A condition printed as a mnemonic suffix (`B.EQ`).
    pub(crate) suffix: bool,
    /// An optional operand at its default value (`RET` for `RET X30`): the
    /// formatter leaves it out.
    pub(crate) elide: bool,
    /// The immediate is a signed value (two's complement in `imm`).
    pub(crate) signed: bool,
    /// Bitmask immediates: the decoded `N:immr:imms` (bit 15 set when present), so
    /// re-encoding keeps `immr` bits the value does not depend on.
    pub(crate) raw_fields: u16,
    /// The qualifier of a governing predicate (`P0/Z`, `P0/M`).
    pub(crate) pred: PredQual,
    /// Register lists: the distance between registers (0 or 1: consecutive; SME2
    /// strided lists `{ Z0.S, Z8.S }`: 8).
    pub(crate) stride: u8,
    /// Memory: the immediate offset is in multiples of the vector length (`#1, MUL
    /// VL`).
    pub(crate) mul_vl: bool,
    /// ZA slices: vertical (`ZA0V.S`) rather than horizontal.
    pub(crate) vertical: bool,
    /// ZA array vectors: the vector group (`VGx2`: 2, `VGx4`: 4; 0: none).
    pub(crate) vg: u8,
    /// The operand is printed in braces as a list of one (`{ ZA0H.B[W12, 0] }`, `{
    /// ZT0 }`).
    pub(crate) braced: bool,
}

impl Operand {
    pub(crate) const NO_LANE: u8 = u8::MAX;

    pub fn kind(&self) -> OpKind {
        self.kind
    }

    pub fn register(&self) -> Register {
        self.reg
    }

    pub fn arrangement(&self) -> Arrangement {
        self.arr
    }

    /// Lane index for `V0.S[1]` and indexed lists.
    pub fn index(&self) -> Option<u8> {
        (self.lane != Self::NO_LANE).then_some(self.lane)
    }

    pub fn list_len(&self) -> u8 {
        self.count
    }

    /// The distance between the registers of a list (1 for consecutive lists).
    pub fn list_stride(&self) -> u8 {
        self.stride.max(1)
    }

    /// Memory: whether the offset is in multiples of the vector length (`MUL VL`).
    pub fn mul_vl(&self) -> bool {
        self.mul_vl
    }

    /// ZA slices: whether the slice is vertical.
    pub fn is_vertical(&self) -> bool {
        self.vertical
    }

    /// ZA array vectors: the vector group (2 for `VGx2`, 4 for `VGx4`, 0 for none).
    pub fn vector_group(&self) -> u8 {
        self.vg
    }

    /// The index register of a memory operand, a ZA slice or array, or `PSEL`'s
    /// predicate.
    pub fn index_register(&self) -> Register {
        self.index_reg
    }

    /// Whether the operand is printed in braces as a list of one.
    pub fn in_braces(&self) -> bool {
        self.braced
    }

    pub fn immediate(&self) -> u64 {
        self.imm
    }

    pub fn shift(&self) -> ShiftKind {
        self.shift
    }

    pub fn extend(&self) -> ExtendKind {
        self.extend
    }

    pub fn amount(&self) -> u8 {
        self.amount
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The predicate qualifier of a governing predicate (`/Z`, `/M`).
    pub fn predicate(&self) -> PredQual {
        self.pred
    }

    pub(crate) fn raw(&self) -> Option<u16> {
        (self.raw_fields >> 15 == 1).then_some(self.raw_fields & 0x7fff)
    }

    pub(crate) fn none() -> Operand {
        Operand {
            lane: Self::NO_LANE,
            ..Operand::default()
        }
    }
}

/// Constructors, for building instructions (`Instruction::with`, the encoder and
/// `CodeAssembler`).
impl Operand {
    /// A register (`X0`, `W1`, `SP`, `D2`, `Q3`, ...), or a whole vector `V0`.
    pub fn reg(r: Register) -> Operand {
        Operand {
            kind: OpKind::Register,
            reg: r,
            ..Operand::none()
        }
    }

    /// A vector with an arrangement: `V0.4S`.
    pub fn vec(r: Register, arr: Arrangement) -> Operand {
        Operand {
            arr,
            ..Operand::reg(r)
        }
    }

    /// A governing predicate with a qualifier: `P0/Z`, `P1/M`.
    pub fn pred(r: Register, q: PredQual) -> Operand {
        Operand {
            pred: q,
            ..Operand::reg(r)
        }
    }

    /// A vector element: `V0.S[1]`.
    pub fn elem(r: Register, es: Arrangement, index: u8) -> Operand {
        Operand {
            arr: es,
            lane: index,
            ..Operand::reg(r)
        }
    }

    /// An element selected by a register and an immediate: `P11.S[W13, 1]` (PSEL).
    pub fn elem_sel(r: Register, es: Arrangement, index_reg: Register, index: u8) -> Operand {
        Operand {
            index_reg,
            ..Operand::elem(r, es, index)
        }
    }

    /// A register list: `{ V0.16B, V1.16B }`.
    pub fn list(first: Register, count: u8, arr: Arrangement) -> Operand {
        Operand {
            kind: OpKind::RegisterList,
            reg: first,
            count,
            arr,
            ..Operand::none()
        }
    }

    /// An indexed register list: `{ V0.S, V1.S }[1]`.
    pub fn list_elem(first: Register, count: u8, es: Arrangement, index: u8) -> Operand {
        Operand {
            lane: index,
            ..Operand::list(first, count, es)
        }
    }

    /// A register with writeback (`X1!` in `CPY*`/`SET*`).
    pub fn reg_wb(r: Register) -> Operand {
        Operand {
            writeback: true,
            ..Operand::reg(r)
        }
    }

    pub fn imm(v: u64) -> Operand {
        Operand {
            kind: OpKind::Immediate,
            imm: v,
            ..Operand::none()
        }
    }

    pub fn simm(v: i64) -> Operand {
        Operand {
            signed: true,
            ..Operand::imm(v as u64)
        }
    }

    /// `#imm, LSL #amount`.
    pub fn imm_lsl(v: u64, amount: u8) -> Operand {
        Operand {
            shift: ShiftKind::Lsl,
            amount,
            has_amount: true,
            ..Operand::imm(v)
        }
    }

    /// `#imm, MSL #amount`.
    pub fn imm_msl(v: u64, amount: u8) -> Operand {
        Operand {
            shift: ShiftKind::Msl,
            amount,
            has_amount: true,
            ..Operand::imm(v)
        }
    }

    /// A logical immediate (the expanded value).
    pub fn bitmask(v: u64) -> Operand {
        Operand {
            kind: OpKind::BitmaskImmediate,
            imm: v,
            ..Operand::none()
        }
    }

    pub fn fp(v: f64) -> Operand {
        Operand {
            kind: OpKind::FpImmediate,
            imm: v.to_bits(),
            ..Operand::none()
        }
    }

    pub fn shifted(r: Register, shift: ShiftKind, amount: u8) -> Operand {
        Operand {
            kind: OpKind::ShiftedRegister,
            reg: r,
            shift,
            amount,
            ..Operand::none()
        }
    }

    pub fn extended(r: Register, extend: ExtendKind, amount: u8) -> Operand {
        Operand {
            kind: OpKind::ExtendedRegister,
            reg: r,
            extend,
            amount,
            has_amount: true,
            ..Operand::none()
        }
    }

    /// `[base]`.
    pub fn mem(base: Register) -> Operand {
        Operand {
            kind: OpKind::Memory,
            reg: base,
            ..Operand::none()
        }
    }

    /// `[base, #offset]`.
    pub fn mem_off(base: Register, offset: i64) -> Operand {
        Operand {
            imm: offset as u64,
            ..Operand::mem(base)
        }
    }

    /// `[base, #offset]!`.
    pub fn mem_pre(base: Register, offset: i64) -> Operand {
        Operand {
            mode: MemoryMode::PreIndex,
            ..Operand::mem_off(base, offset)
        }
    }

    /// `[base], #offset`.
    pub fn mem_post(base: Register, offset: i64) -> Operand {
        Operand {
            mode: MemoryMode::PostIndex,
            ..Operand::mem_off(base, offset)
        }
    }

    /// `[base], Xm`.
    pub fn mem_post_reg(base: Register, index: Register) -> Operand {
        Operand {
            mode: MemoryMode::PostIndex,
            index_reg: index,
            ..Operand::mem(base)
        }
    }

    /// `[base, index{, extend {#amount}}]`; `ExtendKind::Lsl` for `[Xn, Xm, LSL #3]`.
    pub fn mem_index(
        base: Register,
        index: Register,
        extend: ExtendKind,
        amount: Option<u8>,
    ) -> Operand {
        Operand {
            index_reg: index,
            extend,
            amount: amount.unwrap_or(0),
            has_amount: amount.is_some(),
            ..Operand::mem(base)
        }
    }

    /// `[base, #offset, MUL VL]` (SVE): the offset in multiples of the vector length.
    pub fn mem_mul_vl(base: Register, offset: i64) -> Operand {
        Operand {
            mul_vl: true,
            ..Operand::mem_off(base, offset)
        }
    }

    /// `[Zn.D{, #offset}]` (SVE vector base) or `[Xn, Zm.D, SXTW #2]` (vector index):
    /// the element size of the vector base or index.
    pub fn mem_vec(mut m: Operand, es: Arrangement) -> Operand {
        m.arr = es;
        m
    }

    /// A ZA tile: `ZA3.S` is `Operand::za_tile(Register::ZAS3)`.
    pub fn za_tile(tile: Register) -> Operand {
        Operand {
            kind: OpKind::ZaTile,
            reg: tile,
            ..Operand::none()
        }
    }

    /// A ZA tile slice: `ZA1H.S[W12, 3]` is `za_slice(ZAS1, false, W12, 3, 1)`; a
    /// range `[W12, 2:3]` has `count` 2.
    pub fn za_slice(
        tile: Register,
        vertical: bool,
        index: Register,
        offset: u64,
        count: u8,
    ) -> Operand {
        let es = tile.za_tile_info().map_or(0, |t| t.0);
        Operand {
            kind: OpKind::ZaSlice,
            reg: tile,
            arr: ARR_ES[es as usize],
            vertical,
            index_reg: index,
            imm: offset,
            count,
            ..Operand::none()
        }
    }

    /// ZA array vectors: `ZA.D[W8, 1, VGx4]` is `za_array(Arrangement::D, W8, 1, 1,
    /// 4)`; `ZA[W12, 0]` has no element size and no vector group.
    pub fn za_array(es: Arrangement, index: Register, offset: u64, count: u8, vg: u8) -> Operand {
        Operand {
            kind: OpKind::ZaArray,
            reg: Register::ZA,
            arr: es,
            index_reg: index,
            imm: offset,
            count,
            vg,
            ..Operand::none()
        }
    }

    /// The `ZERO` tile mask (bit `n`: `ZA<n>.D`).
    pub fn za_mask(mask: u8) -> Operand {
        Operand {
            kind: OpKind::ZaMask,
            imm: u64::from(mask),
            ..Operand::none()
        }
    }

    /// The operand in braces, as a list of one (`{ ZT0 }`, `{ ZA0H.B[W12, 0] }`).
    #[must_use]
    pub fn braced(self) -> Operand {
        Operand {
            braced: true,
            ..self
        }
    }

    /// A list of `count` registers `stride` apart (SME2 strided lists).
    pub fn list_strided(first: Register, count: u8, stride: u8, arr: Arrangement) -> Operand {
        Operand {
            stride,
            ..Operand::list(first, count, arr)
        }
    }

    /// `[base]!` (`CPY*`/`SET*`).
    pub fn mem_wb(base: Register) -> Operand {
        Operand {
            writeback: true,
            ..Operand::mem(base)
        }
    }

    /// A PC-relative target (an absolute address).
    pub fn label(target: u64) -> Operand {
        Operand {
            kind: OpKind::Label,
            imm: target,
            ..Operand::none()
        }
    }

    pub fn cond(c: ConditionCode) -> Operand {
        Operand {
            kind: OpKind::Condition,
            imm: u64::from(c.bits()),
            ..Operand::none()
        }
    }

    /// A system register by `op0:op1:CRn:CRm:op2` (see [`crate::sys_encoding`]).
    pub fn sysreg(encoding: u16) -> Operand {
        Operand {
            kind: OpKind::SystemRegister,
            imm: u64::from(encoding),
            ..Operand::none()
        }
    }

    /// A system operation (`DC`, `IC`, `AT`, `TLBI` operand) by `op0:op1:CRn:CRm:op2`.
    pub fn sysop(encoding: u16) -> Operand {
        Operand {
            kind: OpKind::SystemOp,
            imm: u64::from(encoding),
            ..Operand::none()
        }
    }

    /// A PSTATE field by `op1:CRm:op2`.
    pub fn pstate(op1: u32, crm: u32, op2: u32) -> Operand {
        Operand {
            kind: OpKind::PState,
            imm: u64::from(op1 << 7 | crm << 3 | op2),
            ..Operand::none()
        }
    }

    /// `Cn`/`Cm` of `SYS`.
    pub fn cr(n: u32) -> Operand {
        Operand {
            kind: OpKind::ControlRegister,
            imm: u64::from(n),
            ..Operand::none()
        }
    }

    /// A barrier option by value (`ISH` is 11).
    pub fn barrier(v: u32) -> Operand {
        Operand {
            kind: OpKind::Barrier,
            imm: u64::from(v),
            ..Operand::none()
        }
    }

    /// A prefetch operation by value (`PLDL1KEEP` is 0).
    pub fn prefetch(v: u32) -> Operand {
        Operand {
            kind: OpKind::Prefetch,
            imm: u64::from(v),
            ..Operand::none()
        }
    }

    /// A named operand by its field value (BTI targets, `CSYNC`, ...).
    pub fn named(v: u32) -> Operand {
        Operand {
            kind: OpKind::Name,
            imm: u64::from(v),
            ..Operand::none()
        }
    }
}

impl From<Register> for Operand {
    fn from(r: Register) -> Operand {
        Operand::reg(r)
    }
}

impl From<u64> for Operand {
    fn from(v: u64) -> Operand {
        Operand::imm(v)
    }
}

impl From<u32> for Operand {
    fn from(v: u32) -> Operand {
        Operand::imm(u64::from(v))
    }
}

impl From<i64> for Operand {
    fn from(v: i64) -> Operand {
        Operand::simm(v)
    }
}

impl From<i32> for Operand {
    fn from(v: i32) -> Operand {
        Operand::simm(i64::from(v))
    }
}

impl From<f64> for Operand {
    fn from(v: f64) -> Operand {
        Operand::fp(v)
    }
}

impl From<ConditionCode> for Operand {
    fn from(c: ConditionCode) -> Operand {
        Operand::cond(c)
    }
}
