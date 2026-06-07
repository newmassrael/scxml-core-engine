// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Per-machine scheduled-send-id storage policy (watching-zenoh RFC §5.J.2).
//!
//! The delayed-event scheduler ([`PullScheduler`](crate::engine::PullScheduler))
//! keeps a `send_id` on every `ScheduledEntry` so W3C SCXML 6.3
//! `<cancel sendid>` can find and remove the matching pending entry. That id is
//! read **only** by [`PullScheduler::cancel_event`](crate::engine::PullScheduler::cancel_event):
//! the timer-fire drain ([`Engine::tick`](crate::Engine::tick)) hands the popped
//! event to [`raise_external`](crate::Engine::raise_external) with an empty
//! `send_id`/`origin`, so the stored id never reaches the fired event's
//! metadata on either build profile. A state machine whose document contains no
//! `<cancel>` element therefore never reads the stored id at all.
//!
//! For such a cancel-free machine the per-entry `heapless::String<256>`
//! (~264 B × [`MAX_SCHEDULED_EVENTS`](crate::MAX_SCHEDULED_EVENTS)) is pure dead
//! weight under `--features=no_std` — after the delayed-send `event_data`
//! elision it is the single largest resident in the no_std `Engine`. This trait
//! lets the generated [`StatePolicy::ScheduledSendId`](crate::StatePolicy::ScheduledSendId)
//! associated type pick the storage per machine: [`SceString`] when the document
//! uses `<cancel>` (load-bearing on both profiles), [`ElidedSendId`] (zero-size)
//! when it does not. Mirrors the per-machine
//! [`StatePolicy::EventQueue`](crate::StatePolicy::EventQueue) sizing lever.

use crate::SceString;
use core::fmt::Debug;

/// Storage abstraction for a scheduled entry's cancel key (the `send_id`).
///
/// Two impls ship: [`SceString`] (stores the id, matches by string equality —
/// load-bearing for `<cancel>`) and [`ElidedSendId`] (stores nothing, never
/// matches — selected by codegen for cancel-free documents). `Debug` is a
/// supertrait because the scheduler types derive `Debug`.
pub trait ScheduledSendIdLike: Debug {
    /// Capture the resolved send id into the stored representation.
    ///
    /// Takes the id by reference so the [`ElidedSendId`] impl can drop it
    /// without an owning clone; the [`SceString`] impl clones it.
    fn store(send_id: &SceString) -> Self;

    /// Whether this stored id equals `send_id` (the `<cancel sendid>` key).
    fn matches(&self, send_id: &str) -> bool;
}

impl ScheduledSendIdLike for SceString {
    #[inline]
    fn store(send_id: &SceString) -> Self {
        send_id.clone()
    }

    #[inline]
    fn matches(&self, send_id: &str) -> bool {
        self.as_str() == send_id
    }
}

/// Zero-size [`ScheduledSendIdLike`] for cancel-free machines.
///
/// Stores nothing and never matches. The always-`false` `matches` is
/// unreachable in practice: a document with no `<cancel>` emits no
/// `cancel_event` call site, so [`PullScheduler::cancel_event`](crate::engine::PullScheduler::cancel_event)
/// is never invoked on a scheduler parameterised with this type. Keeping it a
/// sound no-op rather than `unreachable!()` means a stray direct call (e.g. a
/// runtime unit test) degrades to "found nothing" instead of panicking.
///
/// Selecting this over [`SceString`] removes the per-entry string from
/// `ScheduledEntry`, shrinking the no_std scheduler ring by
/// ~264 B × [`MAX_SCHEDULED_EVENTS`](crate::MAX_SCHEDULED_EVENTS). Under std the
/// scheduler is heap-backed and the id is likewise never read for a cancel-free
/// machine, so the same emission is behaviour-preserving on both profiles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ElidedSendId;

impl ScheduledSendIdLike for ElidedSendId {
    #[inline]
    fn store(_send_id: &SceString) -> Self {
        ElidedSendId
    }

    #[inline]
    fn matches(&self, _send_id: &str) -> bool {
        false
    }
}
