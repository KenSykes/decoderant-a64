//! The A64 decoder: classifies a word with the generated decode dispatch, then
//! decodes its operands.

use crate::cond::{Ctx, holds};
use crate::desc::operand_count;
use crate::feature_set::FeatureSet;
use crate::generated::Code;
use crate::generated::tables::{CANDIDATES, DECODE_OPS, DISPATCH, DISPATCH_ROOT, ENCODINGS};
use crate::instruction::{Instruction, MAX_OPERANDS};
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

// Every operand list of the tables fits an `Instruction`.
const _: () = {
    let mut i = 0;
    while i < ENCODINGS.len() {
        let d = &ENCODINGS[i];
        if let Some(ops) = d.ops {
            assert!(operand_count(ops) <= MAX_OPERANDS);
        }
        let mut k = 0;
        while k < d.aliases.len() {
            if let Some(ops) = d.aliases[k].ops {
                assert!(operand_count(ops) <= MAX_OPERANDS);
            }
            k += 1;
        }
        i += 1;
    }
};

/// Encodings whose operands reproduce every variable bit of any word, so that
/// decoding has no don't-care bits to look for.
#[cfg(feature = "encoder")]
static NO_DONT_CARE: [bool; crate::generated::CODE_COUNT - 1] = {
    let mut t = [false; crate::generated::CODE_COUNT - 1];
    let mut i = 0;
    while i < t.len() {
        let d = &ENCODINGS[i];
        if let Some(ops) = d.ops
            && let Some(c) = crate::cover::cover(ops, None)
        {
            t[i] = d.mask | d.sb_mask | crate::undefined::pinned(d).0 | c == u32::MAX;
        }
        i += 1;
    }
    t
};

/// Decodes one word.
pub fn decode_word(word: u32, pc: u64, options: &DecoderOptions) -> Instruction {
    let mut insn = Instruction::invalid(word, pc, options);
    decode_into(&mut insn, options);
    insn
}

impl Instruction {
    /// An invalid instruction for `word` at `pc`, where decoding starts.
    #[inline]
    fn invalid(word: u32, pc: u64, options: &DecoderOptions) -> Instruction {
        Instruction {
            pc,
            word,
            features: options.features,
            ..Instruction::EMPTY
        }
    }
}

/// Decodes `insn` (from [`Instruction::invalid`]) in place: an `Instruction` is a
/// few hundred bytes, so each caller writes it once rather than building and
/// copying it.
#[inline]
fn decode_into(insn: &mut Instruction, options: &DecoderOptions) {
    let (word, pc) = (insn.word, insn.pc);
    let code = classify(word, &options.features);
    if code == Code::Invalid {
        return;
    }
    let i = code as usize - 1;
    let d = &ENCODINGS[i];
    if crate::undefined::undefined(d, word) {
        return;
    }
    if d.ops.is_some() {
        match DECODE_OPS[i](word, pc, &mut insn.ops) {
            Some(n) => {
                insn.op_count = n;
                insn.has_operands = true;
            }
            // A reserved operand value: UNALLOCATED (without the operands decoded
            // so far).
            None => {
                insn.ops = Instruction::EMPTY.ops;
                return;
            }
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
            && crate::cover::cover(ops, Some(word)).is_none_or(|c| fixed | c != u32::MAX)
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
        let mut insn = Instruction::EMPTY;
        self.decode_out(&mut insn);
        insn
    }

    /// [`decode`](Self::decode) into an existing instruction.
    pub fn decode_out(&mut self, insn: &mut Instruction) {
        let Some(b) = self.data.get(self.pos..self.pos + 4) else {
            *insn = Instruction::invalid(0, self.pc, &self.options);
            return;
        };
        let word = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        *insn = Instruction::invalid(word, self.pc, &self.options);
        decode_into(insn, &self.options);
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
    use crate::bits::xorshift;
    use crate::generated::Feature;

    /// The dispatch classifies like the decode tree: on random words, on words
    /// built from every encoding's fixed bits, with all features, none, and a
    /// subset.
    #[test]
    fn dispatch_matches_the_decode_tree() {
        let mut next = xorshift(0x2545_f491_4f6c_dd1d);
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
                check(next() as u32);
            }
            for d in &ENCODINGS {
                for _ in 0..64 {
                    check(d.value | next() as u32 & !d.mask);
                }
            }
        }
    }

    /// Each encoding's generated operand decoder decodes like the descriptor
    /// interpreter, on words built from its fixed bits.
    #[test]
    fn generated_operand_decoders_match_the_descriptors() {
        use crate::desc::decode_ops;
        use crate::instruction::MAX_OPERANDS;
        use crate::operand::Operand;

        let mut next = xorshift(0x5851_f42d_4c95_7f2d);
        for (i, d) in ENCODINGS.iter().enumerate() {
            let Some(ops) = d.ops else { continue };
            for _ in 0..256 {
                let w = d.value | next() as u32 & !d.mask;
                let pc = next() & !3;
                let mut a = [Operand::NONE; MAX_OPERANDS];
                let mut b = [Operand::NONE; MAX_OPERANDS];
                let ra = DECODE_OPS[i](w, pc, &mut a);
                let rb = decode_ops(ops, w, pc, &mut b);
                assert_eq!(ra, rb, "{} {w:#010x}", d.name);
                if let Some(n) = ra {
                    let n = n as usize;
                    assert_eq!(a[..n], b[..n], "{} {w:#010x}", d.name);
                }
            }
        }
    }

    /// Past the end the decoder returns an invalid instruction, with its options.
    #[test]
    fn past_the_end() {
        let features = FeatureSet::EMPTY.with(Feature::SVE);
        let mut d = Decoder::new(
            &[0x20, 0x00, 0x02],
            0x40,
            DecoderOptions::with_features(features),
        );
        assert!(!d.can_decode());
        let i = d.decode();
        assert!(i.is_invalid());
        assert_eq!((i.pc(), i.features, d.position()), (0x40, features, 0));
    }
}

#[cfg(all(test, feature = "encoder"))]
mod dont_care_tests {
    use super::*;
    use crate::bits::xorshift;

    /// The constant cover is a lower bound of the cover of sample words.
    #[test]
    fn static_cover_matches_word_cover() {
        let mut next = xorshift(0x1234_5678_9abc_def1);
        for d in &ENCODINGS {
            let Some(ops) = d.ops else { continue };
            let Some(c) = crate::cover::cover(ops, None) else {
                continue;
            };
            for _ in 0..256 {
                let w = d.value | next() as u32 & !d.mask;
                if let Some(cw) = crate::cover::cover(ops, Some(w)) {
                    assert_eq!(cw & c, c, "{} {w:#010x}", d.name);
                }
            }
        }
    }
}
