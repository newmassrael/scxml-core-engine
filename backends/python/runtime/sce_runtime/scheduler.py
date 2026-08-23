# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
from typing import Any, Generic, Iterator, List, Optional, Set, TypeVar

E = TypeVar("E")


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


class Scheduler(Generic[E]):
    """Min-heap of `(due_ms, seq)` keyed scheduled events.

    Single-threaded. The caller is responsible for advancing virtual
    time and forwarding drained events into the engine's external queue.
    """

    def __init__(self) -> None:
        self._heap: List[ScheduledEvent[E]] = []
        self._cancelled: Set[str] = set()
        self._counter = itertools.count(1)

    def schedule(
        self,
        due_ms: int,
        sendid: str,
        event: E,
        data: Any = "",
        host_send: Any = None,
    ) -> None:
        """Queue `event` for delivery at `due_ms`. `sendid` identifies the
        entry for later `<cancel>` lookups; empty string ids cannot be
        cancelled (matches W3C SCXML 6.2.2 where `<cancel>` requires a
        sendid). `data` is the marshalled `<send>` payload preserved
        across the delay.

        `host_send` makes the entry a W3C SCXML 6.2.5 host-served send to
        be PERFORMED at `due_ms` rather than an event to be delivered —
        the same queue, so it is ordered against every other delayed send
        by deadline and cancelled by the same id."""
        heapq.heappush(
            self._heap,
            ScheduledEvent(
                due_ms=due_ms,
                seq=next(self._counter),
                sendid=sendid,
                event=event,
                data=data,
                host_send=host_send,
            ),
        )

    def cancel(self, sendid: str) -> None:
        """W3C SCXML 6.2.2 — mark `sendid` cancelled; the matching entry
        is skipped the next time it would be drained. No-op on empty
        `sendid` (matches the W3C "id must be set to cancel" semantics)."""
        if sendid:
            self._cancelled.add(sendid)

    def drain_due(self, now_ms: int) -> Iterator[ScheduledEvent[E]]:
        """Yield every scheduled event whose `due_ms <= now_ms`, popping
        them off the heap. Cancelled entries are silently discarded as
        they would have been delivered.

        Draining is a generator, so a caller that runs a macrostep per
        entry sees each cancellation the previous one performed — see
        `pop_due`, which is the one-at-a-time form the engine uses."""
        while self._heap and self._heap[0].due_ms <= now_ms:
            entry = heapq.heappop(self._heap)
            if entry.sendid and entry.sendid in self._cancelled:
                self._cancelled.discard(entry.sendid)
                continue
            yield entry

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
        delivered, or None if there is none.

        Cancelled entries stay on the heap until a drain walks past
        them, so the front of the heap is not necessarily a deadline
        anyone will see; they are dropped here first. Callers use this
        to compute the next wake deadline — see
        `Engine.time_until_next_scheduled_ms`."""
        while self._heap:
            head = self._heap[0]
            if head.sendid and head.sendid in self._cancelled:
                heapq.heappop(self._heap)
                self._cancelled.discard(head.sendid)
                continue
            return head.due_ms
        return None

    def __len__(self) -> int:
        return len(self._heap)
