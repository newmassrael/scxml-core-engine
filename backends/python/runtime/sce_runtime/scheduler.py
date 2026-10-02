# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2 delayed-event scheduler with cancel-by-sendid (W3C 6.2.2).

Pull-based: callers (the engine) call `drain_due(now_ms)` to harvest events
whose deadlines have arrived. Virtual time is used so the engine remains
single-threaded and deterministic — wall-clock integration is a caller
concern.
"""

from __future__ import annotations

import heapq
import itertools
from dataclasses import dataclass, field
from typing import Any, Generic, Iterator, List, Optional, Tuple, TypeVar

E = TypeVar("E")


@dataclass(frozen=True)
class ScheduledRoute:
    """W3C SCXML 6.2 — where a delayed `<send>`'s event goes when it comes due.

    A delay postpones a send; it does not change where the send goes. The
    target is resolved when the send is made and recorded here, so the
    delivery at the end of the delay reaches it. An entry without one goes to
    the sending session's own external queue.

    `kind` is ``"internal"`` (this session's internal queue), ``"invocation"``
    (`#_<invokeid>`, named by `invoke_id`) or ``"parent"`` (`#_parent`).
    `event_name` is the event as the receiving machine resolves it: a child or
    a parent is another machine with its own events, so the name is what
    crosses, as it does on an immediate send.
    """

    kind: str
    event_name: str = ""
    invoke_id: str = ""
    session_id: str = ""
    """The child session, for ``"session"`` — a child's published location
    (§scxml-C-1)."""


@dataclass(order=True)
class ScheduledEvent(Generic[E]):
    """One entry in the scheduler's priority queue.

    `due_ms` and `seq` are the ordering keys (stable FIFO on ties);
    `sendid`, `event`, and `data` ride along but do not participate in
    comparison. `data` is the marshalled `<send>` payload (W3C SCXML
    5.10) preserved across the scheduler delay so it surfaces on
    `_event.data` at delivery time.
    """

    due_ms: int
    seq: int
    sendid: str = field(compare=False)
    event: E = field(compare=False)
    data: Any = field(default="", compare=False)
    #: §scxml-6.2.5 — the host-served send this entry performs instead
    #: of delivering `event`, or ``None`` for an ordinary delayed send.
    #:
    #: §scxml-6.2.4 makes a delay a property of the SEND and not of the
    #: processor it named, so a host-served send carrying one is an
    #: ordinary delayed send whose delivery happens to be somebody else's.
    #: Keeping it in THIS queue is what makes that true in practice: one
    #: deadline order across both kinds, one ``<cancel sendid>`` path, one
    #: ``peek_next_due_ms`` answer. A parallel list would oblige every
    #: present and future query to remember it existed.
    host_send: Any = field(default=None, compare=False)
    #: The `HostInvokeDeadline` this entry is, or ``None``. In this queue for
    #: the reason `host_send` is: one deadline order, one answer about when
    #: the host must next advance time. It carries no sendid — a deadline is
    #: the engine's, not the document's, so no ``<cancel sendid>`` can name
    #: it — and leaves through `drop_host_invoke_deadline` or by firing.
    host_invoke_deadline: Any = field(default=None, compare=False)
    #: §scxml-6.2 — the `ScheduledRoute` the delayed send resolved when it
    #: was made, or ``None`` for this session's own external queue.
    route: Optional[ScheduledRoute] = field(default=None, compare=False)
    #: §scxml-6.2.4 — the ids of the open questions the delayed send's ROUTE
    #: rests on (see `Engine.last_unhandled_error_rests_on`). A delayed send is
    #: delivered, or refused, when its wait is over, long after the generated
    #: send site returned, so a refusal made then can only name them if the entry
    #: carried them here. Empty for a send whose route rests on none.
    rests_on: Tuple[str, ...] = field(default=(), compare=False)


class Scheduler(Generic[E]):
    """Min-heap of `(due_ms, seq)` keyed scheduled events.

    Single-threaded. The caller is responsible for advancing virtual
    time and forwarding drained events into the engine's external queue.
    """

    def __init__(self) -> None:
        self._heap: List[ScheduledEvent[E]] = []
        self._counter = itertools.count(1)

    def schedule(
        self,
        due_ms: int,
        sendid: str,
        event: E,
        data: Any = "",
        host_send: Any = None,
        host_invoke_deadline: Any = None,
        route: Optional[ScheduledRoute] = None,
        rests_on: Tuple[str, ...] = (),
    ) -> None:
        """Queue `event` for delivery at `due_ms`. `sendid` identifies the
        entry for later `<cancel>` lookups; empty string ids cannot be
        cancelled (matches W3C SCXML 6.2.2 where `<cancel>` requires a
        sendid). `data` is the marshalled `<send>` payload preserved
        across the delay.

        `host_send` makes the entry a W3C SCXML 6.2.5 host-served send to
        be PERFORMED at `due_ms` rather than an event to be delivered —
        the same queue, so it is ordered against every other delayed send
        by deadline and cancelled by the same id.

        `host_invoke_deadline` makes the entry the deadline of a host-run
        invocation start, to be judged at `due_ms`.

        `route` is where the event goes when due (W3C SCXML 6.2), ``None``
        for this session's own external queue. `rests_on` is the open
        questions that route rests on, handed back to whoever reports a
        refusal made when the entry comes due."""
        heapq.heappush(
            self._heap,
            ScheduledEvent(
                due_ms=due_ms,
                seq=next(self._counter),
                sendid=sendid,
                event=event,
                data=data,
                host_send=host_send,
                host_invoke_deadline=host_invoke_deadline,
                route=route,
                rests_on=rests_on,
            ),
        )

    def drop_host_invoke_deadline(self, token: int) -> None:
        """Drop the pending deadline of host-run invocation start `token`,
        whose invocation ended another way — completed or cancelled. Leaving
        it would keep a host advancing time toward a deadline that can no
        longer do anything."""
        kept = [
            entry
            for entry in self._heap
            if entry.host_invoke_deadline is None
            or entry.host_invoke_deadline.token != token
        ]
        if len(kept) != len(self._heap):
            heapq.heapify(kept)
            self._heap = kept

    def cancel(self, sendid: str) -> bool:
        """W3C SCXML 6.3 — remove every entry still queued under `sendid`.
        Answers whether there was one.

        `<cancel>` reaches the delayed events an earlier `<send>` queued and
        that are STILL queued; it is not a standing order about the id. So
        nothing is remembered: a `<send>` made after the cancel, with the same
        id, is a new send and is delivered. A state that arms `<send id="t"
        delay>` on entry and cancels `t` on exit cancels, when its own timeout
        has just fired, an id nothing is pending under, and is then re-entered
        and arms `t` again — remembering the id dropped that second timer, and
        the machine waited for ever (measured 2026-10-01 on a retry machine).

        The same answer the Rust scheduler gives (`PullScheduler::cancel_event`,
        which retains what does not match) and Go's `CancelEvent`. An empty
        `sendid` cancels nothing: an entry without an id cannot be named
        (W3C "id must be set to cancel"), and a host-run invocation's deadline
        carries none, so a `<cancel sendidexpr>` that evaluates to "" must not
        reach it."""
        if not sendid:
            return False
        kept = [entry for entry in self._heap if entry.sendid != sendid]
        if len(kept) == len(self._heap):
            return False
        heapq.heapify(kept)
        self._heap = kept
        return True

    def drain_due(self, now_ms: int) -> Iterator[ScheduledEvent[E]]:
        """Yield every scheduled event whose `due_ms <= now_ms`, popping
        them off the heap.

        Draining is a generator, so a caller that runs a macrostep per
        entry sees each cancellation the previous one performed — see
        `pop_due`, which is the one-at-a-time form the engine uses."""
        while self._heap and self._heap[0].due_ms <= now_ms:
            yield heapq.heappop(self._heap)

    def pop_due(self, now_ms: int) -> Optional[ScheduledEvent[E]]:
        """Pop the single earliest entry due at `now_ms`, or None.

        The engine dispatches one of these per macrostep so a `<cancel>`
        performed by an earlier event still reaches a later one that has
        not been delivered yet. Popping them all at once instead makes
        every later entry undroppable, which is how a settle timer —
        arm a long `<send delay>`, cancel it when the short signal
        arrives first — delivers the event it was told to cancel."""
        for entry in self.drain_due(now_ms):
            return entry
        return None

    def peek_next_due_ms(self) -> Optional[int]:
        """The `due_ms` of the earliest entry that would actually be
        delivered, or None if there is none. A cancelled entry is not on the
        heap, so the front of it is a deadline someone will see. Callers use
        this to compute the next wake deadline — see
        `Engine.time_until_next_scheduled_ms`."""
        return self._heap[0].due_ms if self._heap else None

    def __len__(self) -> int:
        return len(self._heap)
