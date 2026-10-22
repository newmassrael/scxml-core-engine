// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! The bounded buffers a configuration, and every set of states Appendix D's
//! procedures build, are held in.
//!
//! The walks over the hierarchy — domains, exit sets, entry sets — are
//! [`microstep`](super::microstep)'s. They used to be here as an LCA and a pair
//! of entry/exit chain builders, which could name one target and one region at
//! a time; the appendix's procedures replaced them, and what is left is the
//! storage they share.
//!
//! ## Where a capacity comes from
//!
//! A buffer's capacity is the document's, not the runtime's. The generator
//! reads the document's state tree and writes, on the policy it generates, the
//! most states one configuration can hold
//! ([`MAX_CONFIGURATION`](crate::StatePolicy::MAX_CONFIGURATION)), the most
//! transitions one selection can enable
//! ([`MAX_ENABLED`](crate::StatePolicy::MAX_ENABLED)) and how many levels the
//! deepest state sits at ([`MAX_DEPTH`](crate::StatePolicy::MAX_DEPTH)), and
//! spells the buffers of those sizes as associated types
//! ([`Configuration`](crate::StatePolicy::Configuration) and the others). So a
//! machine of three flat states holds three-slot buffers and a machine with a
//! wide `<parallel>` holds as many as its regions need, where one constant of
//! the runtime's was too small for the second and several times too large for
//! the first. The runtime names no capacity of its own.
//!
//! Every number is an upper bound and not an estimate: a configuration is a
//! root-to-leaf path with every region of each `<parallel>` on it, so its size
//! follows from the tree; a selection takes at most one transition from each
//! atomic state of it; an exit set, an entry set and a list of effective targets
//! are sets of states that are, or are about to be, in a configuration.
//!
//! ## Cycle detection
//!
//! Every walk up the hierarchy is bounded at the document's own depth
//! ([`MAX_DEPTH`](crate::StatePolicy::MAX_DEPTH)) to detect cyclic parent relationships. A cycle
//! indicates a generator bug or corrupted SCXML; the walk panics with a
//! diagnostic message. Matches C++ `throw std::runtime_error` semantics — both
//! are fatal, unrecoverable errors.
//!
//! ## The two profiles
//!
//! Under `std` a [`BoundedVec`] is a `Vec`, which grows, and the capacity the
//! generator wrote is a hint nothing reads. Without it, it is a
//! `heapless::Vec` of exactly that capacity, so the push paths cannot fail for
//! any configuration the document can hold — they panic only as a
//! generator-bug tripwire.

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
