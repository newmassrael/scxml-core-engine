// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! The bounded buffer a configuration is handed to a host in.
//!
//! Appendix D's procedures build their sets in the machine's
//! [`Workspace`](super::microstep::Workspace), which is the same for every
//! machine (docs/adr/0064). What is left here is the typed buffer the engine
//! publishes a configuration through,
//! [`Configuration`](crate::StatePolicy::Configuration), whose capacity is the
//! most states one configuration of the document holds: the generator reads
//! it from the state tree, so a machine of three flat states holds three slots
//! and a machine with a wide `<parallel>` as many as its regions need. The
//! runtime names no capacity of its own.
//!
//! ## The two profiles
//!
//! Under `std` a [`BoundedVec`] is a `Vec`, which grows, and the capacity the
//! generator wrote is a hint nothing reads. Without it, it is a
//! `heapless::Vec` of exactly that capacity (the bare-metal variant of the
//! statechart runtime, SCE Protocol-Synthesis RFC §synth-5-J-2), so the push
//! paths cannot fail for any configuration the document can hold — they panic
//! only as a generator-bug tripwire.

use core::fmt::Debug;
use core::ops::{Deref, DerefMut};

use crate::BoundedPush;

/// A sequence with a capacity fixed when it is spelled, as every set of states
/// a microstep builds is: a configuration, an exit or entry set, a list of
/// effective targets, a history record, the transitions a selection enables.
///
/// What it adds to a slice is what building one takes: it starts empty
/// ([`Default`]), gains an item ([`BoundedPush::push_bounded`]) and loses them
/// ([`clear`](Self::clear), [`truncate`](Self::truncate),
/// [`retain`](Self::retain)), and it is consumed by value into its items
/// (`IntoIterator`), as the `Vec` it is under `std` always was. Implemented for
/// `heapless::Vec<T, N>` in both profiles and for `Vec<T>` under `std`, so a
/// generated policy names its buffers through [`BoundedVec`] and the same text
/// compiles against either.
pub trait Bounded<T>:
    BoundedPush<T> + Default + Clone + Debug + Deref<Target = [T]> + DerefMut + IntoIterator<Item = T>
{
    /// Remove every item.
    fn clear(&mut self);

    /// Keep the first `len` items; do nothing when there are not that many.
    fn truncate(&mut self, len: usize);

    /// Keep the items `keep` answers `true` for, in their order.
    fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F);
}

impl<T: Clone + Debug, const N: usize> Bounded<T> for ::heapless::Vec<T, N> {
    #[inline]
    fn clear(&mut self) {
        ::heapless::Vec::clear(self);
    }

    #[inline]
    fn truncate(&mut self, len: usize) {
        ::heapless::Vec::truncate(self, len);
    }

    #[inline]
    fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
        ::heapless::Vec::retain(self, keep);
    }
}

#[cfg(feature = "std")]
impl<T: Clone + Debug> Bounded<T> for ::std::vec::Vec<T> {
    #[inline]
    fn clear(&mut self) {
        ::std::vec::Vec::clear(self);
    }

    #[inline]
    fn truncate(&mut self, len: usize) {
        ::std::vec::Vec::truncate(self, len);
    }

    #[inline]
    fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
        ::std::vec::Vec::retain(self, keep);
    }
}

/// The buffer of capacity `N` a generated policy spells its
/// [`Bounded`] associated types with.
///
/// - **std build**: [`Vec<T>`] — grows; `N` is not read.
/// - **no_std build**: `heapless::Vec<T, N>` — stack-allocated, of exactly the
///   capacity the document needs.
#[cfg(feature = "std")]
pub type BoundedVec<T, const N: usize> = ::std::vec::Vec<T>;
/// no_std variant of [`BoundedVec`]. See the std-variant doc-comment above.
#[cfg(not(feature = "std"))]
pub type BoundedVec<T, const N: usize> = ::heapless::Vec<T, N>;
