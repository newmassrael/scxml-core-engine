# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The bound a codec's byte string is held to, as a type (docs/adr/0077 §3).

A codec's encode printed from its plan (docs/adr/0081 §2) reads each byte
string of a source as a value of a subclass of :class:`BoundedBytes` that names
the element's bound. Such a value is made only by its constructor, which checks
the bound, and ``bytes`` cannot change after it is made, so a value of it holds
its bound for as long as it lives. The encode over it then has nothing of the
bound to refuse, as the Rust and C++ encodes have nothing over their bounded
types. Mirrors ``Bounded`` of the Rust runtime and ``SCE::Forge::Bytes`` of the
C++ runtime.
"""

from __future__ import annotations

from typing import Iterable, Optional, Union

from .codec import LengthOutOfRange


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
