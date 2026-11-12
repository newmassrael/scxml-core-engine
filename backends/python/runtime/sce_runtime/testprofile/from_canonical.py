# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A value read back from the canonical form of the test log
(docs/SCE_TEST_PROFILE.md §12.5; docs/SCE_TTCN3_LANE.md §10.3): the inverse of
``canonical``, which a recorded input trace's values are read with
(``schemas/sce-test-trace.v1.schema.json``).

Each reader takes one form and accepts its one spelling, so a value read here
and written back by ``canonical`` is the same bytes:

- an integer is the text of its decimal digits, with no sign but a minus, no
  leading zero and no ``-0``, within the range of the type it is read as;
- a ``bool`` is a truth value;
- a value of an enum is the name its document declares for the variant, or, for
  an enum of an open set, the text of a number no variant declares;
- a text is a text of at most its bound in UTF-8 bytes;
- a byte string is its octets in upper-case hex, two digits each;
- a record is an object of its fields by their declared ids, an absent field
  omitted, no field it does not declare;
- a list is an array of its elements within its bounds, an absent element of a
  list of ``optional<T>`` ``null``;
- a value of a union is an object of one member, named by its case's variant,
  whose value is the case's record, or ``{}`` for a case that holds none.

A value is one of the JSON value model ``sce_runtime.saved_json.parse_json``
reads (a ``dict``, a ``list``, a ``str``, a ``bool``, a ``JsonNumber`` or
``None``), which is also what ``canonical`` writes. That reader keeps a number
as the text it was written in and refuses an object that names a member twice,
where the ``json`` module would read ``NaN``, ``Infinity`` and a number past a
double and keep the last of two members of one name: so a ``dict`` here names
each member once, and no reader refuses a field given twice, as the Rust reader
of its list of members must.

A value that does not belong to its type is refused with a ``TraceRefusal``,
naming where it stands (``what``) and why, never converted. Which form a value
takes is decided by the types its document declares and not by its Python
type, since a byte string and a list of ``uint8`` are one type in some
languages: the generated reader of a record (``read_<record>`` in the unit of a
test configuration) calls these by the walk its canonical writer is spelled by
(``forge::generator::test_template_walk::canonical_form``).

The reader of a byte string is ``byte_string``, as its writer is, so that
neither hides the built-in ``bytes``.

Cross-language siblings: ``sce_rust_runtime::test::from_canonical`` (Rust),
``com.sce.runtime.test.FromCanonical`` (Kotlin), ``SCE::Test::FromCanonical``
(C++), whose refusals these name in the same words.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple, TypeVar

from ..saved_json import JsonNumber
from .trace_refusal import TraceRefusal

__all__ = [
    "INT8",
    "INT16",
    "INT32",
    "INT64",
    "UINT8",
    "UINT16",
    "UINT32",
    "UINT64",
    "IntegerType",
    "boolean",
    "byte_string",
    "decimal",
    "elements",
    "enumerated",
    "field",
    "integer",
    "no_record",
    "nullable",
    "open_enumerated",
    "optional_field",
    "record",
    "text",
    "union_case",
]

#: The value a reader reads: what its read gives back.
T = TypeVar("T")

#: A value of an enum.
E = TypeVar("E")


@dataclass(frozen=True)
class IntegerType:
    """An integer type of the profile, as a reader holds the text of its digits
    to it: the least and the greatest value it holds."""

    #: The type's name, as the profile names it.
    name: str
    #: The least value it holds.
    least: int
    #: The greatest value it holds.
    greatest: int

    def holds(self, number: int) -> bool:
        """Whether ``number`` is a value of the type."""
        return self.least <= number <= self.greatest


def _signed(name: str, bits: int) -> IntegerType:
    return IntegerType(name, -(1 << (bits - 1)), (1 << (bits - 1)) - 1)


def _unsigned(name: str, bits: int) -> IntegerType:
    return IntegerType(name, 0, (1 << bits) - 1)


#: An ``int8``.
INT8 = _signed("int8", 8)
#: An ``int16``.
INT16 = _signed("int16", 16)
#: An ``int32``.
INT32 = _signed("int32", 32)
#: An ``int64``.
INT64 = _signed("int64", 64)
#: A ``uint8``.
UINT8 = _unsigned("uint8", 8)
#: A ``uint16``.
UINT16 = _unsigned("uint16", 16)
#: A ``uint32``.
UINT32 = _unsigned("uint32", 32)
#: A ``uint64``.
UINT64 = _unsigned("uint64", 64)

#: The range a canonical integer's text is read within before its type holds
#: it, a 128-bit integer's, which the Rust reader of the same text holds it to.
_WIDEST = _signed("int128", 128)

#: The canonical spelling of an integer: an optional minus and ASCII decimal
#: digits with no leading zero. ``[0-9]`` is the ASCII digits alone, where
#: ``int`` would also read the digits of every script, a sign, an underscore
#: and whitespace.
_DECIMAL = re.compile(r"-?(0|[1-9][0-9]*)")

#: The canonical spelling of a byte string: upper-case hex digits, two per
#: octet.
_UPPER_HEX = re.compile(r"(?:[0-9A-F]{2})*")


def _refused(what: str, why: str) -> TraceRefusal:
    """The refusal of the value at ``what``, for ``why``."""
    return TraceRefusal(f"{what}: {why}")


def _kind(value: Any) -> str:
    """The kind of a JSON value, as a refusal names it."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "a truth value"
    if isinstance(value, (JsonNumber, int, float)):
        return "a number"
    if isinstance(value, str):
        return "a text"
    if isinstance(value, list):
        return "an array"
    if isinstance(value, dict):
        return "an object"
    return "a value of no JSON kind"


def _not_a(value: Any, what: str, form: str) -> TraceRefusal:
    """The refusal of ``value``, at ``what``, which is not of the form
    ``form``."""
    return _refused(what, f"{_kind(value)} is not {form}")


def decimal(digits: str) -> Optional[int]:
    """The integer ``digits`` spells as the canonical form writes one: an
    optional minus and decimal digits with no leading zero, never ``-0``, within
    the range of a 128-bit integer. ``None`` for any other text."""
    if _DECIMAL.fullmatch(digits) is None or digits == "-0":
        return None
    number = int(digits)
    return number if _WIDEST.holds(number) else None


def integer(value: Any, width: IntegerType, what: str) -> int:
    """An integer of the type ``width``: the text of its decimal digits."""
    if not isinstance(value, str):
        raise _not_a(value, what, "an integer, which is the text of its decimal digits")
    number = decimal(value)
    if number is None:
        raise _refused(what, f"'{value}' is not the text of an integer's decimal digits")
    if not width.holds(number):
        raise _refused(what, f"{value} is past the range of its type")
    return number


def boolean(value: Any, what: str) -> bool:
    """A ``bool``."""
    if not isinstance(value, bool):
        raise _not_a(value, what, "a truth value")
    return value


def text(value: Any, bound: int, what: str) -> str:
    """A text of at most ``bound`` bytes of UTF-8, the bound its type declares.

    A text that holds a surrogate standing alone is no text of UTF-8, and is
    refused: the reader of a trace's text reads such an escape as U+FFFD, so
    only a value built in place holds one."""
    if not isinstance(value, str):
        raise _not_a(value, what, "a text")
    try:
        count = len(value.encode("utf-8"))
    except UnicodeEncodeError:
        raise _refused(what, "a text that holds a surrogate standing alone is no text of UTF-8") from None
    if count > bound:
        raise _refused(what, f"a text of {count} bytes is past the {bound} its type holds")
    return value


def byte_string(value: Any, bound: int, what: str) -> bytes:
    """A byte string of at most ``bound`` octets: the text of its octets in
    upper-case hex, two digits each."""
    if not isinstance(value, str):
        raise _not_a(value, what, "a byte string, which is a text of hex")
    if _UPPER_HEX.fullmatch(value) is None:
        raise _refused(what, f"'{value}' is not octets in upper-case hex, two digits each")
    octets = bytes.fromhex(value)
    if len(octets) > bound:
        raise _refused(what, f"a byte string of {len(octets)} octets is past the {bound} its type holds")
    return octets


def enumerated(value: Any, declared: Sequence[Tuple[str, E]], what: str) -> E:
    """A value of an enum of a closed set: the variant whose declared name the
    text is, from ``declared``, each variant's name and its value."""
    if not isinstance(value, str):
        raise _not_a(value, what, "a value of an enum, which is its name")
    for name, variant in declared:
        if name == value:
            return variant
    raise _refused(what, f"'{value}' names no variant of its enum")


def open_enumerated(
    value: Any,
    declared: Sequence[Tuple[str, E]],
    underlying: IntegerType,
    from_underlying: Callable[[int], E],
    what: str,
) -> E:
    """A value of an enum of an open set (``sce:strict-variants="false"``): the
    variant whose declared name the text is, or the value ``from_underlying``
    makes of the number the text spells, of the enum's ``underlying`` type,
    where no variant declares that number. A declared number is written by its
    variant's name, so its digits are refused as the second spelling of one
    value."""
    if not isinstance(value, str):
        raise _not_a(value, what, "a value of an enum, which is its name")
    for name, variant in declared:
        if name == value:
            return variant
    number = decimal(value)
    if number is None:
        raise _refused(what, f"'{value}' names no variant of its enum, and is no number")
    if not underlying.holds(number):
        raise _refused(what, f"{value} is past the range of its enum")
    held = from_underlying(number)
    if any(variant == held for _, variant in declared):
        raise _refused(what, f"{value} is the number of a declared variant, which is written by its name")
    return held


def record(value: Any, fields: Sequence[str], what: str) -> Dict[str, Any]:
    """A record whose fields are ``fields``: an object, each member of which
    names one of them. Answers the object, whose fields ``field`` and
    ``optional_field`` read."""
    if not isinstance(value, dict):
        raise _not_a(value, what, "a record, which is an object of its fields")
    for name in value:
        if name not in fields:
            raise _refused(what, f"'{name}' is no field of its record")
    return value


def field(of: Dict[str, Any], id: str, what: str) -> Any:  # noqa: A002 -- the profile's word
    """The field ``id`` of the record ``of``, whose type is not optional, so it
    is present: its value, which may be JSON ``null``, which the reader of its
    type refuses."""
    if id not in of:
        raise _refused(what, "the field is absent, and its type is not optional")
    return of[id]


def optional_field(of: Dict[str, Any], id: str, read: Callable[[Any], T]) -> Optional[T]:  # noqa: A002
    """The field ``id`` of the record ``of``, whose type is optional: ``None``
    where it is absent, which the canonical form writes by omitting it, and the
    value ``read`` reads where it is present. A member that is present and
    ``null`` is handed to ``read``, whose reader refuses it, since an absent
    field is omitted and never written as ``null``."""
    if id not in of:
        return None
    return read(of[id])


def elements(value: Any, least: int, most: int, what: str) -> List[Any]:
    """The elements of a list of at least ``least`` and at most ``most``
    elements."""
    if not isinstance(value, list):
        raise _not_a(value, what, "a list, which is an array")
    if not least <= len(value) <= most:
        raise _refused(what, f"a list of {len(value)} elements is outside its bounds, {least} to {most}")
    return value


def nullable(value: Any, read: Callable[[Any], T]) -> Optional[T]:
    """An element of a list of ``optional<T>``: ``None`` for ``null``, which the
    canonical form writes in the place of an absent element, and the value
    ``read`` reads otherwise."""
    if value is None:
        return None
    return read(value)


def union_case(value: Any, variants: Sequence[str], what: str) -> Tuple[str, Any]:
    """A value of a union whose cases are ``variants``: an object of one member,
    named by one of them. Answers the variant of its case and the case's value.
    A variant no case declares is refused here, so a reader that chooses by the
    variant read has a branch for each declared one and needs none for another.

    Python's test profile matches no union yet
    (``codegen_matrix::TEST_PROFILE_UNION_BACKENDS``), so no generated reader
    calls this yet; it reads the form ``canonical.union_case``
    writes, which a host's own reader of a union reads with."""
    if not isinstance(value, dict):
        raise _not_a(value, what, "a value of a union, which is an object of one member")
    if len(value) != 1:
        raise _refused(what, f"a value of a union is an object of one member, and this one has {len(value)}")
    ((variant, case),) = value.items()
    if variant not in variants:
        raise _refused(what, f"'{variant}' names no case of its union")
    return variant, case


def no_record(value: Any, what: str) -> None:
    """The value of a case of a union that holds no record: the empty
    object."""
    if not isinstance(value, dict) or value:
        raise _refused(what, "a case that holds no record is the empty object")
