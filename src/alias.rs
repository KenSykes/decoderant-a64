//! Preferred-alias selection.

use crate::cond::{Ctx, holds};
use crate::feature_set::FeatureSet;
use crate::tables::{EncodingData, NO_COND};

/// Index + 1 of the alias the spec prefers for `word` (0: none). Only aliases with
/// operand support are chosen, so the formatter can always print the choice.
pub(crate) fn preferred(d: &EncodingData, word: u32, features: &FeatureSet) -> u8 {
    let cx = Ctx { word, features };
    for (i, a) in d.aliases.iter().enumerate() {
        if a.ops.is_none() {
            continue;
        }
        let ok = |c: u16| c == NO_COND || holds(c, &cx);
        if ok(a.cond) && ok(a.preferred) {
            return (i + 1) as u8;
        }
    }
    0
}
