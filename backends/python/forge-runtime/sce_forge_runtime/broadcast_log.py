# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# §synth-5-S: the runtime half of the `broadcast-log` kind for Python. The ledger
# reads comments and not docstrings, so the section is cited here as well as in
# the module's docstring below.

"""The runtime half of ``sce:kind="broadcast-log"`` (SCE Protocol-Synthesis RFC
§synth-5-S, ADR 0062) for Python: a bounded log that one publisher fills and any number of readers read, each at
its own pace, in which a full log forgets its oldest entry.

The contract is the sequential log of ADR 0062 section 2, and it is the same
contract the Rust row keeps (``backends/rust/forge-runtime/src/broadcast_log.rs``):
``h`` entries have been published, only the last ``retention`` are retained,
and a reader owns a cursor. :meth:`BroadcastLog.read` returns the entry at the
cursor (:class:`Item`), or says nothing newer is published (:class:`Empty`), or
says how many entries the reader missed and moves it to the oldest retained
(:class:`Lagged`). Publishing never refuses an entry.

Python has no compare-and-swap primitive and no fences, so the algorithm here
is not the stamped ring and the seqlock read of ADR 0062 section 4. It is one
ring under one lock, as the queue row is, and the progress it gives is
``blocking`` (:data:`PROGRESS`): a participant that is stopped holding the lock
stops the others. The generator refuses a document that declares more for this
backend (``broadcast-log/progress-unreachable-on-backend``) rather than lowering
it to a log that gives less. Because every operation is atomic under the lock,
the sequence numbers, the window and every result are exactly what the
sequential specification gives, with no torn copy to detect. The ring still
holds ``retention + 1`` slots and a read still promises only ``retention``
entries, as the Rust row does, so that a log is built from the same constructor
argument on every backend and the contract tests can compare the rows.

What differs from the languages with constant generics:

* The entry's width ``words`` and the slot count ``slots`` are arguments of the
  constructor, and the retention is ``slots - 1``. The storage is allocated by
  the constructor and never again: a publish replaces one slot's words.
* An entry is a list of ``words`` integers, each in ``[0, 2**64)``, the word
  view of the element's wire form (ADR 0049/0058) that every backend shares.
* A handle holds its place until :meth:`Publisher.release` (or the ``with``
  block that holds it ends). There is one publisher place; a reader owns no
  place at all, so the log never learns how many readers exist.
"""

from __future__ import annotations

import dataclasses
import threading
from typing import Optional, Union

from .queue import Progress

PROGRESS = Progress.BLOCKING
"""What every operation of this row guarantees: one lock under every operation."""

HORIZON = (2**64 - 3) // 2
"""The most entries a log can publish: the Rust row's stamps are ``2n + 2``,
which must fit a ``u64``. The row keeps the limit so that all rows stop at the
same sequence number; :meth:`Publisher.publish` raises past it."""

_WORD_LIMIT = 2**64


@dataclasses.dataclass(frozen=True)
class Item:
    """The entry ``seq`` was copied into the caller's buffer, and the reader now
    holds ``seq + 1``."""

    seq: int


@dataclasses.dataclass(frozen=True)
class Empty:
    """The reader is at the head: nothing newer is published. Its cursor is
    unchanged."""


@dataclasses.dataclass(frozen=True)
class Lagged:
    """The reader had fallen behind the window and missed ``skipped >= 1``
    entries. It now holds the oldest retained entry, so its next read returns
    that. The caller's buffer is left as it was."""

    skipped: int


Read = Union[Item, Empty, Lagged]
"""What a read found."""


class Reader:
    """A reader's position, owned by the reader and shared with nothing.

    Opening a reader costs the log nothing: the publisher does not know how many
    readers exist. A reader is used by one thread at a time. Open one with
    :meth:`BroadcastLog.reader_oldest`, :meth:`BroadcastLog.reader_head` or
    :meth:`BroadcastLog.reader_at`.
    """

    __slots__ = ("_log", "_cursor", "_lost")

    def __init__(self, log: "BroadcastLog", cursor: int) -> None:
        self._log = log
        self._cursor = cursor
        self._lost = 0

    @property
    def cursor(self) -> int:
        """The sequence number this reader reads next."""
        return self._cursor

    @property
    def lost(self) -> int:
        """The entries this reader has been told it missed, summed over its reads."""
        return self._lost


class BroadcastLog:
    """A bounded log of entries of ``words`` 64-bit words, kept in ``slots``
    slots and retaining the last ``slots - 1`` entries: one ring under one lock.

    The constructor raises :class:`ValueError` for ``words`` below one or
    ``slots`` below two (a log retains at least one entry).
    """

    def __init__(self, words: int, slots: int) -> None:
        if words < 1:
            raise ValueError("an entry of a broadcast log has at least one word")
        if slots < 2:
            raise ValueError("a log retains at least one entry, so it has at least two slots")
        self._words = words
        self._slots = slots
        self._ring: list[tuple[int, ...]] = [(0,) * words for _ in range(slots)]
        self._head = 0
        self._publisher_taken = False
        self._lock = threading.Lock()

    @property
    def words(self) -> int:
        """The words of one entry."""
        return self._words

    @property
    def slots(self) -> int:
        """The slots of the ring."""
        return self._slots

    @property
    def retention(self) -> int:
        """The entries a reader may rely on: ``slots - 1``."""
        return self._slots - 1

    def head(self) -> int:
        """The number of entries published so far."""
        with self._lock:
            return self._head

    def publisher(self) -> Optional["Publisher"]:
        """The publisher, or ``None`` while another exists. Releasing it frees the
        place, and its successor continues the sequence."""
        with self._lock:
            if self._publisher_taken:
                return None
            self._publisher_taken = True
        return Publisher(self)

    def reader_oldest(self) -> Reader:
        """A reader at the oldest retained entry."""
        with self._lock:
            return Reader(self, max(0, self._head - self.retention))

    def reader_head(self) -> Reader:
        """A reader at the head: it sees only what is published after this call."""
        with self._lock:
            return Reader(self, self._head)

    def reader_at(self, seq: int) -> Reader:
        """A reader at the sequence number ``seq``, or at the head when ``seq`` is
        past it. Raises :class:`ValueError` for a negative ``seq``."""
        if seq < 0:
            raise ValueError("a sequence number is not negative")
        with self._lock:
            return Reader(self, min(seq, self._head))

    def read(self, reader: Reader, out: list[int]) -> Read:
        """Reads the entry at ``reader``'s cursor into ``out``, which must be a
        list of exactly :attr:`words` integers; ``out`` is replaced in place on
        :class:`Item` and left as it was otherwise. Blocking: the call waits for
        the lock, and holds it for a number of steps that depends on the entry's
        width and on nothing the other side does.

        Raises :class:`ValueError` for a reader of another log or a buffer of the
        wrong width.
        """
        if reader._log is not self:
            raise ValueError("the reader belongs to another log")
        if len(out) != self._words:
            raise ValueError(f"the buffer holds {len(out)} words and an entry has {self._words}")
        with self._lock:
            r = reader._cursor
            h = self._head
            if r >= h:
                return Empty()
            if h - r > self.retention:
                return self._lagged(reader, h)
            out[:] = self._ring[r % self._slots]
            reader._cursor = r + 1
            return Item(r)

    def _lagged(self, reader: Reader, h: int) -> Lagged:
        """Moves ``reader`` to the oldest entry retained when the head is ``h``
        and says how far that was. The caller holds the lock."""
        oldest = max(0, h - self.retention)
        skipped = oldest - reader._cursor
        reader._cursor = oldest
        reader._lost += skipped
        return Lagged(skipped)

    def _publish(self, entry: list[int]) -> int:
        if len(entry) != self._words:
            raise ValueError(f"the entry holds {len(entry)} words and the log's entries have {self._words}")
        words = tuple(entry)
        for word in words:
            if not 0 <= word < _WORD_LIMIT:
                raise ValueError(f"the word {word} is not a 64-bit unsigned integer")
        with self._lock:
            n = self._head
            if n >= HORIZON:
                raise OverflowError("the log has published its horizon of entries")
            self._ring[n % self._slots] = words
            self._head = n + 1
            return n

    def _release_publisher(self) -> None:
        with self._lock:
            self._publisher_taken = False


class Publisher:
    """The publishing side of a :class:`BroadcastLog`. There is one at a time; it
    holds the place until :meth:`release`."""

    def __init__(self, log: BroadcastLog) -> None:
        self._log: Optional[BroadcastLog] = log

    def _held(self) -> BroadcastLog:
        if self._log is None:
            raise RuntimeError("the publisher was released")
        return self._log

    def publish(self, entry: list[int]) -> int:
        """Publishes ``entry`` and returns the sequence number it took. Never
        refuses the entry; the oldest retained one is forgotten when the log is
        full. Blocking: the call waits for the lock.

        Raises :class:`ValueError` for an entry of the wrong width or a word
        outside ``[0, 2**64)``, and :class:`OverflowError` past :data:`HORIZON`
        entries, which is not reached in practice.
        """
        return self._held()._publish(entry)

    def release(self) -> None:
        """Gives the publisher place back. The handle is not usable afterwards."""
        if self._log is not None:
            self._log._release_publisher()
            self._log = None

    def __enter__(self) -> "Publisher":
        return self

    def __exit__(self, *_exc: object) -> None:
        self.release()
