# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Lists refined by the kind of entry a chain's gate names (docs/adr/0065 §6 R6).

A field gated on ``chain.has(k)`` is on the wire exactly when an entry of the
kind ``k`` is in the chain. The gates on one chain's entries, and the flags
joined with them, decide its fields by their values together, so a value
holds one arm for each assignment of them, and the arm says of each kind
whether the chain has an entry of it. A value made of these classes says which
arm it is by the class it holds, so the fields the gates decide cannot
disagree with the chain:

- :class:`Chain` is the chain whole with an entry of each kind of ``HOLDS``
  and of none of ``LACKS``: the arm's, for entries with content, which the
  value holds where they stand.
- :class:`Placed` is the other entries and the place of the first entry of
  the kind ``KEY`` among them, where the one gate on a bare entry holds: the
  entry's identifier is all it is, so the codec makes it (``KEY.named()``) and
  the value holds its place. The place is part of the value, since the
  protocols these chains carry do not order their entries: a frame is written
  back with its entries where they came.

A kind is a class of the codec that holds the chain, with ``matches(entry)``,
whether an entry is of it, and for a bare entry ``named()``, the entry of it.
Each is made only by a constructor that checks it once, and cannot change
after it is made, so it stays what it was checked to be. Mirrors
``membership`` of the Rust runtime and ``SCE::Forge::Chain`` and
``SCE::Forge::Placed`` of the C++ runtime.
"""

from __future__ import annotations

from typing import ClassVar, Iterable, Iterator, Optional, Tuple

from .bounded import BoundedTuple
from .codec import LengthOutOfRange, PresentIfMismatch, TooManyElements


class Chain(BoundedTuple):
    """A chain of entries within its bound (``LEAST`` to ``MOST``) that holds
    an entry of each kind of ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065
    §6 R6b): the chain one arm of the gates on its entries holds, which says of
    each kind those gates ask for whether the chain has one. A subclass names
    the bound and the kinds of one arm's chain by its class attributes.

    A tuple of its entries where they stand, so the first entry of a kind is
    where the chain has it and nothing apart from the entries says so. Raises
    :class:`PresentIfMismatch`, naming the class, for entries that lack a kind
    of ``HOLDS`` or hold one of ``LACKS``, and what :class:`BoundedTuple`
    raises for entries outside the bound."""

    HOLDS: ClassVar[Tuple[type, ...]] = ()
    LACKS: ClassVar[Tuple[type, ...]] = ()

    def __new__(cls, entries: Iterable[object] = ()) -> "Chain":
        held = super().__new__(cls, entries)
        for kind in cls.HOLDS:
            if not any(kind.matches(entry) for entry in held):
                raise PresentIfMismatch(cls.__name__)
        for kind in cls.LACKS:
            if any(kind.matches(entry) for entry in held):
                raise PresentIfMismatch(cls.__name__)
        return held


class Placed:
    """A chain within its bound (``LEAST`` to ``MOST`` entries, the named one
    among them) that holds an entry of the kind ``KEY``: the others, and the
    place of the first entry of the kind among them, no entry before the place
    being of the kind (docs/adr/0065 §6 R6a). A subclass names the bound and
    the kind of one chain by its class attributes.

    It reads as the chain whole: its length counts the named entry, and its
    entries in order are the others with ``KEY.named()`` at the place, which
    is what the encode writes. Made only by its constructor, which raises
    :class:`TooManyElements` or :class:`LengthOutOfRange` for a chain outside
    its bound, :class:`ValueError` for a place past the others, and
    :class:`PresentIfMismatch`, naming the class, where an entry before the
    place is of the kind (the named entry is then that one, not the one at the
    place); it cannot change after it is made."""

    LEAST: ClassVar[int] = 0
    MOST: ClassVar[Optional[int]] = None
    KEY: ClassVar[type]

    __slots__ = ("_rest", "_place")

    def __init__(self, rest: Iterable[object] = (), place: int = 0) -> None:
        others = tuple(rest)
        count = len(others) + 1
        if self.MOST is not None and count > self.MOST:
            raise TooManyElements(type(self).__name__)
        if count < self.LEAST:
            raise LengthOutOfRange(type(self).__name__)
        if isinstance(place, bool) or not isinstance(place, int) or not 0 <= place <= len(others):
            raise ValueError(f"{type(self).__name__}: the place {place!r} is not among the entries")
        if any(self.KEY.matches(entry) for entry in others[:place]):
            raise PresentIfMismatch(type(self).__name__)
        object.__setattr__(self, "_rest", others)
        object.__setattr__(self, "_place", place)

    @classmethod
    def place_first(cls, entries: Iterable[object]) -> "Placed":
        """The chain ``entries``, which holds an entry of the kind, as the
        others and the place of the first such entry among them: what a decode
        makes of the chain it read. Raises :class:`PresentIfMismatch` where it
        holds none."""
        listed = tuple(entries)
        for place, entry in enumerate(listed):
            if cls.KEY.matches(entry):
                return cls(listed[:place] + listed[place + 1 :], place)
        raise PresentIfMismatch(cls.__name__)

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError(f"{type(self).__name__} cannot change after it is made")

    @property
    def rest(self) -> Tuple[object, ...]:
        """The entries other than the named one."""
        return self._rest

    @property
    def place(self) -> int:
        """Where the named entry stands among the others: how many come
        before it."""
        return self._place

    def __len__(self) -> int:
        """How many entries the chain holds, the named one included."""
        return len(self._rest) + 1

    def __iter__(self) -> Iterator[object]:
        """The entries in order, the named one made at its place."""
        yield from self._rest[: self._place]
        yield self.KEY.named()
        yield from self._rest[self._place :]

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Placed):
            return NotImplemented
        return self._rest == other._rest and self._place == other._place

    def __hash__(self) -> int:
        return hash((self._rest, self._place))

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self._rest!r}, {self._place!r})"
