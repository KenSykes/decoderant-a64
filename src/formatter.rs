//! The formatter: Arm assembler syntax, with the spec's preferred aliases.

use core::fmt::{self, Write};

use crate::desc;
use crate::generated::{Mnemonic, Register};
use crate::instruction::{Instruction, MAX_OPERANDS};
use crate::operand::{ConditionCode, ExtendKind, MemoryMode, OpKind, Operand, PredQual, ShiftKind};
use crate::regs::RegisterClass;
use crate::sysreg::{SysKind, lookup};

/// Formatter options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatterOptions {
    /// Print the spec's preferred aliases (`MOV`, `CMP`, `LSL`, ...). Default: true.
    pub preferred_aliases: bool,
    /// Immediates of 10 and above in hex. Default: true.
    pub hex_immediates: bool,
    /// Upper-case mnemonics, registers and names. Default: false.
    pub uppercase: bool,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        FormatterOptions {
            preferred_aliases: true,
            hex_immediates: true,
            uppercase: false,
        }
    }
}

/// What a piece of formatted text is, for coloured or structured output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatterTextKind {
    Mnemonic,
    Text,
    Punctuation,
    Register,
    Number,
    Label,
    Keyword,
}

/// Receives formatted text.
pub trait FormatterOutput {
    fn write(&mut self, text: &str, kind: FormatterTextKind);
}

#[cfg(feature = "std")]
impl FormatterOutput for std::string::String {
    fn write(&mut self, text: &str, _kind: FormatterTextKind) {
        self.push_str(text);
    }
}

/// Adapts a [`core::fmt::Write`] (a fixed buffer, a `String`) as output.
pub struct FmtOutput<'a, W: Write>(pub &'a mut W);

impl<W: Write> FormatterOutput for FmtOutput<'_, W> {
    fn write(&mut self, text: &str, _kind: FormatterTextKind) {
        let _ = self.0.write_str(text);
    }
}

/// Formats [`Instruction`]s.
#[derive(Debug, Clone, Copy, Default)]
pub struct Formatter {
    pub options: FormatterOptions,
}

/// A small stack buffer for one token.
struct Buf {
    b: [u8; 64],
    n: usize,
}

impl Buf {
    fn new() -> Buf {
        Buf { b: [0; 64], n: 0 }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.b[..self.n]).unwrap_or("")
    }
}

impl Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let end = (self.n + bytes.len()).min(self.b.len());
        self.b[self.n..end].copy_from_slice(&bytes[..end - self.n]);
        self.n = end;
        Ok(())
    }
}

struct Out<'a> {
    out: &'a mut dyn FormatterOutput,
    upper: bool,
}

impl Out<'_> {
    fn put(&mut self, s: &str, kind: FormatterTextKind) {
        let mut b = Buf::new();
        for c in s.chars() {
            let c = if self.upper {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            let _ = b.write_char(c);
        }
        self.out.write(b.as_str(), kind);
    }

    fn raw(&mut self, s: &str, kind: FormatterTextKind) {
        self.out.write(s, kind);
    }
}

impl Formatter {
    pub fn new() -> Self {
        Formatter::default()
    }

    pub fn with_options(options: FormatterOptions) -> Self {
        Formatter { options }
    }

    /// Formats `insn`; an invalid instruction is `.inst 0x...`.
    pub fn format(&self, insn: &Instruction, output: &mut dyn FormatterOutput) {
        let mut o = Out {
            out: output,
            upper: self.options.uppercase,
        };
        if insn.is_invalid() || !insn.has_operands {
            self.raw_word(insn, &mut o);
            return;
        }
        let d = insn.op_code().data().expect("valid code has data");
        let mut ops = [Operand::none(); MAX_OPERANDS];
        let (mnemonic, suffix, n) = match (self.options.preferred_aliases, insn.alias) {
            (true, a) if a > 0 => {
                let al = &d.aliases[a as usize - 1];
                let descs = al.ops.unwrap_or(&[]);
                match desc::decode_ops(descs, insn.word, insn.pc, &mut ops) {
                    Some(n) => (al.mnemonic, desc::suffix(descs, insn.word), n as usize),
                    None => {
                        ops = insn.ops;
                        (
                            insn.mnemonic(),
                            desc::suffix(d.ops.unwrap_or(&[]), insn.word),
                            insn.op_count(),
                        )
                    }
                }
            }
            _ => {
                ops = insn.ops;
                (
                    insn.mnemonic(),
                    desc::suffix(d.ops.unwrap_or(&[]), insn.word),
                    insn.op_count(),
                )
            }
        };
        let ops = &ops[..n];
        o.put(mnemonic.name(), FormatterTextKind::Mnemonic);
        if !suffix.is_empty() {
            o.put(suffix, FormatterTextKind::Mnemonic);
        }
        let mut first = true;
        for (i, op) in ops.iter().enumerate() {
            if op.kind == OpKind::Condition && op.suffix {
                o.raw(".", FormatterTextKind::Mnemonic);
                o.put(
                    ConditionCode::from_bits(op.imm as u32).name(),
                    FormatterTextKind::Mnemonic,
                );
                continue;
            }
            if op.elide {
                continue;
            }
            o.raw(
                if first { " " } else { ", " },
                FormatterTextKind::Punctuation,
            );
            first = false;
            self.operand(op, ops, i, mnemonic, &mut o);
        }
    }

    fn raw_word(&self, insn: &Instruction, o: &mut Out<'_>) {
        let mut b = Buf::new();
        let _ = write!(b, ".inst {:#010x}", insn.word);
        o.put(b.as_str(), FormatterTextKind::Keyword);
    }

    fn number(&self, v: u64, signed: bool, o: &mut Out<'_>) {
        let mut b = Buf::new();
        let neg = signed && (v as i64) < 0;
        let mag = if neg { (v as i64).unsigned_abs() } else { v };
        if neg {
            let _ = b.write_char('-');
        }
        if self.options.hex_immediates && mag >= 10 {
            let _ = write!(b, "{mag:#x}");
        } else {
            let _ = write!(b, "{mag}");
        }
        o.put(b.as_str(), FormatterTextKind::Number);
    }

    fn imm(&self, v: u64, signed: bool, o: &mut Out<'_>) {
        o.raw("#", FormatterTextKind::Punctuation);
        self.number(v, signed, o);
    }

    fn reg(&self, r: Register, o: &mut Out<'_>) {
        o.put(r.name(), FormatterTextKind::Register);
    }

    fn operand(
        &self,
        op: &Operand,
        all: &[Operand],
        _i: usize,
        mnemonic: Mnemonic,
        o: &mut Out<'_>,
    ) {
        // `{ ZT0 }`, `{ ZA0H.B[W12, 0] }`: one operand printed as a list.
        if op.braced {
            o.raw("{ ", FormatterTextKind::Punctuation);
            self.operand(
                &Operand {
                    braced: false,
                    ..*op
                },
                all,
                _i,
                mnemonic,
                o,
            );
            o.raw(" }", FormatterTextKind::Punctuation);
            return;
        }
        match op.kind {
            OpKind::Register => {
                self.reg(op.reg, o);
                if !op.arr.name().is_empty() {
                    o.raw(".", FormatterTextKind::Punctuation);
                    o.put(op.arr.name(), FormatterTextKind::Keyword);
                }
                if op.pred != PredQual::None {
                    o.raw("/", FormatterTextKind::Punctuation);
                    o.put(op.pred.name(), FormatterTextKind::Keyword);
                }
                if let Some(l) = op.index() {
                    o.raw("[", FormatterTextKind::Punctuation);
                    // `P2.B[W12, 3]` (PSEL).
                    if op.index_reg != Register::None {
                        self.reg(op.index_reg, o);
                        o.raw(", ", FormatterTextKind::Punctuation);
                    }
                    self.number(u64::from(l), false, o);
                    // `ZT0[1, MUL VL]` (MOVT).
                    if op.mul_vl {
                        o.raw(", ", FormatterTextKind::Punctuation);
                        o.put("MUL VL", FormatterTextKind::Keyword);
                    }
                    o.raw("]", FormatterTextKind::Punctuation);
                }
                if op.writeback {
                    o.raw("!", FormatterTextKind::Punctuation);
                }
            }
            OpKind::RegisterList => {
                o.raw("{ ", FormatterTextKind::Punctuation);
                let n = op.reg.number();
                let (count, stride) = (u32::from(op.count), u32::from(op.list_stride()));
                let elem = |k: u32| match op.reg.class() {
                    RegisterClass::Z => Register::z((n + k * stride) % 32),
                    RegisterClass::P => Register::p((n + k * stride) % 16),
                    RegisterClass::PN => Register::pn((n + k * stride) % 16),
                    _ => Register::v((n + k) % 32),
                };
                let sized = |r: Register, o: &mut Out<'_>| {
                    self.reg(r, o);
                    if !op.arr.name().is_empty() {
                        o.raw(".", FormatterTextKind::Punctuation);
                        o.put(op.arr.name(), FormatterTextKind::Keyword);
                    }
                };
                // SVE and SME: three or more consecutive registers as a range
                // (`{ Z0.S-Z3.S }`), unless the list wraps from Z31 to Z0.
                let sve = op.reg.class() != RegisterClass::V;
                if sve && count >= 3 && stride == 1 && n + count <= 32 {
                    sized(elem(0), o);
                    o.raw("-", FormatterTextKind::Punctuation);
                    sized(elem(count - 1), o);
                } else {
                    for k in 0..count {
                        if k > 0 {
                            o.raw(", ", FormatterTextKind::Punctuation);
                        }
                        sized(elem(k), o);
                    }
                }
                o.raw(" }", FormatterTextKind::Punctuation);
                if let Some(l) = op.index() {
                    o.raw("[", FormatterTextKind::Punctuation);
                    self.number(u64::from(l), false, o);
                    o.raw("]", FormatterTextKind::Punctuation);
                }
            }
            OpKind::Immediate => {
                // `MUL #3` after an SVE pattern.
                if !op.name.is_empty() {
                    o.put(op.name, FormatterTextKind::Keyword);
                    o.raw(" ", FormatterTextKind::Text);
                }
                self.imm(op.imm, op.signed, o);
                if op.has_amount && (op.amount != 0 || op.shift == ShiftKind::Msl) {
                    o.raw(", ", FormatterTextKind::Punctuation);
                    o.put(op.shift.name(), FormatterTextKind::Keyword);
                    o.raw(" ", FormatterTextKind::Text);
                    self.imm(u64::from(op.amount), false, o);
                }
            }
            OpKind::BitmaskImmediate => {
                let mut b = Buf::new();
                let _ = write!(b, "#{:#x}", op.imm);
                o.put(b.as_str(), FormatterTextKind::Number);
            }
            OpKind::FpImmediate => {
                let mut b = Buf::new();
                let _ = write!(b, "#{:?}", f64::from_bits(op.imm));
                o.put(b.as_str(), FormatterTextKind::Number);
            }
            OpKind::ShiftedRegister => {
                self.reg(op.reg, o);
                if !(op.shift == ShiftKind::Lsl && op.amount == 0) {
                    o.raw(", ", FormatterTextKind::Punctuation);
                    o.put(op.shift.name(), FormatterTextKind::Keyword);
                    o.raw(" ", FormatterTextKind::Text);
                    self.imm(u64::from(op.amount), false, o);
                }
            }
            OpKind::ExtendedRegister => {
                self.reg(op.reg, o);
                // LSL is preferred where the destination or first source is the
                // stack pointer and the extend is the identity for the width.
                let wide = all.first().is_some_and(|d| d.reg.size() == 8);
                let sp = all.iter().take(2).any(|x| x.reg.is_sp());
                let identity = if wide {
                    op.extend == ExtendKind::Uxtx
                } else {
                    op.extend == ExtendKind::Uxtw
                };
                if sp && identity {
                    if op.amount != 0 {
                        o.raw(", ", FormatterTextKind::Punctuation);
                        o.put("LSL", FormatterTextKind::Keyword);
                        o.raw(" ", FormatterTextKind::Text);
                        self.imm(u64::from(op.amount), false, o);
                    }
                } else {
                    o.raw(", ", FormatterTextKind::Punctuation);
                    o.put(op.extend.name(), FormatterTextKind::Keyword);
                    if op.amount != 0 {
                        o.raw(" ", FormatterTextKind::Text);
                        self.imm(u64::from(op.amount), false, o);
                    }
                }
            }
            OpKind::Memory => self.memory(op, o),
            OpKind::Label => {
                let mut b = Buf::new();
                let _ = write!(b, "{:#x}", op.imm);
                o.put(b.as_str(), FormatterTextKind::Label);
            }
            OpKind::Condition => {
                o.put(
                    ConditionCode::from_bits(op.imm as u32).name(),
                    FormatterTextKind::Keyword,
                );
            }
            OpKind::SystemRegister => {
                let enc = op.imm as u16;
                let kind = if matches!(mnemonic, Mnemonic::Mrs | Mnemonic::Mrrs) {
                    SysKind::Mrs
                } else {
                    SysKind::Msr
                };
                let kind = if matches!(mnemonic, Mnemonic::Mrrs) {
                    SysKind::Mrrs
                } else if matches!(mnemonic, Mnemonic::Msrr) {
                    SysKind::Msrr
                } else {
                    kind
                };
                match lookup(kind, enc).filter(|e| !e.name.contains('<')) {
                    Some(e) => o.put(e.name, FormatterTextKind::Register),
                    None => {
                        let mut b = Buf::new();
                        let _ = write!(
                            b,
                            "s{}_{}_c{}_c{}_{}",
                            enc >> 14,
                            enc >> 11 & 7,
                            enc >> 7 & 15,
                            enc >> 3 & 15,
                            enc & 7
                        );
                        o.put(b.as_str(), FormatterTextKind::Register);
                    }
                }
            }
            OpKind::PState => {
                // op1:CRm:op2 → the MSR (immediate) table's op0:op1:CRn:CRm:op2.
                let v = op.imm as u32;
                let enc = crate::sysreg::sys_encoding(0, v >> 7 & 7, 4, v >> 3 & 15, v & 7);
                match lookup(SysKind::MsrImm, enc) {
                    Some(e) => o.put(e.name, FormatterTextKind::Register),
                    None => {
                        let mut b = Buf::new();
                        let _ = write!(b, "#{:#x}", v);
                        o.put(b.as_str(), FormatterTextKind::Number);
                    }
                }
            }
            OpKind::ControlRegister => {
                let mut b = Buf::new();
                let _ = write!(b, "c{}", op.imm);
                o.put(b.as_str(), FormatterTextKind::Register);
            }
            OpKind::Barrier | OpKind::Prefetch => {
                if op.name.is_empty() {
                    self.imm(op.imm, false, o);
                } else {
                    o.put(op.name, FormatterTextKind::Keyword);
                }
            }
            OpKind::Name => o.put(op.name, FormatterTextKind::Keyword),
            OpKind::SystemOp => {
                let kind = if mnemonic == Mnemonic::Sysl {
                    SysKind::Sysl
                } else if matches!(mnemonic, Mnemonic::Tlbip | Mnemonic::Sysp) {
                    SysKind::Sysp
                } else {
                    SysKind::Sys
                };
                match lookup(kind, op.imm as u16)
                    .filter(|e| !e.name.is_empty() && !e.name.contains('<'))
                {
                    Some(e) => o.put(e.name, FormatterTextKind::Keyword),
                    None => self.imm(op.imm, false, o),
                }
            }
            OpKind::ZaTile => self.reg(op.reg, o),
            OpKind::ZaSlice => {
                let (es, n) = op.reg.za_tile_info().unwrap_or((0, 0));
                let mut b = Buf::new();
                let hv = if op.vertical { 'v' } else { 'h' };
                let _ = write!(
                    b,
                    "za{n}{hv}.{}",
                    crate::operand::ARR_ES[es as usize].name()
                );
                o.put(b.as_str(), FormatterTextKind::Register);
                self.za_index(op, o);
            }
            OpKind::ZaArray => {
                o.put("za", FormatterTextKind::Register);
                if !op.arr.name().is_empty() {
                    o.raw(".", FormatterTextKind::Punctuation);
                    o.put(op.arr.name(), FormatterTextKind::Keyword);
                }
                self.za_index(op, o);
            }
            OpKind::ZaMask => {
                o.raw("{", FormatterTextKind::Punctuation);
                let mut first = true;
                for t in za_mask_tiles(op.imm as u8) {
                    o.raw(
                        if first { " " } else { ", " },
                        FormatterTextKind::Punctuation,
                    );
                    first = false;
                    self.reg(t, o);
                }
                o.raw(" }", FormatterTextKind::Punctuation);
            }
            OpKind::None => {}
        }
    }

    /// `[W12, 3]`, `[W8, 0:1, VGx2]`: the index register, the offset or offset range
    /// (as the template writes them, without `#`), and the vector group.
    fn za_index(&self, op: &Operand, o: &mut Out<'_>) {
        o.raw("[", FormatterTextKind::Punctuation);
        self.reg(op.index_reg, o);
        o.raw(", ", FormatterTextKind::Punctuation);
        self.number(op.imm, false, o);
        if op.count > 1 {
            o.raw(":", FormatterTextKind::Punctuation);
            self.number(op.imm + u64::from(op.count) - 1, false, o);
        }
        if op.vg != 0 {
            let mut b = Buf::new();
            let _ = write!(b, ", VGx{}", op.vg);
            o.put(b.as_str(), FormatterTextKind::Keyword);
        }
        o.raw("]", FormatterTextKind::Punctuation);
    }

    /// A vector base or index of an SVE memory operand carries the element size.
    fn mem_reg(&self, r: Register, m: &Operand, o: &mut Out<'_>) {
        self.reg(r, o);
        if r.class() == RegisterClass::Z && !m.arr.name().is_empty() {
            o.raw(".", FormatterTextKind::Punctuation);
            o.put(m.arr.name(), FormatterTextKind::Keyword);
        }
    }

    fn memory(&self, m: &Operand, o: &mut Out<'_>) {
        o.raw("[", FormatterTextKind::Punctuation);
        self.mem_reg(m.reg, m, o);
        let off = m.imm as i64;
        match m.mode {
            MemoryMode::PostIndex => {
                o.raw("], ", FormatterTextKind::Punctuation);
                if m.index_reg != Register::None {
                    self.reg(m.index_reg, o);
                } else {
                    self.imm(m.imm, true, o);
                }
                return;
            }
            MemoryMode::PreIndex => {
                o.raw(", ", FormatterTextKind::Punctuation);
                self.imm(m.imm, true, o);
                o.raw("]!", FormatterTextKind::Punctuation);
                return;
            }
            _ => {}
        }
        if m.index_reg != Register::None {
            o.raw(", ", FormatterTextKind::Punctuation);
            self.mem_reg(m.index_reg, m, o);
            let lsl = m.extend == ExtendKind::Lsl;
            if !(lsl && !m.has_amount) {
                o.raw(", ", FormatterTextKind::Punctuation);
                o.put(m.extend.name(), FormatterTextKind::Keyword);
                if m.has_amount {
                    o.raw(" ", FormatterTextKind::Text);
                    self.imm(u64::from(m.amount), false, o);
                }
            }
        } else if off != 0 {
            o.raw(", ", FormatterTextKind::Punctuation);
            self.imm(m.imm, true, o);
            if m.mul_vl {
                o.raw(", ", FormatterTextKind::Punctuation);
                o.put("MUL VL", FormatterTextKind::Keyword);
            }
        }
        o.raw("]", FormatterTextKind::Punctuation);
        if m.writeback {
            o.raw("!", FormatterTextKind::Punctuation);
        }
    }
}

/// The tiles a `ZERO` mask names, as the largest tiles that cover it: `{ ZA }` for
/// all, else the halfword tiles (`ZA0.H` is the doubleword tiles 0, 2, 4, 6), word
/// tiles (`ZA1.S` is 1 and 5), then doubleword tiles.
fn za_mask_tiles(mask: u8) -> impl Iterator<Item = Register> {
    let mut out = [Register::None; 8];
    let mut n = 0;
    let mut left = mask;
    if mask == 0xff {
        out[0] = Register::ZA;
        n = 1;
        left = 0;
    }
    for (es, unit, count) in [(1u32, 0x55u8, 2u32), (2, 0x11, 4)] {
        for t in 0..count {
            let m = unit << t;
            if left & m == m {
                out[n] = Register::za_tile(es, t).unwrap_or(Register::None);
                n += 1;
                left &= !m;
            }
        }
    }
    for t in 0..8 {
        if left >> t & 1 == 1 {
            out[n] = Register::za_tile(3, t).unwrap_or(Register::None);
            n += 1;
        }
    }
    out.into_iter().take(n)
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Buf2 { f, err: false };
        Formatter::new().format(self, &mut s);
        if s.err { Err(fmt::Error) } else { Ok(()) }
    }
}

struct Buf2<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
    err: bool,
}

impl FormatterOutput for Buf2<'_, '_> {
    fn write(&mut self, text: &str, _kind: FormatterTextKind) {
        if self.f.write_str(text).is_err() {
            self.err = true;
        }
    }
}
