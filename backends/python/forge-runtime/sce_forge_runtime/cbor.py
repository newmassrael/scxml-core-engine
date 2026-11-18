# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The CBOR (RFC 8949) items a ``sce:encoding="cbor"`` codec reads and writes
(SCE_FORGE.md §4.6.1, docs/adr/0101): one definite-length map whose keys are
small unsigned integers, and below it the form of each type of the language:
an integer of either sign, a float, a truth value, a text or byte string, an
array, a record's map keyed by its fields' ids as text, and a union's map of
one entry keyed by its case's variant name.

Mirrors ``backends/rust/forge-runtime/src/cbor.rs``, rule for rule. A
generated codec calls these; it spells no CBOR of its own.

Writing is deterministic (RFC 8949 section 4.2.1): every head in its shortest
form and a float in the shortest precision that holds it exactly, so two
backends given the same value write the same bytes. Reading takes the
definite-length profile of CBOR and, within it, every well-formed encoding of
a value (a head longer than it needs, a float wider than it needs), and raises
the :class:`~sce_forge_runtime.codec.CodecError` subclass that names the class
of what it refuses (docs/adr/0101 section 3.3): not well-formed or outside the
profile, an item the place does not take, a value outside its type.
"""

from __future__ import annotations

import math
import struct
from typing import Optional, Sequence, Tuple

from .codec import CodecError, InvalidUtf8, NeedMoreBytes, SceCursor, SceSink


class CborMalformed(CodecError):
    """An item that is not well-formed CBOR (reserved additional information,
    a break code, a simple value in the two-byte form below 32), or one
    outside the definite-length profile the codec reads: an indefinite-length
    array, map, text or byte string, at any depth (docs/adr/0101 section
    3.1)."""


class CborRequiredKeyMissing(CodecError):
    """The map lacks an entry of a bare type, one that is not ``optional<T>``."""


class CborWrongLength(CodecError):
    """A byte string is not its entry's exact ``sce:length``."""


class CborTooDeep(CodecError):
    """An unknown entry's value nests deeper than :data:`MAX_SKIP_DEPTH`."""


class CborOutOfRange(CodecError):
    """A value does not fit its entry's type or ``sce:max-size``."""


class CborWrongType(CodecError):
    """An item of a major type, a simple value or a tag the type at its place
    does not take: a text where a number stands, a negative integer where an
    unsigned type stands, ``null`` where the place is not a list's optional
    element, a tagged item, or a key of the wrong major type (docs/adr/0101
    section 3.3)."""


class CborDuplicateKey(CodecError):
    """A map holds one of the keys it declares twice, at any depth."""


class CborUnknownVariant(CodecError):
    """A union's one-entry map is keyed by a name no case declares."""


class CborUnionEntries(CodecError):
    """A union's map holds no entry, or more than one."""


MAJOR_UNSIGNED = 0
MAJOR_NEGATIVE = 1
MAJOR_BYTES = 2
MAJOR_TEXT = 3
MAJOR_ARRAY = 4
MAJOR_MAP = 5
MAJOR_TAG = 6
MAJOR_SIMPLE = 7

#: The deepest an unknown entry's value may nest before a skip refuses it
#: (SCE_FORGE.md §4.6.1).
MAX_SKIP_DEPTH = 16

# The additional information of `false` and `true` (RFC 8949 section 3.3).
_SIMPLE_FALSE = 20
_SIMPLE_TRUE = 21

# The additional information of a half, a single and a double precision float.
_INFO_HALF = 25
_INFO_SINGLE = 26
_INFO_DOUBLE = 27

# The least simple value the two-byte form may carry: one below it is not
# well-formed.
_LEAST_TWO_BYTE_SIMPLE = 32

# The one NaN written, the quiet NaN of half precision (docs/adr/0086,
# section 9.2).
_NAN = bytes([(MAJOR_SIMPLE << 5) | _INFO_HALF, 0x7E, 0x00])


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


def write_int(w: SceSink, value: int) -> None:
    """Write a signed integer: major type 0 for zero or more, and below zero
    major type 1, whose head carries ``-1 - value`` (RFC 8949 section 3.1)."""
    if value >= 0:
        write_head(w, MAJOR_UNSIGNED, value)
    else:
        write_head(w, MAJOR_NEGATIVE, -1 - value)


def _half_of(x: float) -> Optional[bytes]:
    """The half precision float that is ``x`` exactly, as its two bytes, or
    ``None`` when no half is. ``x`` is not a NaN."""
    try:
        packed = struct.pack(">e", x)
    except OverflowError:
        return None
    return packed if struct.unpack(">e", packed)[0] == x else None


def _single_of(x: float) -> Optional[bytes]:
    """The single precision float that is ``x`` exactly, as its four bytes,
    or ``None`` when no single is. ``x`` is not a NaN."""
    try:
        packed = struct.pack(">f", x)
    except OverflowError:
        return None
    return packed if struct.unpack(">f", packed)[0] == x else None


def write_float(w: SceSink, x: float) -> None:
    """Write a float in the shortest of half, single and double precision
    that holds it exactly, and a NaN as ``f9 7e 00`` (RFC 8949 sections 4.1
    and 4.2.1). A ``float32`` and a ``float64`` share it: a single is a
    double exactly."""
    if math.isnan(x):
        w.write_bytes(_NAN)
        return
    half = _half_of(x)
    if half is not None:
        w.write_bytes(bytes([(MAJOR_SIMPLE << 5) | _INFO_HALF]) + half)
        return
    single = _single_of(x)
    if single is not None:
        w.write_bytes(bytes([(MAJOR_SIMPLE << 5) | _INFO_SINGLE]) + single)
        return
    w.write_bytes(bytes([(MAJOR_SIMPLE << 5) | _INFO_DOUBLE]) + struct.pack(">d", x))


def write_bool(w: SceSink, value: bool) -> None:
    """Write ``false`` or ``true``."""
    w.write_bytes(bytes([(MAJOR_SIMPLE << 5) | (_SIMPLE_TRUE if value else _SIMPLE_FALSE)]))


def write_text(w: SceSink, value: str) -> None:
    """Write a text string."""
    data = value.encode("utf-8")
    write_head(w, MAJOR_TEXT, len(data))
    w.write_bytes(data)


def write_bytes(w: SceSink, value: bytes) -> None:
    """Write a byte string."""
    write_head(w, MAJOR_BYTES, len(value))
    w.write_bytes(value)


def write_array_head(w: SceSink, items: int) -> None:
    """Write the head of a definite-length array of ``items`` items: a list an
    entry holds."""
    write_head(w, MAJOR_ARRAY, items)


def write_map_head(w: SceSink, entries: int) -> None:
    """Write the head of a definite-length map of ``entries`` entries."""
    write_head(w, MAJOR_MAP, entries)


def write_variant(w: SceSink, name: str) -> None:
    """Write the head of a union's value: a map of one entry keyed by the
    case's variant ``name`` as a text string (docs/adr/0101 section 2.1). The
    case's record, or the empty map for a case that holds none, is written
    after it."""
    write_map_head(w, 1)
    write_text(w, name)


# ── Reading ─────────────────────────────────────────────────────────────


def _read_be(c: SceCursor, n: int) -> int:
    data = c.peek_slice(n)
    c.advance(n)
    return int.from_bytes(data, "big")


def _read_item_head(c: SceCursor) -> Tuple[int, int, int]:
    """Read one head: its major type, its additional information, and the
    value that information carries (for a string, an array or a map, the
    length; for a float, its bits). What is not well-formed (the reserved
    values 28-30, a break code, a simple value in the two-byte form below 32)
    and an indefinite length, outside the profile the codec reads
    (docs/adr/0101 section 3.1), raise :class:`CborMalformed`."""
    initial = _read_be(c, 1)
    major = initial >> 5
    info = initial & 0x1F
    if info < 24:
        value = info
    elif info == 24:
        value = _read_be(c, 1)
    elif info == 25:
        value = _read_be(c, 2)
    elif info == 26:
        value = _read_be(c, 4)
    elif info == 27:
        value = _read_be(c, 8)
    else:
        raise CborMalformed()
    if major == MAJOR_SIMPLE and info == 24 and value < _LEAST_TWO_BYTE_SIMPLE:
        raise CborMalformed()
    return major, info, value


def read_head(c: SceCursor) -> Tuple[int, int]:
    """Read one head: its major type and the value its additional
    information carries (for a string or a map, the length). An indefinite
    length, the reserved values 28-30 and a two-byte simple value below 32
    raise :class:`CborMalformed`."""
    major, _, value = _read_item_head(c)
    return major, value


def _expect(c: SceCursor, major: int) -> int:
    m, value = read_head(c)
    if m != major:
        raise CborWrongType()
    return value


def read_uint(c: SceCursor) -> int:
    """Read an unsigned integer; a negative one, as any other item, raises
    :class:`CborWrongType`."""
    return _expect(c, MAJOR_UNSIGNED)


def read_uint_upto(c: SceCursor, max_value: int) -> int:
    """Read an unsigned integer an entry of ``max_value`` holds."""
    value = read_uint(c)
    if value > max_value:
        raise CborOutOfRange()
    return value


def read_int_within(c: SceCursor, least: int, greatest: int) -> int:
    """Read a signed integer of ``least`` to ``greatest``, the range of its
    type: major type 0, or major type 1 below zero. A number outside the
    range raises :class:`CborOutOfRange`, another item
    :class:`CborWrongType`."""
    major, value = read_head(c)
    if major == MAJOR_UNSIGNED:
        number = value
    elif major == MAJOR_NEGATIVE:
        number = -1 - value
    else:
        raise CborWrongType()
    if not least <= number <= greatest:
        raise CborOutOfRange()
    return number


def _half_value(bits: int) -> float:
    """The value of the half precision float whose bits are ``bits``."""
    return struct.unpack(">e", bits.to_bytes(2, "big"))[0]


def read_float(c: SceCursor) -> float:
    """Read a ``float64``: a half, a single or a double, each read to the
    value it is exactly. Any other item, an integer among them, raises
    :class:`CborWrongType`."""
    major, info, value = _read_item_head(c)
    if major == MAJOR_SIMPLE and info == _INFO_HALF:
        return _half_value(value)
    if major == MAJOR_SIMPLE and info == _INFO_SINGLE:
        return struct.unpack(">f", value.to_bytes(4, "big"))[0]
    if major == MAJOR_SIMPLE and info == _INFO_DOUBLE:
        return struct.unpack(">d", value.to_bytes(8, "big"))[0]
    raise CborWrongType()


def read_float32(c: SceCursor) -> float:
    """Read a ``float32``: a float of any precision whose value a single
    holds exactly. A double that no single is raises :class:`CborOutOfRange`;
    a NaN of any precision is a NaN."""
    x = read_float(c)
    if math.isnan(x) or _single_of(x) is not None:
        return x
    raise CborOutOfRange()


def read_bool(c: SceCursor) -> bool:
    """Read ``false`` or ``true``. Every other item, ``null`` and a float
    among them, raises :class:`CborWrongType`."""
    major, info, _ = _read_item_head(c)
    if major != MAJOR_SIMPLE or info not in (_SIMPLE_FALSE, _SIMPLE_TRUE):
        raise CborWrongType()
    return info == _SIMPLE_TRUE


def _read_payload(c: SceCursor, n: int) -> bytes:
    if n > c.remaining():
        raise NeedMoreBytes()
    data = bytes(c.peek_slice(n))
    c.advance(n)
    return data


def read_text(c: SceCursor, max_size: Optional[int]) -> str:
    """Read a UTF-8 text string of at most ``max_size`` bytes (``None``: no
    bound), its length judged before its bytes are read. A text in chunks
    has an indefinite length and raises :class:`CborMalformed`."""
    n = _expect(c, MAJOR_TEXT)
    if max_size is not None and n > max_size:
        raise CborOutOfRange()
    data = _read_payload(c, n)
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        raise InvalidUtf8() from None


def read_bytes(c: SceCursor, max_size: Optional[int]) -> bytes:
    """Read a byte string of at most ``max_size`` bytes (``None``: no bound).
    One in chunks has an indefinite length and raises
    :class:`CborMalformed`."""
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


def read_array_len(c: SceCursor) -> int:
    """Read the head of a definite-length array: how many items follow. An
    indefinite length is refused, as a map's is."""
    return _expect(c, MAJOR_ARRAY)


def read_map_len(c: SceCursor) -> int:
    """Read the head of a definite-length map: how many entries follow."""
    return _expect(c, MAJOR_MAP)


def read_variant(c: SceCursor, names: Sequence[str]) -> int:
    """Read the head of a union's value and name its case (docs/adr/0101
    sections 2.1 and 3.3): a map, judged for exactly one entry before its key
    is read (:class:`CborUnionEntries`), whose key is a text string
    (:class:`CborWrongType`) naming one of ``names``
    (:class:`CborUnknownVariant`). Answers the case's index in ``names``; the
    case's record, or the empty map, follows."""
    if read_map_len(c) != 1:
        raise CborUnionEntries()
    name = read_text(c, None)
    try:
        return list(names).index(name)
    except ValueError:
        raise CborUnknownVariant() from None


def skip(c: SceCursor) -> None:
    """Skip one item — the value of a key the codec does not declare — and
    everything nested in it, refusing one nested deeper than
    :data:`MAX_SKIP_DEPTH` and an indefinite length anywhere in it."""
    _skip_at(c, 0)


def _skip_at(c: SceCursor, depth: int) -> None:
    if depth >= MAX_SKIP_DEPTH:
        raise CborTooDeep()
    major, value = read_head(c)
    # Unsigned, negative, simple / float: the head is the whole item.
    if major in (MAJOR_UNSIGNED, MAJOR_NEGATIVE, MAJOR_SIMPLE):
        return
    if major in (MAJOR_BYTES, MAJOR_TEXT):
        _read_payload(c, value)
        return
    if major == MAJOR_ARRAY:
        for _ in range(value):
            _skip_at(c, depth + 1)
        return
    if major == MAJOR_MAP:
        for _ in range(value):
            _skip_at(c, depth + 1)
            _skip_at(c, depth + 1)
        return
    # A tag: its one enclosed item follows.
    if major == MAJOR_TAG:
        _skip_at(c, depth + 1)
        return
    raise CborMalformed()
