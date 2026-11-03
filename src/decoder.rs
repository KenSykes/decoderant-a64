//! The A64 decoder: classifies a word with the generated decode dispatch, then
//! decodes its operands.

use crate::cond::{Ctx, holds};
use crate::feature_set::FeatureSet;
use crate::generated::Code;
use crate::generated::tables::{CANDIDATES, DISPATCH, DISPATCH_ROOT, ENCODINGS};
use crate::instruction::Instruction;
use crate::tables::{NO_COND, SLOT_LIST};

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

/// The encoding `word` belongs to, without decoding operands: the first encoding,
/// in decode-tree order, whose fixed bits match and whose path conditions hold. The
/// dispatch narrows the encodings down to those whose fixed bits allow the word's
/// value of a few fields (see `gen/src/dispatch.rs`).
#[inline]
pub(crate) fn classify(word: u32, features: &FeatureSet) -> Code {
    let mut s = DISPATCH_ROOT;
    while s & SLOT_LIST == 0 {
        let (lsb, width) = (s >> 24, s >> 20 & 15);
        s = DISPATCH[(s & 0xf_ffff) as usize + (word >> lsb & ((1 << width) - 1)) as usize];
    }
    let start = (s & 0xf_ffff) as usize;
    let len = (s >> 20 & 0x7ff) as usize;
    let cx = Ctx { word, features };
    for c in &CANDIDATES[start..start + len] {
        if word & c.mask == c.value && (c.cond == NO_COND || holds(c.cond, &cx)) {
            return c.code;
        }
    }
    Code::Invalid
}

/// The decode tree walk the dispatch replaces: the reference for the tests.
#[cfg(test)]
pub(crate) fn classify_by_tree(word: u32, features: &FeatureSet) -> Code {
    use crate::generated::tables::{CHILDREN, NODES, ROOT};

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

    let cx = Ctx { word, features };
    if !node_matches(ROOT, &cx) {
        return Code::Invalid;
    }
    walk(ROOT, &cx)
}

/// Encodings whose operands reproduce every variable bit of any word, so that
/// decoding has no don't-care bits to look for.
#[cfg(feature = "encoder")]
static NO_DONT_CARE: [bool; crate::generated::CODE_COUNT - 1] = {
    let mut t = [false; crate::generated::CODE_COUNT - 1];
    let mut i = 0;
    while i < t.len() {
        let d = &ENCODINGS[i];
        if let Some(ops) = d.ops
            && let Some(c) = crate::desc::cover_const(ops)
        {
            t[i] = d.mask | d.sb_mask | crate::undefined::pinned(d).0 | c == u32::MAX;
        }
        i += 1;
    }
    t
};

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
    let i = code as usize - 1;
    let d = &ENCODINGS[i];
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
        // Bits the operands do not reproduce are don't-care bits. Re-encoding finds
        // them; where the operands certainly reproduce every variable bit there are
        // none.
        let (pm, pv) = crate::undefined::pinned(d);
        let fixed = d.mask | d.sb_mask | pm;
        if !NO_DONT_CARE[i]
            && crate::desc::cover(ops, word).is_none_or(|c| fixed | c != u32::MAX)
            && let Ok(w) = crate::encoder::encode_ops(
                ops,
                insn.operands(),
                insn.suffix,
                pc,
                d.value | insn.sb | pv,
                fixed,
            )
        {
            insn.extra = (word ^ w) & !fixed;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::Feature;

    /// The dispatch classifies like the decode tree: on random words, on words
    /// built from every encoding's fixed bits, with all features, none, and a
    /// subset.
    #[test]
    fn dispatch_matches_the_decode_tree() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u32
        };
        let some = FeatureSet::ALL
            .without(Feature::PAuth)
            .without(Feature::SVE)
            .without(Feature::SME);
        for features in [FeatureSet::ALL, FeatureSet::EMPTY, some] {
            let check = |w: u32| {
                assert_eq!(
                    classify(w, &features),
                    classify_by_tree(w, &features),
                    "{w:#010x}"
                );
            };
            for _ in 0..200_000 {
                check(next());
            }
            for d in &ENCODINGS {
                for _ in 0..64 {
                    check(d.value | next() & !d.mask);
                }
            }
        }
    }
}

#[cfg(all(test, feature = "encoder"))]
mod dont_care_tests {
    use super::*;

    /// The constant cover is a lower bound of the cover of sample words.
    #[test]
    fn static_cover_matches_word_cover() {
        let mut seed = 0x1234_5678_9abc_def1u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u32
        };
        for d in &ENCODINGS {
            let Some(ops) = d.ops else { continue };
            let Some(c) = crate::desc::cover_const(ops) else {
                continue;
            };
            for _ in 0..256 {
                let w = d.value | next() & !d.mask;
                if let Some(cw) = crate::desc::cover(ops, w) {
                    assert_eq!(cw & c, c, "{} {w:#010x}", d.name);
                }
            }
        }
    }
}
