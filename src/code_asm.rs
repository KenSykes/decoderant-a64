//! `CodeAssembler`: a fluent assembler over the encoder, in the manner of
//! `iced_x86::code_asm` (`a.adds(X0, X1, X2)?`).
//!
//! Each mnemonic method tries every encoding with that mnemonic, and every alias
//! (`MOV`, `CMP`, `LSL`, ...), and keeps the first that encodes the operands with
//! the fewest operand conversions (a register where a shifted or extended register
//! is expected, an integer for a bitmask immediate). An alias is kept only when the
//! word decodes back to that same alias. The per-mnemonic methods are generated
//! (`generated/code_asm.rs`).
//!
//! The assembler stores its words in a [`CodeBuffer`]: a `Vec<u32>` (the `alloc`
//! feature, and what [`CodeAssembler::new`] uses), or a [`FixedBuffer`] or
//! [`SliceBuffer`] when there is no allocator ([`CodeAssembler::with_buffer`]).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub use crate::generated::code_asm::registers;

use crate::DecoderOptions;
use crate::MAX_OPERANDS;
use crate::decoder::decode_word;
use crate::desc::{self, Op};
use crate::encoder::{EncodeError, encode_ops};
use crate::generated::tables::ENCODINGS;
use crate::generated::{Code, Mnemonic};
use crate::operand::{ExtendKind, OpKind, Operand, ShiftKind};

/// Where a [`CodeAssembler`] keeps its words.
pub trait CodeBuffer {
    /// Appends a word; [`EncodeError::BufferFull`] when there is no room.
    fn push(&mut self, word: u32) -> Result<(), EncodeError>;
    /// The words so far.
    fn words(&self) -> &[u32];
}

#[cfg(feature = "alloc")]
impl CodeBuffer for Vec<u32> {
    fn push(&mut self, word: u32) -> Result<(), EncodeError> {
        Vec::push(self, word);
        Ok(())
    }

    fn words(&self) -> &[u32] {
        self
    }
}

/// A buffer of up to `N` words, held inline.
#[derive(Debug, Clone, Copy)]
pub struct FixedBuffer<const N: usize> {
    words: [u32; N],
    len: usize,
}

impl<const N: usize> FixedBuffer<N> {
    pub const fn new() -> Self {
        FixedBuffer {
            words: [0; N],
            len: 0,
        }
    }
}

impl<const N: usize> Default for FixedBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> CodeBuffer for FixedBuffer<N> {
    fn push(&mut self, word: u32) -> Result<(), EncodeError> {
        *self
            .words
            .get_mut(self.len)
            .ok_or(EncodeError::BufferFull)? = word;
        self.len += 1;
        Ok(())
    }

    fn words(&self) -> &[u32] {
        &self.words[..self.len]
    }
}

/// A buffer over a caller's slice.
#[derive(Debug)]
pub struct SliceBuffer<'a> {
    words: &'a mut [u32],
    len: usize,
}

impl<'a> SliceBuffer<'a> {
    pub fn new(words: &'a mut [u32]) -> Self {
        SliceBuffer { words, len: 0 }
    }
}

impl CodeBuffer for SliceBuffer<'_> {
    fn push(&mut self, word: u32) -> Result<(), EncodeError> {
        *self
            .words
            .get_mut(self.len)
            .ok_or(EncodeError::BufferFull)? = word;
        self.len += 1;
        Ok(())
    }

    fn words(&self) -> &[u32] {
        &self.words[..self.len]
    }
}

// With `alloc` the buffer defaults to a `Vec`, so `CodeAssembler` alone names the
// type `CodeAssembler::new` makes. Without it there is no default.

/// A fluent assembler. Instructions are placed at consecutive addresses from the
/// start PC; label operands are absolute addresses.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct CodeAssembler<B = Vec<u32>> {
    start: u64,
    buf: B,
}

/// A fluent assembler. Instructions are placed at consecutive addresses from the
/// start PC; label operands are absolute addresses.
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct CodeAssembler<B> {
    start: u64,
    buf: B,
}

/// How far an operand had to be converted to fit a descriptor (0: as given).
fn convert(op: &Op, o: &Operand) -> Option<(Operand, u32)> {
    let inner = match op {
        Op::Opt(i, ..) => *i,
        _ => op,
    };
    Some(match (inner, o.kind) {
        (Op::Shifted(..), OpKind::Register) => (Operand::shifted(o.reg, ShiftKind::Lsl, 0), 1),
        (Op::Extended(..), OpKind::Register) => (Operand::extended(o.reg, ExtendKind::Lsl, 0), 2),
        (Op::BitImm(_), OpKind::Immediate) => (Operand::bitmask(o.imm), 1),
        _ => (*o, 0),
    })
}

/// Operands converted for a descriptor list.
struct Fitted {
    ops: [Operand; MAX_OPERANDS],
    len: usize,
    cost: u32,
}

impl Fitted {
    fn ops(&self) -> &[Operand] {
        &self.ops[..self.len]
    }
}

/// Converts operands for a descriptor list; `None` when the counts differ.
fn fit(ops: &[Op], operands: &[Operand]) -> Option<Fitted> {
    let mut out = Fitted {
        ops: [Operand::none(); MAX_OPERANDS],
        len: 0,
        cost: 0,
    };
    let mut descs = ops.iter().filter(|o| !matches!(o, Op::Suffix(_)));
    for o in operands {
        let (c, k) = convert(descs.next()?, o)?;
        *out.ops.get_mut(out.len)? = c;
        out.len += 1;
        out.cost += k;
    }
    descs.next().is_none().then_some(out)
}

/// The suffix-table index of `suffix` (`"2"`, `"B"`, `"T"`; `""` for none).
fn suffix_index(ops: &[Op], suffix: &str) -> Option<u8> {
    match desc::suffix_table(ops) {
        Some(t) => t.names.iter().position(|n| *n == suffix).map(|i| i as u8),
        None => suffix.is_empty().then_some(0),
    }
}

#[cfg(feature = "alloc")]
impl CodeAssembler<Vec<u32>> {
    pub fn new(start: u64) -> Self {
        Self::with_buffer(start, Vec::new())
    }

    /// The code as little-endian bytes.
    pub fn assemble(&self) -> Vec<u8> {
        self.bytes().collect()
    }
}

impl<B: CodeBuffer> CodeAssembler<B> {
    /// An assembler that keeps its words in `buf`.
    pub fn with_buffer(start: u64, buf: B) -> Self {
        CodeAssembler { start, buf }
    }

    /// The address of the next instruction.
    pub fn pc(&self) -> u64 {
        self.start + 4 * self.buf.words().len() as u64
    }

    pub fn words(&self) -> &[u32] {
        self.buf.words()
    }

    /// The code as little-endian bytes.
    pub fn bytes(&self) -> impl Iterator<Item = u8> + '_ {
        self.buf.words().iter().flat_map(|w| w.to_le_bytes())
    }

    /// Writes the code as little-endian bytes to `out`; the length, or `None` when
    /// `out` is too short.
    pub fn assemble_into(&self, out: &mut [u8]) -> Option<usize> {
        let n = self.buf.words().len() * 4;
        let out = out.get_mut(..n)?;
        for (chunk, w) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.buf.words()) {
            *chunk = w.to_le_bytes();
        }
        Some(n)
    }

    /// Appends a raw word.
    pub fn word(&mut self, w: u32) -> Result<&mut Self, EncodeError> {
        self.buf.push(w)?;
        Ok(self)
    }

    /// Encodes `mnemonic` (with a mnemonic suffix such as `"2"`, or `""`) and the
    /// operands, without appending.
    pub fn encode(
        &self,
        mnemonic: Mnemonic,
        suffix: &str,
        operands: &[Operand],
    ) -> Result<u32, EncodeError> {
        let pc = self.pc();
        let mut best: Option<(u32, u32)> = None;
        let mut last_err = EncodeError::NoOperandSupport(Code::Invalid);
        for (i, d) in ENCODINGS.iter().enumerate() {
            let code = Code::from_index(i + 1).unwrap_or(Code::Invalid);
            if d.mnemonic == mnemonic
                && let Some(ops) = d.ops
                && let (Some(fitted), Some(sfx)) = (fit(ops, operands), suffix_index(ops, suffix))
            {
                let (conv, cost) = (fitted.ops(), fitted.cost);
                let (pm, pv) = crate::undefined::pinned(d);
                match encode_ops(
                    ops,
                    conv,
                    sfx,
                    pc,
                    d.value | d.sb_value | pv,
                    d.mask | d.sb_mask | pm,
                ) {
                    Ok(w)
                        if !crate::undefined::undefined(d, w)
                            && decode_word(w, pc, &DecoderOptions::default()).code() == code =>
                    {
                        if best.is_none_or(|(c, _)| cost < c) {
                            best = Some((cost, w));
                        }
                    }
                    Ok(_) => {}
                    Err(e) => last_err = e,
                }
            }
            for (k, a) in d.aliases.iter().enumerate() {
                if a.mnemonic != mnemonic {
                    continue;
                }
                let Some(ops) = a.ops else { continue };
                let (Some(fitted), Some(sfx)) = (fit(ops, operands), suffix_index(ops, suffix))
                else {
                    continue;
                };
                let (conv, cost) = (fitted.ops(), fitted.cost);
                let (fm, fv) = a.fixed;
                let (pm, pv) = crate::undefined::pinned(d);
                let base = d.value | d.sb_value | fv | pv;
                let mask = d.mask | d.sb_mask | fm | pm;
                match encode_ops(ops, conv, sfx, pc, base, mask) {
                    Ok(w0) => {
                        // Fields the condition makes equal: copy one way, then the other.
                        let mut tries = [w0, w0];
                        for &(f1, f2) in a.copies {
                            let (mut x, mut y) = (tries[0], tries[1]);
                            let (vx, vy) = (f2.get(x), f1.get(y));
                            f1.set(&mut x, vx);
                            f2.set(&mut y, vy);
                            tries = [x, y];
                        }
                        for w in tries {
                            let back = decode_word(w, pc, &DecoderOptions::default());
                            // The word must decode to this encoding with this alias
                            // preferred, and the alias operands must read back.
                            let mut got = [Operand::none(); crate::MAX_OPERANDS];
                            let same = back.code() == code
                                && back.alias as usize == k + 1
                                && desc::decode_ops(ops, w, pc, &mut got)
                                    .is_some_and(|n| same_operands(&got[..n as usize], conv));
                            if same && best.is_none_or(|(c, _)| cost < c) {
                                best = Some((cost, w));
                            }
                        }
                    }
                    Err(e) => last_err = e,
                }
            }
        }
        best.map(|(_, w)| w).ok_or(last_err)
    }

    /// Encodes and appends.
    pub fn emit(
        &mut self,
        mnemonic: Mnemonic,
        suffix: &str,
        operands: &[Operand],
    ) -> Result<&mut Self, EncodeError> {
        let w = self.encode(mnemonic, suffix, operands)?;
        self.word(w)
    }
}

/// Operand equality for the alias check, ignoring decode-only details.
fn same_operands(a: &[Operand], b: &[Operand]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.kind == y.kind
                && x.reg == y.reg
                && x.imm == y.imm
                && x.arr == y.arr
                && x.index_reg == y.index_reg
                && (x.kind != OpKind::RegisterList || x.count == y.count)
                && (x.lane == y.lane)
        })
}
