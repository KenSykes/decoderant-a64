//! The decode dispatch: a decision tree over bit fields of the word, built from the
//! encodings in decode order.
//!
//! The decode tree's answer for a word is the first encoding, in depth-first order
//! with children most specific first, whose fixed bits match and whose path
//! conditions hold. The dispatch keeps that order: each bucket lists, in decode order,
//! the encodings whose fixed bits allow the bucket's field values, so the first
//! candidate that matches (fixed bits, then conditions) is the decode tree's answer.
//! An encoding with `x` bits in a field is listed in every bucket it allows.

use std::collections::BTreeMap;

/// Slot encoding (`aarch64_isa::tables::Slot`): a list of candidates, or a node
/// that indexes further slots by a field of the word.
const LIST: u32 = 1 << 31;
/// At most this many candidates end the splitting.
const MAX_LIST: usize = 2;
const MAX_WIDTH: u32 = 8;
const ROOT_WIDTH: u32 = 10;
/// The cost of a table slot in candidates scanned: a split is scored by the
/// average list length it leaves plus this per slot, so wide tables are only used
/// where they shorten the lists.
const SLOT_COST: f64 = 1.0 / 16.0;

pub struct Dispatch {
    /// The root slot (a node).
    pub root: u32,
    pub slots: Vec<u32>,
    /// Candidate lists: indices into the pattern slice `build` was given.
    pub lists: Vec<usize>,
    /// The longest list, and the average list length and table depth over all
    /// 2^32 words.
    pub max_list: usize,
    pub mean_list: f64,
    pub mean_depth: f64,
}

pub fn node_slot(lsb: u32, width: u32, base: usize) -> u32 {
    assert!(base < 1 << 20 && width < 16 && lsb < 32);
    lsb << 24 | width << 20 | base as u32
}

pub fn list_slot(start: usize, len: usize) -> u32 {
    assert!(start < 1 << 20 && len < 1 << 11);
    LIST | (len as u32) << 20 | start as u32
}

struct Builder<'a> {
    pats: &'a [(u32, u32)],
    slots: Vec<u32>,
    lists: Vec<usize>,
    list_index: BTreeMap<Vec<usize>, usize>,
    max_list: usize,
    mean_list: f64,
    mean_depth: f64,
}

impl Builder<'_> {
    /// A list slot reached by `share` of all words through `depth` tables.
    fn leaf(&mut self, c: &[usize], depth: usize, share: f64) -> u32 {
        self.max_list = self.max_list.max(c.len());
        self.mean_list += share * c.len() as f64;
        self.mean_depth += share * depth as f64;
        self.list(c)
    }

    fn list(&mut self, c: &[usize]) -> u32 {
        let start = match self.list_index.get(c) {
            Some(&s) => s,
            None => {
                let s = self.lists.len();
                self.lists.extend(c);
                self.list_index.insert(c.to_vec(), s);
                s
            }
        };
        list_slot(start, c.len())
    }

    /// Calls `f` with every value of the field (`width` bits from `lsb`) that
    /// candidate `c`'s fixed bits allow.
    fn for_each_value(&self, c: usize, lsb: u32, width: u32, mut f: impl FnMut(usize)) {
        let wm = (1u32 << width) - 1;
        let (m, v) = self.pats[c];
        let (m, v) = (m >> lsb & wm, v >> lsb & wm);
        // v | s for each subset s of the free bits.
        let free = !m & wm;
        let mut s = 0u32;
        loop {
            f((v | s) as usize);
            if s == free {
                break;
            }
            s = (s.wrapping_sub(free)) & free;
        }
    }

    /// The candidates of each value of the field (`width` bits from `lsb`), in order.
    fn buckets(&self, cands: &[usize], lsb: u32, width: u32) -> Vec<Vec<usize>> {
        let mut b = vec![Vec::new(); 1 << width];
        for &c in cands {
            self.for_each_value(c, lsb, width, |x| b[x].push(c));
        }
        b
    }

    fn node(&mut self, cands: &[usize], known: u32, depth: usize, share: f64) -> u32 {
        if cands.len() <= MAX_LIST {
            return self.leaf(cands, depth, share);
        }
        // The field that most shortens the average list, net of its table size.
        let mut best: Option<(f64, u32, u32)> = None;
        let max_width = if depth == 0 { ROOT_WIDTH } else { MAX_WIDTH };
        for width in 1..=max_width {
            let wm = (1u32 << width) - 1;
            for lsb in 0..=32 - width {
                let fm = wm << lsb;
                if fm & known != 0 {
                    continue;
                }
                // Splitting on bits no candidate fixes gains nothing.
                if cands.iter().all(|&c| self.pats[c].0 & fm == 0) {
                    continue;
                }
                let mut counts = vec![0usize; 1 << width];
                let mut total = 0usize;
                for &c in cands {
                    self.for_each_value(c, lsb, width, |x| {
                        counts[x] += 1;
                        total += 1;
                    });
                }
                let max = counts.iter().copied().max().unwrap_or(0);
                if max >= cands.len() {
                    continue;
                }
                let slots = f64::from(1u32 << width);
                let score = total as f64 / slots + SLOT_COST * slots;
                if best.is_none_or(|(s, _, _)| score < s) {
                    best = Some((score, lsb, width));
                }
            }
        }
        let Some((_, lsb, width)) = best else {
            return self.leaf(cands, depth, share);
        };
        let buckets = self.buckets(cands, lsb, width);
        let base = self.slots.len();
        self.slots.resize(base + buckets.len(), 0);
        let known = known | ((1u32 << width) - 1) << lsb;
        for (i, b) in buckets.iter().enumerate() {
            let s = self.node(b, known, depth + 1, share / buckets.len() as f64);
            self.slots[base + i] = s;
        }
        node_slot(lsb, width, base)
    }
}

/// Builds the dispatch for `pats` (mask, value), given in decode order.
pub fn build(pats: &[(u32, u32)]) -> Dispatch {
    let mut b = Builder {
        pats,
        slots: Vec::new(),
        lists: Vec::new(),
        list_index: BTreeMap::new(),
        max_list: 0,
        mean_list: 0.0,
        mean_depth: 0.0,
    };
    let all: Vec<usize> = (0..pats.len()).collect();
    let root = b.node(&all, 0, 0, 1.0);
    assert!(root & LIST == 0, "the root must split");
    Dispatch {
        root,
        slots: b.slots,
        lists: b.lists,
        max_list: b.max_list,
        mean_list: b.mean_list,
        mean_depth: b.mean_depth,
    }
}
