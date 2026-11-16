# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Lists whose entries say their inputs by a law (docs/adr/0065 §6 R8).

An entry codec can say a flag input itself (R4a) while the list that holds it
hands every entry a value from elsewhere: a flag of the parent bound to every
entry, or, in a chain, whether the entry before is of a kind. The two agree or
the value has no frame. Both are a language over the list that a small
automaton decides: a state, the value each entry must say in it, and the state
after the entry. A bound flag is the automaton whose state is the flag and
never changes (:class:`Uniform`); the entry before is the one whose state
starts clear and becomes whether the entry is of the kind (:class:`AfterKey`).
A list with both runs them together.

:class:`Lawful` holds a list with the start its automata begin in, which is
the value's to give where the start is not fixed: the flag a parent writes is
read from it, so an empty list still holds one. It is made only by its
constructor, which runs the automata once, as :class:`BoundedTuple` counts
once, and it cannot change after it is made. Mirrors ``law`` of the Rust
runtime and ``SCE::Forge::Lawful`` of the C++ runtime.
"""

from __future__ import annotations

from typing import Callable, ClassVar, Iterable, Iterator, Optional, Tuple, Type, Union

from .bounded import BoundedTuple
from .codec import PresentIfMismatch

#: What an entry says of one input: 0 or 1, or ``None`` where it says nothing
#: of it (docs/adr/0065 §6 R4e), its codec's module's ``input_<name>``.
Says = Callable[[object], Optional[int]]


class Uniform:
    """The law of a flag bound to every entry: each entry says the input as
    the start says it, and the start is the flag, a ``bool`` the value
    gives."""

    __slots__ = ("says",)

    def __init__(self, says: Says) -> None:
        self.says = says

    def given(self, start: object) -> bool:
        """The start a value gives, as the automaton holds it: 0 or 1, or a
        ``bool``. Raises :class:`ValueError` for anything else."""
        if start not in (0, 1):
            raise ValueError(f"the start of a bound flag is 0 or 1, not {start!r}")
        return bool(start)

    def start(self, start: bool) -> bool:
        """The state before the first entry: the flag."""
        return start

    def step(self, state: bool, entry: object) -> Optional[bool]:
        """The state after ``entry``, or ``None`` where it says otherwise than
        the flag."""
        said = self.says(entry)
        if said is not None and (said != 0) != state:
            return None
        return state

    def start_of(self, first: Optional[object]) -> bool:
        """The start a list that begins with ``first`` follows from: what its
        first entry says, clear where it has none or says nothing."""
        if first is None:
            return False
        said = self.says(first)
        return said is not None and said != 0


class AfterKey:
    """The law of an input a chain supplies from the entry before
    (``<sce:entry-flag-bind previous-entry-id>``): each entry says the input
    set exactly when the entry before is of the kind ``matches`` tells, and the
    first says it clear. The start is fixed, so a value gives ``None``."""

    __slots__ = ("says", "matches")

    def __init__(self, says: Says, matches: Callable[[object], bool]) -> None:
        self.says = says
        self.matches = matches

    def given(self, start: object) -> None:
        """The start a value gives: none, since it is fixed. Raises
        :class:`ValueError` for any other."""
        if start is not None:
            raise ValueError(f"the start of an input the entry before decides is fixed, not {start!r}")
        return None

    def start(self, start: None) -> bool:
        """The state before the first entry: no entry before it."""
        return False

    def step(self, state: bool, entry: object) -> Optional[bool]:
        """The state after ``entry``, whether it is of the kind, or ``None``
        where it says otherwise than the entry before was."""
        said = self.says(entry)
        if said is not None and (said != 0) != state:
            return None
        return bool(self.matches(entry))

    def start_of(self, first: Optional[object]) -> None:
        """The start of any list: fixed."""
        return None


#: An automaton over a list's entries.
Automaton = Union[Uniform, AfterKey]


class Lawful:
    """A list whose entries follow the law ``LAW`` from the start the value
    gives (docs/adr/0065 §6 R8), held in the class of its bound, ``ENTRIES``.
    A subclass names the law of one list by its two class attributes: ``LAW``
    holds one automaton for each input the list hands its entries, and
    ``start`` is the start of the one, or a tuple of each one's in that order
    where there are several.

    Made only by its constructor, which makes the entries in the class of
    their bound and runs the law over them once, raising
    :class:`PresentIfMismatch`, naming the class, where an entry says a value
    other than the one its place asks of it. It reads as the list it holds
    (its length, its entries in order) and cannot change after it is made, so
    its entries cannot leave the law. Mirrors ``Lawful`` of the Rust runtime
    and ``SCE::Forge::Lawful`` of the C++ runtime."""

    ENTRIES: ClassVar[Type[BoundedTuple]] = BoundedTuple
    LAW: ClassVar[Tuple[Automaton, ...]] = ()

    __slots__ = ("_start", "_entries")

    def __init__(self, start: object, entries: Iterable[object] = ()) -> None:
        held = self.ENTRIES(entries)
        starts = self._starts(start)
        states = [law.start(given) for law, given in zip(self.LAW, starts)]
        for entry in held:
            for i, law in enumerate(self.LAW):
                state = law.step(states[i], entry)
                if state is None:
                    raise PresentIfMismatch(type(self).__name__)
                states[i] = state
        object.__setattr__(self, "_start", starts[0] if len(starts) == 1 else starts)
        object.__setattr__(self, "_entries", held)

    @classmethod
    def _starts(cls, start: object) -> Tuple[object, ...]:
        """``start`` as one given start for each automaton of the law."""
        if len(cls.LAW) == 1:
            return (cls.LAW[0].given(start),)
        if not isinstance(start, tuple) or len(start) != len(cls.LAW):
            raise ValueError(f"{cls.__name__} starts from one start for each of its {len(cls.LAW)} inputs")
        return tuple(law.given(given) for law, given in zip(cls.LAW, start))

    @classmethod
    def of(cls, entries: Iterable[object] = ()) -> "Lawful":
        """``entries`` held to the law from the start its first entry says,
        what a list made without a start, by default, is given."""
        listed = tuple(entries)
        first = listed[0] if listed else None
        starts = tuple(law.start_of(first) for law in cls.LAW)
        return cls(starts[0] if len(starts) == 1 else starts, listed)

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError(f"{type(self).__name__} cannot change after it is made")

    @property
    def start(self) -> object:
        """The start the entries follow from: for the law of a bound flag,
        the flag; ``None`` for a start that is fixed."""
        return self._start

    @property
    def entries(self) -> BoundedTuple:
        """The entries, in the class of their bound."""
        return self._entries

    def __len__(self) -> int:
        """How many entries it holds."""
        return len(self._entries)

    def __iter__(self) -> Iterator[object]:
        """Its entries, in order."""
        return iter(self._entries)

    def __getitem__(self, index: int) -> object:
        """The entry at ``index``."""
        return self._entries[index]

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Lawful):
            return NotImplemented
        return self._start == other._start and tuple(self._entries) == tuple(other._entries)

    def __hash__(self) -> int:
        return hash((self._start, tuple(self._entries)))

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self._start!r}, {tuple(self._entries)!r})"
