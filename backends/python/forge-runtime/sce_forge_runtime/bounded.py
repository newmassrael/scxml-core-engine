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

from typing import Iterable, Optional, Union

from .codec import InvalidUtf8, LengthOutOfRange, TooManyElements


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
