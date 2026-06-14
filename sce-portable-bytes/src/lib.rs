// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Profile-portable owned `bytes` storage shared by the codec runtime
//! (`sce-forge-runtime` — the `{Codec}Owned` byte fields) and the
//! statechart-emit runtime (`sce-rust-runtime` — typed `_event.data` byte
//! payloads). Both are the same concept: an owned byte sequence with a
//! per-field capacity `N` (the SCXML `sce:max-size`) that must compile on
//! both the std/alloc and the heap-free no_std profiles. This crate holds
//! the single definition; both runtimes `pub use` it, so the construction
//! logic (the fallible no-alloc copy, the advisory-`N` alloc copy) lives in
//! exactly one place and cannot drift between the two consumers.
//!
//! `N` rides on the newtype rather than being erased to a bare `Vec<u8>`
//! alias under `alloc`: a hand-assembled owner (a codec encode-side builder,
//! or a typed-event inject caller) then infers the cap from the destination
//! field's type — `SceBytes::from_slice(&v)?` — instead of hardcoding it at
//! every call site (which would duplicate the `sce:max-size` source of
//! truth into consumer code).

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Returned by [`SceBytes::from_slice`] when a no-alloc copy would exceed the
/// fixed capacity `N`. Under `alloc` the copy is unbounded and never fails,
/// so this is unreachable there. Downstream runtimes map it into their own
/// error enum (e.g. the codec runtime's `CodecError::TooManyElements`) via a
/// `From` impl, so a generated `try_into_owned` keeps threading one `?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityExceeded;

/// Profile-portable owned `bytes` storage. Wraps a growable `Vec<u8>` under
/// `alloc` (`N` advisory — the on-wire protocol caps no payload, so the AP
/// profile must not either) or a fixed `heapless::Vec<u8, N>` otherwise (`N`
/// the hard capacity). `N` rides on the type so a downstream owned-builder
/// infers the cap from the field rather than hardcoding it.
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceBytes<const N: usize>(alloc::vec::Vec<u8>, core::marker::PhantomData<[u8; N]>);
/// no-alloc variant of [`SceBytes`]: fixed-capacity inline storage capped at
/// `N`, heap-free for the MCU tier.
#[cfg(not(feature = "alloc"))]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceBytes<const N: usize>(heapless::Vec<u8, N>);

impl<const N: usize> SceBytes<N> {
    /// Copy a borrowed `&[u8]` view into the owned form. Infallible heap copy
    /// under `alloc` (`N` advisory); fixed-capacity copy without `alloc`,
    /// raising [`CapacityExceeded`] past `N`. The uniform `Result` return
    /// lets a generated `try_into_owned` thread one `?` on every profile, and
    /// `N` is inferred from the destination field's type (no turbofish).
    #[cfg(feature = "alloc")]
    pub fn from_slice(s: &[u8]) -> Result<Self, CapacityExceeded> {
        Ok(Self(s.to_vec(), core::marker::PhantomData))
    }
    /// no-alloc counterpart: fixed-capacity copy, fallible past `N`.
    #[cfg(not(feature = "alloc"))]
    pub fn from_slice(s: &[u8]) -> Result<Self, CapacityExceeded> {
        heapless::Vec::from_slice(s)
            .map(Self)
            .map_err(|_| CapacityExceeded)
    }
    /// Borrow the owned bytes back as a slice — the projection an owned
    /// value's `as_borrowed` re-uses.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const N: usize> core::ops::Deref for SceBytes<N> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

// Byte-literal / slice comparison parity, so a consumer (or the generated
// round-trip sidecar) writes `assert_eq!(owned.payload, b"...")` and
// `owned.payload == &slice` without an explicit `.as_slice()`. Only the
// reference forms are kept — those are what the sidecar (`b"..."` is
// `&[u8; M]`) and owned round-trip tests (`&[u8]`) actually consume; the
// owned-array/unsized forms have no consumer.
impl<const N: usize> PartialEq<&[u8]> for SceBytes<N> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_slice() == *other
    }
}
impl<const N: usize, const M: usize> PartialEq<&[u8; M]> for SceBytes<N> {
    fn eq(&self, other: &&[u8; M]) -> bool {
        self.as_slice() == other.as_slice()
    }
}
