// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The atomics and cells the queue algorithms are written against.
//!
//! An ordinary build gets `core`'s. A build with `--cfg loom` gets loom's
//! instrumented ones, which is how the loom models (SCE Protocol-Synthesis
//! RFC §synth-5-P, verification layer 3) explore every interleaving and
//! weak-memory outcome of the same algorithm text that ships. Nothing else
//! differs between the two builds, so nothing else can make them disagree.
//!
//! The cell is used through `with` / `with_mut` closures, loom's API, so
//! that every access to a slot is one loom can see; the `core` version
//! compiles to a plain pointer access.

#[cfg(loom)]
pub(crate) use loom::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering};

#[cfg(not(loom))]
pub(crate) use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// The 64-bit atomics the SCQ rows need. A target without them has no
/// such row, and the module that uses them is absent there too.
#[cfg(all(not(loom), target_has_atomic = "64"))]
pub(crate) use core::sync::atomic::{AtomicI64, AtomicU64};

#[cfg(loom)]
pub(crate) use loom::cell::UnsafeCell;

#[cfg(not(loom))]
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct UnsafeCell<T>(core::cell::UnsafeCell<T>);

#[cfg(not(loom))]
impl<T> UnsafeCell<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(core::cell::UnsafeCell::new(value))
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(*const T) -> R) -> R {
        f(self.0.get())
    }

    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
        f(self.0.get())
    }
}
