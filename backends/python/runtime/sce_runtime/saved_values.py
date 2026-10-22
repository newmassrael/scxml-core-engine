# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""How each ``sce:type`` is written into a saved state and read back: only if it
is one, since a value of another kind, or one its width cannot hold, is refused
rather than converted. The functions a generated machine's ``save`` and
``restore`` call, one per type, named as the type is.

A value is written as the JSON value model holds it (a :class:`JsonNumber`, a
``str``, a ``bool``, a ``list``, a ``dict``, ``None``) and read back from it, a
refusal naming what holds a value that is not the type's.

Cross-language siblings: ``backends/go/runtime/saved_values.go``,
``com.sce.runtime.SavedValues`` (Kotlin) and the ``SavedValue`` trait of
``sce_rust_runtime::saved_state`` (Rust).
"""

from __future__ import annotations

import math
import struct
from typing import Any, Callable, List, Optional, TypeVar

from .number_text import number_text
from .saved_json import JsonNumber
from .saved_state import StateRefusal

T = TypeVar("T")
O = TypeVar("O")

_INT64_MIN = -(1 << 63)
_INT64_MAX = (1 << 63) - 1
_UINT64_MAX = (1 << 64) - 1


def _not_a(value: Any, what: str, kind: str) -> StateRefusal:
    """The refusal of ``value``, saved for ``what``, which is not ``kind`` (``a
    number``, ``a text``, ...). A ``None`` is named apart: it is an absent value
    (docs/adr/0055), which only a place the restoring build declares optional
    reads, so a state saved after a widening is refused at its first null by a
    build from before it."""
    if value is None:
        return StateRefusal(f"'{what}' is absent (null), and its type is not optional")
    return StateRefusal(f"'{what}' is not {kind}")


def _saved_number(value: Any, what: str) -> str:
    if not isinstance(value, JsonNumber):
        raise _not_a(value, what, "a number")
    return value.text


def _is_signed_digits(text: str) -> bool:
    digits = text[1:] if text.startswith("-") else text
    return digits != "" and all("0" <= c <= "9" for c in digits)


def _saved_whole(value: Any, what: str, least: int, most: int) -> int:
    """A number of at least ``least`` and at most ``most``, written as a whole."""
    text = _saved_number(value, what)
    # `+` and `_` are spellings `int` takes and a JSON number has not.
    if not _is_signed_digits(text) or not _INT64_MIN <= int(text) <= _INT64_MAX or not least <= int(text) <= most:
        raise StateRefusal(f"'{what}' ({text}) is not a whole number its type can hold")
    return int(text)


def _whole_number(value: int) -> JsonNumber:
    return JsonNumber(str(value))


def saved_write_uint8(value: int) -> Any:
    """The ``write`` functions give a value its saved form. A 64-bit integer is
    written as a text: a reader that holds JSON numbers as doubles would lose its
    low bits, and a saved state outlives the backend that wrote it."""
    return _whole_number(value)


def saved_write_uint16(value: int) -> Any:
    return _whole_number(value)


def saved_write_uint32(value: int) -> Any:
    return _whole_number(value)


def saved_write_int8(value: int) -> Any:
    return _whole_number(value)


def saved_write_int16(value: int) -> Any:
    return _whole_number(value)


def saved_write_int32(value: int) -> Any:
    return _whole_number(value)


def saved_write_uint64(value: int) -> Any:
    return str(value)


def saved_write_int64(value: int) -> Any:
    return str(value)


def saved_write_float64(value: float) -> Any:
    """A real's saved form: the one number text every engine writes
    (ARCHITECTURE.md, "JSON Number Text"), and, for a value that is not finite,
    which JSON has no spelling for, the text every backend reads it back from."""
    if math.isnan(value):
        return "NaN"
    if value == math.inf:
        return "Infinity"
    if value == -math.inf:
        return "-Infinity"
    return JsonNumber(number_text(value))


def saved_write_float32(value: float) -> Any:
    """A 32-bit real's saved form: that of the 64-bit real it widens to, which is
    exact, so the number a reader that holds JSON numbers as doubles finds is the
    number the machine holds, ``0.30000001192092896`` and not the shorter ``0.3`` a
    single prints for it, which is another number to that reader. Python holds a
    32-bit real in a ``float`` already rounded to a single."""
    return saved_write_float64(value)


def saved_write_bool(value: bool) -> Any:
    return value


def saved_write_string(value: str) -> Any:
    return value


def saved_write_bytes(value: bytes) -> Any:
    """A byte string's byte-exact Latin-1 text, each byte the character of that
    code point (docs/adr/0005, decision 2): the spelling it takes on the wire, so
    one value is one text on every backend, and a JSON reader that has no byte
    string reads it as a string."""
    return bytes(value).decode("latin-1")


def saved_write_list(value: List[T], write: Callable[[T], Any]) -> Any:
    """A list, each element by ``write``."""
    return [write(item) for item in value]


def saved_read_uint8(value: Any, what: str) -> int:
    return _saved_whole(value, what, 0, (1 << 8) - 1)


def saved_read_uint16(value: Any, what: str) -> int:
    return _saved_whole(value, what, 0, (1 << 16) - 1)


def saved_read_uint32(value: Any, what: str) -> int:
    return _saved_whole(value, what, 0, (1 << 32) - 1)


def saved_read_int8(value: Any, what: str) -> int:
    return _saved_whole(value, what, -(1 << 7), (1 << 7) - 1)


def saved_read_int16(value: Any, what: str) -> int:
    return _saved_whole(value, what, -(1 << 15), (1 << 15) - 1)


def saved_read_int32(value: Any, what: str) -> int:
    return _saved_whole(value, what, -(1 << 31), (1 << 31) - 1)


def _saved_wide_text(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise _not_a(value, what, "a 64-bit whole number written as a text")
    return value


def saved_read_uint64(value: Any, what: str) -> int:
    text = _saved_wide_text(value, what)
    if not _is_unsigned_digits(text) or int(text) > _UINT64_MAX:
        raise StateRefusal(f"'{what}' ({text}) is not a whole number its type can hold")
    return int(text)


def _is_unsigned_digits(text: str) -> bool:
    return text != "" and all("0" <= c <= "9" for c in text)


def saved_read_int64(value: Any, what: str) -> int:
    text = _saved_wide_text(value, what)
    if not _is_signed_digits(text) or not _INT64_MIN <= int(text) <= _INT64_MAX:
        raise StateRefusal(f"'{what}' ({text}) is not a whole number its type can hold")
    return int(text)


def _saved_real(value: Any, what: str) -> float:
    if isinstance(value, str):
        if value == "NaN":
            return math.nan
        if value == "Infinity":
            return math.inf
        if value == "-Infinity":
            return -math.inf
    text = _saved_number(value, what)
    # A number past the range of a 64-bit real reads as the infinity it rounds to,
    # as every other backend reads it; only a text that is no number is refused.
    try:
        return float(text)
    except ValueError:
        raise StateRefusal(f"'{what}' ({text}) is not a number") from None


def _to_single(value: float) -> float:
    """``value`` rounded to the nearest binary32, as the double that holds it
    exactly; a value past the single's range is the infinity it rounds to."""
    try:
        return struct.unpack("<f", struct.pack("<f", value))[0]
    except OverflowError:
        return math.inf if value > 0 else -math.inf


def saved_read_float32(value: Any, what: str) -> float:
    return _to_single(_saved_real(value, what))


def saved_read_float64(value: Any, what: str) -> float:
    return _saved_real(value, what)


def saved_read_bool(value: Any, what: str) -> bool:
    if not isinstance(value, bool):
        raise _not_a(value, what, "a truth value")
    return value


def saved_read_string(value: Any, what: str) -> str:
    if not isinstance(value, str):
        raise _not_a(value, what, "a text")
    return value


def saved_read_bounded_string(value: Any, what: str, capacity: int) -> str:
    """A text read back only if it holds no more than the ``capacity`` UTF-8 bytes
    the machine bounds it by: a machine never holds more, and a restored one must
    not be the first to."""
    text = saved_read_string(value, what)
    held = len(text.encode("utf-8", "surrogatepass"))
    if held > capacity:
        raise StateRefusal(f"'{what}' holds {held} UTF-8 bytes, past the {capacity} it is bounded by")
    return text


def saved_read_bytes(value: Any, what: str, capacity: int) -> bytes:
    """A byte string read back from its Latin-1 text (:func:`saved_write_bytes`),
    only if it holds no more than the ``capacity`` bytes the machine bounds it by.
    A character past U+00FF is no byte, and is refused rather than cut."""
    text = saved_read_string(value, what)
    for character in text:
        if ord(character) > 0xFF:
            raise StateRefusal(f"'{what}' holds U+{ord(character):04X}, which is past U+00FF and so no byte")
    held = text.encode("latin-1")
    if len(held) > capacity:
        raise StateRefusal(f"'{what}' holds {len(held)} bytes, past the {capacity} it is bounded by")
    return held


def saved_read_list(
    value: Any, what: str, capacity: int, element: Callable[[Any, str], T]
) -> List[T]:
    """A list read back element by element, only if it holds no more than the
    ``capacity`` the machine bounds it by: a machine never holds more, and a
    restored one must not be the first to."""
    if not isinstance(value, list):
        raise _not_a(value, what, "an array")
    if len(value) > capacity:
        raise StateRefusal(f"'{what}' holds {len(value)} elements, past the {capacity} it is bounded by")
    return [element(item, f"{what}[{at}]") for at, item in enumerate(value)]


def saved_read_optional(value: Any, what: str, present: Callable[[Any, str], T]) -> Optional[T]:
    """A value that may be absent read back (docs/adr/0055, section 4): ``None``
    where the saved state holds null, and otherwise the value ``present`` reads,
    held to its type as any value is. Only a place the machine declares optional
    reads through this, so a null saved where the machine holds a value of ``T`` is
    refused by the reader of ``T``, naming what holds it. Python holds an optional
    value as the value or ``None`` (docs/adr/0055, section 5), so no wrapper is
    made."""
    if value is None:
        return None
    return present(value, what)


def saved_bounded_string(capacity: int) -> Callable[[Any, str], str]:
    """The reader of a text held to ``capacity`` UTF-8 bytes, for a place that
    reads through a function value (:func:`saved_read_optional`)."""
    return lambda value, what: saved_read_bounded_string(value, what, capacity)


def saved_bounded_bytes(capacity: int) -> Callable[[Any, str], bytes]:
    """The reader of a byte string held to ``capacity`` bytes, for a place that
    reads through a function value (:func:`saved_read_optional`)."""
    return lambda value, what: saved_read_bytes(value, what, capacity)


def saved_field(value: Any, record: str, name: str) -> Any:
    """The field ``name`` of a saved record value: what a generated record reads
    each field with."""
    if not isinstance(value, dict):
        raise StateRefusal(f"'{record}' is not an object")
    if name not in value:
        raise StateRefusal(f"'{record}' has no field '{name}'")
    return value[name]
