//! Per-encoding metadata: what the generator and the coverage report work from.

use crate::cond::{Ctx, holds};
use crate::feature_set::FeatureSet;
use crate::generated::tables::ENCODINGS;
use crate::generated::{Code, Feature, Mnemonic};
use crate::tables::{EncodingData, FieldInfo};

/// Metadata of one encoding (`Code`).
#[derive(Debug, Clone, Copy)]
pub struct OpCodeInfo {
    code: Code,
    data: Option<&'static EncodingData>,
}

impl Code {
    /// The encoding's metadata.
    pub fn op_code(self) -> OpCodeInfo {
        let data = (self as usize)
            .checked_sub(1)
            .and_then(|i| ENCODINGS.get(i));
        OpCodeInfo { code: self, data }
    }
}

/// An alias of an encoding.
#[derive(Debug, Clone, Copy)]
pub struct AliasInfo {
    pub mnemonic: Mnemonic,
    /// The alias's assembler template.
    pub template: &'static str,
}

impl OpCodeInfo {
    pub fn code(&self) -> Code {
        self.code
    }

    /// The AARCHMRS encoding name (`ADD_64_addsub_shift`); `"INVALID"` for
    /// `Code::Invalid`.
    pub fn name(&self) -> &'static str {
        self.data.map_or("INVALID", |d| d.name)
    }

    /// The A64 iclass: the innermost decode group (`addsub_shift`).
    pub fn iclass(&self) -> &'static str {
        self.data.map_or("", |d| d.iclass)
    }

    /// The top-level decode group (`dpreg`, `ldst`, `simd_dp`, `sve`, ...).
    pub fn group(&self) -> &'static str {
        self.data.map_or("", |d| d.group)
    }

    pub fn mnemonic(&self) -> Mnemonic {
        self.data.map_or(Mnemonic::Invalid, |d| d.mnemonic)
    }

    /// Fixed bits: a matching word has `word & mask == value`. Excludes should-be
    /// bits. Informational: [`matches`](Self::matches) also applies the field
    /// conditions.
    pub fn mask(&self) -> u32 {
        self.data.map_or(0, |d| d.mask)
    }

    pub fn value(&self) -> u32 {
        self.data.map_or(0, |d| d.value)
    }

    /// Should-be bits (`(0)`/`(1)`) and their stated values.
    pub fn should_be_mask(&self) -> u32 {
        self.data.map_or(0, |d| d.sb_mask)
    }

    pub fn should_be_value(&self) -> u32 {
        self.data.map_or(0, |d| d.sb_value)
    }

    /// The named fields of the encoding, most significant first.
    pub fn fields(&self) -> &'static [FieldInfo] {
        self.data.map_or(&[], |d| d.fields)
    }

    pub fn field(&self, name: &str) -> Option<FieldInfo> {
        self.fields().iter().find(|f| f.name == name).copied()
    }

    /// Features named by the encoding's decode conditions (`FEAT_SVE || FEAT_SME`
    /// lists both).
    pub fn features(&self) -> &'static [Feature] {
        self.data.map_or(&[], |d| d.features)
    }

    /// Whether the encoding exists with the given features.
    pub fn is_available(&self, features: &FeatureSet) -> bool {
        let Some(d) = self.data else { return false };
        let cx = Ctx {
            word: d.value,
            features,
        };
        d.feature_conds.iter().all(|&c| holds(c, &cx))
    }

    /// The Arm assembler template (`ADD <Xd>, <Xn>, <Xm>{, <shift> #<amount>}`).
    pub fn template(&self) -> &'static str {
        self.data.map_or("", |d| d.template)
    }

    /// Encodings that take precedence where their pattern overlaps this one: a word
    /// that [`matches`](Self::matches) decodes to this encoding or to one of these.
    pub fn precedence(&self) -> &'static [Code] {
        self.data.map_or(&[], |d| d.precedence)
    }

    /// Whether operands are decoded for this encoding.
    pub fn has_operands(&self) -> bool {
        self.data.is_some_and(|d| d.ops.is_some())
    }

    /// Whether `word` is an instance of this encoding: the fixed bits, every field
    /// condition on the decode path (with all features) and, where operands are
    /// supported, no reserved operand value.
    pub fn matches(&self, word: u32) -> bool {
        self.matches_with(word, &FeatureSet::ALL)
    }

    pub fn matches_with(&self, word: u32, features: &FeatureSet) -> bool {
        let Some(d) = self.data else { return false };
        if word & d.mask != d.value {
            return false;
        }
        let cx = Ctx { word, features };
        if !d.conds.iter().all(|&c| holds(c, &cx)) || crate::undefined::undefined(d, word) {
            return false;
        }
        match d.ops {
            Some(ops) => crate::desc::decode_ops(
                ops,
                word,
                0,
                &mut [crate::Operand::none(); crate::MAX_OPERANDS],
            )
            .is_some(),
            None => true,
        }
    }

    pub fn aliases(&self) -> impl Iterator<Item = AliasInfo> {
        self.data
            .map_or(&[][..], |d| d.aliases)
            .iter()
            .map(|a| AliasInfo {
                mnemonic: a.mnemonic,
                template: a.template,
            })
    }

    pub(crate) fn data(&self) -> Option<&'static EncodingData> {
        self.data
    }
}
