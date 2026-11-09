# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
received messages and the snapshot of its head (ES 201 873-4 §8.3.3).

Cross-language siblings: ``sce_rust_runtime::test::Port`` (Rust),
``SCE::Test::Port`` (C++), ``com.sce.runtime.test.Port`` (Kotlin),
``testprofile.Port`` (Go).
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections import deque
from dataclasses import dataclass, field
from typing import Any, Callable, Deque, Optional, TypeVar

from .canonical import CanonicalValue

V = TypeVar("V")

#: An instant of the logical clock ``NOW``: microseconds since the test case
#: started (docs/SCE_TEST_PROFILE.md §3.3).
Instant = int

#: A length of logical time, in microseconds.
Duration = int


class TestValue(ABC):
    """A value a message carries, its type's name and its canonical form in the
    test log (docs/SCE_TEST_PROFILE.md §12.5).

    The value is read by its type (``receives``); its canonical form is written
    by the types its document declares (``canonical``), which its Python type
    alone does not say, since a byte string and a list of ``uint8`` are one type
    in some languages and two in others. ``message_value`` makes one of a value,
    its type's name and the writer of its form, which is what a generated send
    and a system adapter give; a host may subclass it with a type of its own.

    Cross-language siblings: ``sce_rust_runtime::test::TestValue`` (Rust),
    ``com.sce.runtime.test.TestValue`` (Kotlin).
    """

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    @property
    @abstractmethod
    def value(self) -> Any:
        """The value, a record of the type the unit of its ``event-schema``
        declares."""

    @property
    @abstractmethod
    def type_name(self) -> str:
        """The name of the value's record as the types module of the suite's
        TTCN-3 export names it (docs/SCE_TEST_PROFILE.md §9.5): the unit name of
        its ``event-schema`` document. A value in TCI always carries its type,
        and so does a message of the test log."""

    @abstractmethod
    def canonical(self) -> CanonicalValue:
        """The value in the canonical form of the test log: built from texts,
        truth values, lists and dicts alone (``canonical``), so that the JSON
        Canonicalization Scheme writes it as the same bytes on every runtime. It
        runs while the run's state is held, so it reads the value and nothing
        else."""


@dataclass(frozen=True)
class _WithCanonical(TestValue):
    """A value, its type's name and the writer of its canonical form: what
    ``message_value`` makes."""

    held: Any
    held_type_name: str
    # The writer is shown by its type's name alone, which says what it writes.
    write: Callable[[Any], CanonicalValue] = field(repr=False)

    @property
    def value(self) -> Any:
        return self.held

    @property
    def type_name(self) -> str:
        return self.held_type_name

    def canonical(self) -> CanonicalValue:
        return self.write(self.held)


def message_value(value: V, type_name: str, canonical: Callable[[V], CanonicalValue]) -> TestValue:
    """``value`` as a message's value, of the record ``type_name``, whose
    canonical form ``canonical`` writes: the ``TYPE`` and the ``canonical`` of
    the unit of a template over its record, as a generated send gives them
    (``<template>.TYPE``, ``<template>.canonical``)."""
    return _WithCanonical(value, type_name, canonical)


@dataclass(frozen=True)
class Message:
    """A message on a port: the event name of its record and the record's value,
    with its type's name and the writer of its canonical form.

    The value is of the record type the unit of its ``event-schema`` declares; a
    template's ``matches`` reads it as that type (``receives``). Matching is on
    the value and never on an encoding (ADR 0091 C2), and so is the test log
    (``canonical_value``). A value is never changed once sent, so a message is
    shared freely: every holder reads the same record.
    """

    #: The event name of the message's record.
    event: str
    #: The record's value, with its type's name and the writer of its canonical
    #: form.
    test_value: TestValue

    @classmethod
    def of(
        cls,
        event: str,
        value: V,
        type_name: str,
        canonical: Callable[[V], CanonicalValue],
    ) -> "Message":
        """A message of the record ``event`` whose value is ``value``, of the
        record type ``type_name``, whose canonical form ``canonical`` writes
        (``message_value``)."""
        return cls(event, message_value(value, type_name, canonical))

    @property
    def value(self) -> Any:
        """The record's value, read by its type."""
        return self.test_value.value

    @property
    def type_name(self) -> str:
        """The name of the value's record type in the test log
        (``TestValue.type_name``)."""
        return self.test_value.type_name

    def canonical_value(self) -> CanonicalValue:
        """The value in the canonical form of the test log."""
        return self.test_value.canonical()


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
