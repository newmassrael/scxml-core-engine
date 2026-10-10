# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# §synth-5-P: the runtime half of the `queue` kind for Python. The ledger reads
# comments and not docstrings, so the section is cited here as well as in the
# module's docstring below.

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

import collections
import enum
import threading
from typing import Generic, Optional, Protocol, TypeVar

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


class Progress(enum.IntEnum):
    """What an operation guarantees about how long it takes whatever the other
    participants are doing. Ordered by strength, so "at least lock-free" is
    ``>= Progress.LOCK_FREE``."""

    BLOCKING = 0
    """May wait for another participant."""
    LOCK_FREE = 1
    """Some participant's operation completes in a bounded number of steps."""
    WAIT_FREE = 2
    """Every operation completes in a bounded number of steps."""


class SegmentAllocator(Protocol):
    """The allocator a ``segmented`` queue is injected with (SCE Protocol-Synthesis
    RFC §synth-5-P, *Storage modes*): the one thing that bounds how much such a
    queue holds.

    A collected runtime has no blocks to hand out and take back: the segment is a
    list, and the collector frees it once nothing reaches it. What the allocator
    keeps is the decision. :meth:`allocate` says whether the queue may have one
    more segment; :meth:`deallocate` says that a segment has left the queue. A
    pool, an arena with a byte budget or a counter that stands in for a memory
    ceiling is an allocator here as it is in the languages that address memory.

    It states the progress its own operations give as :attr:`progress`, because
    the progress check of a queue document runs at build time and the allocator
    arrives only at run time (RFC §synth-5-P, *Segment allocator progress is
    declared in the document*). The queue refuses at construction an allocator
    that gives less than the document declared. The queue calls it under its own
    lock, so an implementation needs no synchronization of its own for that.
    """

    @property
    def progress(self) -> Progress:
        """The progress :meth:`allocate` and :meth:`deallocate` give."""
        ...

    def allocate(self) -> bool:
        """Whether one more segment may be had. A refusal is the queue's
        :attr:`PushStatus.OUT_OF_MEMORY`."""
        ...

    def deallocate(self) -> None:
        """Takes a segment back into account: it has left the queue."""
        ...


class SegmentedQueue(Generic[T]):
    """The ``segmented`` row for every cardinality: a list of segments of
    ``segment`` elements under one lock, bounded only by the allocator it is given
    (RFC §synth-5-P, Backends).

    The producer fills the newest segment and, when it is full, asks the
    allocator for another and links it behind; the consumer reads the oldest
    segment and, when it has read all of it, hands the segment back to the
    allocator. Python has no compare-and-swap primitive, so every operation runs
    under the lock and gives ``blocking`` whatever the table would give, and there
    is no reclamation domain: the collector frees a segment. A push the allocator
    refuses reports :attr:`PushStatus.OUT_OF_MEMORY` and leaves the element with
    the caller.

    ``producer_places`` and ``consumer_places`` are how many handles of each side
    may be held at once. Build one with :meth:`create`.
    """

    def __init__(self, segment: int, allocator: SegmentAllocator, producer_places: int, consumer_places: int) -> None:
        self._segment = segment
        self._allocator = allocator
        self._lock = threading.Lock()
        self._segments: collections.deque[list[Optional[T]]] = collections.deque()
        # The filled count of the newest segment and the read count of the oldest.
        self._written = 0
        self._read = 0
        self._producer_places = producer_places
        self._consumer_places = consumer_places
        self._producers_alive = 0
        self._consumers_alive = 0

    @classmethod
    def create(
        cls,
        segment: int,
        allocator: SegmentAllocator,
        required: Progress,
        producer_places: int,
        consumer_places: int,
    ) -> Optional["SegmentedQueue[T]"]:
        """An empty queue over ``allocator``, or ``None`` when the allocator will
        not give the first segment. Raises :class:`ValueError` for a segment or a
        place count below one, and for an allocator whose progress is below
        ``required``, the progress the document declared for it: a queue that would
        claim more than its allocator gives does not build."""
        if segment < 1:
            raise ValueError("a segment holds at least one element")
        if producer_places < 1 or consumer_places < 1:
            raise ValueError("a side of a queue must have at least one place")
        if allocator.progress < required:
            raise ValueError(
                f"the allocator gives {allocator.progress.name}, less than the {required.name} the document declared for it"
            )
        queue: SegmentedQueue[T] = cls(segment, allocator, producer_places, consumer_places)
        if not allocator.allocate():
            return None
        queue._segments.append([None] * segment)
        return queue

    @property
    def segment(self) -> int:
        """The number of elements one segment holds."""
        return self._segment

    def producer(self) -> Optional["SegmentedProducer[T]"]:
        """A producer handle, or ``None`` when every producer place is taken."""
        with self._lock:
            if self._producers_alive >= self._producer_places:
                return None
            self._producers_alive += 1
        return SegmentedProducer(self)

    def consumer(self) -> Optional["SegmentedConsumer[T]"]:
        """A consumer handle, or ``None`` when every consumer place is taken."""
        with self._lock:
            if self._consumers_alive >= self._consumer_places:
                return None
            self._consumers_alive += 1
        return SegmentedConsumer(self)

    def _push(self, value: T) -> PushStatus:
        with self._lock:
            if self._written == self._segment:
                if not self._allocator.allocate():
                    return PushStatus.OUT_OF_MEMORY
                self._segments.append([None] * self._segment)
                self._written = 0
            self._segments[-1][self._written] = value
            self._written += 1
            return PushStatus.OK

    def _pop(self) -> Optional[T]:
        with self._lock:
            if self._read == self._segment:
                # Every element of the oldest segment is taken. It is given back
                # once a successor exists; the newest segment is never given up.
                if len(self._segments) == 1:
                    return None
                self._segments.popleft()
                self._allocator.deallocate()
                self._read = 0
            if len(self._segments) == 1 and self._read == self._written:
                return None
            head = self._segments[0]
            value = head[self._read]
            # The slot is cleared so the queue keeps no reference to an element
            # that has left it.
            head[self._read] = None
            self._read += 1
            return value

    def _release_producer(self) -> None:
        with self._lock:
            self._producers_alive -= 1

    def _release_consumer(self) -> None:
        with self._lock:
            self._consumers_alive -= 1


class SegmentedProducer(Generic[T]):
    """A producing side of a :class:`SegmentedQueue`. It holds one producer place
    until :meth:`release`."""

    def __init__(self, queue: SegmentedQueue[T]) -> None:
        self._queue: Optional[SegmentedQueue[T]] = queue

    def _held(self) -> SegmentedQueue[T]:
        if self._queue is None:
            raise RuntimeError("the producer was released")
        return self._queue

    def try_clone(self) -> Optional["SegmentedProducer[T]"]:
        """Another producer of the same queue, or ``None`` when every place is taken."""
        return self._held().producer()

    def try_push(self, value: T) -> PushStatus:
        """Takes ``value``, or reports :attr:`PushStatus.OUT_OF_MEMORY` when the
        newest segment is full and the allocator will not give another; the
        element is the caller's either way. Blocking: the call waits for the lock."""
        return self._held()._push(value)

    @property
    def segment(self) -> int:
        """The elements one segment of the queue this side fills holds."""
        return self._held().segment

    def release(self) -> None:
        """Gives the producer place back. The handle is not usable afterwards."""
        if self._queue is not None:
            self._queue._release_producer()
            self._queue = None

    def __enter__(self) -> "SegmentedProducer[T]":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.release()


class SegmentedConsumer(Generic[T]):
    """A consuming side of a :class:`SegmentedQueue`. It holds one consumer place
    until :meth:`release`."""

    def __init__(self, queue: SegmentedQueue[T]) -> None:
        self._queue: Optional[SegmentedQueue[T]] = queue

    def _held(self) -> SegmentedQueue[T]:
        if self._queue is None:
            raise RuntimeError("the consumer was released")
        return self._queue

    def try_clone(self) -> Optional["SegmentedConsumer[T]"]:
        """Another consumer of the same queue, or ``None`` when every place is taken."""
        return self._held().consumer()

    def try_pop(self) -> Optional[T]:
        """Takes the oldest element, or returns ``None`` when the queue is empty.
        Blocking: the call waits for the lock."""
        return self._held()._pop()

    @property
    def segment(self) -> int:
        """The elements one segment of the queue this side drains holds."""
        return self._held().segment

    def release(self) -> None:
        """Gives the consumer place back. The handle is not usable afterwards."""
        if self._queue is not None:
            self._queue._release_consumer()
            self._queue = None

    def __enter__(self) -> "SegmentedConsumer[T]":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.release()
