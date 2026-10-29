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
pub(crate) use loom::sync::atomic::{
    fence, AtomicBool, AtomicI64, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering,
};

/// What every queue uses, on every target: loads and stores of a word.
#[cfg(not(loom))]
pub(crate) use core::sync::atomic::{AtomicUsize, Ordering};

/// What the modules that compare and swap a flag use (`linked`, `hazard`): present where those modules
/// are, so that a target without the atomics (ARMv6-M) has neither the modules nor an import nothing uses.
#[cfg(all(not(loom), target_has_atomic = "8"))]
pub(crate) use core::sync::atomic::{AtomicBool, AtomicPtr};

/// What `intrusive` uses, where it is.
#[cfg(all(not(loom), target_has_atomic = "32"))]
pub(crate) use core::sync::atomic::AtomicU32;

/// The 64-bit atomics the SCQ rows need. A target without them has no
/// such row, and the module that uses them is absent there too.
#[cfg(all(not(loom), target_has_atomic = "64"))]
pub(crate) use core::sync::atomic::{AtomicI64, AtomicU64};

/// The fences of the broadcast log's sequence-lock read (ADR 0062 section 4),
/// present wherever the log is: on every target whose pointer is at least 32 bits,
/// with loads and stores alone as well as with compare-and-swap, because a fence
/// needs no read-modify-write instruction.
#[cfg(all(
    not(loom),
    any(target_pointer_width = "32", target_pointer_width = "64")
))]
pub(crate) use core::sync::atomic::fence;

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
