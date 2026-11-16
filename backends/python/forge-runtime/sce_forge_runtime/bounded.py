# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The bound a codec's byte string or text is held to, as a type (docs/adr/0077
§3).

A codec's encode printed from its plan (docs/adr/0081 §2) reads each byte
string of a source as a value of a subclass of :class:`BoundedBytes`, and each
text as one of :class:`BoundedText`, that names the element's bound. Such a value is made only by its constructor, which checks
the bound, and ``bytes`` cannot change after it is made, so a value of it holds
its bound for as long as it lives. The encode over it then has nothing of the
bound to refuse, as the Rust and C++ encodes have nothing over their bounded
types. Mirrors ``Bounded`` of the Rust runtime and ``SCE::Forge::Bytes`` of the
C++ runtime.
"""

from __future__ import annotations

from typing import FrozenSet, Generic, Iterable, Optional, Sized, TypeVar, Union

from .codec import InconsistentSource, InvalidUtf8, LengthOutOfRange, TooManyElements


class BoundedBytes(bytes):
    """Bytes held to a bound: at least ``LEAST`` of them and at most ``MOST``,
    ``MOST`` being ``None`` where the protocol states no most (docs/adr/0077
    §7). A subclass names the bound of one element by its two class
    attributes; this class itself holds any bytes.

    Made from anything ``bytes`` is made from but a count: a count of zero
    bytes is spelled ``bytes(n)``, so that no number is read as bytes by
    accident. Raises :class:`LengthOutOfRange`, naming the class, for bytes
    outside the bound."""

    LEAST: int = 0
    MOST: Optional[int] = None

    def __new__(
        cls, data: Union[bytes, bytearray, memoryview, Iterable[int]] = b""
    ) -> "BoundedBytes":
        if isinstance(data, int):
            raise TypeError(f"{cls.__name__} is made of bytes, not of a count")
        held = super().__new__(cls, data)
        if len(held) < cls.LEAST or (cls.MOST is not None and len(held) > cls.MOST):
            raise LengthOutOfRange(cls.__name__)
        return held


class BoundedText(str):
    """A text held to a bound counted in the UTF-8 bytes it encodes to, as
    every backend counts a text's length on the wire: at least ``LEAST`` of
    them and at most ``MOST``, ``MOST`` being ``None`` where the protocol
    states no most (docs/adr/0077 §3, §7). A subclass names the bound of one
    element by its two class attributes.

    Raises :class:`InvalidUtf8`, naming the class, for a text no UTF-8 holds (a
    lone surrogate), and :class:`LengthOutOfRange` for one outside the bound.
    Mirrors ``SCE::Forge::Text`` of the C++ runtime."""

    LEAST: int = 0
    MOST: Optional[int] = None

    def __new__(cls, text: str = "") -> "BoundedText":
        if not isinstance(text, str):
            raise TypeError(f"{cls.__name__} is made of a text")
        try:
            size = len(text.encode("utf-8"))
        except UnicodeEncodeError as error:
            raise InvalidUtf8(cls.__name__) from error
        if size < cls.LEAST or (cls.MOST is not None and size > cls.MOST):
            raise LengthOutOfRange(cls.__name__)
        return super().__new__(cls, text)


class BoundedTuple(tuple):
    """A list of entries held to a bound (docs/adr/0065 §6 R2): at least
    ``LEAST`` entries and at most ``MOST``, ``MOST`` being ``None`` where the
    protocol states no most (docs/adr/0077 §7). A subclass names the bound of
    one list by its two class attributes. A tuple, so the list cannot grow past
    its bound after it is made.

    Raises :class:`TooManyElements`, naming the class, for more entries than
    the bound holds, and :class:`LengthOutOfRange` for fewer than its least.
    Mirrors ``Bounded`` of the Rust runtime and ``SCE::Forge::List`` of the C++
    runtime."""

    LEAST: int = 0
    MOST: Optional[int] = None

    def __new__(cls, entries: Iterable[object] = ()) -> "BoundedTuple":
        held = super().__new__(cls, entries)
        if cls.MOST is not None and len(held) > cls.MOST:
            raise TooManyElements(cls.__name__)
        if len(held) < cls.LEAST:
            raise LengthOutOfRange(cls.__name__)
        return held


class UnknownTag(int):
    """The tag the default arm of a variant holds (docs/adr/0065 §6 R3): a
    number the tag's bits hold, at most ``MOST``, that no arm of the variant
    names (``KNOWN``). A subclass names the tags of one variant by its two
    class attributes.

    A default arm that held a tag an arm names would be read back as that arm,
    so such a tag is no value of the class: its constructor raises
    :class:`ValueError`, as ``chr`` does for a number no character is, since a
    tag is not a length and no :class:`CodecError` names it. An ``int`` cannot
    change after it is made, so a value of the class holds what it was checked
    to be for as long as it lives. Mirrors ``UnknownTag`` of the Rust
    runtime's generated codecs."""

    MOST: int = 0xFF
    KNOWN: FrozenSet[int] = frozenset()

    def __new__(cls, tag: int) -> "UnknownTag":
        if isinstance(tag, bool) or not isinstance(tag, int):
            raise TypeError(f"{cls.__name__} is made of an int")
        if tag < 0 or tag > cls.MOST or tag in cls.KNOWN:
            raise ValueError(f"{cls.__name__}: {tag} is a tag an arm names or past the bits")
        return super().__new__(cls, tag)


A = TypeVar("A", bound=Sized)
B = TypeVar("B", bound=Sized)


class Aligned(Generic[A, B]):
    """Two lists of one count (docs/adr/0065 §6 R5): a codec that writes one
    number for two lists holds both as this, so the number cannot disagree with
    either.

    Made only by its constructor, which compares the two counts once and raises
    :class:`InconsistentSource`, naming the class, where they differ; the two
    lists are held in read-only attributes, so they stay of one count for as
    long as the value lives, as a :class:`BoundedTuple` stays within its bound.
    Mirrors ``Aligned`` of the Rust runtime and ``SCE::Forge::Aligned`` of the
    C++ runtime."""

    __slots__ = ("_first", "_second")

    def __init__(self, first: A, second: B) -> None:
        if len(first) != len(second):
            raise InconsistentSource(type(self).__name__)
        object.__setattr__(self, "_first", first)
        object.__setattr__(self, "_second", second)

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError(f"{type(self).__name__} cannot change after it is made")

    @property
    def first(self) -> A:
        """The first list."""
        return self._first

    @property
    def second(self) -> B:
        """The second list."""
        return self._second

    def __len__(self) -> int:
        """How many entries each list holds."""
        return len(self._first)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Aligned):
            return NotImplemented
        return self._first == other._first and self._second == other._second

    def __hash__(self) -> int:
        return hash((self._first, self._second))

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self._first!r}, {self._second!r})"
