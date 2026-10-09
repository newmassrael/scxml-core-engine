# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The runtime half of ``sce:kind="queue"`` (SCE Protocol-Synthesis RFC
§synth-5-P) for Python: a queue that hands elements from one thread to another.

The document states a contract and names no algorithm; each backend's runtime
carries the algorithm its platform can build. Python has no compare-and-swap
primitive, so the algorithm here is one ring under one lock and the progress it
gives is ``blocking`` for every storage mode and cardinality (RFC §synth-5-P,
Backends): a participant that is stopped holding the lock stops the others. The
generator refuses a document that declares ``wait-free`` or ``lock-free`` for
this backend (``queue/progress-unreachable-on-backend``) rather than lowering it
to a queue that gives less.

The contract every backend's queue keeps is kept here: a linearizable FIFO in
which the push of an element happens-before the pop that returns it, never
holding more than the capacity declared and holding exactly that when nothing is
running. Because every operation runs under the lock, a push is never refused
while slots are free, which is stronger than the SCQ row the other backends use
for the same cardinality.

What differs from the languages with destructors and constant generics:

* The capacity is an argument of the constructor. The storage is one list,
  allocated by the constructor and never again: no operation allocates.
* A popped element's slot is cleared, so a queue holds no reference to an
  element that has left it.
* A handle holds one place of its side until :meth:`release` is called (or the
  ``with`` block that holds it ends). The number of places is one for a ``one``
  side and the document's participants for a ``many`` side.
"""

from __future__ import annotations

import enum
import threading
from typing import Generic, Optional, TypeVar

T = TypeVar("T")


class PushStatus(enum.Enum):
    """Whether a push took its element.

    ``OUT_OF_MEMORY`` is the ``segmented`` storage mode's, reported when its
    injected allocator refuses a segment; a ``bounded`` queue only ever reports
    ``FULL``.
    """

    OK = "ok"
    """The element is in the queue."""
    FULL = "full"
    """The queue held its capacity. The element was not taken."""
    OUT_OF_MEMORY = "out-of-memory"
    """A segmented queue's allocator refused a segment."""


class Queue(Generic[T]):
    """A bounded queue of exactly ``capacity`` elements: one ring under one lock.

    ``producer_places`` and ``consumer_places`` are how many handles of each
    side may be held at once. The constructor raises :class:`ValueError` for a
    capacity or a place count below one.
    """

    def __init__(self, capacity: int, producer_places: int, consumer_places: int) -> None:
        if capacity < 1:
            raise ValueError("a queue's capacity must be at least one element")
        if producer_places < 1 or consumer_places < 1:
            raise ValueError("a side of a queue must have at least one place")
        self._capacity = capacity
        self._slots: list[Optional[T]] = [None] * capacity
        self._head = 0
        self._count = 0
        self._lock = threading.Lock()
        self._producer_places = producer_places
        self._consumer_places = consumer_places
        self._producers_alive = 0
        self._consumers_alive = 0

    @property
    def capacity(self) -> int:
        """The number of elements the queue holds, exactly."""
        return self._capacity

    def producer(self) -> Optional["Producer[T]"]:
        """A producer handle, or ``None`` when every producer place is taken."""
        with self._lock:
            if self._producers_alive >= self._producer_places:
                return None
            self._producers_alive += 1
        return Producer(self)

    def consumer(self) -> Optional["Consumer[T]"]:
        """A consumer handle, or ``None`` when every consumer place is taken."""
        with self._lock:
            if self._consumers_alive >= self._consumer_places:
                return None
            self._consumers_alive += 1
        return Consumer(self)

    def _push(self, value: T) -> PushStatus:
        with self._lock:
            if self._count == self._capacity:
                return PushStatus.FULL
            self._slots[(self._head + self._count) % self._capacity] = value
            self._count += 1
            return PushStatus.OK

    def _pop(self) -> Optional[T]:
        with self._lock:
            if self._count == 0:
                return None
            value = self._slots[self._head]
            # The slot is cleared so the queue keeps no reference to an element
            # that has left it.
            self._slots[self._head] = None
            self._head = (self._head + 1) % self._capacity
            self._count -= 1
            return value

    def _release_producer(self) -> None:
        with self._lock:
            self._producers_alive -= 1

    def _release_consumer(self) -> None:
        with self._lock:
            self._consumers_alive -= 1


class Producer(Generic[T]):
    """A producing side of a :class:`Queue`. It holds one producer place until
    :meth:`release`."""

    def __init__(self, queue: Queue[T]) -> None:
        self._queue: Optional[Queue[T]] = queue

    def _held(self) -> Queue[T]:
        if self._queue is None:
            raise RuntimeError("the producer was released")
        return self._queue

    def try_clone(self) -> Optional["Producer[T]"]:
        """Another producer of the same queue, or ``None`` when every place is taken."""
        return self._held().producer()

    def try_push(self, value: T) -> PushStatus:
        """Takes ``value``, or reports :attr:`PushStatus.FULL` when the queue
        holds its capacity. Blocking: the call waits for the lock."""
        return self._held()._push(value)

    @property
    def capacity(self) -> int:
        """The capacity of the queue this side fills."""
        return self._held().capacity

    def release(self) -> None:
        """Gives the producer place back. The handle is not usable afterwards."""
        if self._queue is not None:
            self._queue._release_producer()
            self._queue = None

    def __enter__(self) -> "Producer[T]":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.release()


class Consumer(Generic[T]):
    """A consuming side of a :class:`Queue`. It holds one consumer place until
    :meth:`release`."""

    def __init__(self, queue: Queue[T]) -> None:
        self._queue: Optional[Queue[T]] = queue

    def _held(self) -> Queue[T]:
        if self._queue is None:
            raise RuntimeError("the consumer was released")
        return self._queue

    def try_clone(self) -> Optional["Consumer[T]"]:
        """Another consumer of the same queue, or ``None`` when every place is taken."""
        return self._held().consumer()

    def try_pop(self) -> Optional[T]:
        """Takes the oldest element, or returns ``None`` when the queue is
        empty. Blocking: the call waits for the lock."""
        return self._held()._pop()

    @property
    def capacity(self) -> int:
        """The capacity of the queue this side drains."""
        return self._held().capacity

    def release(self) -> None:
        """Gives the consumer place back. The handle is not usable afterwards."""
        if self._queue is not None:
            self._queue._release_consumer()
            self._queue = None

    def __enter__(self) -> "Consumer[T]":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.release()
