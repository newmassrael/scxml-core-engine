// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Storage policy for the owned mirror of a codec (`{Codec}Owned`).
//!
//! A decoded wire value has one schema but more than one legitimate resting
//! place. An application-processor path wants growable containers and no
//! ceiling; a real-time or MCU path wants every byte inline with a hard bound
//! and no allocator. The generated owned type is therefore parameterised by a
//! [`CodecStorage`] policy — `{Codec}Owned<Heap>` and `{Codec}Owned<Inline>`
//! are the same generated type at two storage profiles, and both exist in the
//! same binary.
//!
//! # Why a policy parameter rather than a second generated type
//!
//! The alternative — emitting a separate `{Codec}Inline` beside
//! `{Codec}Owned` — doubles the generated surface, doubles the conversion
//! matrix, and lets the two mirrors of one wire type drift apart. The policy
//! keeps one type, one `try_into_owned`, one `as_borrowed`, one test vector
//! set, and makes the storage choice a caller's decision at the use site
//! rather than the code generator's decision at build-configuration time.
//!
//! The profile is a *type*, so:
//!
//! - a heap-capable program can still pin a value to non-allocating storage
//!   (`{Codec}Owned<Inline>`), which a `cfg`-switched container cannot express
//!   — under `alloc` the switched form is growable for everyone;
//! - the owned mirror is no longer gated behind `#[cfg(feature = "alloc")]`,
//!   so an MCU build gets owned values for list- and embed-bearing codecs
//!   instead of only for flat scalar ones;
//! - moving a value between profiles is a checked projection
//!   (`{Codec}Owned::transcode_in`), not a re-decode.
//!
//! Naming follows the standard library's allocator parameterisation
//! (`Box::new_in`, `Vec::with_capacity_in`): the plain method uses
//! [`DefaultStorage`], the `_in` suffixed method takes the profile explicitly.
//!
//! # Container contract
//!
//! [`SceList`], [`SceStr`] and [`SceByteBuf`] are the three container shapes a
//! codec field can need. They are deliberately minimal — construct, append or
//! copy-in, and read back as a borrowed view — because that is the whole of
//! what the generated owned↔borrowed projections do. Every method is fallible
//! so one call shape serves both profiles; on the growable profile the error
//! is unreachable rather than absent, which keeps the generated code free of
//! per-profile branches.

use crate::codec::CodecError;

#[cfg(feature = "alloc")]
pub use sce_portable_bytes::HeapBytes;
pub use sce_portable_bytes::{InlineBytes, SceBytes};

/// A bounded or growable sequence of `T`, as a codec's owned list field.
///
/// `N` lives on the *type* (`heapless::Vec<T, N>`) rather than on the trait,
/// so the growable implementation is free of a capacity it does not enforce.
pub trait SceList<T>: Sized {
    /// An empty list. The fixed-capacity implementation reserves `N` inline.
    fn empty() -> Self;

    /// Append one element, raising [`CodecError::TooManyElements`] when a
    /// fixed-capacity list is full. The growable implementation never fails.
    fn try_push(&mut self, value: T) -> Result<(), CodecError>;

    /// Read the elements back as a slice — the input to the owned→borrowed
    /// projection, which re-bounds them into the borrowed view's inline list.
    fn as_slice(&self) -> &[T];
}

/// Owned text storage for a codec's `string` field.
pub trait SceStr: Sized {
    /// Copy a borrowed decode view into owned storage, raising
    /// [`CodecError::TooManyElements`] past a fixed capacity.
    fn from_view(s: &str) -> Result<Self, CodecError>;

    /// Borrow the text back — the owned→borrowed projection's read side.
    fn as_str(&self) -> &str;
}

/// Owned byte storage for a codec's `bytes` field.
pub trait SceByteBuf: Sized {
    /// Copy a borrowed decode view into owned storage, raising
    /// [`CodecError::TooManyElements`] past a fixed capacity.
    fn from_slice(b: &[u8]) -> Result<Self, CodecError>;

    /// Borrow the bytes back — the owned→borrowed projection's read side.
    fn as_slice(&self) -> &[u8];
}

/// A storage profile: the containers a `{Codec}Owned` uses for its list,
/// text and byte fields.
///
/// The supertraits are what the generated `#[derive(Debug, Clone, PartialEq)]`
/// on a storage-generic owned struct needs in order to hold for every profile;
/// the profiles themselves are zero-sized markers, so carrying them costs
/// nothing. The same three bounds repeat on each associated type because a
/// derived impl reasons about the *field* types, not about `Self`.
pub trait CodecStorage: core::fmt::Debug + Clone + PartialEq {
    /// Owned list storage for a `<sce:repeat>` / `<sce:tlv-chain>` field whose
    /// declared `sce:max-count` is `N`.
    type List<T, const N: usize>: SceList<T> + core::fmt::Debug + Clone + PartialEq
    where
        T: core::fmt::Debug + Clone + PartialEq;

    /// Owned text storage for a `string` field whose `sce:max-size` is `N`.
    type Str<const N: usize>: SceStr + core::fmt::Debug + Clone + PartialEq;

    /// Owned byte storage for a `bytes` field whose `sce:max-size` is `N`.
    type Bytes<const N: usize>: SceByteBuf + core::fmt::Debug + Clone + PartialEq;
}

/// Growable storage: heap containers, declared capacities advisory.
///
/// The profile for hosts with an allocator, where the on-wire protocol places
/// no ceiling on a payload and the application must not invent one.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heap;

/// Non-allocating storage: every container fixed at its declared capacity.
///
/// The profile for MCU tiers and for real-time paths inside otherwise
/// heap-capable programs. Declared capacities are hard bounds, and exceeding
/// one is [`CodecError::TooManyElements`] rather than an allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inline;

/// The storage profile a build treats as the default: [`Heap`] where an
/// allocator is available, [`Inline`] on the heap-free tier.
///
/// `{Codec}Owned` defaults to it, so code with no reason to pin a profile
/// keeps spelling the bare type and follows its target.
#[cfg(feature = "alloc")]
pub type DefaultStorage = Heap;
/// no-alloc build: the default profile is the non-allocating one.
#[cfg(not(feature = "alloc"))]
pub type DefaultStorage = Inline;

#[cfg(feature = "alloc")]
impl CodecStorage for Heap {
    // The declared capacity is deliberately unused here: on this profile it is
    // advisory, and dropping it from the container type is what makes the
    // growable form growable.
    type List<T, const N: usize>
        = alloc::vec::Vec<T>
    where
        T: core::fmt::Debug + Clone + PartialEq;
    type Str<const N: usize> = HeapStr<N>;
    type Bytes<const N: usize> = HeapBytes<N>;
}

impl CodecStorage for Inline {
    type List<T, const N: usize>
        = heapless::Vec<T, N>
    where
        T: core::fmt::Debug + Clone + PartialEq;
    type Str<const N: usize> = InlineStr<N>;
    type Bytes<const N: usize> = InlineBytes<N>;
}

// ── list containers ──────────────────────────────────────────────

#[cfg(feature = "alloc")]
impl<T> SceList<T> for alloc::vec::Vec<T> {
    fn empty() -> Self {
        alloc::vec::Vec::new()
    }

    fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        self.push(value);
        Ok(())
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> SceList<T> for heapless::Vec<T, N> {
    fn empty() -> Self {
        heapless::Vec::new()
    }

    fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        self.push(value).map_err(|_| CodecError::TooManyElements)
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }
}

// ── text containers ──────────────────────────────────────────────

/// Growable owned text storage: wraps `String`, so `N` is advisory. `N` still
/// rides on the type so a hand-assembled owned value infers the cap from the
/// field it is being placed in, and so the value can be transcoded into
/// [`InlineStr`] of the matching capacity.
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq)]
pub struct HeapStr<const N: usize>(alloc::string::String, core::marker::PhantomData<[u8; N]>);

/// Fixed-capacity owned text storage: wraps `heapless::String<N>`, so the
/// value never allocates and `N` is a hard bound. Compiled on every profile,
/// including alongside [`HeapStr`].
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq)]
pub struct InlineStr<const N: usize>(heapless::String<N>);

/// The text storage profile this build treats as the default — the sibling of
/// [`SceBytes`] for `string` fields.
#[cfg(feature = "alloc")]
pub type SceString<const N: usize> = HeapStr<N>;
/// no-alloc build: the default text profile is the fixed-capacity form.
#[cfg(not(feature = "alloc"))]
pub type SceString<const N: usize> = InlineStr<N>;

#[cfg(feature = "alloc")]
impl<const N: usize> HeapStr<N> {
    /// Copy a borrowed `&str` decode view into owned storage. The heap copy is
    /// unbounded (`N` advisory); the `Result` matches [`InlineStr::from_view`]
    /// so generated code threads one `?` on every profile.
    pub fn from_view(s: &str) -> Result<Self, CodecError> {
        Ok(Self(
            alloc::string::String::from(s),
            core::marker::PhantomData,
        ))
    }

    /// Borrow the owned text back as `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<const N: usize> InlineStr<N> {
    /// Copy a borrowed `&str` decode view into fixed-capacity storage, raising
    /// [`CodecError::TooManyElements`] past `N`.
    pub fn from_view(s: &str) -> Result<Self, CodecError> {
        heapless::String::try_from(s)
            .map(Self)
            .map_err(|_| CodecError::TooManyElements)
    }

    /// Borrow the owned text back as `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Text-view parity surface, written once for both profiles so the growable
/// and fixed forms cannot drift apart in how they are read or compared.
///
/// `Deref<Target = str>` gives the whole `&str` surface; the `PartialEq<&str>`
/// arm lets a consumer write `assert_eq!(owned.locator, "abc")` without an
/// explicit `.as_str()`. Only the `&str` form is implemented — string literals
/// are `&str`, and the unsized right-hand side has no consumer.
macro_rules! str_view_parity {
    ($storage:ident) => {
        impl<const N: usize> SceStr for $storage<N> {
            fn from_view(s: &str) -> Result<Self, CodecError> {
                Self::from_view(s)
            }

            fn as_str(&self) -> &str {
                Self::as_str(self)
            }
        }

        impl<const N: usize> core::ops::Deref for $storage<N> {
            type Target = str;
            fn deref(&self) -> &str {
                self.0.as_str()
            }
        }

        impl<const N: usize> PartialEq<&str> for $storage<N> {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }
    };
}

#[cfg(feature = "alloc")]
str_view_parity!(HeapStr);
str_view_parity!(InlineStr);

// ── byte containers ──────────────────────────────────────────────

/// `SceByteBuf` over the two `sce-portable-bytes` profiles. The construction
/// logic lives in that crate (shared with the statechart-emit runtime); this
/// only re-labels its capacity error as the codec error a generated
/// `try_into_owned` already threads.
macro_rules! byte_buf_impl {
    ($storage:ident) => {
        impl<const N: usize> SceByteBuf for $storage<N> {
            fn from_slice(b: &[u8]) -> Result<Self, CodecError> {
                Self::from_slice(b).map_err(CodecError::from)
            }

            fn as_slice(&self) -> &[u8] {
                Self::as_slice(self)
            }
        }
    };
}

#[cfg(feature = "alloc")]
byte_buf_impl!(HeapBytes);
byte_buf_impl!(InlineBytes);

// ── projections ──────────────────────────────────────────────────

/// Build an owned list by projecting each element of a decoded borrowed list.
///
/// The SSOT for the borrowed→owned list step: every generated
/// `try_into_owned` calls this rather than open-coding the loop, so the
/// capacity semantics live in one place. The destination profile is inferred
/// from the field the result is being placed in, which is what lets one
/// generated body serve every storage profile: on [`Inline`] an over-long
/// source raises [`CodecError::TooManyElements`], on [`Heap`] it cannot fail.
///
/// The inverse direction (owned→borrowed) is
/// [`crate::codec::try_project_bounded`], which always targets the borrowed
/// view's inline list and so is capacity-checked on every profile.
pub fn try_collect_list<L, T, U>(
    src: impl IntoIterator<Item = T>,
    mut f: impl FnMut(T) -> Result<U, CodecError>,
) -> Result<L, CodecError>
where
    L: SceList<U>,
{
    let mut out = L::empty();
    for item in src {
        out.try_push(f(item)?)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_list_is_bounded_at_its_declared_capacity() {
        let full: Result<heapless::Vec<u8, 2>, _> = try_collect_list([1u8, 2], Ok);
        assert!(full.is_ok());
        let over: Result<heapless::Vec<u8, 2>, _> = try_collect_list([1u8, 2, 3], Ok);
        assert_eq!(over, Err(CodecError::TooManyElements));
    }

    #[test]
    fn projection_error_propagates_before_capacity() {
        let out: Result<heapless::Vec<u8, 8>, _> =
            try_collect_list([1u8, 2], |_| Err(CodecError::InvalidUtf8));
        assert_eq!(out, Err(CodecError::InvalidUtf8));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn heap_list_accepts_more_than_the_declared_capacity() {
        // The same call shape on the growable profile: `N` is advisory, so a
        // run longer than any inline bound still collects.
        let out: Result<alloc::vec::Vec<u8>, _> = try_collect_list(0..64u8, Ok);
        assert_eq!(out.unwrap().len(), 64);
    }

    #[test]
    fn inline_str_is_bounded_even_where_heap_exists() {
        assert!(InlineStr::<4>::from_view("abcd").is_ok());
        assert_eq!(
            InlineStr::<4>::from_view("abcde"),
            Err(CodecError::TooManyElements)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn heap_str_is_unbounded() {
        let s = HeapStr::<4>::from_view("far past four").unwrap();
        assert_eq!(s.as_str(), "far past four");
        assert_eq!(s, "far past four");
    }

    // The property the whole policy exists for: both profiles are usable from
    // one build, so a storage choice is made per value rather than per binary.
    #[cfg(feature = "alloc")]
    #[test]
    fn both_profiles_resolve_in_one_binary() {
        fn text<S: CodecStorage>(src: &str) -> Result<S::Str<8>, CodecError> {
            <S::Str<8> as SceStr>::from_view(src)
        }
        assert_eq!(text::<Heap>("ok").unwrap().as_str(), "ok");
        assert_eq!(text::<Inline>("ok").unwrap().as_str(), "ok");
        // ...and only the inline profile enforces the declared capacity.
        assert!(text::<Heap>("nine char").is_ok());
        assert_eq!(
            text::<Inline>("nine char").err(),
            Some(CodecError::TooManyElements)
        );
    }
}
