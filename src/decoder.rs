//! The A64 decoder: walks the generated decode tree.

use crate::cond::{Ctx, holds};
use crate::feature_set::FeatureSet;
use crate::generated::Code;
use crate::generated::tables::{CHILDREN, ENCODINGS, NODES, ROOT};
use crate::instruction::Instruction;
use crate::tables::NO_COND;

/// Decoder options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecoderOptions {
    /// Encodings of features outside the set decode as if removed from the decode
    /// tree: the word decodes as whatever remains (`HINT` for the hint-space
    /// instructions of a missing feature), or as `Code::Invalid`. Default: every
    /// feature in the pinned release.
    pub features: FeatureSet,
}

impl DecoderOptions {
    pub fn with_features(features: FeatureSet) -> Self {
        DecoderOptions { features }
    }
}

fn node_matches(i: usize, cx: &Ctx<'_>) -> bool {
    let n = &NODES[i];
    cx.word & n.mask == n.value && (n.cond == NO_COND || holds(n.cond, cx))
}

fn walk(i: usize, cx: &Ctx<'_>) -> Code {
    let n = &NODES[i];
    if n.code != Code::Invalid {
        return n.code;
    }
    let first = n.first as usize;
    for &c in &CHILDREN[first..first + n.count as usize] {
        let c = c as usize;
        if node_matches(c, cx) {
            let r = walk(c, cx);
            if r != Code::Invalid {
                return r;
            }
        }
    }
    Code::Invalid
}

/// The encoding `word` belongs to, without decoding operands.
pub(crate) fn classify(word: u32, features: &FeatureSet) -> Code {
    let cx = Ctx { word, features };
    if !node_matches(ROOT, &cx) {
        return Code::Invalid;
    }
    walk(ROOT, &cx)
}

/// Decodes one word.
pub fn decode_word(word: u32, pc: u64, options: &DecoderOptions) -> Instruction {
    let mut insn = Instruction::default();
    decode_into(word, pc, options, &mut insn);
    insn
}

fn decode_into(word: u32, pc: u64, options: &DecoderOptions, insn: &mut Instruction) {
    *insn = Instruction {
        pc,
        word,
        features: options.features,
        ..Instruction::default()
    };
    let code = classify(word, &options.features);
    if code == Code::Invalid {
        return;
    }
    let d = &ENCODINGS[code as usize - 1];
    if crate::undefined::undefined(d, word) {
        return;
    }
    if let Some(ops) = d.ops {
        match crate::desc::decode_ops(ops, word, pc, &mut insn.ops) {
            Some(n) => {
                insn.op_count = n;
                insn.has_operands = true;
            }
            // A reserved operand value: UNALLOCATED.
            None => return,
        }
    }
    insn.code = code;
    insn.sb = word & d.sb_mask;
    insn.suffix = d.ops.map_or(0, |ops| crate::desc::suffix_code(ops, word));
    #[cfg(feature = "encoder")]
    if let Some(ops) = d.ops {
        // Bits the operands do not reproduce are don't-care bits.
        let (pm, pv) = crate::undefined::pinned(d);
        if let Ok(w) = crate::encoder::encode_ops(
            ops,
            insn.operands(),
            insn.suffix,
            pc,
            d.value | insn.sb | pv,
            d.mask | d.sb_mask | pm,
        ) {
            insn.extra = (word ^ w) & !(d.mask | d.sb_mask | pm);
        }
    }
    insn.alias = crate::alias::preferred(d, word, &options.features);
}

/// Decodes little-endian A64 code from a byte slice, in the manner of
/// `iced_x86::Decoder`.
#[derive(Debug, Clone)]
pub struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
    pc: u64,
    options: DecoderOptions,
}

impl<'a> Decoder<'a> {
    /// `pc` is the address of `data[0]`.
    pub fn new(data: &'a [u8], pc: u64, options: DecoderOptions) -> Self {
        Decoder {
            data,
            pos: 0,
            pc,
            options,
        }
    }

    /// The address of the next instruction.
    pub fn pc(&self) -> u64 {
        self.pc
    }

    pub fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    /// Sets the byte position; the PC is not changed.
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    /// Whether a whole word remains.
    pub fn can_decode(&self) -> bool {
        self.data.len() - self.pos >= 4
    }

    pub fn options(&self) -> &DecoderOptions {
        &self.options
    }

    /// Decodes the next word. Past the end (or on a partial word) returns an
    /// invalid instruction and does not advance.
    pub fn decode(&mut self) -> Instruction {
        let mut insn = Instruction::default();
        self.decode_out(&mut insn);
        insn
    }

    pub fn decode_out(&mut self, insn: &mut Instruction) {
        if !self.can_decode() {
            *insn = Instruction {
                pc: self.pc,
                ..Instruction::default()
            };
            return;
        }
        let b = &self.data[self.pos..self.pos + 4];
        let word = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        decode_into(word, self.pc, &self.options, insn);
        self.pos += 4;
        self.pc = self.pc.wrapping_add(4);
    }
}

impl Iterator for Decoder<'_> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        self.can_decode().then(|| self.decode())
    }
}
