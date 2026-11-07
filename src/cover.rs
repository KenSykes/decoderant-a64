//! Cover: which word bits re-encoding an instruction's operands is certain to
//! reproduce. The decoder uses it to skip the search for don't-care bits (see
//! `decoder::decode_word`) where there can be none.
//!
//! One implementation serves both a given word (`Some(word)`, at decode time) and
//! a bound over all words (`None`, for the constant table of encodings without
//! don't-care bits), so the two cannot drift apart.

use crate::desc::{Amount, Arr, Imm, ImmShift, MemOff, Op, Post, Pq, RegSel, Tab};

/// `?` for `const fn`s.
macro_rules! some {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return None,
        }
    };
}

/// Whether entry `i` of `t` is the first for its code: the one the encoder picks.
const fn first_for_code(t: &Tab, i: usize) -> bool {
    let mut j = 0;
    while j < i {
        if t.e[j].2 == t.e[i].2 {
            return false;
        }
        j += 1;
    }
    true
}

/// Whether `t.nums[code]` exists and is the first entry with its value: the number
/// the encoder looks up gives back `code`.
const fn first_for_num(t: &Tab, code: usize) -> bool {
    if code >= t.nums.len() {
        return false;
    }
    let mut j = 0;
    while j < code {
        if t.nums[j] == t.nums[code] {
            return false;
        }
        j += 1;
    }
    true
}

/// The word bits re-encoding a table's code reproduces. For a word: the fixed bits
/// of the entry it matches, when that is the entry the encoder picks for its code.
/// For any word (`None`): the bits every entry fixes, when each code has one entry.
const fn tab_cover(t: &Tab, word: Option<u32>) -> Option<u32> {
    let fixed = match word {
        Some(w) => {
            let i = some!(t.entry(w));
            if !first_for_code(t, i) {
                return None;
            }
            t.e[i].1
        }
        None => {
            let mut common = u32::MAX;
            let mut i = 0;
            while i < t.e.len() {
                if !first_for_code(t, i) {
                    return None;
                }
                common &= t.e[i].1;
                i += 1;
            }
            common
        }
    };
    let mut m = 0;
    t.f.set(&mut m, fixed);
    Some(m)
}

const fn sel_cover(k: RegSel, word: Option<u32>) -> Option<u32> {
    match k {
        RegSel::Fixed(_) => Some(0),
        RegSel::Tab(t) => tab_cover(t, word),
    }
}

const fn arr_cover(a: Arr, word: Option<u32>) -> Option<u32> {
    match a {
        Arr::Fixed(_) => Some(0),
        Arr::Tab(t) => tab_cover(t, word),
    }
}

const fn imm_cover(i: Imm, word: Option<u32>) -> Option<u32> {
    Some(match i {
        Imm::Const(_) => 0,
        Imm::U(f, _)
        | Imm::S(f, _)
        | Imm::SMul(f, _)
        | Imm::Add(f, _)
        | Imm::Rsub(_, f)
        | Imm::NegU(f, _)
        | Imm::ShrEs(f)
        | Imm::ShlEs(f)
        | Imm::Fp8(f)
        | Imm::ByteMask(f) => f.mask(),
        Imm::NegMod(f, m) if 1u32 << f.width() <= m as u32 => f.mask(),
        // `imms` from the value and `immr`, which another operand sets.
        Imm::Width(imms, _) => imms.mask(),
        Imm::Bitmask { n, immr, imms, .. } => n.mask() | immr.mask() | imms.mask(),
        Imm::Tab(t) => {
            // The encoder looks the value up by its first code.
            let unique = match word {
                Some(w) => match t.find(w) {
                    Some(code) => first_for_num(t, code as usize),
                    None => false,
                },
                None => {
                    let mut i = 0;
                    while i < t.nums.len() && first_for_num(t, i) {
                        i += 1;
                    }
                    i == t.nums.len()
                }
            };
            if !unique {
                return None;
            }
            some!(tab_cover(t, word))
        }
        Imm::SveBitmask { .. } | Imm::NegMod(..) | Imm::LslShift { .. } | Imm::WideMov { .. } => {
            return None;
        }
    })
}

/// Whether a shift table gives a code above `MSL` (all of which decode to `MSL`).
const fn above_msl(t: &Tab, word: Option<u32>) -> bool {
    match word {
        Some(w) => match t.find(w) {
            Some(code) => code > 4,
            None => true,
        },
        None => {
            let mut i = 0;
            while i < t.e.len() {
                if t.e[i].2 > 4 {
                    return true;
                }
                i += 1;
            }
            false
        }
    }
}

const fn op_cover(op: &Op, word: Option<u32>) -> Option<u32> {
    Some(match *op {
        Op::Reg(f, k, _) => f.mask() | some!(sel_cover(k, word)),
        Op::RegFixed(_) | Op::RegPair(..) | Op::Lit(_) => 0,
        Op::Vec(f, a, _) | Op::List(f, _, a, None) => f.mask() | some!(arr_cover(a, word)),
        Op::Elem(..) | Op::List(_, _, _, Some(_)) => return None,
        Op::Imm(i, s) => {
            some!(imm_cover(i, word))
                | match s {
                    ImmShift::None => 0,
                    ImmShift::Lsl(a) | ImmShift::Msl(a) => some!(imm_cover(a, word)),
                }
        }
        Op::BitImm(i) | Op::FpImm(i) | Op::Label(i, _) => some!(imm_cover(i, word)),
        Op::Shifted(f, k, t, amount) => {
            if above_msl(t, word) {
                return None;
            }
            f.mask() | amount.mask() | some!(sel_cover(k, word)) | some!(tab_cover(t, word))
        }
        Op::Extended(f, k, option, amount) => {
            f.mask() | option.mask() | amount.mask() | some!(sel_cover(k, word))
        }
        Op::Mem(ref m) => {
            let off = match m.off {
                MemOff::None => 0,
                MemOff::OptReg(f, _) => f.mask(),
                MemOff::Imm(i) => some!(imm_cover(i, word)),
                MemOff::Reg {
                    m: f,
                    k,
                    ext,
                    amount,
                } => {
                    let ext = match ext {
                        Some(t) => some!(tab_cover(t, word)),
                        None => 0,
                    };
                    let amount = match amount {
                        Amount::None | Amount::Fixed(_) => 0,
                        Amount::OnBit(f, _) if f.width() == 1 => f.mask(),
                        Amount::OnBit(..) | Amount::Tab(_) => return None,
                    };
                    f.mask() | some!(sel_cover(k, word)) | ext | amount
                }
            };
            let post = match m.post {
                Post::None => 0,
                Post::Imm(i) => some!(imm_cover(i, word)),
                Post::Reg(f) => f.mask(),
            };
            m.base.mask() | off | post | some!(arr_cover(m.arr, word))
        }
        Op::Cond(f, ..) if f.width() <= 4 => f.mask(),
        Op::SysReg(f) if f.width() <= 15 => f.mask(),
        Op::SysOperation(f) if f.width() <= 14 => f.mask(),
        Op::PState(f) | Op::Cr(f) | Op::RegWb(f, _) => f.mask(),
        Op::Name(t, _) => t.f.mask(),
        Op::Opt(inner, ..) | Op::Braced(inner) => some!(op_cover(inner, word)),
        Op::Suffix(t) => some!(tab_cover(t, word)),
        Op::Zr(f, _, a, _, pq) => {
            f.mask()
                | some!(arr_cover(a, word))
                | match pq {
                    Pq::Tab(t) => some!(tab_cover(t, word)),
                    _ => 0,
                }
        }
        Op::ZElem(..) => return None,
        Op::MulImm(i) => some!(imm_cover(i, word)),
        Op::ZList(f, _, _, _, _, _, a, None) | Op::ZrMap(f, _, a, None) | Op::ZaTile(f, a) => {
            f.mask() | some!(arr_cover(a, word))
        }
        Op::ZList(.., Some(_)) | Op::ZSel(..) | Op::ZrMap(..) => return None,
        Op::ZaSlice(t, a, hv, ws, _, offs, _) => {
            t.mask() | hv.mask() | ws.mask() | offs.mask() | some!(arr_cover(a, word))
        }
        Op::ZaArray(a, wv, _, offs, _, _) => wv.mask() | offs.mask() | some!(arr_cover(a, word)),
        Op::ZaMask(f) | Op::Zt0Vl(f) => f.mask(),
        Op::Cond(..) | Op::SysReg(_) | Op::SysOperation(_) => return None,
    })
}

/// The word bits that re-encoding the operands decoded from `word` is certain to
/// reproduce, or `None` when some operand's encoding may differ from the word (a
/// table with several entries for one value, a value with several encodings, ...).
/// Variable bits outside the result are don't-care bits; the decoder finds them by
/// re-encoding, which this lets it skip when there are none.
///
/// With `word` `None`: a lower bound over all words, or `None` where some word's
/// cover can be `None` or the bound needs the word (usable in constants).
pub(crate) const fn cover(ops: &[Op], word: Option<u32>) -> Option<u32> {
    let mut m = 0;
    let mut i = 0;
    while i < ops.len() {
        m |= some!(op_cover(&ops[i], word));
        i += 1;
    }
    Some(m)
}
