# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The inbox of an engine: the one part of it a thread that does not own it may
use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").

A multi-producer, single-consumer queue of what other threads have for an
engine, which the engine's owner applies at its next macrostep boundary, each
input by the owner-thread operation of the same name. What may be posted is a
closed set, each input carrying what the operation that applies it takes.

The queue is a `collections.deque` behind one `threading.Lock`, not a
`queue.SimpleQueue`. The inbox needs two things a bare concurrent queue does not
give together: a post refused once the inbox is closed, decided under the same
lock as the append, so no post lands after a close took what the inbox held;
and a drain that takes what the inbox held when the boundary was reached in one
step, so posters that never stop cannot hold the owner there. The lock is held
for an append or a swap and nothing else, so a post never waits for a
macrostep, and the waker is called after it is released. It holds on a
free-threaded interpreter as on one with a global lock.

Unbounded (docs/adr/0084, D2): a post is refused only when the inbox is closed.
"""

from __future__ import annotations

import threading
from collections import deque
from dataclasses import dataclass
from typing import Any, Callable, Deque, Generic, Optional, TypeVar, Union

from .child_mesh import MeshCall
from .event import EventMetadata
from .host_processor import HostInvokeEnding

E = TypeVar("E")


@dataclass(frozen=True)
class InboxExternalEvent(Generic[E]):
    """An external event of the machine, applied with `Engine.send_event`'s
    enqueue: it joins the external queue, and the macrostep loop the owner is
    running takes it. `metadata` ``None`` is what ``send_event(event)`` gives
    an event."""

    event: E
    metadata: Optional[EventMetadata] = None


@dataclass(frozen=True)
class InboxExternalEventByName:
    """An external event named by text, applied with
    `Engine.send_external_by_name` and its arguments of the same names:
    resolved against the document's names when it is applied, and dropped then
    if no event of the document could take it."""

    name: str
    data: Any = ""
    send_id: str = ""
    invoke_id: str = ""
    origin: str = ""
    origin_type: str = ""


@dataclass(frozen=True)
class InboxHostInvokeEnding:
    """The end of a start of a host-run invocation.

    `session_id` ``None`` ends an invocation of the machine whose inbox this is,
    applied with `Engine.complete_host_invoke` or `Engine.fail_host_invoke`; a
    session id ends one of that session of the tree, applied with the body of
    `Engine.end_host_invoke_in`. The token check those make is made when the
    input is applied, so an ending posted for a start that was cancelled or
    started again by then raises nothing, as a direct one would."""

    ending: HostInvokeEnding
    session_id: Optional[str] = None


@dataclass(frozen=True)
class InboxMeshDelivery:
    """What a Mesh endpoint has for a session, applied with
    `Engine.apply_mesh_call`: on the machine whose inbox this is when
    `session_id` is ``None``, on that session of the tree otherwise, and on none
    when the tree no longer holds it."""

    call: MeshCall
    session_id: Optional[str] = None


#: One input of an engine's inbox. Closed: the inputs that have a caller on
#: another thread.
InboxInput = Union[
    InboxExternalEvent[E],
    InboxExternalEventByName,
    InboxHostInvokeEnding,
    InboxMeshDelivery,
]

#: What a host installs with `EngineInbox.wake_with`: called with no arguments
#: on the posting thread after each post the inbox took.
InboxWaker = Callable[[], None]


class EngineInboxState(Generic[E]):
    """What an engine and every handle to its inbox share: the queue, the gate
    that opens and closes it, and the waker the host installed.

    The posting half (`post`, `wake_with`, `refused_posts`) is safe from any
    thread and is what `EngineInbox` hands out. The rest (`open`, `close`,
    `take`, `holds_posts`) is the owner's, called by the engine."""

    __slots__ = ("_lock", "_held", "_closed", "_waker", "_refused")

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._held: Deque[InboxInput[E]] = deque()
        self._closed = False
        self._waker: Optional[InboxWaker] = None
        self._refused = 0

    def post(self, posted: InboxInput[E]) -> bool:
        """Post `posted`; whether the inbox took it. Refused, and counted, once
        the inbox is closed. After a post the inbox took, the installed waker is
        called on this thread."""
        with self._lock:
            if self._closed:
                self._refused += 1
                return False
            self._held.append(posted)
            waker = self._waker
        if waker is not None:
            waker()
        return True

    def wake_with(self, waker: Optional[InboxWaker]) -> None:
        """Install `waker`; ``None`` removes it."""
        with self._lock:
            self._waker = waker

    def refused_posts(self) -> int:
        """How many posts the inbox refused since it was made."""
        with self._lock:
            return self._refused

    def open(self) -> None:
        """Take posts from now on. The owner's."""
        with self._lock:
            self._closed = False

    def close(self) -> None:
        """Refuse every post from now on. What the inbox holds when this returns
        is all it will hold until it is opened again. The owner's."""
        with self._lock:
            self._closed = True

    def take(self) -> Deque[InboxInput[E]]:
        """What the inbox holds now, oldest first, leaving it empty. What is
        posted after this returns waits for the next call. The owner's."""
        with self._lock:
            taken, self._held = self._held, deque()
        return taken

    def holds_posts(self) -> bool:
        """Whether a post was taken that the owner has not applied yet. The
        owner's."""
        with self._lock:
            return bool(self._held)


class EngineInbox(Generic[E]):
    """The handle a host posts to an engine's inbox through (`Engine.inbox`).

    Thread-safe: any thread may post, and any number at once. What is posted is
    applied by the engine's owner at its next macrostep boundary, before the
    external queue is consulted, in the order the inbox took it. Every handle of
    one engine names the same inbox."""

    __slots__ = ("_state",)

    def __init__(self, state: EngineInboxState[E]) -> None:
        self._state = state

    def post(self, posted: InboxInput[E]) -> bool:
        """Post `posted`; whether the inbox took it. The inbox takes every post
        from the moment the engine is built until its run ends at a top-level
        ``<final>`` or is stopped, and refuses every post after either until a
        run is entered again (`Engine.initialize`, `Engine.enter_at`). A post
        taken before the first run is applied at that run's first boundary; one
        still held when the run ends is applied then, as a direct call would be
        at that moment."""
        return self._state.post(posted)

    def wake_with(self, waker: Optional[InboxWaker]) -> None:
        """Install `waker`, called on the posting thread after each post the
        inbox took, so a host that drives the engine knows there is something to
        apply: its next drive applies it, and ``advance_time(0)`` is the drive
        that moves no clock. ``None`` removes it."""
        self._state.wake_with(waker)

    def refused_posts(self) -> int:
        """How many posts this inbox refused, because it was closed."""
        return self._state.refused_posts()
