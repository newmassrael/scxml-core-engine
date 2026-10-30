// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The delayed-send queue a machine keeps, chosen by its policy.
//!
//! A document that writes no `<send delay>`, no `<send delayexpr>`, no
//! `<cancel>` and no host-run deadline arms nothing on the delayed-send queue
//! and never asks it to drop anything, so a queue there is capacity nobody can
//! fill. Under `--no-default-features` that is the whole of a
//! [`PullScheduler`]'s fixed ring, [`MAX_SCHEDULED_EVENTS`](crate::MAX_SCHEDULED_EVENTS)
//! entries of it, in every engine of such a document. The generator therefore
//! picks the queue per document, the way it picks the event queue's depth and
//! the cancel key's storage: [`StatePolicy::Scheduler`](crate::StatePolicy::Scheduler)
//! is [`PullScheduler`] for a document that delays or cancels a send, and the
//! zero-size [`NoDelayedSends`] for one that does neither.
//!
//! The choice is the policy's, the operations are the engine's: a host names
//! one of the two types and drives the engine, never the queue. The trait is
//! sealed for that reason. Its operations live on a supertrait no other crate
//! can name, so a profile that has an operation the other lacks (the bare-metal
//! drain pops events, the hosted one pops acts) gives no crate a name that stops
//! resolving when another crate of the same build turns `std` on
//! (docs/adr/0053), and no implementation outside this crate can be broken by
//! the hosted profile adding one.

use crate::engine::{PullScheduler, SchedTimePoint};
#[cfg(feature = "std")]
use crate::engine::{ScheduledAct, ScheduledRoute};
use crate::helpers::tables::EventId;
use crate::sched_send_id::ScheduledSendIdLike;
use crate::SceString;

/// The delayed-send queue of a machine: [`PullScheduler`] or
/// [`NoDelayedSends`], the only two types that implement it.
///
/// Named by [`StatePolicy::Scheduler`](crate::StatePolicy::Scheduler). The
/// operations the engine performs on it are on a sealed supertrait (see the
/// module documentation for why).
pub trait SchedulerStorage: sealed::SchedulerOps {}

impl<S: ScheduledSendIdLike> SchedulerStorage for PullScheduler<S> {}

impl SchedulerStorage for NoDelayedSends {}

/// The delayed-send queue of a document that has no delayed send to wait in
/// it: no `<send delay>`, no `<send delayexpr>`, no `<cancel>`, no host-run
/// `<invoke>` that names a deadline. It holds nothing and is zero-size.
///
/// Every query answers that nothing is pending: nothing is ready, nothing comes
/// due, a cancel finds nothing, a save lists nothing, a tick pops nothing. A
/// tick of such a machine therefore drains no queue and its loop compiles to
/// nothing, and `Engine::time_until_next_scheduled_ms` answers from the
/// machine's children alone.
///
/// Arming an entry stops the program, with a message that says the machine
/// keeps no delayed-send queue. Only a delayed send or a host-run deadline can
/// arm one, and the generator chooses this type only for a document that has
/// neither (and no `<cancel>`), so a generated machine never reaches it.
/// It is the refusal the bounded queue makes when an entry does not fit (the
/// full ring stops the program rather than drop the send), for a queue whose
/// capacity is none: a send dropped in silence would be a transition that never
/// happens, which no host could see. A host that arms one by hand, through
/// `Engine::schedule_event`, names [`PullScheduler`] in its policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoDelayedSends;

/// The one way an entry reaches a queue that has no room: stop, and say why.
#[cold]
#[inline(never)]
fn no_delayed_sends() -> ! {
    panic!(
        "a delayed send was armed on a machine whose document has none: its policy names \
         `NoDelayedSends`, which the generator chooses only for a document with no \
         `<send delay>`, `<send delayexpr>`, `<cancel>` or host-run deadline, so the \
         machine keeps no delayed-send queue. A policy that arms one names `PullScheduler`."
    )
}

/// The operations of [`SchedulerStorage`], on a trait no other crate can name.
pub(crate) mod sealed {
    use super::*;

    /// What the engine does with its delayed-send queue: arm an entry, drop one
    /// a cancel names, ask what is ready or when the next entry comes due, and
    /// pop the one due first.
    ///
    /// The methods are [`PullScheduler`]'s, under the same names, so its
    /// documentation is the contract; [`NoDelayedSends`] answers each as a
    /// queue that holds nothing.
    pub trait SchedulerOps {
        /// An empty queue, as a constant: the engine is built in a constant
        /// expression (`Engine::new_const`) as well as at run time.
        const EMPTY: Self;

        /// Whether the queue can hold an entry at all. A restore reads it to
        /// refuse a saved state that lists a waiting send, or a host-run
        /// deadline, for a machine that keeps no queue to put it in.
        const KEEPS_DELAYED_SENDS: bool;

        /// Arm `event` to come due at `ready_at`, under `send_id`, or under an
        /// id made for it when `send_id` is empty; returns the id used.
        fn schedule_event_at(
            &mut self,
            event: EventId,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
        ) -> SceString {
            self.schedule_event_named_at(event, ready_at, send_id, event_data, origin, "")
        }

        /// [`schedule_event_at`](Self::schedule_event_at) for an event sent
        /// under `name`, when that is not the member's own.
        fn schedule_event_named_at(
            &mut self,
            event: EventId,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            name: &str,
        ) -> SceString;

        /// [`schedule_event_named_at`](Self::schedule_event_named_at) under the
        /// id the caller names, which it does not get back.
        fn schedule_event_as(
            &mut self,
            event: EventId,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            name: &str,
        );

        /// Arm a send whose target is not this session's own external queue;
        /// returns the id used.
        #[cfg(feature = "std")]
        fn schedule_routed_at(
            &mut self,
            event: Option<EventId>,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            route: ScheduledRoute,
        ) -> SceString {
            self.schedule_routed_named_at(event, ready_at, send_id, event_data, origin, route, "")
        }

        /// [`schedule_routed_at`](Self::schedule_routed_at) for an event sent
        /// under `name`, when that is not the member's own.
        #[cfg(feature = "std")]
        #[allow(clippy::too_many_arguments)]
        fn schedule_routed_named_at(
            &mut self,
            event: Option<EventId>,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            route: ScheduledRoute,
            name: &str,
        ) -> SceString;

        /// Arm a send a host-supplied processor serves; returns the id used.
        #[cfg(feature = "std")]
        fn schedule_host_send_at(
            &mut self,
            request: crate::host_processor::HostSendRequest,
            ready_at: SchedTimePoint,
            send_id: &str,
        ) -> SceString;

        /// Arm the deadline of the host-run invocation start `token`.
        #[cfg(feature = "std")]
        fn schedule_host_invoke_deadline_at(
            &mut self,
            processor_type: &str,
            invoke_id: &str,
            token: u64,
            ready_at: SchedTimePoint,
        );

        /// Drop the pending deadline of start `token`, if one is armed.
        #[cfg(feature = "std")]
        fn drop_host_invoke_deadline(&mut self, token: u64);

        /// Drop every entry armed under `send_id`; whether one was.
        fn cancel_event(&mut self, send_id: &str) -> bool;

        /// Whether an entry is due at or before `now`.
        fn has_ready_events_at(&self, now: SchedTimePoint) -> bool;

        /// The entry the next pop would take, left in place: the instant it
        /// came due and the event it delivers to this machine's own queues.
        fn next_ready_at_with_event(
            &self,
            now: SchedTimePoint,
        ) -> Option<(SchedTimePoint, Option<EventId>)>;

        /// When the earliest entry comes due, ready or not.
        fn next_ready_at(&self) -> Option<SchedTimePoint>;

        /// Every entry with the instant it comes due, in the order the pops
        /// would take them.
        #[cfg(feature = "std")]
        fn pending(&self) -> Vec<(SchedTimePoint, &ScheduledAct)>;

        /// Take the act that came due first.
        #[cfg(feature = "std")]
        fn pop_ready_act_at(&mut self, now: SchedTimePoint) -> Option<ScheduledAct>;

        /// Take the event that came due first: the bare-metal drain, where an
        /// entry is only ever an event.
        #[cfg(not(feature = "std"))]
        fn pop_ready_event_at(&mut self, now: SchedTimePoint) -> Option<EventId>;
    }

    impl<S: ScheduledSendIdLike> SchedulerOps for PullScheduler<S> {
        const EMPTY: Self = Self::new();
        const KEEPS_DELAYED_SENDS: bool = true;

        #[inline]
        fn schedule_event_named_at(
            &mut self,
            event: EventId,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            name: &str,
        ) -> SceString {
            PullScheduler::schedule_event_named_at(
                self, event, ready_at, send_id, event_data, origin, name,
            )
        }

        #[inline]
        fn schedule_event_as(
            &mut self,
            event: EventId,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            name: &str,
        ) {
            PullScheduler::schedule_event_as(
                self, event, ready_at, send_id, event_data, origin, name,
            )
        }

        #[cfg(feature = "std")]
        #[inline]
        #[allow(clippy::too_many_arguments)]
        fn schedule_routed_named_at(
            &mut self,
            event: Option<EventId>,
            ready_at: SchedTimePoint,
            send_id: &str,
            event_data: &str,
            origin: &str,
            route: ScheduledRoute,
            name: &str,
        ) -> SceString {
            PullScheduler::schedule_routed_named_at(
                self, event, ready_at, send_id, event_data, origin, route, name,
            )
        }

        #[cfg(feature = "std")]
        #[inline]
        fn schedule_host_send_at(
            &mut self,
            request: crate::host_processor::HostSendRequest,
            ready_at: SchedTimePoint,
            send_id: &str,
        ) -> SceString {
            PullScheduler::schedule_host_send_at(self, request, ready_at, send_id)
        }

        #[cfg(feature = "std")]
        #[inline]
        fn schedule_host_invoke_deadline_at(
            &mut self,
            processor_type: &str,
            invoke_id: &str,
            token: u64,
            ready_at: SchedTimePoint,
        ) {
            PullScheduler::schedule_host_invoke_deadline_at(
                self,
                processor_type,
                invoke_id,
                token,
                ready_at,
            )
        }

        #[cfg(feature = "std")]
        #[inline]
        fn drop_host_invoke_deadline(&mut self, token: u64) {
            PullScheduler::drop_host_invoke_deadline(self, token)
        }

        #[inline]
        fn cancel_event(&mut self, send_id: &str) -> bool {
            PullScheduler::cancel_event(self, send_id)
        }

        #[inline]
        fn has_ready_events_at(&self, now: SchedTimePoint) -> bool {
            PullScheduler::has_ready_events_at(self, now)
        }

        #[inline]
        fn next_ready_at_with_event(
            &self,
            now: SchedTimePoint,
        ) -> Option<(SchedTimePoint, Option<EventId>)> {
            PullScheduler::next_ready_at_with_event(self, now)
        }

        #[inline]
        fn next_ready_at(&self) -> Option<SchedTimePoint> {
            PullScheduler::next_ready_at(self)
        }

        #[cfg(feature = "std")]
        #[inline]
        fn pending(&self) -> Vec<(SchedTimePoint, &ScheduledAct)> {
            PullScheduler::pending(self)
        }

        #[cfg(feature = "std")]
        #[inline]
        fn pop_ready_act_at(&mut self, now: SchedTimePoint) -> Option<ScheduledAct> {
            PullScheduler::pop_ready_act_at(self, now)
        }

        #[cfg(not(feature = "std"))]
        #[inline]
        fn pop_ready_event_at(&mut self, now: SchedTimePoint) -> Option<EventId> {
            PullScheduler::pop_ready_event_at(self, now)
        }
    }

    impl SchedulerOps for NoDelayedSends {
        const EMPTY: Self = NoDelayedSends;
        const KEEPS_DELAYED_SENDS: bool = false;

        fn schedule_event_named_at(
            &mut self,
            _event: EventId,
            _ready_at: SchedTimePoint,
            _send_id: &str,
            _event_data: &str,
            _origin: &str,
            _name: &str,
        ) -> SceString {
            no_delayed_sends()
        }

        fn schedule_event_as(
            &mut self,
            _event: EventId,
            _ready_at: SchedTimePoint,
            _send_id: &str,
            _event_data: &str,
            _origin: &str,
            _name: &str,
        ) {
            no_delayed_sends()
        }

        #[cfg(feature = "std")]
        #[allow(clippy::too_many_arguments)]
        fn schedule_routed_named_at(
            &mut self,
            _event: Option<EventId>,
            _ready_at: SchedTimePoint,
            _send_id: &str,
            _event_data: &str,
            _origin: &str,
            _route: ScheduledRoute,
            _name: &str,
        ) -> SceString {
            no_delayed_sends()
        }

        #[cfg(feature = "std")]
        fn schedule_host_send_at(
            &mut self,
            _request: crate::host_processor::HostSendRequest,
            _ready_at: SchedTimePoint,
            _send_id: &str,
        ) -> SceString {
            no_delayed_sends()
        }

        #[cfg(feature = "std")]
        fn schedule_host_invoke_deadline_at(
            &mut self,
            _processor_type: &str,
            _invoke_id: &str,
            _token: u64,
            _ready_at: SchedTimePoint,
        ) {
            no_delayed_sends()
        }

        /// Nothing to drop: no deadline was ever armed. A host-run invocation
        /// with no deadline ends through here as well, so this answers rather
        /// than refuses.
        #[cfg(feature = "std")]
        #[inline]
        fn drop_host_invoke_deadline(&mut self, _token: u64) {}

        /// Nothing pending, so nothing found.
        #[inline]
        fn cancel_event(&mut self, _send_id: &str) -> bool {
            false
        }

        #[inline]
        fn has_ready_events_at(&self, _now: SchedTimePoint) -> bool {
            false
        }

        #[inline]
        fn next_ready_at_with_event(
            &self,
            _now: SchedTimePoint,
        ) -> Option<(SchedTimePoint, Option<EventId>)> {
            None
        }

        #[inline]
        fn next_ready_at(&self) -> Option<SchedTimePoint> {
            None
        }

        #[cfg(feature = "std")]
        #[inline]
        fn pending(&self) -> Vec<(SchedTimePoint, &ScheduledAct)> {
            Vec::new()
        }

        #[cfg(feature = "std")]
        #[inline]
        fn pop_ready_act_at(&mut self, _now: SchedTimePoint) -> Option<ScheduledAct> {
            None
        }

        #[cfg(not(feature = "std"))]
        #[inline]
        fn pop_ready_event_at(&mut self, _now: SchedTimePoint) -> Option<EventId> {
            None
        }
    }
}
