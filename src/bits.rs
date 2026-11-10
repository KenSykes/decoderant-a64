//! Bit helpers and the pseudocode functions operand decoding needs.

/// A field of the instruction word: `width` bits from `lsb` (width 0 = absent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct F {
    pub lsb: u8,
    pub width: u8,
}

impl F {
    pub const NONE: F = F { lsb: 0, width: 0 };

    pub const fn new(lsb: u8, width: u8) -> F {
        F { lsb, width }
    }

    #[inline]
    pub const fn mask(self) -> u32 {
        if self.width == 0 {
            0
        } else {
            (u32::MAX >> (32 - self.width as u32)) << self.lsb
        }
    }

    #[inline]
    pub const fn get(self, word: u32) -> u32 {
        (word & self.mask()) >> self.lsb
    }

    #[cfg(feature = "encoder")]
    #[inline]
    pub const fn set(self, word: &mut u32, v: u32) {
        *word = (*word & !self.mask()) | ((v << self.lsb) & self.mask());
    }
}

/// Up to four fields concatenated, most significant first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fs(pub [F; 4]);

impl Fs {
    pub const NONE: Fs = Fs([F::NONE; 4]);

    /// One field.
    #[cfg(feature = "encoder")]
    pub const fn one(f: F) -> Fs {
        Fs([f, F::NONE, F::NONE, F::NONE])
    }

    /// The least significant field.
    #[cfg(feature = "encoder")]
    pub const fn last(self) -> F {
        let mut i = 4;
        while i > 1 && self.0[i - 1].width == 0 {
            i -= 1;
        }
        self.0[i - 1]
    }

    pub const fn width(self) -> u32 {
        self.0[0].width as u32
            + self.0[1].width as u32
            + self.0[2].width as u32
            + self.0[3].width as u32
    }

    #[inline(always)]
    pub const fn get(self, word: u32) -> u32 {
        let mut v = 0u32;
        let mut i = 0;
        while i < 4 {
            let f = self.0[i];
            if f.width != 0 {
                v = v << f.width | f.get(word);
            }
            i += 1;
        }
        v
    }

    #[cfg(feature = "encoder")]
    pub const fn set(self, word: &mut u32, mut v: u32) {
        let mut i = 4;
        while i > 0 {
            i -= 1;
            let f = self.0[i];
            if f.width != 0 {
                f.set(word, v);
                v >>= f.width;
            }
        }
    }

    #[cfg(feature = "encoder")]
    pub const fn mask(self) -> u32 {
        self.0[0].mask() | self.0[1].mask() | self.0[2].mask() | self.0[3].mask()
    }

    pub const fn is_none(self) -> bool {
        self.0[0].width == 0
    }
}

/// Sign-extends the low `width` bits of `v`.
pub(crate) const fn sext(v: u64, width: u32) -> i64 {
    if width == 0 || width >= 64 {
        return v as i64;
    }
    let shift = 64 - width;
    ((v << shift) as i64) >> shift
}

/// Index of the highest set bit, or `None` for zero.
pub(crate) const fn highest_set_bit(v: u32) -> Option<u32> {
    if v == 0 {
        None
    } else {
        Some(31 - v.leading_zeros())
    }
}

const fn ones(n: u32) -> u64 {
    if n >= 64 { u64::MAX } else { (1u64 << n) - 1 }
}

const fn ror(v: u64, r: u32, esize: u32) -> u64 {
    let r = r % esize;
    if r == 0 {
        return v;
    }
    ((v >> r) | (v << (esize - r))) & ones(esize)
}

const fn replicate(v: u64, esize: u32, m: u32) -> u64 {
    let mut out = 0u64;
    let mut i = 0;
    while i < m {
        out |= v << i;
        i += esize;
    }
    out & ones(m)
}

/// The pseudocode's `DecodeBitMasks(immN, imms, immr, immediate, M)`: `(wmask,
/// tmask)`, or `None` where the pseudocode is UNDEFINED.
pub(crate) const fn decode_bit_masks(
    n: u32,
    imms: u32,
    immr: u32,
    immediate: bool,
    m: u32,
) -> Option<(u64, u64)> {
    let Some(len) = highest_set_bit((n & 1) << 6 | (!imms & 0x3f)) else {
        return None;
    };
    if len < 1 || (1u32 << len) > m {
        return None;
    }
    let levels = ones(len) as u32;
    if immediate && imms & levels == levels {
        return None;
    }
    let s = imms & levels;
    let r = immr & levels;
    let esize = 1u32 << len;
    let d = (s.wrapping_sub(r)) & levels;
    let welem = ones(s + 1);
    let telem = ones(d + 1);
    Some((
        replicate(ror(welem, r, esize), esize, m),
        replicate(telem, esize, m),
    ))
}

/// Inverse of [`decode_bit_masks`] for logical immediates: the `(N, immr, imms)`
/// that encode `value` as an `m`-bit bitmask immediate.
#[cfg(feature = "encoder")]
pub(crate) fn encode_bit_mask(value: u64, m: u32) -> Option<(u32, u32, u32)> {
    let value = value & ones(m);
    if value == 0 || value == ones(m) {
        return None;
    }
    // Find the smallest element size that replicates to the value.
    let mut esize = m;
    while esize > 2 {
        let half = esize / 2;
        if (value & ones(half)) != ((value >> half) & ones(half)) {
            break;
        }
        esize = half;
    }
    let elem = value & ones(esize);
    // The element must be a rotated run of ones: rotate so that it starts at bit 0.
    let ones_count = elem.count_ones();
    for r in 0..esize {
        // ROR(welem, r) == elem, with welem = ones(s + 1) at the bottom.
        let candidate = ror(ones(ones_count), r, esize);
        if candidate == elem {
            let s = ones_count - 1;
            let len = esize.trailing_zeros();
            let n = u32::from(len == 6);
            // imms is s with the element size above it: ones above bit `len`,
            // a zero at bit `len` (N carries the 64-bit case).
            let imms = if len == 6 {
                s
            } else {
                ((u32::MAX << (len + 1)) & 0x3f) | s
            };
            return Some((n, r, imms));
        }
    }
    None
}

/// The pseudocode's `VFPExpandImm(imm8)` for an `n`-bit format (16, 32 or 64).
pub(crate) fn vfp_expand_imm(imm8: u32, n: u32) -> u64 {
    let e = match n {
        16 => 5,
        32 => 8,
        _ => 11,
    };
    let f = n - e - 1;
    let sign = u64::from(imm8 >> 7 & 1);
    let b6 = u64::from(imm8 >> 6 & 1);
    let exp = ((1 - b6) << (e - 1))
        | (if b6 == 1 { ones(e - 3) << 2 } else { 0 })
        | u64::from(imm8 >> 4 & 3);
    let frac = u64::from(imm8 & 0xf) << (f - 4);
    sign << (n - 1) | exp << f | frac
}

/// The value of an 8-bit FP immediate as an `f64`.
pub(crate) fn fp_imm8_value(imm8: u32) -> f64 {
    f64::from_bits(vfp_expand_imm(imm8, 64))
}

/// Inverse of [`fp_imm8_value`].
#[cfg(feature = "encoder")]
pub(crate) fn fp_imm8_encode(v: f64) -> Option<u32> {
    (0..256u32).find(|&i| fp_imm8_value(i).to_bits() == v.to_bits())
}

/// A xorshift64 generator for the unit tests' random words.
#[cfg(test)]
pub(crate) fn xorshift(mut seed: u64) -> impl FnMut() -> u64 {
    move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields() {
        let f = F::new(5, 5);
        assert_eq!(f.get(0x1f << 5), 31);
        let fs = Fs([F::new(29, 2), F::new(5, 19), F::NONE, F::NONE]);
        assert_eq!(fs.get(3 << 29 | 0x7_ffff << 5), 0x1f_ffff);
        assert_eq!(fs.width(), 21);
    }

    #[test]
    #[cfg(feature = "encoder")]
    fn set_fields() {
        let f = F::new(5, 5);
        let mut w = 0;
        f.set(&mut w, 3);
        assert_eq!(w, 3 << 5);
        let fs = Fs([F::new(29, 2), F::new(5, 19), F::NONE, F::NONE]);
        let mut w = 0;
        fs.set(&mut w, 0x1f_ffff);
        assert_eq!(fs.get(w), 0x1f_ffff);
        assert_eq!(w, 3 << 29 | 0x7_ffff << 5);
        assert_eq!(fs.last(), F::new(5, 19));
        assert_eq!(Fs::one(f).last(), f);
        assert_eq!(Fs::NONE.last(), F::NONE);
    }

    #[test]
    fn bit_masks() {
        // AND x0, x1, #0xff: N=1, immr=0, imms=7.
        assert_eq!(decode_bit_masks(1, 7, 0, true, 64).unwrap().0, 0xff);
        // 0x5555...: N=0, imms=0b111100 (esize 2, one bit set).
        assert_eq!(
            decode_bit_masks(0, 0b11_1100, 0, true, 64).unwrap().0,
            0x5555_5555_5555_5555
        );
        assert!(decode_bit_masks(0, 0b11_1111, 0, true, 64).is_none());
        assert!(decode_bit_masks(1, 0, 0, true, 32).is_none());
    }

    #[test]
    #[cfg(feature = "encoder")]
    fn bit_masks_round_trip() {
        for n in 0..2 {
            for immr in 0..64 {
                for imms in 0..64 {
                    for m in [32, 64] {
                        if let Some((v, _)) = decode_bit_masks(n, imms, immr, true, m) {
                            let (n2, r2, s2) = encode_bit_mask(v, m).unwrap();
                            assert_eq!(
                                decode_bit_masks(n2, s2, r2, true, m).unwrap().0,
                                v,
                                "{n} {immr} {imms} {m}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn fp_imm() {
        assert_eq!(fp_imm8_value(0x70), 1.0);
        assert_eq!(fp_imm8_value(0x00), 2.0);
        assert_eq!(fp_imm8_value(0xf0), -1.0);
        assert_eq!(vfp_expand_imm(0x70, 16), 0x3c00);
        assert_eq!(vfp_expand_imm(0x70, 32), 0x3f80_0000);
    }

    #[test]
    #[cfg(feature = "encoder")]
    fn fp_imm_round_trip() {
        assert_eq!(fp_imm8_encode(0.5), Some(0x60));
        assert_eq!(fp_imm8_encode(0.0), None);
        for i in 0..256 {
            assert_eq!(fp_imm8_encode(fp_imm8_value(i)), Some(i));
        }
    }
}
