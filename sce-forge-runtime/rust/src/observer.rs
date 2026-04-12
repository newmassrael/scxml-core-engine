// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Observer building blocks: hysteresis state, domain-tagged events, fixed-
//! capacity event queue. See SCE_FORGE.md Section 4.11.

use core::mem::MaybeUninit;

/// Trait describing an event domain. Each generated observer is parameterized
/// over a domain type whose associated `Tag` enum lists the events valid in
/// that domain. Different domains produce incompatible [`Event`] types — this
/// is the type-safety mechanism for cross-file event composition.
pub trait EventDomain {
    /// Tag enum listing every event name valid in this domain.
    /// Only `Copy + Eq` is required: tags need value semantics for storage in
    /// [`EventQueue`] and equality comparison for downstream dispatch. No
    /// `Default` is required, so user enums need not declare a sentinel
    /// variant they may not have a meaningful value for.
    type Tag: Copy + Eq;
}

/// Domain-tagged event value. `Copy`/`Clone` are unconditional because
/// `D::Tag: Copy` is required by [`EventDomain`]; we hand-implement them so
/// the auto-derive does not impose a bogus `D: Copy` bound on the domain
/// marker type itself.
pub struct Event<D: EventDomain> {
    pub tag: D::Tag,
}

impl<D: EventDomain> Event<D> {
    pub const fn new(tag: D::Tag) -> Self {
        Self { tag }
    }
}

impl<D: EventDomain> Copy for Event<D> {}

impl<D: EventDomain> Clone for Event<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: EventDomain> core::fmt::Debug for Event<D>
where
    D::Tag: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Event").field("tag", &self.tag).finish()
    }
}

/// Models a 1-bit hysteresis state machine. The generated `update()` loop
/// calls [`enter_if`](Self::enter_if) and [`leave_if`](Self::leave_if); both
/// return `true` exactly when a transition actually occurred, so the generated
/// code can push the corresponding event without re-checking state.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThresholdState {
    active: bool,
}

impl ThresholdState {
    pub const fn new() -> Self {
        Self { active: false }
    }

    pub fn enter_if(&mut self, condition: bool) -> bool {
        if !self.active && condition {
            self.active = true;
            return true;
        }
        false
    }

    pub fn leave_if(&mut self, condition: bool) -> bool {
        if self.active && condition {
            self.active = false;
            return true;
        }
        false
    }

    pub const fn active(&self) -> bool {
        self.active
    }

    pub fn reset(&mut self) {
        self.active = false;
    }
}

/// Fixed-capacity FIFO of [`Event<D>`]. Returned by value from observer
/// `update()` methods. Default capacity is 8.
///
/// Backed by `[MaybeUninit<Event<D>>; CAP]` so the queue does not require any
/// "default" or sentinel tag value. Only the prefix `[0..size]` is initialized
/// at any time; the [`as_slice`](Self::as_slice) method exposes exactly that
/// prefix as a `&[Event<D>]`.
pub struct EventQueue<D: EventDomain, const CAP: usize = 8> {
    buffer: [MaybeUninit<Event<D>>; CAP],
    size: usize,
}

impl<D: EventDomain, const CAP: usize> EventQueue<D, CAP> {
    pub fn new() -> Self {
        // SAFETY: an array of `MaybeUninit` is itself fully initialized, since
        // each slot is permitted to hold uninitialized memory. This is the
        // standard MaybeUninit array idiom.
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            size: 0,
        }
    }

    pub fn push(&mut self, tag: D::Tag) -> bool {
        if self.size >= CAP {
            return false;
        }
        self.buffer[self.size].write(Event::new(tag));
        self.size += 1;
        true
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub const fn capacity(&self) -> usize {
        CAP
    }

    pub fn as_slice(&self) -> &[Event<D>] {
        // SAFETY: the prefix `[0..self.size]` has been written via `push()`,
        // so every entry in the range is initialized. `Event<D>` is `Copy`,
        // so reinterpreting `MaybeUninit<Event<D>>` as `Event<D>` is sound.
        unsafe {
            core::slice::from_raw_parts(
                self.buffer.as_ptr() as *const Event<D>,
                self.size,
            )
        }
    }

    pub fn clear(&mut self) {
        self.size = 0;
    }
}

impl<D: EventDomain, const CAP: usize> Default for EventQueue<D, CAP> {
    fn default() -> Self {
        Self::new()
    }
}
