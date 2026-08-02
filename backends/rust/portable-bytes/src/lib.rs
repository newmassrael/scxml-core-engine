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
//!
//! # The profile is a type, not a compilation mode
//!
//! [`HeapBytes`] (growable, `N` advisory) and [`InlineBytes`] (fixed `N`,
//! heap-free) are two *types*, both compiled whenever `alloc` is on, rather
//! than one name whose meaning flips with a `cfg`. A single binary therefore
//! holds both storage profiles at once: an application-processor path can
//! keep growable owned values while a real-time path in the same program
//! stores its values inline with a hard capacity. [`SceBytes`] is the alias
//! naming whichever profile is the *default* for the current build, so the
//! plain spelling stays correct on both tiers, and the no-alloc tier — where
//! `HeapBytes` does not exist at all — cannot accidentally name the growable
//! form.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Returned by `from_slice` / `push` / `extend_from_slice` when a copy would
/// exceed a fixed capacity `N`. The growable profile has no such bound and
/// never returns it. Downstream runtimes map it into their own error enum
/// (e.g. the codec runtime's `CodecError::TooManyElements`) via a `From` impl,
/// so a generated `try_into_owned` keeps threading one `?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityExceeded;

/// Growable owned byte storage: wraps `Vec<u8>`, so `N` is advisory — the
/// on-wire protocol caps no payload, and the application-processor profile
/// must not either. `N` still rides on the type so a downstream owned-builder
/// infers the cap from the field rather than hardcoding it, and so a value can
/// be transcoded into [`InlineBytes`] of the matching capacity.
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeapBytes<const N: usize>(alloc::vec::Vec<u8>, core::marker::PhantomData<[u8; N]>);

/// Fixed-capacity owned byte storage: wraps `heapless::Vec<u8, N>` (the C11
/// `char[N]` analog), so the value never allocates and `N` is a hard bound.
///
/// Compiled on every profile — including alongside [`HeapBytes`] under `alloc`
/// — so a heap-capable program can still hold values that are guaranteed not
/// to allocate.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InlineBytes<const N: usize>(heapless::Vec<u8, N>);

/// The byte storage profile this build treats as the default: [`HeapBytes`]
/// where an allocator is available, [`InlineBytes`] on the heap-free tier.
///
/// Code that has no reason to pin a profile spells `SceBytes<N>` and follows
/// the target; code that must not allocate names [`InlineBytes`] explicitly
/// and keeps that guarantee even in a build where the default is growable.
#[cfg(feature = "alloc")]
pub type SceBytes<const N: usize> = HeapBytes<N>;
/// no-alloc build: the default profile is the fixed-capacity form.
#[cfg(not(feature = "alloc"))]
pub type SceBytes<const N: usize> = InlineBytes<N>;

#[cfg(feature = "alloc")]
impl<const N: usize> HeapBytes<N> {
    /// Copy a borrowed `&[u8]` view into the owned form. The heap copy is
    /// unbounded (`N` advisory), but the return stays a `Result` so a
    /// generated `try_into_owned` threads one `?` on every profile and the
    /// two profiles keep one call shape.
    pub fn from_slice(s: &[u8]) -> Result<Self, CapacityExceeded> {
        Ok(Self(s.to_vec(), core::marker::PhantomData))
    }

    /// Borrow the owned bytes back as a slice — the projection an owned
    /// value's `as_borrowed` re-uses.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// An empty buffer that grows on demand. Used by the algorithm kind's
    /// `<sce:var type="bytes" capacity="N">` to seed a buffer that `push` /
    /// `extend_from_slice` then fill.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one byte. The heap form grows, so this never reports overflow;
    /// the `Result` matches [`InlineBytes::push`] so a generated byte-building
    /// algorithm threads one `?` per append rather than branching on the
    /// allocator.
    pub fn push(&mut self, b: u8) -> Result<(), CapacityExceeded> {
        self.0.push(b);
        Ok(())
    }

    /// Append a borrowed slice. Same contract as [`HeapBytes::push`].
    pub fn extend_from_slice(&mut self, s: &[u8]) -> Result<(), CapacityExceeded> {
        self.0.extend_from_slice(s);
        Ok(())
    }
}

impl<const N: usize> InlineBytes<N> {
    /// Copy a borrowed `&[u8]` view into the fixed-capacity form, raising
    /// [`CapacityExceeded`] past `N`.
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

    /// An empty buffer holding the fixed capacity `N` inline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one byte, raising [`CapacityExceeded`] once the fixed `N` is
    /// full — never silent truncation, never a panic.
    pub fn push(&mut self, b: u8) -> Result<(), CapacityExceeded> {
        self.0.push(b).map_err(|_| CapacityExceeded)
    }

    /// Append a borrowed slice. Either the whole slice is appended or none is
    /// — the bound is checked before copying.
    pub fn extend_from_slice(&mut self, s: &[u8]) -> Result<(), CapacityExceeded> {
        self.0.extend_from_slice(s).map_err(|_| CapacityExceeded)
    }
}

/// Byte-view parity surface, written once for both profiles so the growable
/// and fixed forms cannot drift apart in how they are read or compared.
///
/// `Deref<Target = [u8]>` gives `len()` / iteration / slicing; the `PartialEq`
/// arms let a consumer (or a generated round-trip sidecar) write
/// `assert_eq!(owned.payload, b"...")` and `owned.payload == &slice` without
/// an explicit `.as_slice()`. Only the reference forms are implemented —
/// those are what the sidecar (`b"..."` is `&[u8; M]`) and the owned
/// round-trip tests (`&[u8]`) actually consume.
macro_rules! byte_view_parity {
    ($storage:ident) => {
        impl<const N: usize> core::ops::Deref for $storage<N> {
            type Target = [u8];
            fn deref(&self) -> &[u8] {
                self.0.as_slice()
            }
        }

        impl<const N: usize> PartialEq<&[u8]> for $storage<N> {
            fn eq(&self, other: &&[u8]) -> bool {
                self.as_slice() == *other
            }
        }

        impl<const N: usize, const M: usize> PartialEq<&[u8; M]> for $storage<N> {
            fn eq(&self, other: &&[u8; M]) -> bool {
                self.as_slice() == other.as_slice()
            }
        }
    };
}

#[cfg(feature = "alloc")]
byte_view_parity!(HeapBytes);
byte_view_parity!(InlineBytes);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty_then_push_fills() {
        let mut b = SceBytes::<4>::new();
        assert_eq!(b.as_slice(), &[] as &[u8]);
        assert!(b.push(1).is_ok());
        assert!(b.push(2).is_ok());
        assert_eq!(b.as_slice(), &[1u8, 2]);
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn extend_from_slice_appends_in_order() {
        let mut b = SceBytes::<8>::new();
        assert!(b.extend_from_slice(&[1, 2, 3]).is_ok());
        assert!(b.push(4).is_ok());
        assert!(b.extend_from_slice(&[5, 6]).is_ok());
        assert_eq!(b.as_slice(), &[1u8, 2, 3, 4, 5, 6]);
    }

    // The fixed-capacity profile is a hard bound on EVERY build, not only on
    // the heap-free tier: naming `InlineBytes` under `alloc` must still refuse
    // to grow. This is the guarantee a cfg-switched single type cannot give.
    #[test]
    fn inline_profile_is_bounded_even_where_heap_exists() {
        let mut b = InlineBytes::<2>::new();
        assert!(b.push(1).is_ok());
        assert!(b.push(2).is_ok());
        assert_eq!(b.push(3), Err(CapacityExceeded));
        assert_eq!(b.as_slice(), &[1u8, 2]);
    }

    #[test]
    fn inline_extend_past_capacity_is_atomic() {
        let mut b = InlineBytes::<3>::new();
        assert!(b.push(9).is_ok());
        // Three more would exceed N=3 — heapless rejects without partial copy.
        assert_eq!(b.extend_from_slice(&[1, 2, 3]), Err(CapacityExceeded));
        assert_eq!(b.as_slice(), &[9u8]);
    }

    #[test]
    fn inline_from_slice_rejects_past_capacity() {
        assert!(InlineBytes::<3>::from_slice(&[1, 2, 3]).is_ok());
        assert_eq!(
            InlineBytes::<3>::from_slice(&[1, 2, 3, 4]),
            Err(CapacityExceeded)
        );
    }

    // alloc profile: `N` is advisory; the growable form passes it without error.
    #[cfg(feature = "alloc")]
    #[test]
    fn heap_profile_is_unbounded() {
        let mut b = HeapBytes::<2>::new();
        for i in 0..10u8 {
            assert!(b.push(i).is_ok());
        }
        assert_eq!(b.len(), 10);
        assert!(HeapBytes::<2>::from_slice(&[0; 64]).is_ok());
    }

    // Both profiles coexist in one binary and carry the same bytes — the
    // property that makes a per-use-site storage choice possible at all.
    #[cfg(feature = "alloc")]
    #[test]
    fn both_profiles_coexist_over_the_same_payload() {
        let payload: &[u8] = b"same wire bytes";
        let heap = HeapBytes::<32>::from_slice(payload).unwrap();
        let inline = InlineBytes::<32>::from_slice(payload).unwrap();
        assert_eq!(heap.as_slice(), inline.as_slice());
        assert_eq!(heap, payload);
        assert_eq!(inline, payload);
    }
}
