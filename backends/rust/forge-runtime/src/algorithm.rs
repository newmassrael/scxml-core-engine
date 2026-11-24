// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! it is attempted. A shift by a count it cannot prove at least 0 is lowered
//! to `shl`, `shr` or `ushr`, which fail `out-of-range` for a count below 0,
//! where a shift has no exact result, and answer the exact result reduced to
//! the width for every other count, one at or past the width included.
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
    /// A `<sce:require>` precondition that does not hold: an input outside
    /// the algorithm's domain.
    Precondition,
    /// An index below 0 or not below its collection's length.
    OutOfRange,
}

impl AlgorithmError {
    /// The failure's name in the contract — the spelling every backend's
    /// failure shares, and what a conformance case's `"fails"` names.
    pub fn contract_name(self) -> &'static str {
        match self {
            AlgorithmError::Overflow => "overflow",
            AlgorithmError::DivideByZero => "divide-by-zero",
            AlgorithmError::CapacityExceeded => "capacity-exceeded",
            AlgorithmError::Precondition => "precondition",
            AlgorithmError::OutOfRange => "out-of-range",
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

/// The integer widths a shift is made at (SCE_FORGE.md §3.4.1, "The width of
/// a shift"), with the total shifts the generator prints: each is the exact
/// result reduced to the width for every count at least 0, so a count at or
/// past the width leaves 0, or the sign for an arithmetic `>>` of a negative
/// value. Rust's own `<<` and `>>` panic there in a build that checks
/// overflow, and take the count modulo the width in one that does not.
/// Implemented for exactly the SCE integer types.
pub trait ShiftInt: Copy {
    /// `self * 2^n` reduced to the width.
    fn shl_total(self, n: u32) -> Self;
    /// The floor of `self / 2^n`.
    fn shr_total(self, n: u32) -> Self;
    /// The floor of `u / 2^n`, `u` the width's unsigned bits of `self`,
    /// read back at its type.
    fn ushr_total(self, n: u32) -> Self;
}

macro_rules! shift_int {
    ($($t:ty => $u:ty, $last:expr);*) => {$(
        impl ShiftInt for $t {
            fn shl_total(self, n: u32) -> Self { self.checked_shl(n).unwrap_or(0) }
            fn shr_total(self, n: u32) -> Self { self.wrapping_shr(n.min($last)) }
            fn ushr_total(self, n: u32) -> Self {
                (self as $u).checked_shr(n).unwrap_or(0) as $t
            }
        }
    )*};
}

// An unsigned `>>` past the width is 0, which `wrapping_shr` by the last bit
// is not, so each unsigned type's arithmetic and logical shifts are one.
macro_rules! shift_uint {
    ($($t:ty),*) => {$(
        impl ShiftInt for $t {
            fn shl_total(self, n: u32) -> Self { self.checked_shl(n).unwrap_or(0) }
            fn shr_total(self, n: u32) -> Self { self.checked_shr(n).unwrap_or(0) }
            fn ushr_total(self, n: u32) -> Self { self.shr_total(n) }
        }
    )*};
}

shift_int!(i8 => u8, 7; i16 => u16, 15; i32 => u32, 31; i64 => u64, 63);
shift_uint!(u8, u16, u32, u64);

/// A shift's count `n` as the number of places it shifts, or an out-of-range
/// failure when it is below 0, where a shift has no exact result. A count past
/// `u32` is past every width, and shifts as `u32::MAX` does.
fn places<C: TryInto<i128>>(n: C) -> Result<u32, AlgorithmError> {
    let n: i128 = n.try_into().map_err(|_| AlgorithmError::OutOfRange)?;
    if n < 0 {
        return Err(AlgorithmError::OutOfRange);
    }
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// `a << n` at the width of `T`, the count at its own type: 0 for a count at
/// or past the width, and an out-of-range failure for one below 0.
pub fn shl<T: ShiftInt, C: TryInto<i128>>(a: T, n: C) -> Result<T, AlgorithmError> {
    places(n).map(|n| a.shl_total(n))
}

/// `a >> n`, the floor of `a / 2^n`: 0 or -1 for a count at or past the width,
/// and an out-of-range failure for one below 0.
pub fn shr<T: ShiftInt, C: TryInto<i128>>(a: T, n: C) -> Result<T, AlgorithmError> {
    places(n).map(|n| a.shr_total(n))
}

/// `a >>> n`, the logical shift of the width's bits of `a`: 0 for a count at
/// or past the width, and an out-of-range failure for one below 0.
pub fn ushr<T: ShiftInt, C: TryInto<i128>>(a: T, n: C) -> Result<T, AlgorithmError> {
    places(n).map(|n| a.ushr_total(n))
}

/// The element of `xs` at `i`, or an out-of-range failure when `i` is below
/// 0 or not below `xs`'s length — never a read past the end, and never a
/// panic.
pub fn at<T: Copy, I: TryInto<usize>>(xs: &[T], i: I) -> Result<T, AlgorithmError> {
    i.try_into()
        .ok()
        .and_then(|i| xs.get(i).copied())
        .ok_or(AlgorithmError::OutOfRange)
}

/// [`at`] for an element that is not `Copy`, handed back by reference: a
/// record that owns text, a field of which an expression reads through the
/// reference and never moves out of the list.
pub fn at_ref<T, I: TryInto<usize>>(xs: &[T], i: I) -> Result<&T, AlgorithmError> {
    i.try_into()
        .ok()
        .and_then(|i| xs.get(i))
        .ok_or(AlgorithmError::OutOfRange)
}

/// The operations of an algorithm that does not declare `may-fail`, whose
/// interval analysis proved, while the document was built, that none of them
/// reaches a failure of the integer arithmetic contract (SCE_FORGE.md §3.4.1):
/// no integer `+ - * / %` or unary `-` leaves its width or divides by zero,
/// and no index is below 0 or not below its collection's length.
///
/// The generator lowers each such operation of a Rust body to a function here
/// rather than to the operator or the index it stands for, so that the crate
/// a generated file is compiled in holds no arithmetic operator and no index
/// of its own: a crate that denies `clippy::arithmetic_side_effects` and
/// `clippy::indexing_slicing` compiles the file as it is, with no `allow`.
/// What is left of each operation's failure is the proof's, and it is here,
/// once, in the runtime that owns the proof.
///
/// Each function computes what the operator computes. An arithmetic operation
/// is the operator itself, so a build that checks overflow
/// (`overflow-checks`) asserts the proof as it asserted it at the operator,
/// and one that does not computes what it computed there; nothing changes
/// meaning. An index read takes the element where it is, and panics past the
/// end as the index panics, from one cold site for every read of every
/// algorithm: a broken proof stops the program, as it did, and never reads
/// past the end.
pub mod proven {
    use super::CheckedInt;

    /// `a + b`, proven in range.
    #[inline(always)]
    pub fn add<T: CheckedInt + core::ops::Add<Output = T>>(a: T, b: T) -> T {
        a + b
    }

    /// `a - b`, proven in range.
    #[inline(always)]
    pub fn sub<T: CheckedInt + core::ops::Sub<Output = T>>(a: T, b: T) -> T {
        a - b
    }

    /// `a * b`, proven in range.
    #[inline(always)]
    pub fn mul<T: CheckedInt + core::ops::Mul<Output = T>>(a: T, b: T) -> T {
        a * b
    }

    /// `a / b`, truncated toward zero, `b` proven not zero and the quotient
    /// proven in range.
    #[inline(always)]
    pub fn div<T: CheckedInt + core::ops::Div<Output = T>>(a: T, b: T) -> T {
        a / b
    }

    /// `a % b`, of the dividend's sign, `b` proven not zero.
    #[inline(always)]
    pub fn rem<T: CheckedInt + core::ops::Rem<Output = T>>(a: T, b: T) -> T {
        a % b
    }

    /// `-a`, proven in range.
    #[inline(always)]
    pub fn neg<T: CheckedInt + core::ops::Neg<Output = T>>(a: T) -> T {
        -a
    }

    /// `a << n`, the count `n` at its own type, proven at least 0 and below
    /// the width of `T` (SCE_FORGE.md §3.4.1, "The width of a shift").
    #[inline(always)]
    pub fn shl<T: CheckedInt + core::ops::Shl<C, Output = T>, C>(a: T, n: C) -> T {
        a << n
    }

    /// `a >> n`, the count `n` at its own type, proven at least 0 and below
    /// the width of `T`: arithmetic for a signed `T`, and the logical shift
    /// `>>>` is for an unsigned one.
    #[inline(always)]
    pub fn shr<T: CheckedInt + core::ops::Shr<C, Output = T>, C>(a: T, n: C) -> T {
        a >> n
    }

    /// The element of `xs` at `i`, `i` proven at least 0 and below the
    /// length. The index is taken at its own type, as [`super::at`] takes it.
    #[inline(always)]
    pub fn at<T: Copy, I: TryInto<usize>>(xs: &[T], i: I) -> T {
        *at_ref(xs, i)
    }

    /// [`at`] for an element that is not `Copy`, handed back by reference.
    #[inline(always)]
    pub fn at_ref<T, I: TryInto<usize>>(xs: &[T], i: I) -> &T {
        match i.try_into().ok().and_then(|i| xs.get(i)) {
            Some(element) => element,
            None => index_proven_in_range_was_not(),
        }
    }

    /// The panic of [`at_ref`], out of line and cold, so that each read
    /// inlines a test and a call that takes no argument, and a build holds one
    /// panic site and one file name for every proven read of every algorithm.
    #[cold]
    #[inline(never)]
    fn index_proven_in_range_was_not() -> ! {
        panic!("an index SCE's interval analysis proved in range was not (SCE_FORGE.md §3.4.1)")
    }
}

/// A value of type `S` stored where a `T` is declared: the same value, or an
/// overflow when `T` cannot hold it — never a wrapped one.
pub fn narrow<T: CheckedInt + TryFrom<S>, S: CheckedInt>(v: S) -> Result<T, AlgorithmError> {
    T::try_from(v).map_err(|_| AlgorithmError::Overflow)
}

/// The value a reading found, or an out-of-range failure when it found none:
/// the variant an enum's `from_underlying` found for a number, which is what
/// `<alias>.fromNumber(n)` answers (SCE_FORGE.md §4.12, "An enum value in a
/// body"), and the value a lookup whose miss is an error found for a key. A
/// number outside the enum and a key outside the table fail as an index
/// outside a collection does.
pub fn found<T>(value: Option<T>) -> Result<T, AlgorithmError> {
    value.ok_or(AlgorithmError::OutOfRange)
}

/// Whether a list a statechart filled by appends holds the lower count of the
/// field a `<send>` writes it into (docs/adr/0086, section 15.6): `held` its
/// elements, `least` the field's count. A variable states no lower count, so
/// the send holds it here, before it writes the list, and a list short of it
/// fails the param with `precondition`, the event refused whole.
pub fn holds_at_least(held: usize, least: u32) -> Result<(), AlgorithmError> {
    if (held as u64) < u64::from(least) {
        Err(AlgorithmError::Precondition)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proven_operation_is_the_operation_it_stands_for() {
        assert_eq!(proven::add(u32::MAX - 1, 1), u32::MAX);
        assert_eq!(proven::sub(0i8, i8::MAX), -i8::MAX);
        assert_eq!(proven::mul(16u8, 15), 240);
        assert_eq!(proven::div(-7i32, 2), -3);
        assert_eq!(proven::rem(-7i32, 3), -1);
        assert_eq!(proven::neg(5i64), -5);
        let bytes = [3u8, 5, 7];
        for (i, b) in bytes.iter().enumerate() {
            assert_eq!(proven::at(&bytes, i), *b);
            assert_eq!(proven::at(&bytes, i as u32), *b);
            assert_eq!(proven::at_ref(&bytes, i as u8), b);
        }
    }

    #[test]
    fn a_proven_shift_is_the_operator() {
        assert_eq!(proven::shl(1u32, 31u8), 1u32 << 31);
        assert_eq!(proven::shl(-1i16, 15i32), i16::MIN);
        assert_eq!(proven::shr(-128i8, 7u64), -1);
        assert_eq!(proven::shr(0x80u8, 7u8), 1);
    }

    /// SCE_FORGE.md §3.4.1, "The width of a shift": the exact result reduced
    /// to the width for every count at least 0, and no value below 0.
    #[test]
    fn a_shift_past_its_width_is_its_reduced_exact_result() {
        assert_eq!(shl(0xFFu8, 7u8), Ok(0x80));
        assert_eq!(shl(0xFFu8, 8u8), Ok(0));
        assert_eq!(shl(1i64, 200u8), Ok(0));
        assert_eq!(shl(1u32, u64::MAX), Ok(0));
        assert_eq!(shr(-5i32, 1u8), Ok(-3));
        assert_eq!(shr(i64::MIN, 64u8), Ok(-1));
        assert_eq!(shr(i16::MAX, 300i32), Ok(0));
        assert_eq!(shr(u64::MAX, 64u8), Ok(0));
        assert_eq!(ushr(-1i32, 28u8), Ok(0xF));
        assert_eq!(ushr(-1i8, 8u8), Ok(0));
        assert_eq!(ushr(0x8000u16, 15u8), Ok(1));
        assert_eq!(shl(1u8, -1i32), Err(AlgorithmError::OutOfRange));
        assert_eq!(shr(-1i64, i64::MIN), Err(AlgorithmError::OutOfRange));
        assert_eq!(ushr(1u16, -8i8), Err(AlgorithmError::OutOfRange));
    }

    #[test]
    #[should_panic(expected = "proved in range was not")]
    fn a_proven_index_at_the_length_panics() {
        let _ = proven::at(&[3u8, 5, 7], 3u32);
    }

    #[test]
    #[should_panic(expected = "proved in range was not")]
    fn a_proven_index_below_zero_panics_and_reads_nothing_from_the_end() {
        let _ = proven::at_ref(&[3u8, 5, 7], -1i32);
    }

    #[test]
    fn a_list_sent_short_of_its_fields_lower_count_fails_as_a_precondition() {
        assert_eq!(holds_at_least(0, 0), Ok(()));
        assert_eq!(holds_at_least(1, 1), Ok(()));
        assert_eq!(holds_at_least(5, 2), Ok(()));
        assert_eq!(holds_at_least(0, 1), Err(AlgorithmError::Precondition));
        assert_eq!(holds_at_least(2, 3), Err(AlgorithmError::Precondition));
    }

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
        assert_eq!(found(Some(7u8)), Ok(7u8));
        assert_eq!(found::<u8>(None), Err(AlgorithmError::OutOfRange));
        assert_eq!(narrow::<i32, i64>(i64::from(i32::MAX)), Ok(i32::MAX));
        assert_eq!(
            narrow::<i32, i64>(i64::from(i32::MAX) + 1),
            Err(AlgorithmError::Overflow)
        );
        assert_eq!(narrow::<u8, i64>(-1), Err(AlgorithmError::Overflow));
        assert_eq!(narrow::<i8, u64>(u64::MAX), Err(AlgorithmError::Overflow));
        assert_eq!(narrow::<u64, i8>(-1), Err(AlgorithmError::Overflow));
        assert_eq!(narrow::<i64, u8>(200), Ok(200));
        assert_eq!(at(&[1u8, 2, 3], 2u32), Ok(3));
        assert_eq!(at(&[1u8, 2, 3], 3u32), Err(AlgorithmError::OutOfRange));
        assert_eq!(at(&[1u8, 2, 3], -1i32), Err(AlgorithmError::OutOfRange));
        assert_eq!(at::<u8, u8>(&[], 0), Err(AlgorithmError::OutOfRange));
        assert_eq!(at(&[7u16; 256], 255u16), Ok(7));
    }

    // A machine's text is held to its bytes, not its characters, by the type it
    // is copied into, and the refusal threads the same `?` as an overflow.
    #[test]
    fn a_text_past_its_bound_in_bytes_fails_as_capacity() {
        fn held<const N: usize>(text: &str) -> Result<(), AlgorithmError> {
            sce_portable_bytes::SceString::<N>::from_text(text)?;
            Ok(())
        }
        assert_eq!(held::<4>("\u{e9}\u{e9}"), Ok(()));
        // Two characters, five bytes: a count of characters would let it pass.
        assert_eq!(
            held::<4>("\u{e9}\u{20ac}"),
            Err(AlgorithmError::CapacityExceeded)
        );
    }

    #[test]
    fn a_capacity_failure_threads_the_same_question_mark() {
        fn append() -> Result<(), AlgorithmError> {
            Err(CapacityExceeded)?
        }
        assert_eq!(append(), Err(AlgorithmError::CapacityExceeded));
    }
}
