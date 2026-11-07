//! Register helpers over the generated [`Register`] enum, whose classes are laid
//! out contiguously: `W0..W30, WZR, WSP, X0..X30, XZR, SP, B*, H*, S*, D*, Q*, V*,
//! Z*, P*, PN*, ZA, ZT0, ZA tiles, NZCV, FPCR, FPSR, FFR, PC`.

use crate::generated::Register;

const W0: usize = Register::W0 as usize;
const X0: usize = Register::X0 as usize;
const B0: usize = Register::B0 as usize;
const H0: usize = Register::H0 as usize;
const S0: usize = Register::S0 as usize;
const D0: usize = Register::D0 as usize;
const Q0: usize = Register::Q0 as usize;
const V0: usize = Register::V0 as usize;
const Z0: usize = Register::Z0 as usize;
const P0: usize = Register::P0 as usize;
const PN0: usize = Register::PN0 as usize;

// The layout the helpers below rely on: 33 W and X registers (with ZR and SP),
// 32 of each SIMD&FP and SVE view, 16 predicates.
const _: () = assert!(
    X0 == W0 + 33
        && B0 == X0 + 33
        && H0 == B0 + 32
        && S0 == H0 + 32
        && D0 == S0 + 32
        && Q0 == D0 + 32
        && V0 == Q0 + 32
        && Z0 == V0 + 32
        && P0 == Z0 + 32
        && PN0 == P0 + 16
        && Register::ZA as usize == PN0 + 16
        && Register::WZR as usize == W0 + 31
        && Register::SP as usize == X0 + 32
);

/// The class of a register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RegisterClass {
    None,
    /// `W0`–`W30`, `WZR`, `WSP`.
    Gp32,
    /// `X0`–`X30`, `XZR`, `SP`.
    Gp64,
    B,
    H,
    S,
    D,
    Q,
    /// `V0`–`V31` (the 128-bit vector registers).
    V,
    Z,
    P,
    PN,
    /// `ZA`, `ZT0` and the ZA tiles.
    Za,
    /// `NZCV`, `FPCR`, `FPSR`, `FFR`, `PC`.
    Special,
}

impl Register {
    fn at(i: usize) -> Register {
        Register::from_index(i).unwrap_or(Register::None)
    }

    /// `Wn`; 31 is `WZR`.
    pub fn w(n: u32) -> Register {
        Self::at(W0 + n.min(31) as usize)
    }

    /// `Xn`; 31 is `XZR`.
    pub fn x(n: u32) -> Register {
        Self::at(X0 + n.min(31) as usize)
    }

    /// `Wn`, or `WSP` for 31.
    pub fn w_or_sp(n: u32) -> Register {
        if n >= 31 { Register::WSP } else { Self::w(n) }
    }

    /// `Xn`, or `SP` for 31.
    pub fn x_or_sp(n: u32) -> Register {
        if n >= 31 { Register::SP } else { Self::x(n) }
    }

    pub fn b(n: u32) -> Register {
        Self::at(B0 + (n & 31) as usize)
    }
    pub fn h(n: u32) -> Register {
        Self::at(H0 + (n & 31) as usize)
    }
    pub fn s(n: u32) -> Register {
        Self::at(S0 + (n & 31) as usize)
    }
    pub fn d(n: u32) -> Register {
        Self::at(D0 + (n & 31) as usize)
    }
    pub fn q(n: u32) -> Register {
        Self::at(Q0 + (n & 31) as usize)
    }
    pub fn v(n: u32) -> Register {
        Self::at(V0 + (n & 31) as usize)
    }
    pub fn z(n: u32) -> Register {
        Self::at(Z0 + (n & 31) as usize)
    }
    pub fn p(n: u32) -> Register {
        Self::at(P0 + (n & 15) as usize)
    }
    pub fn pn(n: u32) -> Register {
        Self::at(PN0 + (n & 15) as usize)
    }

    /// The ZA tile `ZA<n>.<size>` (`es_log2`: 0 for B .. 4 for Q), if it exists.
    pub fn za_tile(es_log2: u32, n: u32) -> Option<Register> {
        if es_log2 > 4 || n >= 1 << es_log2 {
            return None;
        }
        // ZAB0, ZAH0-1, ZAS0-3, ZAD0-7, ZAQ0-15 follow ZA and ZT0.
        let first = Register::ZAB0 as usize + (1 << es_log2) - 1;
        Register::from_index(first + n as usize)
    }

    /// A ZA tile's element size (log2 bytes) and number.
    pub fn za_tile_info(self) -> Option<(u32, u32)> {
        let i = (self as usize).checked_sub(Register::ZAB0 as usize)?;
        if self.class() != RegisterClass::Za {
            return None;
        }
        let es = (i as u32 + 1).ilog2();
        Some((es, i as u32 + 1 - (1 << es)))
    }

    pub fn class(self) -> RegisterClass {
        let i = self as usize;
        match i {
            0 => RegisterClass::None,
            _ if i < X0 => RegisterClass::Gp32,
            _ if i < B0 => RegisterClass::Gp64,
            _ if i < H0 => RegisterClass::B,
            _ if i < S0 => RegisterClass::H,
            _ if i < D0 => RegisterClass::S,
            _ if i < Q0 => RegisterClass::D,
            _ if i < V0 => RegisterClass::Q,
            _ if i < Z0 => RegisterClass::V,
            _ if i < P0 => RegisterClass::Z,
            _ if i < PN0 => RegisterClass::P,
            _ if i < PN0 + 16 => RegisterClass::PN,
            _ if (Register::ZA as usize..Register::NZCV as usize).contains(&i) => RegisterClass::Za,
            _ => RegisterClass::Special,
        }
    }

    /// The register number within its class (`XZR` and `SP` are both 31).
    pub fn number(self) -> u32 {
        let i = self as usize;
        let base = match self.class() {
            RegisterClass::Gp32 => {
                return if self == Register::WSP {
                    31
                } else {
                    (i - W0) as u32
                };
            }
            RegisterClass::Gp64 => {
                return if self == Register::SP {
                    31
                } else {
                    (i - X0) as u32
                };
            }
            RegisterClass::B => B0,
            RegisterClass::H => H0,
            RegisterClass::S => S0,
            RegisterClass::D => D0,
            RegisterClass::Q => Q0,
            RegisterClass::V => V0,
            RegisterClass::Z => Z0,
            RegisterClass::P => P0,
            RegisterClass::PN => PN0,
            _ => return 0,
        };
        (i - base) as u32
    }

    pub fn is_sp(self) -> bool {
        matches!(self, Register::SP | Register::WSP)
    }

    pub fn is_zero(self) -> bool {
        matches!(self, Register::XZR | Register::WZR)
    }

    /// The architectural register this one is a view of: `Wn` → `Xn`, `WSP` → `SP`,
    /// `Bn`..`Qn` → `Vn`. Other registers map to themselves.
    pub fn full_register(self) -> Register {
        match self.class() {
            RegisterClass::Gp32 => match self {
                Register::WSP => Register::SP,
                Register::WZR => Register::XZR,
                _ => Register::x(self.number()),
            },
            RegisterClass::B
            | RegisterClass::H
            | RegisterClass::S
            | RegisterClass::D
            | RegisterClass::Q => Register::v(self.number()),
            _ => self,
        }
    }

    /// Size in bytes (vectors: 16; `Z`/`P`/`ZA`: 0, scalable).
    pub fn size(self) -> u32 {
        match self.class() {
            RegisterClass::Gp32 => 4,
            RegisterClass::Gp64 => 8,
            RegisterClass::B => 1,
            RegisterClass::H => 2,
            RegisterClass::S => 4,
            RegisterClass::D => 8,
            RegisterClass::Q | RegisterClass::V => 16,
            RegisterClass::Special => match self {
                Register::PC => 8,
                Register::NZCV | Register::FPCR | Register::FPSR => 4,
                _ => 0,
            },
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout() {
        assert_eq!(Register::w(0), Register::W0);
        assert_eq!(Register::w(31), Register::WZR);
        assert_eq!(Register::x(30), Register::X30);
        assert_eq!(Register::x_or_sp(31), Register::SP);
        assert_eq!(Register::b(5), Register::B5);
        assert_eq!(Register::q(31), Register::Q31);
        assert_eq!(Register::v(7), Register::V7);
        assert_eq!(Register::z(31), Register::Z31);
        assert_eq!(Register::p(15), Register::P15);
        assert_eq!(Register::pn(3), Register::PN3);
        assert_eq!(Register::SP.number(), 31);
        assert_eq!(Register::W5.full_register(), Register::X5);
        assert_eq!(Register::S9.full_register(), Register::V9);
        assert_eq!(Register::ZAD7.class(), RegisterClass::Za);
        assert_eq!(Register::za_tile(2, 3), Some(Register::ZAS3));
        assert_eq!(Register::za_tile(3, 8), None);
        assert_eq!(Register::za_tile(4, 15), Some(Register::ZAQ15));
        assert_eq!(Register::ZAH1.za_tile_info(), Some((1, 1)));
        assert_eq!(Register::ZAB0.za_tile_info(), Some((0, 0)));
        assert_eq!(Register::ZT0.za_tile_info(), None);
        assert_eq!(Register::NZCV.class(), RegisterClass::Special);
        assert_eq!(Register::X3.name(), "x3");
    }
}
