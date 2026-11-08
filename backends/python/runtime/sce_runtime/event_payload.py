"""Lifting an event's typed `_event.data` view out of the data it carries.

NL→IR Item C1 Path A gives a schema'd event a TYPED payload, which the
natively lowered guards read. One producer fills it: the generated
`raise_<event>` inject seam. Every other producer — `<send>` with `<param>`,
namelist or `<content>`, an invoke forwarding an event either way, autoforward,
BasicHTTP, mesh — fills `EventMetadata.data`, the wire W3C SCXML 5.10 describes
and W3C SCXML B.2.8.1 reads.

Until this module the two never met. A native guard could only fire on an event
the inject seam produced, so the same guard answered differently depending on
where its event came from, and a typed payload could not cross an invoke
boundary at all — the receiver was documented as re-hydrating from `data`, and
nothing did. Measured 2026-09-22: a document with a native typed guard and an
`<assign expr="_event.data.x">` on one event could not be given a value both
could see.

So `data` is the canonical carrier and the typed view is lifted from it here,
at the one point an event is dequeued, the engine's door:

    no data at all              refused whole, error.execution in its place
    a field the data lacks      refused whole, error.execution in its place
    a value of another type     refused whole, error.execution in its place
    the fields, well typed      taken, and the guard evaluates

A delivery refused whole selects no transition (docs/adr/0086, section 4.2).

A refusal is a `TypedPayloadError`: the sentence the delivery's
`error.execution` carries, and beside it what a statechart reads as data
(docs/adr/0087, section 2.4): a `RefusalCode`, the place as a strict RFC 6901
pointer (`/items/2/tags/0`, `~` written `~0` and `/` written `~1`, empty for the
whole text) and the bound the value broke. A repeated name anywhere in the
text, and a text holding U+0000, are refused, so every runtime's reader holds
one set of texts (tests/json_text/payload_refusal.json).
"""

from __future__ import annotations

import enum
import json
import math
import struct
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

from .number_text import number_text
from .payload_reading import PayloadReading, payload_reading_of_text

# The largest finite binary32: a payload number past it does not fit a field a
# schema declares `float32`.
FLOAT32_MAX = 3.4028234663852886e38


class Single(float):
    """Names a payload field a schema declares `float32` in a field spec.

    Python's `float` is a binary64, so a spec that said `float` for it would hold
    the double the payload carried — `0.1` — where every engine with a single of
    its own holds the binary32 nearest it, `0.10000000149011612`. Naming the
    width lets the lift round, and refuse a value past it as it refuses an
    integer past its width."""


class Wide(int):
    """Names a payload field a schema declares `int64` or `uint64` in a field spec.

    A 64-bit integer crosses as the text of its digits: a reader that takes a JSON
    number through a double carries one past 2^53 with its low bits wrong and no
    error. It is read from that text, or from a number exact in a double
    (`|n| <= 2^53`); a number past that is refused here on every engine, so the
    same payload gives the same field wherever it is read."""


class OptionalOf:
    """Names a payload field a schema declares `optional<T>` in a field spec,
    around what the spec names `T` by (`int`, `str`, `Wide`, `Single`, an enum
    type): the field holds a value of `T`, or `None` for absent
    (docs/adr/0055, section 4).

    On the wire absence is JSON `null` and nothing else. A delivery that lacks
    the key is not an absent value: the key's absence says the sender's revision
    of the schema does not have the field, so it is refused as a missing field
    of `T` is, unless the schema's defaults name it (a field a later revision
    added, whose default `null` reads absent)."""

    __slots__ = ("declared",)

    def __init__(self, declared: Any) -> None:
        self.declared = declared

    def __repr__(self) -> str:
        return f"OptionalOf({self.declared!r})"


class Whole:
    """Names a whole number of a width in a type: a value outside ``low`` to
    ``high`` does not fit it. What a field spec's ``(name, int, low, high)`` says
    of a field, said of a value at any depth."""

    __slots__ = ("low", "high")

    def __init__(self, low: int, high: int) -> None:
        self.low = low
        self.high = high

    def __repr__(self) -> str:
        return f"Whole({self.low}, {self.high})"


class WideOf:
    """Names a 64-bit whole number in a type (see ``Wide``), signed when ``low``
    is below zero: read from the text of its digits, or from a number exact in a
    double, and held to ``low`` to ``high``."""

    __slots__ = ("low", "high")

    def __init__(self, low: int, high: int) -> None:
        self.low = low
        self.high = high

    def __repr__(self) -> str:
        return f"WideOf({self.low}, {self.high})"


class Text:
    """Names ``string[bound]``: a text of at most ``bound`` UTF-8 bytes, held to
    it wherever it stands, the top level of a payload included
    (docs/adr/0086, section 4.4)."""

    __slots__ = ("bound",)

    def __init__(self, bound: int) -> None:
        self.bound = bound

    def __repr__(self) -> str:
        return f"Text({self.bound})"


class Bytes:
    """Names ``bytes[bound]``: a byte string of at most ``bound`` bytes."""

    __slots__ = ("bound",)

    def __init__(self, bound: int) -> None:
        self.bound = bound

    def __repr__(self) -> str:
        return f"Bytes({self.bound})"


class ListOf:
    """Names ``list<T>[least..bound]``, around what names ``T``: an array of at
    least ``least`` and at most ``bound`` elements, each a value of ``T``, read
    in order. Both counts are held before any element is read (ADR 0077,
    section 2); ``least`` is 0 for a list that states no lower count
    (docs/adr/0086, section 15)."""

    __slots__ = ("declared", "bound", "least")

    def __init__(self, declared: Any, bound: int, least: int = 0) -> None:
        self.declared = declared
        self.bound = bound
        self.least = least

    def __repr__(self) -> str:
        if self.least:
            return f"ListOf({self.declared!r}, {self.bound}, least={self.least})"
        return f"ListOf({self.declared!r}, {self.bound})"


def _count_short(held: int, least: int) -> str:
    """Why a list of ``held`` elements is no value of a type whose lower count
    is ``least``, in the words the reader and the check both use."""
    return f"holds {held} elements, fewer than the {least} its type declares"


class RecordOf:
    """Names a record type: an object whose ``fields`` are ``(name, declared)``
    pairs, in declaration order. ``defaults``, for a record whose schema a later
    revision added fields to, is the JSON object of their defaults, which the
    generator spells from that schema, and a field the value lacks is read out
    of it (docs/adr/0086, section 10.3). ``make`` builds the record from the
    values read, in field order; without one the record is a ``dict``. A key the
    schema does not declare is stepped over."""

    __slots__ = ("fields", "defaults", "make")

    def __init__(self, fields: Sequence[tuple], defaults: str | None = None,
                 make: Callable[..., Any] | None = None) -> None:
        self.fields = tuple(fields)
        self.defaults = defaults
        self.make = make

    def __repr__(self) -> str:
        return f"RecordOf({self.fields!r})"


class CaseOf:
    """Names one case of a union: ``variant``, the member of the tag's enum the
    case is chosen by, and ``record``, the ``RecordOf`` the case holds, or None
    for an empty case, its tag alone. ``make`` builds the case's value, from the
    record read (from nothing for an empty case); without one the value is the
    pair of the variant and the record read (None for an empty case)."""

    __slots__ = ("variant", "record", "make")

    def __init__(self, variant: Any, record: RecordOf | None = None,
                 make: Callable[..., Any] | None = None) -> None:
        self.variant = variant
        self.record = record
        self.make = make

    def __repr__(self) -> str:
        return f"CaseOf({self.variant!r}, {self.record!r})"


class UnionOf:
    """Names a union type (docs/adr/0092, section 4): an object holding its tag
    under the key ``tag``, a value of the enum ``tag_type``, and beside it the
    fields of the case the tag names. ``cases`` are ``CaseOf``, one per variant
    of the enum. The tag is read first, by the enum's reader, wherever it stands
    in the object, and then the case's record from the same object; a key the
    case does not declare is stepped over, as a record steps over one."""

    __slots__ = ("tag", "tag_type", "cases")

    def __init__(self, tag: str, tag_type: type, cases: Sequence[CaseOf]) -> None:
        self.tag = tag
        self.tag_type = tag_type
        self.cases = {case.variant: case for case in cases}

    def __repr__(self) -> str:
        return f"UnionOf({self.tag!r}, {self.tag_type!r}, {list(self.cases.values())!r})"


_COMPOSED = (Whole, WideOf, Text, Bytes, ListOf, RecordOf, UnionOf)


class RefusalCode(enum.IntEnum):
    """The class of a refusal: the enum ``sce:std/lift/refusal_code.scxml``
    declares, each code at the number that document gives it
    (docs/adr/0087, section 2.3). ``declared_name`` is the name it declares
    the code by."""

    NOT_JSON = 1
    NOT_OBJECT = 2
    TOO_DEEP = 3
    MISSING = 4
    NULL_NOT_OPTIONAL = 5
    WRONG_KIND = 6
    BAD_NUMBER = 7
    OUT_OF_RANGE = 8
    TOO_LONG = 9
    TOO_MANY = 10
    UNKNOWN_VARIANT = 11
    NOT_BYTE_TEXT = 12
    NUL_IN_TEXT = 13
    #: A list short of the lower count its type states (docs/adr/0086, section 15).
    TOO_FEW = 14

    @property
    def declared_name(self) -> str:
        """The name ``sce:std/lift/refusal_code.scxml`` declares the code by."""
        head, *rest = self.name.lower().split("_")
        return head + "".join(word.capitalize() for word in rest)

    @classmethod
    def named(cls, name: str) -> Optional["RefusalCode"]:
        """The code the enum declares by ``name``, or None when it declares
        none."""
        for code in cls:
            if code.declared_name == name:
                return code
        return None


class TypedPayloadError(Exception):
    """Why this event's data cannot be read as its schema's fields.

    ``str()`` of it is the sentence the delivery's ``error.execution`` carries;
    beside it are its ``code``, the place as a strict RFC 6901 ``pointer``
    (empty for the whole text), and the ``bound`` the value broke, for a text,
    a byte string or a list past it (``None`` for every other refusal)."""

    def __init__(self, reason: str, code: RefusalCode, pointer: str,
                 bound: Optional[int] = None) -> None:
        super().__init__(reason)
        self.reason = reason
        self.code = code
        self.pointer = pointer
        self.bound = bound


def pointer_below(prefix: str, token: str) -> str:
    """``token`` below the strict RFC 6901 pointer ``prefix`` (empty for the
    whole text): ``/`` and the token with ``~`` written ``~0`` and ``/`` written
    ``~1`` (RFC 6901, section 3)."""
    return prefix + "/" + token.replace("~", "~0").replace("/", "~1")


def _join(prefix: str, segment: str) -> str:
    """``segment`` below ``prefix``, as a JSON Pointer's reference tokens are
    joined."""
    return segment if not prefix else f"{prefix}/{segment}"


def _utf8_size(text: str) -> int:
    return len(text.encode("utf-8", "surrogatepass"))


def _defaults_of(defaults: str) -> dict:
    try:
        parsed = json.loads(defaults)
    except ValueError:
        parsed = None
    if not isinstance(parsed, dict):
        # A text that is none is the generator's, not a sender's, and is
        # refused as no JSON object of the whole.
        raise TypedPayloadError(
            "the schema's defaults are not a JSON object, which the generator "
            "never writes", RefusalCode.NOT_JSON, "")
    return parsed


def _past_width(path: str, pointer: str, value: Any, low: int) -> TypedPayloadError:
    """The refusal of a whole number outside ``low`` to its width's top:
    below zero is no spelling of an unsigned number, and anything else outside
    is past the width."""
    code = (RefusalCode.BAD_NUMBER if low >= 0 and value < 0
            else RefusalCode.OUT_OF_RANGE)
    return TypedPayloadError(
        f"{path!r} does not fit the width its schema declares ({value!r})",
        code, pointer)


def read_value(path: str, declared: Any, value: Any, pointer: str) -> Any:
    """One value of an event's data as its type declares it, at the path through
    every depth that names it (docs/adr/0086, section 4): a field's name, then
    each index and field name below it, joined by ``/``; ``pointer`` is the
    strict RFC 6901 pointer to the same place, which a refusal carries.

    ``declared`` is what a field spec names a type by: a scalar (``int``,
    ``bool``, ``float``, ``Single``, ``str``, ``bytes``, an enum type), or one of
    ``Whole``, ``WideOf``, ``Text``, ``Bytes``, ``OptionalOf``, ``ListOf``,
    ``RecordOf`` and ``UnionOf``, composed to any depth. A value outside its type is refused
    before anything of it is kept, the refusal naming the path."""
    if isinstance(declared, OptionalOf):
        # Absence is the one JSON `null` (docs/adr/0055, section 4).
        return None if value is None else read_value(path, declared.declared, value, pointer)
    if value is None:
        raise TypedPayloadError(f"{path!r} is null, and its type is not optional",
                                RefusalCode.NULL_NOT_OPTIONAL, pointer)
    if isinstance(declared, ListOf):
        if not isinstance(value, list):
            raise TypedPayloadError(f"{path!r} is not a list",
                                    RefusalCode.WRONG_KIND, pointer)
        if len(value) > declared.bound:
            raise TypedPayloadError(
                f"{path!r} holds {len(value)} elements, past the {declared.bound} "
                f"its type declares", RefusalCode.TOO_MANY, pointer, declared.bound)
        if len(value) < declared.least:
            raise TypedPayloadError(
                f"{path!r} {_count_short(len(value), declared.least)}",
                RefusalCode.TOO_FEW, pointer, declared.least)
        return [read_value(_join(path, str(index)), declared.declared, item,
                           pointer_below(pointer, str(index)))
                for index, item in enumerate(value)]
    if isinstance(declared, RecordOf):
        if not isinstance(value, dict):
            raise TypedPayloadError(f"{path!r} is not a record",
                                    RefusalCode.WRONG_KIND, pointer)
        members = value
        if declared.defaults is not None:
            members = {**_defaults_of(declared.defaults), **value}
        held = []
        for name, field_type in declared.fields:
            field_path = _join(path, name)
            field_pointer = pointer_below(pointer, name)
            if name not in members:
                raise TypedPayloadError(f"the event's data has no {field_path!r}",
                                        RefusalCode.MISSING, field_pointer)
            held.append(read_value(field_path, field_type, members[name], field_pointer))
        if declared.make is not None:
            return declared.make(*held)
        return {name: v for (name, _), v in zip(declared.fields, held)}
    if isinstance(declared, UnionOf):
        # The refusals in the one order of every runtime (docs/adr/0092,
        # section 4.1): no object, then the tag, then the case's record.
        if not isinstance(value, dict):
            raise TypedPayloadError(f"{path!r} is not a union",
                                    RefusalCode.WRONG_KIND, pointer)
        tag_path = _join(path, declared.tag)
        tag_pointer = pointer_below(pointer, declared.tag)
        if declared.tag not in value:
            raise TypedPayloadError(f"the event's data has no {tag_path!r}",
                                    RefusalCode.MISSING, tag_pointer)
        variant = read_value(tag_path, declared.tag_type, value[declared.tag], tag_pointer)
        case = declared.cases[variant]
        record = None if case.record is None else read_value(path, case.record, value, pointer)
        if case.make is None:
            return (variant, record)
        return case.make() if case.record is None else case.make(record)
    if isinstance(declared, Text):
        text = _as(path, str, value, pointer)
        size = _utf8_size(text)
        if size > declared.bound:
            raise TypedPayloadError(
                f"{path!r} is {size} bytes, past the {declared.bound} its type declares",
                RefusalCode.TOO_LONG, pointer, declared.bound)
        return text
    if isinstance(declared, Bytes):
        held_bytes = _as(path, bytes, value, pointer)
        if len(held_bytes) > declared.bound:
            raise TypedPayloadError(
                f"{path!r} is {len(held_bytes)} bytes, past the {declared.bound} "
                f"its type declares", RefusalCode.TOO_LONG, pointer, declared.bound)
        return held_bytes
    if isinstance(declared, (Whole, WideOf)):
        if isinstance(declared, WideOf):
            whole = as_wide(path, value, declared.low < 0, pointer)
        else:
            whole = _as(path, int, value, pointer)
        if not declared.low <= whole <= declared.high:
            raise _past_width(path, pointer, whole, declared.low)
        return whole
    return _as(path, declared, value, pointer)


class BoundError(ValueError):
    """A value a host built is past a bound its type declares: raised where the
    value is made, by a generated record's ``__post_init__``."""


def check_value(path: str, declared: Any, value: Any) -> None:
    """Hold a value a host built to its type at every depth, before anything
    reads it (docs/adr/0086, section 5.4): what a generated record's
    ``__post_init__`` runs over each field, through the same type names the
    reader takes. Raises ``BoundError`` naming the path.

    A type that holds no bound of its own is passed over: a truth value, a
    real, an enum type, and ``None``, which the generator names the element of
    a list by where the element has nothing of its own to hold here (an enum
    member, or a record, whose own class held its fields where it was made), so
    that list is held to its counts alone."""
    if isinstance(declared, OptionalOf):
        if value is not None:
            check_value(path, declared.declared, value)
        return
    if isinstance(declared, ListOf):
        if len(value) > declared.bound:
            raise BoundError(
                f"{path!r} holds {len(value)} elements, past the {declared.bound} "
                f"its type declares")
        # A list is built from empty an element at a time, so its lower count
        # is held where it is written (docs/adr/0086, section 15.6).
        if len(value) < declared.least:
            raise BoundError(f"{path!r} {_count_short(len(value), declared.least)}")
        for index, item in enumerate(value):
            check_value(_join(path, str(index)), declared.declared, item)
        return
    if isinstance(declared, RecordOf):
        for name, field_type in declared.fields:
            member = value[name] if isinstance(value, dict) else getattr(value, name)
            check_value(_join(path, name), field_type, member)
        return
    if isinstance(declared, Text):
        size = _utf8_size(value)
        if size > declared.bound:
            raise BoundError(
                f"{path!r} is {size} bytes, past the {declared.bound} its type declares")
        return
    if isinstance(declared, Bytes):
        if len(value) > declared.bound:
            raise BoundError(
                f"{path!r} is {len(value)} bytes, past the {declared.bound} its type declares")
        return
    if isinstance(declared, (Whole, WideOf)):
        if not declared.low <= value <= declared.high:
            raise BoundError(
                f"{path!r} does not fit the width its schema declares ({value!r})")


class Raw(str):
    """A value already spelled as JSON text, which the writers below write as it
    is: a list or a record a writer spelled."""


def value_json(value: Any) -> Raw:
    """The JSON a value of a scalar type is written as: ``null`` for ``None``, a
    truth value, a number as its digits or as ECMAScript spells it, a text, a
    byte string as its Latin-1 text, and a ``Raw`` as it is. A 64-bit integer
    crosses as the text of its digits: ``wide_json``."""
    if isinstance(value, Raw):
        return value
    if value is None:
        return Raw("null")
    if isinstance(value, bool):
        return Raw("true" if value else "false")
    if isinstance(value, int):
        return Raw(str(int(value)))
    if isinstance(value, float):
        return Raw(number_text(value) if math.isfinite(value) else "null")
    if isinstance(value, (bytes, bytearray)):
        value = bytes(value).decode("latin-1")
    return Raw(json.dumps(str(value), ensure_ascii=False))


def wide_json(value: int) -> Raw:
    """A 64-bit integer as the text of its digits."""
    return Raw(json.dumps(str(int(value))))


def list_json(items: Sequence[Any], write: Callable[[Any], Any]) -> Raw:
    """The JSON of one list (docs/adr/0086, section 3.1): an array of what
    ``write`` spells each element as, in the list's order. ``[]`` when empty."""
    return Raw("[" + ",".join(value_json(write(item)) for item in items) + "]")


def record_json(members: Dict[str, Any]) -> Raw:
    """The JSON of one record: each member is a field's id and its value, written
    ascending by the id's UTF-8 bytes whatever order they are given in
    (ARCHITECTURE.md, "JSON Object Key Order", at every depth)."""
    ordered = sorted(members.items(), key=lambda item: item[0].encode("utf-8", "surrogatepass"))
    return Raw("{" + ",".join(
        json.dumps(key, ensure_ascii=False) + ":" + value_json(value)
        for key, value in ordered) + "}")


def union_json(tag: str, variant: str, members: Dict[str, Any]) -> Raw:
    """The JSON of one union value (docs/adr/0092, section 2.1): the tag under
    the key ``tag``, its value ``variant``, the case's name as its enum spells
    it, beside ``members``, the fields of the case's record (none for an empty
    case), written as ``record_json`` writes a record, the tag among them."""
    return record_json({**members, tag: variant})


def write_optional(value: Any, write: Any) -> Any:
    """What the wire carries of a value that may be absent (docs/adr/0055,
    section 4): `None`, which `json.dumps` and `ScriptValue.of` write as JSON
    `null`, for an absent value, and `write(value)`, what a present value of its
    type is written as, otherwise. The key is always written: a writer that left
    it out would say its revision does not have the field. Presence is
    `is not None`, never a truth test: a present `0`, `''` or `False` is
    written as the value it is."""
    return None if value is None else write(value)


# The largest whole number every engine holds exactly as a number (`2^53 - 1`,
# ECMAScript's `Number.MAX_SAFE_INTEGER`).
_EXACT_IN_A_DOUBLE = (1 << 53) - 1


def _is_decimal(text: str, signed: bool) -> bool:
    """Whether `text` is a whole number written as digits and nothing else: no
    sign but `-` (and only where `signed`), no leading zero, no `-0`."""
    digits = text[1:] if text.startswith("-") else text
    if text.startswith("-") and not signed:
        return False
    if not digits or not all("0" <= c <= "9" for c in digits):
        return False
    if digits != "0" and digits.startswith("0"):
        return False
    return not (digits == "0" and text.startswith("-"))


def as_wide(name: str, value: Any, signed: bool, pointer: str) -> int:
    """A 64-bit field's value: the text of its digits, or a whole number exact in a
    double (`|n| <= 2^53 - 1`). A truth value is not a number here."""
    if isinstance(value, str):
        if not _is_decimal(value, signed):
            raise TypedPayloadError(
                f"{name!r} is not a whole number written as digits ({value!r})",
                RefusalCode.BAD_NUMBER, pointer)
        return int(value)
    if isinstance(value, int) and not isinstance(value, bool):
        # `-0` is no whole number, as `"-0"` is not.
        if isinstance(value, _NegativeZero):
            raise TypedPayloadError(f"{name!r} is not a whole number (-0)",
                                    RefusalCode.BAD_NUMBER, pointer)
        if abs(value) > _EXACT_IN_A_DOUBLE:
            raise TypedPayloadError(
                f"{name!r} is a number past 2^53 - 1, which is not exact everywhere "
                f"({value!r}); a 64-bit integer that large is written as a text of "
                f"its digits", RefusalCode.BAD_NUMBER, pointer)
        return value
    # A fraction is a number in no whole spelling; a truth value, a list or a
    # record is another kind of value.
    code = RefusalCode.BAD_NUMBER if isinstance(value, float) else RefusalCode.WRONG_KIND
    raise TypedPayloadError(f"{name!r} is not a whole number ({value!r})", code, pointer)


def as_single(value: float) -> float:
    """`value` rounded to the nearest binary32, as the double that holds it
    exactly; a value past the single's range is the infinity it is there."""
    try:
        return struct.unpack("<f", struct.pack("<f", value))[0]
    except OverflowError:
        return math.inf if value > 0 else -math.inf


def _object_of_distinct_names(pairs: List[Tuple[str, Any]]) -> dict:
    """An object's members as a mapping, refusing a name that repeats.

    An object whose names repeat has no single meaning: `json.loads` keeps the
    last value, a reader that takes the first would read another, so it is
    refused, at any depth, as every other runtime's reader refuses it
    (docs/adr/0087, section 2.3)."""
    members: dict = {}
    for name, value in pairs:
        if name in members:
            raise ValueError(f"the field {name!r} appears twice")
        members[name] = value
    return members


def _no_constant(spelling: str) -> Any:
    """`NaN`, `Infinity` and `-Infinity` are not JSON (RFC 8259, section 6),
    which `json.loads` takes by default."""
    raise ValueError(f"{spelling} is not a JSON number")


class _NegativeZero(int):
    """The JSON number `-0`, which `json.loads` reads as the integer 0: a real
    reads it as zero, and a whole number refuses it, as every runtime does
    (`"-0"` is no spelling of a 64-bit integer either)."""


def _whole_number(spelling: str) -> int:
    """A JSON number of no fraction and no exponent, as `json.loads` hands it,
    `-0` kept apart from 0."""
    return _NegativeZero(0) if spelling == "-0" else int(spelling)


# The whitespace RFC 8259 has between tokens (section 2): space, tab, line feed
# and carriage return, and no other.
_JSON_WHITESPACE = " \t\n\r"


def _hex4(text: str, at: int) -> Optional[int]:
    """The number the four hex digits of ``text`` at ``at`` spell, or None when
    they are not four hex digits (no sign, which `int(…, 16)` would take)."""
    digits = text[at:at + 4]
    if len(digits) != 4 or any(c not in "0123456789abcdefABCDEF" for c in digits):
        return None
    return int(digits, 16)


def _paired_escapes(text: str) -> None:
    """Refuse a `\\u` escape of ``text`` that names half of a UTF-16 pair with no
    other half beside it (RFC 8259, sections 7 and 8.2): a high half not followed
    at once by an escape of a low one, or a low half alone. `json.loads` reads a
    lone half as a lone surrogate, which no other runtime holds. Every other rule
    of a text's grammar is `json.loads`'s, which holds them."""
    in_text = False
    at = 0
    while at < len(text):
        c = text[at]
        if not in_text:
            in_text = c == '"'
        elif c == '"':
            in_text = False
        elif c == "\\" and text[at + 1:at + 2] == "u":
            code = _hex4(text, at + 2)
            if code is None:
                at += 2
                continue
            if 0xD800 <= code <= 0xDBFF:
                low = _hex4(text, at + 8) if text[at + 6:at + 8] == "\\u" else None
                if low is None or not 0xDC00 <= low <= 0xDFFF:
                    raise ValueError("a \\u escape is half of a character")
                at += 12
                continue
            if 0xDC00 <= code <= 0xDFFF:
                raise ValueError("a \\u escape is half of a character")
            at += 6
            continue
        elif c == "\\":
            at += 2
            continue
        at += 1


def _read_json(text: str) -> Any:
    """One JSON text read by RFC 8259 alone, as every runtime reads it: no
    repeated name, no `NaN`, no half of a UTF-16 pair, and `-0` told from 0."""
    _paired_escapes(text)
    return json.loads(text, object_pairs_hook=_object_of_distinct_names,
                      parse_constant=_no_constant, parse_int=_whole_number)


def _decode(data: Any) -> dict:
    """The event's data as a mapping of field to value.

    A producer in this process hands a dict; one that crossed a wire hands the
    JSON spelling of it (`_coerce_event_data_str`). Both are the same payload.

    The JSON read here is W3C SCXML B.2.8.1's second rung — the one the script
    engine's `_json_to_lua_table` takes for the same string — and a refusal is
    named with the shared `PayloadReading` vocabulary. It is not a CALL to that
    function: that one returns Lua source and lives in `scripting/`, which this
    path must not need. A natively lowered guard machine runs with no Lua
    session at all, and reaching into the script engine to read a payload would
    hand the session back.
    """
    # §scxml-B-2-8-1: Read structured event data before projecting schema fields.
    if isinstance(data, dict):
        return data
    if data is None or data == "":
        raise TypedPayloadError("the event carries no data", RefusalCode.NOT_JSON, "")
    if isinstance(data, (bytes, bytearray)):
        data = data.decode("utf-8", errors="replace")
    if isinstance(data, str):
        try:
            decoded = _read_json(data)
        except ValueError as exc:
            rung = payload_reading_of_text(data)
            reading = (
                "announced structure and could not be read"
                if rung is PayloadReading.UNDECODABLE
                else "is a space-normalized string, which names no fields")
            raise TypedPayloadError(
                f"the event's data {reading} (B.2.8.1: {rung.value})",
                RefusalCode.NOT_JSON, "") from exc
        if not isinstance(decoded, dict):
            raise TypedPayloadError(
                "the event's data is a bare value, and this event's schema "
                "declares named fields", RefusalCode.NOT_OBJECT, "")
        return decoded
    raise TypedPayloadError(
        f"the event's data is a {type(data).__name__}, which names no fields",
        RefusalCode.NOT_OBJECT, "")


def _is_number(value: Any) -> bool:
    """Whether a decoded value is a JSON number: a truth value is not one."""
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _text(name: str, value: Any, pointer: str) -> str:
    """The text a value carries, held to no bound: a text field's value and the
    name an enum's variant is read by.

    A text holding U+0000 is refused: a runtime that holds a text NUL-terminated
    would end it there, so no runtime holds one, and every reader refuses the
    same texts (docs/adr/0087, section 2.3). A byte string may hold the byte
    zero."""
    if not isinstance(value, str):
        raise TypedPayloadError(f"{name!r} is not a text ({value!r})",
                                RefusalCode.WRONG_KIND, pointer)
    if "\u0000" in value:
        raise TypedPayloadError(f"{name!r} carries U+0000, which no text holds",
                                RefusalCode.NUL_IN_TEXT, pointer)
    return value


def _as(name: str, declared: type, value: Any, pointer: str) -> Any:
    """One field's value as its schema declares it, or a refusal naming
    ``pointer``.

    ⚠ A truth value is not a number here, and a number is not a truth value.
    JSON spells both, and a schema that declared `uint32` and received `true`
    has been handed something its own type says cannot occur.
    """
    if declared is bool:
        if isinstance(value, bool):
            return value
        raise TypedPayloadError(f"{name!r} is not a truth value ({value!r})",
                                RefusalCode.WRONG_KIND, pointer)
    if declared is int:
        if not _is_number(value) or isinstance(value, float):
            # A fraction is a number in no whole spelling; anything else is
            # another kind of value.
            code = (RefusalCode.BAD_NUMBER if isinstance(value, float)
                    else RefusalCode.WRONG_KIND)
            raise TypedPayloadError(f"{name!r} is not a whole number ({value!r})",
                                    code, pointer)
        if isinstance(value, _NegativeZero):
            raise TypedPayloadError(f"{name!r} is not a whole number (-0)",
                                    RefusalCode.BAD_NUMBER, pointer)
        return value
    if declared is Single:
        if not _is_number(value):
            raise TypedPayloadError(f"{name!r} is not a number ({value!r})",
                                    RefusalCode.WRONG_KIND, pointer)
        number = float(value)
        if abs(number) > FLOAT32_MAX:
            raise TypedPayloadError(
                f"{name!r} does not fit the width its schema declares ({value!r})",
                RefusalCode.OUT_OF_RANGE, pointer)
        return as_single(number)
    if declared is float:
        if not _is_number(value):
            raise TypedPayloadError(f"{name!r} is not a number ({value!r})",
                                    RefusalCode.WRONG_KIND, pointer)
        # A number past the largest double (`1e999`), which `json.loads` reads
        # as an infinity, does not fit, as one past the largest single does not
        # fit a `float32`.
        if math.isinf(float(value)):
            raise TypedPayloadError(
                f"{name!r} does not fit the width its schema declares ({value!r})",
                RefusalCode.OUT_OF_RANGE, pointer)
        return float(value)
    if declared is str:
        return _text(name, value, pointer)
    if declared is bytes:
        # JSON has no byte string, so a text is read back as bytes rather than
        # refused. Latin-1 is the reading because it is the inject seam's
        # spelling and it is byte-exact: every one of the 256 values maps to
        # one character and back. Printable ASCII — what a bytes guard
        # compares — is the same bytes under any of these readings.
        if isinstance(value, (bytes, bytearray)):
            return bytes(value)
        if isinstance(value, str):
            try:
                return value.encode("latin-1")
            except UnicodeEncodeError as exc:
                raise TypedPayloadError(
                    f"{name!r} carries a character above U+00FF, which no "
                    f"single byte spells", RefusalCode.NOT_BYTE_TEXT, pointer) from exc
        raise TypedPayloadError(f"{name!r} is not a byte string ({value!r})",
                                RefusalCode.WRONG_KIND, pointer)
    if isinstance(declared, type) and hasattr(declared, "sce_name"):
        # An enum a `sce-static` machine declares: the wire carries the name its
        # document declares for the value (`sce_name`), as a saved state does,
        # and a name the enum does not declare is a payload that does not fit
        # its schema.
        text = _text(name, value, pointer)
        for member in declared:
            if member.sce_name == text:
                return member
        raise TypedPayloadError(
            f"{name!r} ({text!r}) is not a variant of {declared.__name__}",
            RefusalCode.UNKNOWN_VARIANT, pointer)
    raise TypedPayloadError(
        f"{name!r} is declared {declared!r}, and this module has no reading "
        f"for it", RefusalCode.WRONG_KIND, pointer)


def lift(data: Any, fields: Sequence[tuple], defaults: str | None = None) -> list:
    """Every field of this event's schema, in declaration order.

    A field is `(name, declared)`, or `(name, int, low, high)` for an integer of
    the width its schema declares: a value past that width is a payload that does
    not fit the schema, refused as every other engine refuses it — Python's `int`
    has no width of its own to refuse it by. A `float32` is `(name, Single)`: the
    value is the binary32 nearest the number the payload carried, and a number
    past the single's range is refused the same way.

    `defaults`, for a schema a later revision of which added fields, is the JSON
    object of those fields' defaults, which the generator spells from the schema:
    a field the delivery lacks is read out of it instead, and held to its type as
    a delivered value is (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that
    grows"). A delivery with no data at all then lacks every field and reads as
    one that names none, so a field of the first revision is refused by name.

    A field of `optional<T>` is `(name, OptionalOf(declared), ...)`: JSON `null`
    reads `None`, any other value is read and held to its width as a field of
    `T` is, and a missing key is refused as any missing field is (a default of
    `null` in `defaults` reads `None`). A field of `T` that receives `null` is
    refused: its type is not optional.

    A field of a composed type, or a text or byte string held to its bound, is
    `(name, declared)` with `declared` one of the type names `read_value` takes
    (`ListOf`, `RecordOf`, `Text`, `Bytes`, `Whole`, `WideOf`, `OptionalOf` of
    any of them), and is read at the path its name begins (docs/adr/0086).

    Raises `TypedPayloadError` naming the first thing that is wrong, which the
    caller reports as `error.execution` — the answer W3C SCXML 3.13 gives for
    a guard that cannot be evaluated, and the one the script engine gives for
    the same guard on the same data.
    """
    if defaults is None:
        decoded = _decode(data)
    else:
        no_data = data is None or (isinstance(data, str) and not data.strip(_JSON_WHITESPACE))
        delivered = {} if no_data else _decode(data)
        decoded = {**json.loads(defaults), **delivered}
    values = []
    for name, declared, *width in fields:
        pointer = pointer_below("", name)
        if name not in decoded:
            raise TypedPayloadError(f"the event's data has no {name!r}",
                                    RefusalCode.MISSING, pointer)
        carried = decoded[name]
        if isinstance(declared, OptionalOf):
            # Absence is the one JSON `null` (docs/adr/0055, section 4).
            if carried is None:
                values.append(None)
                continue
            declared = declared.declared
        if carried is None:
            raise TypedPayloadError(f"{name!r} is null, and its type is not optional",
                                    RefusalCode.NULL_NOT_OPTIONAL, pointer)
        if isinstance(declared, _COMPOSED):
            # A field of a composed type, or a scalar held to its bound, is read
            # by the one reader of a value at its path.
            values.append(read_value(name, declared, carried, pointer))
            continue
        if declared is Wide:
            value = as_wide(name, carried, width[0] < 0, pointer)
        else:
            value = _as(name, declared, carried, pointer)
        if width and not width[0] <= value <= width[1]:
            raise _past_width(name, pointer, value, width[0])
        values.append(value)
    return values


class _ScanLevel:
    """One container ``lift_too_deep`` stands in: whether it is an array, the
    elements it has passed, the last text closed in it, and the member a `:`
    after that text named."""

    __slots__ = ("array", "index", "last_text", "member")

    def __init__(self, array: bool) -> None:
        self.array = array
        self.index = 0
        self.last_text: Optional[str] = None
        self.member: Optional[str] = None


def _scan_pointer(open_levels: List[_ScanLevel], path_bound: int) -> str:
    """The pointer to the place ``open_levels`` stands at, as ``lift_too_deep``
    writes it."""
    pointer = ""
    for level in open_levels:
        if level.array:
            token = str(level.index)
        else:
            try:
                name = _read_json(level.member) if level.member is not None else None
                if isinstance(name, str):
                    name.encode("utf-8")
            except (ValueError, UnicodeEncodeError):
                name = None
            if not isinstance(name, str) or "\0" in name:
                break
            token = name
        below = pointer_below(pointer, token)
        if len(below.encode("utf-8")) > path_bound:
            break
        pointer = below
    return pointer


def lift_too_deep(text: str, depth: int, path_bound: int) -> Optional[TypedPayloadError]:
    """The refusal of a container ``text`` opens deeper than ``depth``, the
    deepest a value of a lift's schema holds (docs/adr/0087, section 2.1, item
    4), or None when it opens none.

    Decided over the brackets alone, before the text is read as JSON, so that
    every runtime decides it alike and no reader descends past the type: a text
    is skipped from its opening quote to the next quote no backslash escapes,
    each `{` and `[` outside one opens a level and each `}` and `]` closes one,
    and the first opened past ``depth`` is refused, whatever the text holds
    after it. A text that never closes ends the walk with nothing refused, and
    the reading of the text as JSON refuses it.

    The pointer names the container refused (docs/adr/0087, section 2.4): for
    each container it stands in, outermost first, the element's index in an
    array (the `,` the scan met at that level since the array opened) or the
    member's name in an object (the last text closed at that level before a
    `:`, since the object opened or its last `,`), each an RFC 6901 token. It
    stops before the first place it cannot write: a member with no name before
    its colon, a name that is no text (no JSON string, or one holding U+0000),
    or a token that would take the pointer past ``path_bound`` UTF-8 bytes, the
    room of the refusal record's `path`. The bound is ``depth``. The scan keeps
    one entry per open container, at most ``depth`` of them."""
    open_levels: List[_ScanLevel] = []
    at = 0
    while at < len(text):
        c = text[at]
        top = open_levels[-1] if open_levels else None
        if c == '"':
            start = at
            at += 1
            while at < len(text) and text[at] != '"':
                at += 2 if text[at] == "\\" else 1
            if at >= len(text):
                return None
            if top is not None:
                top.last_text = text[start:at + 1]
        elif c == ":" and top is not None:
            top.member, top.last_text = top.last_text, None
        elif c == "," and top is not None:
            top.member, top.last_text = None, None
            top.index += 1
        elif c in "{[":
            if len(open_levels) >= depth:
                return TypedPayloadError(
                    f"the text opens a container deeper than the {depth} levels a value "
                    f"of its schema holds", RefusalCode.TOO_DEEP,
                    _scan_pointer(open_levels, path_bound), depth)
            open_levels.append(_ScanLevel(c == "["))
        elif c in "}]" and top is not None:
            open_levels.pop()
        at += 1
    return None


def lift_text(text: str, depth: int, path_bound: int) -> dict:
    """The text a `<sce:lift>` reads, as the members of the object it is
    (docs/adr/0087, section 2.1): ``depth`` is the deepest container a value of
    the schema holds and ``path_bound`` the room of the refusal record's `path`
    a `tooDeep` pointer is held to (``lift_too_deep``).

    Three refusals come before the fields are read, in this order: a text that
    is empty or only whitespace (`notJson`); a container opened deeper than
    ``depth``, decided over the text's brackets alone before it is read as JSON
    (``lift_too_deep``, `tooDeep`); and a text that is not one JSON object, as a
    delivery is refused (``_decode``)."""
    if not text.strip(_JSON_WHITESPACE):
        raise TypedPayloadError("the text is empty", RefusalCode.NOT_JSON, "")
    refusal = lift_too_deep(text, depth, path_bound)
    if refusal is not None:
        raise refusal
    return _decode(text)


def lift_record(text: str, depth: int, path_bound: int,
                record: RecordOf) -> Tuple[Any, Optional[TypedPayloadError]]:
    """What a `<sce:lift>` reads (docs/adr/0087, section 2.1): ``text`` as a
    value of ``record``, its fields read by the readers a delivery's are, a field
    a later revision added taking the default ``record`` names; or the refusal
    of the first place that is not one, its code, pointer and bound the
    statechart keeps as a record. ``lift_text`` says what is refused before the
    fields are read. Answers ``(value, None)`` or ``(None, refusal)``. This
    runtime keeps no log, so neither outcome is written to one."""
    try:
        return read_value("", record, lift_text(text, depth, path_bound), ""), None
    except TypedPayloadError as refusal:
        return None, refusal
