# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""Observer building blocks: hysteresis state, domain-tagged events, event
queue. See SCE_FORGE.md Section 4.11.
"""

from typing import Generic, Iterator, TypeVar

# Tag is normally an Enum class, but any hashable type with value semantics
# works. Generated observer code uses an Enum subclass per domain.
Tag = TypeVar("Tag")


class EventDomain(Generic[Tag]):
    """Marker base class for an event domain. Each generated observer is
    parameterized over a domain that lists the events valid in that domain.
    Different domains produce incompatible queue/event types — this is the
    type-safety mechanism for cross-file event composition.

    Subclasses do not need to override anything; the class itself acts as the
    domain identifier and `Tag` is the enum stored in events of that domain.
    """


class ThresholdState:
    """Models a 1-bit hysteresis state machine. The generated `update()` loop
    calls `enter_if(high_condition)` and `leave_if(low_condition)`; both
    return `True` exactly when a transition actually occurred, so the
    generated code can push the corresponding event without re-checking state.
    """

    def __init__(self) -> None:
        self._active = False

    def enter_if(self, condition: bool) -> bool:
        if not self._active and condition:
            self._active = True
            return True
        return False

    def leave_if(self, condition: bool) -> bool:
        if self._active and condition:
            self._active = False
            return True
        return False

    @property
    def active(self) -> bool:
        return self._active

    def reset(self) -> None:
        self._active = False


class EventQueue(Generic[Tag]):
    """FIFO of domain-tagged events. Returned by value from observer
    `update()` methods. Backed by a list — Python has no embedded heap
    constraint, so a list is the natural data structure here. The cross-
    language behavioural contract (push, len, iteration order, clear) matches
    the C++ and Rust implementations exactly.
    """

    def __init__(self) -> None:
        self._buffer: list[Tag] = []

    def push(self, tag: Tag) -> bool:
        self._buffer.append(tag)
        return True

    def __len__(self) -> int:
        return len(self._buffer)

    def __iter__(self) -> Iterator[Tag]:
        return iter(self._buffer)

    def __getitem__(self, index: int) -> Tag:
        return self._buffer[index]

    def is_empty(self) -> bool:
        return not self._buffer

    def clear(self) -> None:
        self._buffer.clear()

    def as_list(self) -> list[Tag]:
        """Return a defensive copy of the underlying list."""
        return list(self._buffer)
