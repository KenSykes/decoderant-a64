//! Sets of `FEAT_*` features, for decoder filtering.

use crate::generated::{FEATURE_COUNT, Feature};

const WORDS: usize = FEATURE_COUNT.div_ceil(64);

/// A set of [`Feature`]s. The decoder treats the encodings of features outside the
/// set as removed from the decode tree.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureSet {
    bits: [u64; WORDS],
}

impl FeatureSet {
    /// No features.
    pub const EMPTY: FeatureSet = FeatureSet { bits: [0; WORDS] };

    /// Every feature in the pinned release.
    pub const ALL: FeatureSet = {
        let mut bits = [0u64; WORDS];
        let mut i = 0;
        while i < FEATURE_COUNT {
            bits[i / 64] |= 1 << (i % 64);
            i += 1;
        }
        FeatureSet { bits }
    };

    pub const fn has(&self, f: Feature) -> bool {
        let i = f as usize;
        self.bits[i / 64] >> (i % 64) & 1 == 1
    }

    #[must_use]
    pub const fn with(mut self, f: Feature) -> FeatureSet {
        let i = f as usize;
        self.bits[i / 64] |= 1 << (i % 64);
        self
    }

    #[must_use]
    pub const fn without(mut self, f: Feature) -> FeatureSet {
        let i = f as usize;
        self.bits[i / 64] &= !(1 << (i % 64));
        self
    }

    pub fn insert(&mut self, f: Feature) {
        *self = self.with(f);
    }

    pub fn remove(&mut self, f: Feature) {
        *self = self.without(f);
    }

    /// The features in the set, in [`Feature`] order.
    pub fn iter(&self) -> impl Iterator<Item = Feature> + '_ {
        Feature::values().filter(|&f| self.has(f))
    }

    pub fn len(&self) -> usize {
        self.bits.iter().map(|w| w.count_ones() as usize).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&w| w == 0)
    }
}

impl Default for FeatureSet {
    /// Everything in the pinned release, as for [`DecoderOptions`](crate::DecoderOptions).
    fn default() -> Self {
        Self::ALL
    }
}

impl core::fmt::Debug for FeatureSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl FromIterator<Feature> for FeatureSet {
    fn from_iter<I: IntoIterator<Item = Feature>>(iter: I) -> Self {
        let mut s = FeatureSet::EMPTY;
        for f in iter {
            s.insert(f);
        }
        s
    }
}
