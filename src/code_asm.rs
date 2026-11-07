//! `CodeAssembler`: a fluent assembler over the encoder, in the manner of
//! `iced_x86::code_asm` (`a.adds(X0, X1, X2)?`).
//!
//! Each mnemonic method tries every encoding with that mnemonic, and every alias
//! (`MOV`, `CMP`, `LSL`, ...), and keeps the first that encodes the operands with
//! the fewest operand conversions (a register where a shifted or extended register
//! is expected, an integer for a bitmask immediate). An alias is kept only when the
//! word decodes back to that same alias. The per-mnemonic methods are generated
//! (`generated/code_asm.rs`).

extern crate alloc;

use alloc::vec::Vec;

pub use crate::generated::code_asm::registers;

use crate::DecoderOptions;
use crate::decoder::decode_word;
use crate::desc::{self, Op};
use crate::encoder::{EncodeError, encode_ops};
use crate::generated::tables::ENCODINGS;
use crate::generated::{Code, Mnemonic};
use crate::operand::{ExtendKind, OpKind, Operand, ShiftKind};

/// A fluent assembler. Instructions are placed at consecutive addresses from the
/// start PC; label operands are absolute addresses.
#[derive(Debug, Clone)]
pub struct CodeAssembler {
    start: u64,
    words: Vec<u32>,
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

/// Converts operands for a descriptor list; `None` when the counts differ.
fn fit(ops: &[Op], operands: &[Operand]) -> Option<(Vec<Operand>, u32)> {
    let descs: Vec<&Op> = ops.iter().filter(|o| !matches!(o, Op::Suffix(_))).collect();
    if descs.len() != operands.len() {
        return None;
    }
    let mut cost = 0;
    let mut out = Vec::with_capacity(operands.len());
    for (d, o) in descs.iter().zip(operands) {
        let (c, k) = convert(d, o)?;
        cost += k;
        out.push(c);
    }
    Some((out, cost))
}

/// The suffix-table index of `suffix` (`"2"`, `"B"`, `"T"`; `""` for none).
fn suffix_index(ops: &[Op], suffix: &str) -> Option<u8> {
    match desc::suffix_table(ops) {
        Some(t) => t.names.iter().position(|n| *n == suffix).map(|i| i as u8),
        None => suffix.is_empty().then_some(0),
    }
}

impl CodeAssembler {
    pub fn new(start: u64) -> Self {
        CodeAssembler {
            start,
            words: Vec::new(),
        }
    }

    /// The address of the next instruction.
    pub fn pc(&self) -> u64 {
        self.start + 4 * self.words.len() as u64
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// The code as little-endian bytes.
    pub fn assemble(&self) -> Vec<u8> {
        self.words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// Appends a raw word.
    pub fn word(&mut self, w: u32) -> &mut Self {
        self.words.push(w);
        self
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
            if d.mnemonic == mnemonic {
                if let Some(ops) = d.ops {
                    if let (Some((conv, cost)), Some(sfx)) =
                        (fit(ops, operands), suffix_index(ops, suffix))
                    {
                        let (pm, pv) = crate::undefined::pinned(d);
                        match encode_ops(
                            ops,
                            &conv,
                            sfx,
                            pc,
                            d.value | d.sb_value | pv,
                            d.mask | d.sb_mask | pm,
                        ) {
                            Ok(w)
                                if !crate::undefined::undefined(d, w)
                                    && decode_word(w, pc, &DecoderOptions::default()).code()
                                        == code =>
                            {
                                if best.is_none_or(|(c, _)| cost < c) {
                                    best = Some((cost, w));
                                }
                            }
                            Ok(_) => {}
                            Err(e) => last_err = e,
                        }
                    }
                }
            }
            for (k, a) in d.aliases.iter().enumerate() {
                if a.mnemonic != mnemonic {
                    continue;
                }
                let Some(ops) = a.ops else { continue };
                let (Some((conv, cost)), Some(sfx)) =
                    (fit(ops, operands), suffix_index(ops, suffix))
                else {
                    continue;
                };
                let (fm, fv) = a.fixed;
                let (pm, pv) = crate::undefined::pinned(d);
                let base = d.value | d.sb_value | fv | pv;
                let mask = d.mask | d.sb_mask | fm | pm;
                match encode_ops(ops, &conv, sfx, pc, base, mask) {
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
                                    .is_some_and(|n| same_operands(&got[..n as usize], &conv));
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
        self.words.push(w);
        Ok(self)
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
