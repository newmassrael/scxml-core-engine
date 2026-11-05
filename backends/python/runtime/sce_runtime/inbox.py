# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The inbox of an engine: the one part of it a thread that does not own it may
use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").

A multi-producer, single-consumer queue of what other threads have for an
engine, which the engine's owner applies at its next macrostep boundary, each
input by the owner-thread operation of the same name. What may be posted is a
closed set, each input carrying what the operation that applies it takes.

Every session of a tree has an inbox of its own, a child's as well as the
root's, and the tree keeps a directory of them by session id with the tree's
one waker (`TreeInboxes`). A post that names a session is put in that session's
inbox when it is posted (`EngineInbox.post`), not routed when it is drained.

The queue is a `collections.deque` behind one `threading.Lock`, not a
`queue.SimpleQueue`. The inbox needs two things a bare concurrent queue does not
give together: a post refused once the inbox is closed, decided under the same
lock as the append, so no post lands after a close took what the inbox held;
and a drain that takes what the inbox held when the boundary was reached in one
step, so posters that never stop cannot hold the owner there. The lock is held
for an append or a swap and nothing else, so a post never waits for a
macrostep, and the waker is called after it is released. It holds on a
free-threaded interpreter as on one with a global lock.

Unbounded (docs/adr/0084, D2): a post is refused only when the inbox is closed,
or when it names a session the tree does not hold.
"""

from __future__ import annotations

import threading
from collections import deque
from dataclasses import dataclass, replace
from typing import Any, Callable, Deque, Dict, Generic, Optional, TypeVar, Union

from .child_mesh import MeshCall
from .event import EventMetadata
from .event_origin import EventOrigin
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
    #: The reply address the host names for the event, in its own terms
    #: (docs/adr/0090, section 2); ``None`` for a host that names no party.
    origin: Optional[EventOrigin] = None


@dataclass(frozen=True)
class InboxHostInvokeEnding:
    """The end of a start of a host-run invocation.

    `session_id` ``None`` ends an invocation of the machine whose inbox it is
    posted to; a session id ends one of that session of the tree, and the post
    puts it in that session's own inbox, or is refused when the tree holds no
    such session (`EngineInbox.post`). The session applies it with
    `Engine.complete_host_invoke` or `Engine.fail_host_invoke`. The token check
    those make is made when the input is applied, so an ending posted for a
    start that was cancelled or started again by then raises nothing, as a
    direct one would."""

    ending: HostInvokeEnding
    session_id: Optional[str] = None


@dataclass(frozen=True)
class InboxMeshDelivery:
    """What a Mesh endpoint has for a session, applied with
    `Engine.apply_mesh_call`: on the machine whose inbox it is posted to when
    `session_id` is ``None``, and otherwise on that session of the tree, whose
    own inbox the post puts it in; the post is refused when the tree no longer
    holds it (`EngineInbox.post`)."""

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
#: on the posting thread after each post an inbox of the tree took.
InboxWaker = Callable[[], None]


def _named_session(posted: InboxInput[E]) -> Optional[str]:
    """The session `posted` names, ``None`` when it names none and is for the
    machine whose inbox it is posted to."""
    if isinstance(posted, (InboxHostInvokeEnding, InboxMeshDelivery)):
        return posted.session_id
    return None


class EngineInboxState(Generic[E]):
    """What a session and every handle to its inbox share: the queue, the gate
    that opens and closes it, the count of refused posts, and the tree the
    session belongs to, whose waker a post calls.

    The posting half (`post`, `tree`, `count_refusal`, `refused_posts`) is safe
    from any thread and is what `EngineInbox` uses. The rest (`join`, `open`,
    `close`, `take`, `holds_posts`) is the owner's, called by the engine."""

    __slots__ = ("_lock", "_held", "_closed", "_tree", "_refused")

    def __init__(self, tree: "TreeInboxes[E]") -> None:
        self._lock = threading.Lock()
        self._held: Deque[InboxInput[E]] = deque()
        self._closed = False
        self._tree = tree
        self._refused = 0

    def post(self, posted: InboxInput[E]) -> bool:
        """Post `posted`, an input for this session; whether the inbox took it.
        Refused once the inbox is closed. After a post the inbox took, the
        tree's waker is called on this thread."""
        with self._lock:
            if self._closed:
                return False
            self._held.append(posted)
            tree = self._tree
        tree.wake()
        return True

    def tree(self) -> "TreeInboxes[E]":
        """The inboxes of the tree this session belongs to."""
        with self._lock:
            return self._tree

    def join(self, tree: "TreeInboxes[E]") -> None:
        """Belong to `tree` from now on: its waker is the one a post calls. The
        owner's, made before the session's first macrostep in that tree."""
        with self._lock:
            self._tree = tree

    def count_refusal(self) -> None:
        """Count one post made through a handle of this inbox and refused."""
        with self._lock:
            self._refused += 1

    def refused_posts(self) -> int:
        """How many posts made through a handle of this inbox were refused since
        it was made."""
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


class TreeInboxes(Generic[E]):
    """The inbox of every session of one tree, by session id, and the tree's one
    waker (docs/adr/0084, D2 as amended at step 10): what a post naming a
    session looks the session up in.

    A machine a host builds makes it and enters itself; a parent hands it to
    each child it starts or resumes before the child runs, as it hands the
    tree's invokers (`Engine.share_tree_with`). A session is entered while its
    run takes posts and removed when its run ends, after its inbox is closed and
    what it held is applied. `find`, `wake_with` and `wake` are safe from any
    thread; `enter` and `remove` are the owner's."""

    __slots__ = ("_lock", "_sessions", "_waker")

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._sessions: Dict[str, EngineInboxState[E]] = {}
        self._waker: Optional[InboxWaker] = None

    def find(self, session_id: str) -> Optional[EngineInboxState[E]]:
        """The inbox of the session `session_id` names; ``None`` when the tree
        holds no such session: never started, or ended."""
        with self._lock:
            return self._sessions.get(session_id)

    def enter(self, session_id: str, inbox: EngineInboxState[E]) -> None:
        """Enter `inbox` as the inbox of `session_id`. The owner's."""
        with self._lock:
            self._sessions[session_id] = inbox

    def remove(self, session_id: str, inbox: EngineInboxState[E]) -> bool:
        """Remove `session_id` when `inbox` is its inbox here; whether it was.
        A session started since under the same id keeps its own. The owner's."""
        with self._lock:
            if self._sessions.get(session_id) is not inbox:
                return False
            del self._sessions[session_id]
            return True

    def wake_with(self, waker: Optional[InboxWaker]) -> None:
        """Install `waker` as the tree's; ``None`` removes it."""
        with self._lock:
            self._waker = waker

    def wake(self) -> None:
        """Call the tree's waker, if one is installed, on this thread."""
        with self._lock:
            waker = self._waker
        if waker is not None:
            waker()


class EngineInbox(Generic[E]):
    """The handle a host posts to an engine's inbox through (`Engine.inbox`).

    Thread-safe: any thread may post, and any number at once. What is posted is
    applied by the owner at the next macrostep boundary of the session it is
    for, before that session's external queue is consulted, in the order its
    inbox took it. Every handle of one engine names the same inbox."""

    __slots__ = ("_state",)

    def __init__(self, state: EngineInboxState[E]) -> None:
        self._state = state

    def post(self, posted: InboxInput[E]) -> bool:
        """Post `posted`; whether an inbox took it.

        An input that names a session (`InboxHostInvokeEnding.session_id`,
        `InboxMeshDelivery.session_id`) is put in that session's own inbox,
        found in the tree's directory now, so it is on that session's queue
        ahead of whatever the session is sent after this returns; a post naming
        a session the tree does not hold is refused. Any other input is put in
        this engine's inbox.

        An inbox takes every post from the moment its engine is built until its
        run ends at a top-level ``<final>`` or is stopped, and refuses every
        post after either until a run is entered again (`Engine.initialize`,
        `Engine.enter_at`). A post taken before the first run is applied at that
        run's first boundary; one still held when the run ends is applied then,
        as a direct call would be at that moment. A refused post is counted
        (`refused_posts`)."""
        target: Optional[EngineInboxState[E]] = self._state
        session_id = _named_session(posted)
        if session_id is not None:
            target = self._state.tree().find(session_id)
            # The inbox it reaches is the named session's own, so it holds the
            # input as one for that session.
            posted = replace(posted, session_id=None)
        if target is not None and target.post(posted):
            return True
        self._state.count_refusal()
        return False

    def wake_with(self, waker: Optional[InboxWaker]) -> None:
        """Install `waker`, the tree's one waker, called on the posting thread
        after each post an inbox of the tree took, a child session's included,
        so a host that drives the engine knows there is something to apply: its
        next drive applies it, and ``advance_time(0)`` is the drive that moves
        no clock. ``None`` removes it."""
        self._state.tree().wake_with(waker)

    def refused_posts(self) -> int:
        """How many posts made through this inbox were refused: because the
        inbox they reached was closed, or because they named a session the tree
        does not hold."""
        return self._state.refused_posts()
