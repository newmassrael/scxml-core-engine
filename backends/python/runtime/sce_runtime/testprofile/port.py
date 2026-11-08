# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
received messages and the snapshot of its head (ES 201 873-4 §8.3.3).

Cross-language siblings: ``sce_rust_runtime::test::Port`` (Rust),
``SCE::Test::Port`` (C++), ``com.sce.runtime.test.Port`` (Kotlin),
``testprofile.Port`` (Go).
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass
from typing import Any, Deque, Optional

#: An instant of the logical clock ``NOW``: microseconds since the test case
#: started (docs/SCE_TEST_PROFILE.md §3.3).
Instant = int

#: A length of logical time, in microseconds.
Duration = int


@dataclass(frozen=True)
class Message:
    """A message on a port: the event name of its record and the record's value.

    The value is the record type the statechart generator writes for the
    ``event-schema`` that declares it; a template's ``matches`` reads it as that
    type (``receives``). Matching is on the value and never on an encoding
    (ADR 0091 C2). A value is never changed once sent, so a message is shared
    freely: every holder reads the same record.
    """

    #: The event name of the message's record.
    event: str
    #: The record's value.
    value: Any


class Port:
    """One component port: its queue and its snapshot (-4 §8.3.3).

    Only the component that owns the port removes from it, so between the
    snapshot and the removal of a branch the head is the message the snapshot
    saw; what others append goes to the back. The snapshot records only whether
    the queue held a message: a message that arrives in an empty queue after the
    snapshot is not in it. A new port is empty, and its snapshot saw nothing.
    """

    __slots__ = ("_queue", "_snapshot_holds")

    def __init__(self) -> None:
        self._queue: Deque[Message] = deque()
        self._snapshot_holds = False

    def enqueue(self, message: Message) -> None:
        """Append a message a connected port sent, or one from the SUT on a
        mapped port, in arrival order (-4 §8.3.3.1)."""
        self._queue.append(message)

    def take_snapshot(self) -> None:
        """Take ``SNAP-VALUE``, the head of the queue, in a round's snapshot
        step (docs/SCE_TEST_PROFILE.md §3.2 step 3)."""
        self._snapshot_holds = len(self._queue) > 0

    def snapshot(self) -> Optional[Message]:
        """The message the last snapshot saw at the head, or ``None`` when it
        saw none."""
        if not self._snapshot_holds or not self._queue:
            return None
        return self._queue[0]

    def remove_head(self) -> Message:
        """``remove-from-port`` of a receive branch that was taken on the
        snapshot (-4 §9.37.1, P4 step 1).

        Raises ``RuntimeError`` when the queue is empty. The caller holds that
        the snapshot saw a message, which a branch's guard established, so an
        empty queue here is a machine that took a receive branch its guard did
        not allow.
        """
        if not self._queue:
            raise RuntimeError("a receive branch is taken only on a snapshot that saw a message")
        self._snapshot_holds = False
        return self._queue.popleft()

    def __len__(self) -> int:
        """How many messages the queue holds."""
        return len(self._queue)

    def is_empty(self) -> bool:
        """Whether the queue holds no message."""
        return not self._queue

    def __repr__(self) -> str:
        return f"Port(len={len(self._queue)}, snapshot_holds={self._snapshot_holds})"
