# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The CBOR (RFC 8949) items a ``sce:encoding="cbor"`` codec reads and writes
(SCE_FORGE.md §4.6.1): one definite-length map whose keys are small unsigned
integers and whose values are unsigned integers, booleans, text strings and
byte strings.

Mirrors ``backends/rust/forge-runtime/src/cbor.rs``, rule for rule. A
generated codec calls these; it spells no CBOR of its own.

Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest form,
so two backends given the same value write the same bytes. Reading takes a
head in any valid length — what it reads is the value — and refuses what the
codec cannot read as its entry: a reserved additional-information value, an
indefinite length, another major type.
"""

from __future__ import annotations

from typing import Optional, Tuple

from .codec import CodecError, InvalidUtf8, NeedMoreBytes, SceCursor, SceSink


class CborMalformed(CodecError):
    """The input is not a map this codec reads: not a definite-length map,
    a key given twice, or an entry of the wrong major type."""


class CborRequiredKeyMissing(CodecError):
    """The map lacks an entry declared ``sce:required="true"``."""


class CborWrongLength(CodecError):
    """A byte string is not its entry's exact ``sce:length``."""


class CborTooDeep(CodecError):
    """An unknown entry's value nests deeper than :data:`MAX_SKIP_DEPTH`."""


class CborOutOfRange(CodecError):
    """A value does not fit its entry's type or ``sce:max-size``."""


MAJOR_UNSIGNED = 0
MAJOR_BYTES = 2
MAJOR_TEXT = 3
MAJOR_MAP = 5
MAJOR_SIMPLE = 7

#: The deepest an unknown entry's value may nest before a skip refuses it
#: (SCE_FORGE.md §4.6.1).
MAX_SKIP_DEPTH = 16


# ── Writing ─────────────────────────────────────────────────────────────


def write_head(w: SceSink, major: int, value: int) -> None:
    """Write a head of ``major`` carrying ``value``, in its shortest form."""
    m = major << 5
    if value < 24:
        w.write_bytes(bytes([m | value]))
    elif value <= 0xFF:
        w.write_bytes(bytes([m | 24, value]))
    elif value <= 0xFFFF:
        w.write_bytes(bytes([m | 25]) + value.to_bytes(2, "big"))
    elif value <= 0xFFFFFFFF:
        w.write_bytes(bytes([m | 26]) + value.to_bytes(4, "big"))
    else:
        w.write_bytes(bytes([m | 27]) + value.to_bytes(8, "big"))


def write_uint(w: SceSink, value: int) -> None:
    """Write an unsigned integer."""
    write_head(w, MAJOR_UNSIGNED, value)


def write_bool(w: SceSink, value: bool) -> None:
    """Write ``false`` or ``true``."""
    w.write_bytes(bytes([(MAJOR_SIMPLE << 5) | (21 if value else 20)]))


def write_text(w: SceSink, value: str) -> None:
    """Write a text string."""
    data = value.encode("utf-8")
    write_head(w, MAJOR_TEXT, len(data))
    w.write_bytes(data)


def write_bytes(w: SceSink, value: bytes) -> None:
    """Write a byte string."""
    write_head(w, MAJOR_BYTES, len(value))
    w.write_bytes(value)


def write_map_head(w: SceSink, entries: int) -> None:
    """Write the head of a definite-length map of ``entries`` entries."""
    write_head(w, MAJOR_MAP, entries)


# ── Reading ─────────────────────────────────────────────────────────────


def _read_be(c: SceCursor, n: int) -> int:
    data = c.peek_slice(n)
    c.advance(n)
    return int.from_bytes(data, "big")


def read_head(c: SceCursor) -> Tuple[int, int]:
    """Read one head: its major type and the value its additional
    information carries (for a string or a map, the length). An indefinite
    length and the reserved values 28–30 are refused."""
    initial = _read_be(c, 1)
    major = initial >> 5
    info = initial & 0x1F
    if info < 24:
        return major, info
    if info == 24:
        return major, _read_be(c, 1)
    if info == 25:
        return major, _read_be(c, 2)
    if info == 26:
        return major, _read_be(c, 4)
    if info == 27:
        return major, _read_be(c, 8)
    raise CborMalformed()


def _expect(c: SceCursor, major: int) -> int:
    m, value = read_head(c)
    if m != major:
        raise CborMalformed()
    return value


def read_uint(c: SceCursor) -> int:
    """Read an unsigned integer."""
    return _expect(c, MAJOR_UNSIGNED)


def read_uint_upto(c: SceCursor, max_value: int) -> int:
    """Read an unsigned integer an entry of ``max_value`` holds."""
    value = read_uint(c)
    if value > max_value:
        raise CborOutOfRange()
    return value


def read_bool(c: SceCursor) -> bool:
    """Read ``false`` or ``true``."""
    m, value = read_head(c)
    if m != MAJOR_SIMPLE or value not in (20, 21):
        raise CborMalformed()
    return value == 21


def _read_payload(c: SceCursor, n: int) -> bytes:
    if n > c.remaining():
        raise NeedMoreBytes()
    data = bytes(c.peek_slice(n))
    c.advance(n)
    return data


def read_text(c: SceCursor, max_size: Optional[int]) -> str:
    """Read a UTF-8 text string of at most ``max_size`` bytes (``None``: no
    bound)."""
    n = _expect(c, MAJOR_TEXT)
    if max_size is not None and n > max_size:
        raise CborOutOfRange()
    data = _read_payload(c, n)
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        raise InvalidUtf8() from None


def read_bytes(c: SceCursor, max_size: Optional[int]) -> bytes:
    """Read a byte string of at most ``max_size`` bytes (``None``: no bound)."""
    n = _expect(c, MAJOR_BYTES)
    if max_size is not None and n > max_size:
        raise CborOutOfRange()
    return _read_payload(c, n)


def read_bytes_exact(c: SceCursor, length: int) -> bytes:
    """Read a byte string of exactly ``length`` bytes."""
    n = _expect(c, MAJOR_BYTES)
    if n != length:
        raise CborWrongLength()
    return _read_payload(c, n)


def read_map_len(c: SceCursor) -> int:
    """Read the head of a definite-length map: how many entries follow."""
    return _expect(c, MAJOR_MAP)


def skip(c: SceCursor) -> None:
    """Skip one item — the value of a key the codec does not declare — and
    everything nested in it, refusing one nested deeper than
    :data:`MAX_SKIP_DEPTH`."""
    _skip_at(c, 0)


def _skip_at(c: SceCursor, depth: int) -> None:
    if depth >= MAX_SKIP_DEPTH:
        raise CborTooDeep()
    major, value = read_head(c)
    # Unsigned, negative, simple / float: the head is the whole item.
    if major in (0, 1, 7):
        return
    if major in (2, 3):
        _read_payload(c, value)
        return
    if major == 4:
        for _ in range(value):
            _skip_at(c, depth + 1)
        return
    if major == 5:
        for _ in range(value):
            _skip_at(c, depth + 1)
            _skip_at(c, depth + 1)
        return
    # A tag: its one enclosed item follows.
    if major == 6:
        _skip_at(c, depth + 1)
        return
    raise CborMalformed()
