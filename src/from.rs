// FEATURE: (BLOCKED) It would be nice to impl From<_> as well, but then the
// generic implementation `impl<T: Into<U>, U> TryFrom<U> for T` conflicts with
// our own implementation. This means we can only implement one.
// In principle this can be worked around by `specialization`, but that
// triggers other compiler issues at the moment.

// impl<T, const BITS: usize> From<T> for Uint<BITS>
// where
//     [(); nlimbs(BITS)]:,
//     Uint<BITS>: TryFrom<T>,
// {
//     fn from(t: T) -> Self {
//         Self::try_from(t).unwrap()
//     }
// }
// See <https://github.com/rust-lang/rust/issues/50133>

// FEATURE: (BLOCKED) It would be nice if we could make TryFrom assignment work
// for all Uints.
// impl<
//         const BITS_SRC: usize,
//         const LIMBS_SRC: usize,
//         const BITS_DST: usize,
//         const LIMBS_DST: usize,
//     > TryFrom<Uint<BITS_SRC, LIMBS_SRC>> for Uint<BITS_DST, LIMBS_DST>
// {
//     type Error = ToUintError;

//     fn try_from(value: Uint<BITS_SRC, LIMBS_SRC>) -> Result<Self,
// Self::Error> {
//     }
// }

use crate::Uint;
use core::{fmt, fmt::Debug};

/// Error for [`TryFrom<T>`][TryFrom] for [`Uint`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ToUintError<T> {
    /// Value is too large to fit the Uint.
    ///
    /// `.0` is `BITS` and `.1` is the wrapped value.
    ValueTooLarge(usize, T),

    /// Negative values can not be represented as Uint.
    ///
    /// `.0` is `BITS` and `.1` is the wrapped value.
    ValueNegative(usize, T),

    /// 'Not a number' (NaN) can not be represented as Uint
    NotANumber(usize),
}

#[cfg(feature = "std")]
impl<T: fmt::Debug> std::error::Error for ToUintError<T> {}

impl<T> fmt::Display for ToUintError<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueTooLarge(bits, _) => write!(f, "Value is too large for Uint<{bits}>"),
            Self::ValueNegative(bits, _) => {
                write!(f, "Negative values cannot be represented as Uint<{bits}>")
            }
            Self::NotANumber(bits) => {
                write!(
                    f,
                    "'Not a number' (NaN) cannot be represented as Uint<{bits}>"
                )
            }
        }
    }
}

/// Error for [`TryFrom<Uint>`][TryFrom].
#[allow(clippy::derive_partial_eq_without_eq)] // False positive
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FromUintError<T> {
    /// The Uint value is too large for the target type.
    ///
    /// `.0` number of `BITS` in the Uint, `.1` is the wrapped value and
    /// `.2` is the maximum representable value in the target type.
    Overflow(usize, T, T),
}

#[cfg(feature = "std")]
impl<T: fmt::Debug> std::error::Error for FromUintError<T> {}

impl<T> fmt::Display for FromUintError<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow(bits, ..) => write!(
                f,
                "Uint<{bits}> value is too large for {}",
                core::any::type_name::<T>()
            ),
        }
    }
}

/// Error for [`TryFrom<Uint>`][TryFrom] for [`ark_ff`](https://docs.rs/ark-ff) and others.
#[allow(dead_code)] // This is used by some support features.
#[derive(Debug, Clone, Copy)]
pub enum ToFieldError {
    /// Number is equal or larger than the target field modulus.
    NotInField,
}

#[cfg(feature = "std")]
impl std::error::Error for ToFieldError {}

impl fmt::Display for ToFieldError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInField => {
                f.write_str("Number is equal or larger than the target field modulus.")
            }
        }
    }
}

impl<const BITS: usize, const LIMBS: usize> Uint<BITS, LIMBS> {
    /// Constructs a new [`Uint`] from a u64.
    ///
    /// Saturates at the maximum value of the [`Uint`] if the value is too
    /// large.
    pub(crate) const fn const_from_u64(x: u64) -> Self {
        if BITS == 0 || (BITS < 64 && x >= 1 << BITS) {
            return Self::MAX;
        }
        let mut limbs = [0; LIMBS];
        limbs[0] = x;
        Self::from_limbs(limbs)
    }

    /// Construct a new [`Uint`] from the value.
    ///
    /// # Panics
    ///
    /// Panics if the conversion fails, for example if the value is too large
    /// for the bit-size of the [`Uint`]. The panic will be attributed to the
    /// call site.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ruint::{Uint, uint, aliases::*};
    /// # uint!{
    /// assert_eq!(U8::from(142_u16), 142_U8);
    /// assert_eq!(U64::from(0x7014b4c2d1f2_U256), 0x7014b4c2d1f2_U64);
    /// assert_eq!(U64::from(3.145), 3_U64);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn from<T>(value: T) -> Self
    where
        Self: UintTryFrom<T>,
    {
        match Self::uint_try_from(value) {
            Ok(n) => n,
            Err(e) => panic!("Uint conversion error: {e}"),
        }
    }

    /// Construct a new [`Uint`] from the value saturating the value to the
    /// minimum or maximum value of the [`Uint`].
    ///
    /// If the value is not a number (like `f64::NAN`), then the result is
    /// set zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ruint::{Uint, uint, aliases::*};
    /// # uint!{
    /// assert_eq!(U8::saturating_from(300_u16), 255_U8);
    /// assert_eq!(U8::saturating_from(-10_i16), 0_U8);
    /// assert_eq!(U32::saturating_from(0x7014b4c2d1f2_U256), U32::MAX);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn saturating_from<T>(value: T) -> Self
    where
        Self: UintTryFrom<T>,
    {
        match Self::uint_try_from(value) {
            Ok(n) => n,
            Err(ToUintError::ValueTooLarge(..)) => Self::MAX,
            Err(ToUintError::ValueNegative(..) | ToUintError::NotANumber(_)) => Self::ZERO,
        }
    }

    /// Construct a new [`Uint`] from the value saturating the value to the
    /// minimum or maximum value of the [`Uint`].
    ///
    /// If the value is not a number (like `f64::NAN`), then the result is
    /// set zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ruint::{Uint, uint, aliases::*};
    /// # uint!{
    /// assert_eq!(U8::wrapping_from(300_u16), 44_U8);
    /// assert_eq!(U8::wrapping_from(-10_i16), 246_U8);
    /// assert_eq!(U32::wrapping_from(0x7014b4c2d1f2_U256), 0xb4c2d1f2_U32);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn wrapping_from<T>(value: T) -> Self
    where
        Self: UintTryFrom<T>,
    {
        match Self::uint_try_from(value) {
            Ok(n) | Err(ToUintError::ValueTooLarge(_, n) | ToUintError::ValueNegative(_, n)) => n,
            Err(ToUintError::NotANumber(_)) => Self::ZERO,
        }
    }

    /// # Panics
    ///
    /// Panics if the conversion fails, for example if the value is too large
    /// for the bit-size of the target type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ruint::{Uint, uint, aliases::*};
    /// # uint!{
    /// assert_eq!(300_U12.to::<i16>(), 300_i16);
    /// assert_eq!(300_U12.to::<U256>(), 300_U256);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn to<T>(&self) -> T
    where
        Self: UintTryTo<T>,
        T: Debug,
    {
        self.uint_try_to().expect("Uint conversion error")
    }

    /// # Examples
    ///
    /// ```
    /// # use ruint::{Uint, uint, aliases::*};
    /// # uint!{
    /// assert_eq!(300_U12.wrapping_to::<i8>(), 44_i8);
    /// assert_eq!(255_U32.wrapping_to::<i8>(), -1_i8);
    /// assert_eq!(0x1337cafec0d3_U256.wrapping_to::<U32>(), 0xcafec0d3_U32);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn wrapping_to<T>(&self) -> T
    where
        Self: UintTryTo<T>,
    {
        match self.uint_try_to() {
            Ok(n) | Err(FromUintError::Overflow(_, n, _)) => n,
        }
    }

    /// # Examples
    ///
    /// ```
    /// # use ruint::{Uint, uint, aliases::*};
    /// # uint!{
    /// assert_eq!(300_U12.saturating_to::<i16>(), 300_i16);
    /// assert_eq!(255_U32.saturating_to::<i8>(), 127);
    /// assert_eq!(0x1337cafec0d3_U256.saturating_to::<U32>(), U32::MAX);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn saturating_to<T>(&self) -> T
    where
        Self: UintTryTo<T>,
    {
        match self.uint_try_to() {
            Ok(n) | Err(FromUintError::Overflow(_, _, n)) => n,
        }
    }

    /// Construct a new [`Uint`] from a potentially different sized [`Uint`].
    ///
    /// # Panics
    ///
    /// Panics if the value is too large for the target type.
    #[inline]
    #[doc(hidden)]
    #[must_use]
    #[track_caller]
    #[deprecated(since = "1.4.0", note = "Use `::from()` instead.")]
    pub fn from_uint<const BITS_SRC: usize, const LIMBS_SRC: usize>(
        value: Uint<BITS_SRC, LIMBS_SRC>,
    ) -> Self {
        Self::from_limbs_slice(value.as_limbs())
    }

    #[inline]
    #[doc(hidden)]
    #[must_use]
    #[deprecated(since = "1.4.0", note = "Use `::checked_from()` instead.")]
    pub fn checked_from_uint<const BITS_SRC: usize, const LIMBS_SRC: usize>(
        value: Uint<BITS_SRC, LIMBS_SRC>,
    ) -> Option<Self> {
        Self::checked_from_limbs_slice(value.as_limbs())
    }

    /// Returns `true` if `self` is larger than 64 bits.
    #[inline]
    fn gt_u64_max(&self) -> bool {
        self.limbs_gt(1)
    }

    /// Returns `true` if `self` is larger than 128 bits.
    #[inline]
    fn gt_u128_max(&self) -> bool {
        self.limbs_gt(2)
    }

    /// Returns `true` if `self` is larger than `64 * n` bits.
    #[inline]
    fn limbs_gt(&self, n: usize) -> bool {
        if LIMBS < n {
            return false;
        }

        if BITS <= 512 {
            // Use branchless `bitor` chain for smaller integers.
            self.as_limbs()[n..]
                .iter()
                .copied()
                .fold(0u64, core::ops::BitOr::bitor)
                != 0
        } else {
            self.bit_len() > 64 * n
        }
    }
}

/// ⚠️ Workaround for [Rust issue #50133](https://github.com/rust-lang/rust/issues/50133).
/// Use [`TryFrom`] instead.
///
/// We cannot implement [`TryFrom<Uint>`] for [`Uint`] directly, but we can
/// create a new identical trait and implement it there. We can even give this
/// trait a blanket implementation inheriting all [`TryFrom<_>`]
/// implementations.
#[allow(clippy::module_name_repetitions)]
pub trait UintTryFrom<T>: Sized {
    #[doc(hidden)]
    fn uint_try_from(value: T) -> Result<Self, ToUintError<Self>>;
}

/// Blanket implementation for any type that implements [`TryFrom<Uint>`].
impl<const BITS: usize, const LIMBS: usize, T> UintTryFrom<T> for Uint<BITS, LIMBS>
where
    Self: TryFrom<T, Error = ToUintError<Self>>,
{
    #[inline]
    fn uint_try_from(value: T) -> Result<Self, ToUintError<Self>> {
        Self::try_from(value)
    }
}

impl<const BITS: usize, const LIMBS: usize, const BITS_SRC: usize, const LIMBS_SRC: usize>
    UintTryFrom<Uint<BITS_SRC, LIMBS_SRC>> for Uint<BITS, LIMBS>
{
    #[inline]
    fn uint_try_from(value: Uint<BITS_SRC, LIMBS_SRC>) -> Result<Self, ToUintError<Self>> {
        let (n, overflow) = Self::overflowing_from_limbs_slice(value.as_limbs());
        if overflow {
            Err(ToUintError::ValueTooLarge(BITS, n))
        } else {
            Ok(n)
        }
    }
}

/// ⚠️ Workaround for [Rust issue #50133](https://github.com/rust-lang/rust/issues/50133).
/// Use [`TryFrom`] instead.
pub trait UintTryTo<T>: Sized {
    #[doc(hidden)]
    fn uint_try_to(&self) -> Result<T, FromUintError<T>>;
}

impl<const BITS: usize, const LIMBS: usize, T> UintTryTo<T> for Uint<BITS, LIMBS>
where
    T: for<'a> TryFrom<&'a Self, Error = FromUintError<T>>,
{
    #[inline]
    fn uint_try_to(&self) -> Result<T, FromUintError<T>> {
        T::try_from(self)
    }
}

impl<const BITS: usize, const LIMBS: usize, const BITS_DST: usize, const LIMBS_DST: usize>
    UintTryTo<Uint<BITS_DST, LIMBS_DST>> for Uint<BITS, LIMBS>
{
    #[inline]
    fn uint_try_to(
        &self,
    ) -> Result<Uint<BITS_DST, LIMBS_DST>, FromUintError<Uint<BITS_DST, LIMBS_DST>>> {
        let (n, overflow) = Uint::overflowing_from_limbs_slice(self.as_limbs());
        if overflow {
            Err(FromUintError::Overflow(BITS_DST, n, Uint::MAX))
        } else {
            Ok(n)
        }
    }
}

// u64 is a single limb, so this is the base case
impl<const BITS: usize, const LIMBS: usize> TryFrom<u64> for Uint<BITS, LIMBS> {
    type Error = ToUintError<Self>;

    #[inline]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match LIMBS {
            0 | 1 if value > Self::MASK => {
                return Err(ToUintError::ValueTooLarge(
                    BITS,
                    Self::from_limbs([value & Self::MASK; LIMBS]),
                ))
            }
            0 => return Ok(Self::ZERO),
            _ => {}
        }
        let mut limbs = [0; LIMBS];
        limbs[0] = value;
        Ok(Self::from_limbs(limbs))
    }
}

// u128 version is handled specially in because it covers two limbs.
impl<const BITS: usize, const LIMBS: usize> TryFrom<u128> for Uint<BITS, LIMBS> {
    type Error = ToUintError<Self>;

    #[inline]
    #[allow(clippy::cast_lossless)]
    #[allow(clippy::cast_possible_truncation)]
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        if value <= u64::MAX as u128 {
            return Self::try_from(value as u64);
        }
        if LIMBS < 2 {
            return Self::try_from(value as u64)
                .and_then(|n| Err(ToUintError::ValueTooLarge(BITS, n)));
        }
        let mut limbs = [0; LIMBS];
        limbs[0] = value as u64;
        limbs[1] = (value >> 64) as u64;
        if LIMBS == 2 && limbs[1] > Self::MASK {
            limbs[1] &= Self::MASK;
            Err(ToUintError::ValueTooLarge(BITS, Self::from_limbs(limbs)))
        } else {
            Ok(Self::from_limbs(limbs))
        }
    }
}

// Unsigned int version upcast to u64
macro_rules! impl_from_unsigned_int {
    ($uint:ty) => {
        impl<const BITS: usize, const LIMBS: usize> TryFrom<$uint> for Uint<BITS, LIMBS> {
            type Error = ToUintError<Self>;

            #[inline]
            fn try_from(value: $uint) -> Result<Self, Self::Error> {
                Self::try_from(value as u64)
            }
        }
    };
}

impl_from_unsigned_int!(bool);
impl_from_unsigned_int!(u8);
impl_from_unsigned_int!(u16);
impl_from_unsigned_int!(u32);
impl_from_unsigned_int!(usize);

// Signed int version check for positive and delegate to the corresponding
// `uint`.
macro_rules! impl_from_signed_int {
    ($int:ty, $uint:ty) => {
        impl<const BITS: usize, const LIMBS: usize> TryFrom<$int> for Uint<BITS, LIMBS> {
            type Error = ToUintError<Self>;

            #[inline]
            fn try_from(value: $int) -> Result<Self, Self::Error> {
                if value.is_negative() {
                    Err(match Self::try_from(value as $uint) {
                        Ok(n) | Err(ToUintError::ValueTooLarge(_, n)) => {
                            ToUintError::ValueNegative(BITS, n)
                        }
                        _ => unreachable!(),
                    })
                } else {
                    Self::try_from(value as $uint)
                }
            }
        }
    };
}

impl_from_signed_int!(i8, u8);
impl_from_signed_int!(i16, u16);
impl_from_signed_int!(i32, u32);
impl_from_signed_int!(i64, u64);
impl_from_signed_int!(i128, u128);
impl_from_signed_int!(isize, usize);

#[cfg(feature = "std")]
impl<const BITS: usize, const LIMBS: usize> TryFrom<f64> for Uint<BITS, LIMBS> {
    type Error = ToUintError<Self>;

    #[inline]
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        // mimics Rust's own float to int conversion
        // https://github.com/rust-lang/compiler-builtins/blob/f4c7940d3b13ec879c9fdc218812f71a65149123/src/float/conv.rs#L163

        let f = value;
        let fixint_min = Self::ZERO;
        let fixint_max = Self::MAX;
        let fixint_bits = Self::BITS;
        let fixint_unsigned = fixint_min == Self::ZERO;

        let sign_bit = 0x8000_0000_0000_0000u64;
        let significand_bits = 52usize;
        let exponent_bias = 1023usize;

        if value < 0.5 {
            return Ok(Self::ZERO);
        }

        // Break a into sign, exponent, significand
        let a_rep = f.to_bits();
        let a_abs = a_rep & !sign_bit;

        // this is used to work around -1 not being available for unsigned
        let sign = if (a_rep & sign_bit) == 0 {
            Sign::Positive
        } else {
            Sign::Negative
        };
        let mut exponent = (a_abs >> significand_bits) as usize;
        let significand = (a_abs & ((1u64 << significand_bits) - 1)) | (1u64 << significand_bits);

        // if < 1 or unsigned & negative
        if exponent < exponent_bias || fixint_unsigned && sign == Sign::Negative {
            return Err(ToUintError::ValueNegative(BITS, fixint_min));
        }
        exponent -= exponent_bias;

        // If the value is infinity, saturate.
        // If the value is too large for the integer type, 0.
        if exponent >= fixint_bits {
            return if sign == Sign::Positive {
                Err(ToUintError::ValueTooLarge(BITS, fixint_max))
            } else {
                Err(ToUintError::ValueNegative(BITS, fixint_min))
            };
        }

        // If 0 <= exponent < significand_bits, right shift to get the result.
        // Otherwise, shift left.
        let r = if exponent < significand_bits {
            // Round to nearest, ties to even
            let shift = significand_bits - exponent;
            let mut r = significand >> shift;
            let remainder = significand & ((1u64 << shift) - 1);
            let halfway = 1u64 << (shift - 1);
            if remainder > halfway || (remainder == halfway && (r & 1) == 1) {
                r = r.wrapping_add(1);
            }
            Self::from(r)
        } else {
            (Self::from(significand)) << (exponent - significand_bits)
        };

        Ok(r)
    }
}

#[derive(PartialEq)]
enum Sign {
    Positive,
    Negative,
}


#[cfg(feature = "std")]
impl<const BITS: usize, const LIMBS: usize> TryFrom<f32> for Uint<BITS, LIMBS> {
    type Error = ToUintError<Self>;

    #[inline]
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        #[allow(clippy::cast_lossless)]
        Self::try_from(value as f64)
    }
}

// Convert Uint to integer types

// Required because a generic rule violates the orphan rule
macro_rules! to_value_to_ref {
    ($t:ty) => {
        impl<const BITS: usize, const LIMBS: usize> TryFrom<Uint<BITS, LIMBS>> for $t {
            type Error = FromUintError<Self>;

            #[inline]
            fn try_from(value: Uint<BITS, LIMBS>) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }
    };
}

to_value_to_ref!(bool);

impl<const BITS: usize, const LIMBS: usize> TryFrom<&Uint<BITS, LIMBS>> for bool {
    type Error = FromUintError<Self>;

    #[inline]
    fn try_from(value: &Uint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        if BITS == 0 {
            return Ok(false);
        }
        if value.gt_u64_max() || value.limbs[0] > 1 {
            return Err(Self::Error::Overflow(BITS, value.bit(0), true));
        }
        Ok(value.limbs[0] != 0)
    }
}

macro_rules! to_int {
    ($($int:ty)*) => {$(
        to_value_to_ref!($int);

        impl<const BITS: usize, const LIMBS: usize> TryFrom<&Uint<BITS, LIMBS>> for $int {
            type Error = FromUintError<Self>;

            #[inline]
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            fn try_from(value: &Uint<BITS, LIMBS>) -> Result<Self, Self::Error> {
                if BITS == 0 {
                    return Ok(0);
                }
                if value.gt_u64_max() || value.limbs[0] > (Self::MAX as u64) {
                    return Err(Self::Error::Overflow(
                        BITS,
                        value.limbs[0] as Self,
                        Self::MAX,
                    ));
                }
                Ok(value.limbs[0] as Self)
            }
        }
    )*};
}

to_int!(i8 u8 i16 u16 i32 u32 i64 u64 isize usize);

to_value_to_ref!(i128);

impl<const BITS: usize, const LIMBS: usize> TryFrom<&Uint<BITS, LIMBS>> for i128 {
    type Error = FromUintError<Self>;

    #[inline]
    #[allow(clippy::cast_possible_wrap)] // Intentional.
    #[allow(clippy::cast_lossless)] // Safe casts
    #[allow(clippy::use_self)] // More readable
    fn try_from(value: &Uint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        if BITS <= 64 {
            return Ok(u64::try_from(value).unwrap().into());
        }
        let result = value.as_double_words()[0].get();
        if value.gt_u128_max() || result > i128::MAX as u128 {
            return Err(Self::Error::Overflow(BITS, result as i128, i128::MAX));
        }
        Ok(result as i128)
    }
}

to_value_to_ref!(u128);

impl<const BITS: usize, const LIMBS: usize> TryFrom<&Uint<BITS, LIMBS>> for u128 {
    type Error = FromUintError<Self>;

    #[inline]
    #[allow(clippy::cast_lossless)] // Safe casts
    #[allow(clippy::use_self)] // More readable
    fn try_from(value: &Uint<BITS, LIMBS>) -> Result<Self, Self::Error> {
        if BITS <= 64 {
            return Ok(u64::try_from(value).unwrap().into());
        }
        let result = value.as_double_words()[0].get();
        if value.gt_u128_max() {
            return Err(Self::Error::Overflow(BITS, result, u128::MAX));
        }
        Ok(result)
    }
}

// Convert Uint to floating point

#[cfg(feature = "std")]
impl<const BITS: usize, const LIMBS: usize> From<Uint<BITS, LIMBS>> for f32 {
    #[inline]
    fn from(value: Uint<BITS, LIMBS>) -> Self {
        Self::from(&value)
    }
}

#[cfg(feature = "std")]
impl<const BITS: usize, const LIMBS: usize> From<&Uint<BITS, LIMBS>> for f32 {
    /// Approximate single precision float.
    ///
    /// Returns `f32::INFINITY` if the value is too large to represent.
    #[inline]
    #[allow(clippy::cast_precision_loss)] // Documented
    fn from(value: &Uint<BITS, LIMBS>) -> Self {
        let (bits, exponent) = value.most_significant_bits();
        (bits as Self) * (exponent as Self).exp2()
    }
}

#[cfg(feature = "std")]
impl<const BITS: usize, const LIMBS: usize> From<Uint<BITS, LIMBS>> for f64 {
    #[inline]
    fn from(value: Uint<BITS, LIMBS>) -> Self {
        Self::from(&value)
    }
}

#[cfg(feature = "std")]
impl<const BITS: usize, const LIMBS: usize> From<&Uint<BITS, LIMBS>> for f64 {
    /// Approximate double precision float.
    ///
    /// Returns `f64::INFINITY` if the value is too large to represent.
    #[inline]
    #[allow(clippy::cast_precision_loss)] // Documented
    fn from(value: &Uint<BITS, LIMBS>) -> Self {
        Self::from_bits(value.to_f64_bits())
    }
}

impl<const BITS: usize, const LIMBS: usize> Uint<BITS, LIMBS> {
    // Returns the IEEE-754 binary64 bit pattern (u64) for this unsigned big int.
    pub fn to_f64_bits(self) -> u64 {
        // Special case zero.
        if self.is_zero() {
            return 0;
        }

        // Normalize: move the leading 1 into the top bit position of the fixed-width integer.
        let n = self.leading_zeros() as usize; // 0 <= n < BITS since value != 0
        let y = self << n;

        // Exponent field with the "minus one so mantissa can overflow into it" trick:
        // e = (bias + (bitlen-1)) - 1 = (1023 + (BITS-1-n)) - 1 = (1021 + BITS) - n
        let mut e = (1021u64 + BITS as u64) - n as u64;

        // If the exponent already exceeds the representable range, saturate to +inf.
        // (This cannot happen for u32/u64/u128, but can for larger BITS.)
        if e >= 0x7FF {
            return 0x7FF0_0000_0000_0000;
        }

        // Extract 53 significant bits (including the hidden bit) into `a`.
        // After this, `a` is a 53-bit value in a u64, "bit 53 still intact".
        let a: u64 = if BITS >= 53 {
            // Bring the top 53 bits down to the bottom.
            let shifted = y >> (BITS - 53);
            shifted.limbs[0]
        } else {
            // Fit the entire value (<= 53 bits) and shift it up so its MSB sits at bit 52.
            // Since y fits in BITS bits, its low 64 limb contains the entire value.
            let lo = y.limbs[0];
            lo << (53 - BITS)
        };

        // Build `b` (64-bit) that carries guard/sticky info for branchless rounding:
        // - b >> 63 = guard bit (the bit right below the 53 kept bits)
        // - b > (1<<63) when sticky bits exist (any dropped bits below guard are 1),
        //   so ties vs. "round up" are distinguished by b values.
        let b: u64 = if BITS > 53 {
            let r = BITS - 53; // number of dropped (insignificant) bits

            // tail = the dropped bits (lowest r bits of y)
            let one = Uint::<BITS, LIMBS>::ONE;
            let tail_mask = (one << r) - one;
            let tail = y & tail_mask;

            // guard = bit r-1 (top of the dropped region)
            let guard: u64 = if r > 0 {
                ((tail >> (r - 1)).limbs[0] & 1) as u64
            } else {
                0
            };

            // sticky = any 1s below the guard bit?
            let sticky: bool = if r > 1 {
                let low_mask = (one << (r - 1)) - one;
                !(tail & low_mask).is_zero()
            } else {
                false
            };

            (guard << 63) | (sticky as u64)
        } else {
            0
        };

        // Tie-to-even, branchless:
        // Add one when we need to round up; break ties to even.
        let m = a + ((b - ((b >> 63) & !a)) >> 63);

        // Combine. Use '+' (not '|') so an overflowing mantissa carry increments the exponent.
        ((e << 52) + m)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{const_for, nlimbs};

    #[test]
    fn test_u64() {
        assert_eq!(Uint::<0, 0>::try_from(0_u64), Ok(Uint::ZERO));
        assert_eq!(
            Uint::<0, 0>::try_from(1_u64),
            Err(ToUintError::ValueTooLarge(0, Uint::ZERO))
        );
        const_for!(BITS in NON_ZERO {
            const LIMBS: usize = nlimbs(BITS);
            assert_eq!(Uint::<BITS, LIMBS>::try_from(0_u64), Ok(Uint::ZERO));
            assert_eq!(Uint::<BITS, LIMBS>::try_from(1_u64).unwrap().as_limbs()[0], 1);
        });
    }

    #[test]
    fn test_u64_max() {
        assert_eq!(
            Uint::<64, 1>::try_from(u64::MAX),
            Ok(Uint::from_limbs([u64::MAX]))
        );
        assert_eq!(
            Uint::<64, 1>::try_from(u64::MAX as u128),
            Ok(Uint::from_limbs([u64::MAX]))
        );
        assert_eq!(
            Uint::<64, 1>::try_from(u64::MAX as u128 + 1),
            Err(ToUintError::ValueTooLarge(64, Uint::ZERO))
        );

        assert_eq!(
            Uint::<128, 2>::try_from(u64::MAX),
            Ok(Uint::from_limbs([u64::MAX, 0]))
        );
        assert_eq!(
            Uint::<128, 2>::try_from(u64::MAX as u128),
            Ok(Uint::from_limbs([u64::MAX, 0]))
        );
        assert_eq!(
            Uint::<128, 2>::try_from(u64::MAX as u128 + 1),
            Ok(Uint::from_limbs([0, 1]))
        );
    }

    #[test]
    fn test_u65() {
        let x = uint!(18446744073711518810_U65);
        assert_eq!(x.bit_len(), 65);
        assert_eq!(
            u64::try_from(x),
            Err(FromUintError::Overflow(65, 1967194, u64::MAX))
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_f64() {
        assert_eq!(Uint::<0, 0>::try_from(0.0_f64), Ok(Uint::ZERO));
        const_for!(BITS in NON_ZERO {
            const LIMBS: usize = nlimbs(BITS);
            assert_eq!(Uint::<BITS, LIMBS>::try_from(0.0_f64), Ok(Uint::ZERO));
            assert_eq!(Uint::<BITS, LIMBS>::try_from(1.0_f64).unwrap().as_limbs()[0], 1);
        });
        assert_eq!(
            Uint::<7, 1>::try_from(123.499_f64),
            Ok(Uint::from_limbs([123]))
        );
        assert_eq!(
            Uint::<7, 1>::try_from(123.500_f64),
            Ok(Uint::from_limbs([124]))
        );
    }
}
