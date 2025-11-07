use fixed::types::U0F32;
use fixed::{FixedI8, FixedI16, FixedI32};
use fixed::types::extra::{IsLessOrEqual, True, U5, U6, U8, U13, U14, U16, U29, U30, U32, Unsigned};

use crate::include_bytes_aligned_as;

 
pub trait FixedCordic: 
    Copy
    + PartialOrd
    + core::ops::AddAssign
    + core::ops::SubAssign
    + core::ops::Neg<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::Add<Output = Self>
    + core::ops::Shr<u8, Output = Self>
    + core::ops::Shl<u8, Output = Self> 
    + core::ops::Mul<Output = Self>
    + core::ops::Div<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    const FRAC_PI_2: Self;
    const PI: Self;
    const E: Self;

    const FRAC_BITS: u8;
    const BITS: u8;

    fn floor(self) -> Self;

    fn from_u0f32(val: U0F32) -> Self;
}

impl<Frac> FixedCordic for FixedI32<Frac> 
where
    Frac: 'static
        + Unsigned
        + IsLessOrEqual<U32, Output = True>
        + IsLessOrEqual<U30, Output = True>
        + IsLessOrEqual<U29, Output = True>,
{
    const ZERO: Self = Self::ZERO;

    const ONE: Self = Self::ONE;

    const FRAC_PI_2: Self = Self::FRAC_PI_2;

    const PI: Self = Self::PI;

    const E: Self = Self::E;

    const FRAC_BITS: u8 = Frac::U8;

    const BITS: u8 = 32;

    fn floor(self) -> Self {
        self.floor()
    }

    fn from_u0f32(val: U0F32) -> Self {
        Self::from_num(val)
    }
}

impl<Frac> FixedCordic for FixedI16<Frac> 
where
    Frac: 'static
        + Unsigned
        + IsLessOrEqual<U16, Output = True>
        + IsLessOrEqual<U14, Output = True>
        + IsLessOrEqual<U13, Output = True>,
{
    const ZERO: Self = Self::ZERO;

    const ONE: Self = Self::ONE;

    const FRAC_PI_2: Self = Self::FRAC_PI_2;

    const PI: Self = Self::PI;

    const E: Self = Self::E;

    const FRAC_BITS: u8 = Frac::U8;

    const BITS: u8 = 16;

    fn floor(self) -> Self {
        self.floor()
    }

    fn from_u0f32(val: U0F32) -> Self {
        Self::from_num(val)
    }
}

impl<Frac> FixedCordic for FixedI8<Frac> 
where
    Frac: 'static
        + Unsigned
        + IsLessOrEqual<U8, Output = True>
        + IsLessOrEqual<U6, Output = True>
        + IsLessOrEqual<U5, Output = True>,
{
    const ZERO: Self = Self::ZERO;

    const ONE: Self = Self::ONE;

    const FRAC_PI_2: Self = Self::FRAC_PI_2;

    const PI: Self = Self::PI;

    const E: Self = Self::E;

    const FRAC_BITS: u8 = Frac::U8;

    const BITS: u8 = 8;

    fn floor(self) -> Self {
        self.floor()
    }

    fn from_u0f32(val: U0F32) -> Self {
        Self::from_num(val)
    }
}

const ATAN_TABLE: &'static [u32] = include_bytes_aligned_as!(u32, "atan_u0f32.bin");
const ATANH_TABLE: &'static [u32] = include_bytes_aligned_as!(u32, "atanh_u0f32.bin");
// const EXPM1_TABLE: &'static [u32] = include_bytes_aligned_as!(u32, "expm1_u0f32.bin");

const INV_GAIN: U0F32 = U0F32::from_bits(0x9B74EDA8); // 0.607252935009
const HYP_GAIN_M1: U0F32 = U0F32::from_bits(0x351E777E); // 0.20749613601

#[inline]
fn cordic_circular<T: FixedCordic>(mut x: T, mut y: T, mut z: T, vecmode: T) -> (T, T, T) {
    let mut i = 0u8;

    while i < T::FRAC_BITS {
        if vecmode >= T::ZERO && y < vecmode || vecmode < T::ZERO && z >= T::ZERO {
            let x1 = x - (y >> i);
            y = y + (x >> i);
            x = x1;
            z = z - T::from_u0f32(U0F32::from_bits(ATAN_TABLE[i as usize]));
        } else {
            let x1 = x + (y >> i);
            y = y - (x >> i);
            x = x1;
            z = z + T::from_u0f32(U0F32::from_bits(ATAN_TABLE[i as usize]));
        }
        i += 1;
    }

    (x, y, z)
}

#[inline]
fn cordic_hyperbolic<T: FixedCordic>(mut x: T, mut y: T, mut z: T, vecmode: T) -> (T, T, T) {
    let mut i = 1u8;
    let mut k = 3u8;

    while i < T::FRAC_BITS {
        let mut j = 0u8;

        while j < 2 {
            if vecmode >= T::ZERO && y < vecmode || vecmode < T::ZERO && z >= T::ZERO {
                let x1 = x + (y >> i);
                y = y + (x >> i);
                x = x1;
                z = z - T::from_u0f32(U0F32::from_bits(ATANH_TABLE[(i-1) as usize]));
            } else {
                let x1 = x - (y >> i);
                y = y - (x >> i);
                x = x1;
                z = z + T::from_u0f32(U0F32::from_bits(ATANH_TABLE[(i-1) as usize]));
            }

            if k > 0 {
                k -= 1;
                break;
            } else { k = 3; }

            j += 1;
        }

        i += 1;
    }

    (x, y, z)
}

#[inline]
fn sin_cos<T: FixedCordic>(mut angle: T) -> (T, T) {
    let mut negative = false;

    while angle > T::FRAC_PI_2 {
        angle -= T::PI;
        negative = !negative;
    }

    while angle < -T::FRAC_PI_2 {
        angle += T::PI;
        negative = !negative;
    }

    let res = cordic_circular(T::from_u0f32(INV_GAIN), T::ZERO, angle, -T::ONE);

    if negative {
        (-res.1, -res.0)
    } else {
        (res.1, res.0)
    }
}

#[inline]
fn asin<T: FixedCordic>(mut val: T) -> T {
    // For asin, we use a double-rotation approach to reduce errors.
    // NOTE: see https://stackoverflow.com/questions/25976656/cordic-arcsine-implementation-fails
    // for details about the inaccuracy of CORDIC for asin.

    let mut theta = T::ZERO;
    let mut x = T::ONE;
    let mut y = T::ONE;
    
    let mut i = 0u8;

    while i < T::FRAC_BITS {
        let sigma = (y <= val) == (x < T::ZERO); // == is XOR for bools

        {
            let dx = if sigma { -(y >> i) } else { y >> i };
            let dy = if sigma { -(x >> i) } else { x >> i };
            x -= dx;
            y += dy;
        }

        {
            let dx = if sigma { -(y >> i) } else { y >> i };
            let dy = if sigma { -(x >> i) } else { x >> i };
            x -= dx;
            y += dy;
        }

        let angle = T::from_u0f32(U0F32::from_bits(ATAN_TABLE[i as usize]));
        theta += if sigma { -(angle << 1) } else { angle << 1 };
        val += val >> (i << 1);

        i += 1;
    }

    theta
}

pub trait FixedCordicMath: FixedCordic {
    fn cordic_circular(x: Self, y: Self, z: Self, vecmode: Self) -> (Self, Self, Self) {
        cordic_circular(x, y, z, vecmode)
    }

    fn cordic_hyperbolic(x: Self, y: Self, z: Self, vecmode: Self) -> (Self, Self, Self) {
        cordic_hyperbolic(x, y, z, vecmode)
    }

    fn sin_cos(self) -> (Self, Self) {
        sin_cos(self)
    }

    fn sin(self) -> Self {
        self.sin_cos().0
    }

    fn cos(self) -> Self {
        self.sin_cos().1
    }

    fn tan(self) -> Self {
        let (sin, cos) = self.sin_cos();
        sin / cos
    }

    fn atan(self) -> Self {
        cordic_circular(Self::ONE, self, Self::ZERO, Self::ZERO).2
    }

    fn asin(self) -> Self {
        asin(self)
    }

    fn acos(self) -> Self {
        Self::FRAC_PI_2 - asin(self)
    }
}

impl<T: FixedCordic> FixedCordicMath for T {}
