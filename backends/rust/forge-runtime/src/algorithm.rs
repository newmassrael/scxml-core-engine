// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The integer arithmetic contract's runtime half — SCE_FORGE.md §3.4.1.
//!
//! An algorithm that declares `<sce:return may-fail="true">` returns
//! `Result<T, AlgorithmError>`, and the generator lowers each of its integer
//! `+ - * / %` and unary `-`, and each store of an integer into a narrower
//! integer type, to one of the helpers here, followed by `?`. A
//! helper computes the operation at the declared width or reports why it
//! has no value there: an overflow (a signed `MIN / -1` or `MIN % -1`
//! included, which is one), or a division by zero, which is refused before
//! it is attempted.
//!
//! `no_std` and allocation-free, like the rest of the crate's default
//! surface: an algorithm is a pure function an ECU may call.

use sce_portable_bytes::CapacityExceeded;

/// Why a `may-fail` algorithm has no value to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgorithmError {
    /// An integer result outside its declared width.
    Overflow,
    /// An integer `/` or `%` by zero.
    DivideByZero,
    /// A buffer append past the buffer's declared capacity.
    CapacityExceeded,
}

impl AlgorithmError {
    /// The failure's name in the contract — the spelling every backend's
    /// failure shares, and what a conformance case's `"fails"` names.
    pub fn contract_name(self) -> &'static str {
        match self {
            AlgorithmError::Overflow => "overflow",
            AlgorithmError::DivideByZero => "divide-by-zero",
            AlgorithmError::CapacityExceeded => "capacity-exceeded",
        }
    }
}

/// A buffer append inside a `may-fail` algorithm threads the same `?` as
/// its arithmetic.
impl From<CapacityExceeded> for AlgorithmError {
    fn from(_: CapacityExceeded) -> Self {
        AlgorithmError::CapacityExceeded
    }
}

/// The integer widths an algorithm declares, with the checked operations the
/// helpers need. Implemented for exactly the SCE integer types.
pub trait CheckedInt: Copy {
    fn checked_add_(self, rhs: Self) -> Option<Self>;
    fn checked_sub_(self, rhs: Self) -> Option<Self>;
    fn checked_mul_(self, rhs: Self) -> Option<Self>;
    fn checked_div_(self, rhs: Self) -> Option<Self>;
    fn checked_rem_(self, rhs: Self) -> Option<Self>;
    fn checked_neg_(self) -> Option<Self>;
    fn is_zero(self) -> bool;
}

macro_rules! checked_int {
    ($($t:ty),*) => {$(
        impl CheckedInt for $t {
            fn checked_add_(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
            fn checked_sub_(self, rhs: Self) -> Option<Self> { self.checked_sub(rhs) }
            fn checked_mul_(self, rhs: Self) -> Option<Self> { self.checked_mul(rhs) }
            fn checked_div_(self, rhs: Self) -> Option<Self> { self.checked_div(rhs) }
            fn checked_rem_(self, rhs: Self) -> Option<Self> { self.checked_rem(rhs) }
            fn checked_neg_(self) -> Option<Self> { self.checked_neg() }
            fn is_zero(self) -> bool { self == 0 }
        }
    )*};
}

checked_int!(u8, u16, u32, u64, i8, i16, i32, i64);

pub fn add<T: CheckedInt>(a: T, b: T) -> Result<T, AlgorithmError> {
    a.checked_add_(b).ok_or(AlgorithmError::Overflow)
}

pub fn sub<T: CheckedInt>(a: T, b: T) -> Result<T, AlgorithmError> {
    a.checked_sub_(b).ok_or(AlgorithmError::Overflow)
}

pub fn mul<T: CheckedInt>(a: T, b: T) -> Result<T, AlgorithmError> {
    a.checked_mul_(b).ok_or(AlgorithmError::Overflow)
}

/// Truncated toward zero, as every backend divides (SCE_FORGE.md §3.4.1).
pub fn div<T: CheckedInt>(a: T, b: T) -> Result<T, AlgorithmError> {
    if b.is_zero() {
        return Err(AlgorithmError::DivideByZero);
    }
    a.checked_div_(b).ok_or(AlgorithmError::Overflow)
}

/// The sign of the dividend, as every backend's remainder has it.
pub fn rem<T: CheckedInt>(a: T, b: T) -> Result<T, AlgorithmError> {
    if b.is_zero() {
        return Err(AlgorithmError::DivideByZero);
    }
    a.checked_rem_(b).ok_or(AlgorithmError::Overflow)
}

pub fn neg<T: CheckedInt>(a: T) -> Result<T, AlgorithmError> {
    a.checked_neg_().ok_or(AlgorithmError::Overflow)
}

/// A value of type `S` stored where a `T` is declared: the same value, or an
/// overflow when `T` cannot hold it — never a wrapped one.
pub fn narrow<T: CheckedInt + TryFrom<S>, S: CheckedInt>(v: S) -> Result<T, AlgorithmError> {
    T::try_from(v).map_err(|_| AlgorithmError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_failure_is_named_and_every_value_passes() {
        assert_eq!(add::<u32>(u32::MAX - 1, 1), Ok(u32::MAX));
        assert_eq!(add::<u32>(u32::MAX, 1), Err(AlgorithmError::Overflow));
        assert_eq!(sub::<u8>(0, 1), Err(AlgorithmError::Overflow));
        assert_eq!(mul::<i16>(256, 128), Err(AlgorithmError::Overflow));
        assert_eq!(div::<i32>(-7, 2), Ok(-3));
        assert_eq!(rem::<i32>(-7, 3), Ok(-1));
        assert_eq!(div::<u8>(1, 0), Err(AlgorithmError::DivideByZero));
        assert_eq!(rem::<i64>(1, 0), Err(AlgorithmError::DivideByZero));
        assert_eq!(div::<i8>(i8::MIN, -1), Err(AlgorithmError::Overflow));
        assert_eq!(rem::<i8>(i8::MIN, -1), Err(AlgorithmError::Overflow));
        assert_eq!(neg::<i32>(i32::MIN), Err(AlgorithmError::Overflow));
        assert_eq!(neg::<i32>(5), Ok(-5));
        assert_eq!(narrow::<i32, i64>(i64::from(i32::MAX)), Ok(i32::MAX));
        assert_eq!(
            narrow::<i32, i64>(i64::from(i32::MAX) + 1),
            Err(AlgorithmError::Overflow)
        );
        assert_eq!(narrow::<u8, i64>(-1), Err(AlgorithmError::Overflow));
        assert_eq!(narrow::<i8, u64>(u64::MAX), Err(AlgorithmError::Overflow));
        assert_eq!(narrow::<u64, i8>(-1), Err(AlgorithmError::Overflow));
        assert_eq!(narrow::<i64, u8>(200), Ok(200));
    }

    #[test]
    fn a_capacity_failure_threads_the_same_question_mark() {
        fn append() -> Result<(), AlgorithmError> {
            Err(CapacityExceeded)?
        }
        assert_eq!(append(), Err(AlgorithmError::CapacityExceeded));
    }
}
