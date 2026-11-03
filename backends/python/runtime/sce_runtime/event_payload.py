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
at the one point an event is dequeued. What the lift refuses is what the SCRIPT
ENGINE already refuses for the same guard, measured on the same document:

    no data at all              error.execution, guard false
    a field the data lacks      error.execution, guard false
    a value of another type     error.execution, guard false
    the fields, well typed      the guard evaluates

A native lowering that answered differently from the engine would make the
optimisation observable, which is the one thing it may not be.
"""

from __future__ import annotations

import json
import math
import struct
from typing import Any, Callable, Dict, Sequence

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
    """Names ``list<T>[bound]``, around what names ``T``: an array of at most
    ``bound`` elements, each a value of ``T``, read in order. The bound is held
    before any element is read (ADR 0077, section 2)."""

    __slots__ = ("declared", "bound")

    def __init__(self, declared: Any, bound: int) -> None:
        self.declared = declared
        self.bound = bound

    def __repr__(self) -> str:
        return f"ListOf({self.declared!r}, {self.bound})"


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


_COMPOSED = (Whole, WideOf, Text, Bytes, ListOf, RecordOf)


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
        raise TypedPayloadError(
            "the schema's defaults are not a JSON object, which the generator "
            "never writes")
    return parsed


def read_value(path: str, declared: Any, value: Any) -> Any:
    """One value of an event's data as its type declares it, at the path through
    every depth that names it (docs/adr/0086, section 4): a field's name, then
    each index and field name below it, joined by ``/``.

    ``declared`` is what a field spec names a type by: a scalar (``int``,
    ``bool``, ``float``, ``Single``, ``str``, ``bytes``, an enum type), or one of
    ``Whole``, ``WideOf``, ``Text``, ``Bytes``, ``OptionalOf``, ``ListOf`` and
    ``RecordOf``, composed to any depth. A value outside its type is refused
    before anything of it is kept, the refusal naming the path."""
    if isinstance(declared, OptionalOf):
        # Absence is the one JSON `null` (docs/adr/0055, section 4).
        return None if value is None else read_value(path, declared.declared, value)
    if value is None:
        raise TypedPayloadError(f"{path!r} is null, and its type is not optional")
    if isinstance(declared, ListOf):
        if not isinstance(value, list):
            raise TypedPayloadError(f"{path!r} is not a list")
        if len(value) > declared.bound:
            raise TypedPayloadError(
                f"{path!r} holds {len(value)} elements, past the {declared.bound} "
                f"its type declares")
        return [read_value(_join(path, str(index)), declared.declared, item)
                for index, item in enumerate(value)]
    if isinstance(declared, RecordOf):
        if not isinstance(value, dict):
            raise TypedPayloadError(f"{path!r} is not a record")
        members = value
        if declared.defaults is not None:
            members = {**_defaults_of(declared.defaults), **value}
        held = []
        for name, field_type in declared.fields:
            field_path = _join(path, name)
            if name not in members:
                raise TypedPayloadError(f"the event's data has no {field_path!r}")
            held.append(read_value(field_path, field_type, members[name]))
        if declared.make is not None:
            return declared.make(*held)
        return {name: v for (name, _), v in zip(declared.fields, held)}
    if isinstance(declared, Text):
        text = _as(path, str, value)
        size = _utf8_size(text)
        if size > declared.bound:
            raise TypedPayloadError(
                f"{path!r} is {size} bytes, past the {declared.bound} its type declares")
        return text
    if isinstance(declared, Bytes):
        held_bytes = _as(path, bytes, value)
        if len(held_bytes) > declared.bound:
            raise TypedPayloadError(
                f"{path!r} is {len(held_bytes)} bytes, past the {declared.bound} "
                f"its type declares")
        return held_bytes
    if isinstance(declared, (Whole, WideOf)):
        if isinstance(declared, WideOf):
            whole = as_wide(path, value, declared.low < 0)
        else:
            whole = _as(path, int, value)
        if not declared.low <= whole <= declared.high:
            raise TypedPayloadError(
                f"{path!r} does not fit the width its schema declares ({whole!r})")
        return whole
    return _as(path, declared, value)


class BoundError(ValueError):
    """A value a host built is past a bound its type declares: raised where the
    value is made, by a generated record's ``__post_init__``."""


def check_value(path: str, declared: Any, value: Any) -> None:
    """Hold a value a host built to its type at every depth, before anything
    reads it (docs/adr/0086, section 5.4): what a generated record's
    ``__post_init__`` runs over each field, through the same type names the
    reader takes. Raises ``BoundError`` naming the path."""
    if isinstance(declared, OptionalOf):
        if value is not None:
            check_value(path, declared.declared, value)
        return
    if isinstance(declared, ListOf):
        if len(value) > declared.bound:
            raise BoundError(
                f"{path!r} holds {len(value)} elements, past the {declared.bound} "
                f"its type declares")
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


def as_wide(name: str, value: Any, signed: bool) -> int:
    """A 64-bit field's value: the text of its digits, or a whole number exact in a
    double (`|n| <= 2^53 - 1`). A truth value is not a number here."""
    if isinstance(value, bool):
        raise TypedPayloadError(f"{name!r} is not a whole number ({value!r})")
    if isinstance(value, str):
        if not _is_decimal(value, signed):
            raise TypedPayloadError(
                f"{name!r} is not a whole number written as digits ({value!r})")
        return int(value)
    if isinstance(value, int):
        if abs(value) > _EXACT_IN_A_DOUBLE:
            raise TypedPayloadError(
                f"{name!r} is a number past 2^53 - 1, which is not exact everywhere "
                f"({value!r}); a 64-bit integer that large is written as a text of "
                f"its digits")
        return value
    raise TypedPayloadError(f"{name!r} is not a whole number ({value!r})")


def as_single(value: float) -> float:
    """`value` rounded to the nearest binary32, as the double that holds it
    exactly; a value past the single's range is the infinity it is there."""
    try:
        return struct.unpack("<f", struct.pack("<f", value))[0]
    except OverflowError:
        return math.inf if value > 0 else -math.inf


class TypedPayloadError(Exception):
    """Why this event's data cannot be read as its schema's fields."""


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
        raise TypedPayloadError("the event carries no data")
    if isinstance(data, (bytes, bytearray)):
        data = data.decode("utf-8", errors="replace")
    if isinstance(data, str):
        try:
            decoded = json.loads(data)
        except ValueError as exc:
            rung = payload_reading_of_text(data)
            reading = (
                "announced structure and could not be read"
                if rung is PayloadReading.UNDECODABLE
                else "is a space-normalized string, which names no fields")
            raise TypedPayloadError(
                f"the event's data {reading} (B.2.8.1: {rung.value})") from exc
        if not isinstance(decoded, dict):
            raise TypedPayloadError(
                "the event's data is a bare value, and this event's schema "
                "declares named fields")
        return decoded
    raise TypedPayloadError(
        f"the event's data is a {type(data).__name__}, which names no fields")


def _as(name: str, declared: type, value: Any) -> Any:
    """One field's value as its schema declares it, or a refusal.

    ⚠ A truth value is not a number here, and a number is not a truth value.
    JSON spells both, and a schema that declared `uint32` and received `true`
    has been handed something its own type says cannot occur.
    """
    if declared is bool:
        if isinstance(value, bool):
            return value
        raise TypedPayloadError(f"{name!r} is not a truth value ({value!r})")
    if declared is int:
        if isinstance(value, bool) or not isinstance(value, int):
            raise TypedPayloadError(f"{name!r} is not a whole number ({value!r})")
        return value
    if declared is Single:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise TypedPayloadError(f"{name!r} is not a number ({value!r})")
        number = float(value)
        if abs(number) > FLOAT32_MAX:
            raise TypedPayloadError(
                f"{name!r} does not fit the width its schema declares ({value!r})")
        return as_single(number)
    if declared is float:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise TypedPayloadError(f"{name!r} is not a number ({value!r})")
        return float(value)
    if declared is str:
        if not isinstance(value, str):
            raise TypedPayloadError(f"{name!r} is not a text ({value!r})")
        return value
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
                    f"single byte spells") from exc
        raise TypedPayloadError(f"{name!r} is not a byte string ({value!r})")
    if isinstance(declared, type) and hasattr(declared, "sce_name"):
        # An enum a `sce-static` machine declares: the wire carries the name its
        # document declares for the value (`sce_name`), as a saved state does,
        # and a name the enum does not declare is a payload that does not fit
        # its schema.
        if not isinstance(value, str):
            raise TypedPayloadError(f"{name!r} is not a text ({value!r})")
        for member in declared:
            if member.sce_name == value:
                return member
        raise TypedPayloadError(
            f"{name!r} ({value!r}) is not a variant of {declared.__name__}")
    raise TypedPayloadError(
        f"{name!r} is declared {declared!r}, and this module has no reading "
        f"for it")


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
        no_data = data is None or (isinstance(data, str) and not data.strip())
        delivered = {} if no_data else _decode(data)
        decoded = {**json.loads(defaults), **delivered}
    values = []
    for name, declared, *width in fields:
        if name not in decoded:
            raise TypedPayloadError(f"the event's data has no {name!r}")
        carried = decoded[name]
        if isinstance(declared, OptionalOf):
            # Absence is the one JSON `null` (docs/adr/0055, section 4).
            if carried is None:
                values.append(None)
                continue
            declared = declared.declared
        if carried is None:
            raise TypedPayloadError(f"{name!r} is null, and its type is not optional")
        if isinstance(declared, _COMPOSED):
            # A field of a composed type, or a scalar held to its bound, is read
            # by the one reader of a value at its path.
            values.append(read_value(name, declared, carried))
            continue
        if declared is Wide:
            value = as_wide(name, carried, width[0] < 0)
        else:
            value = _as(name, declared, carried)
        if width and not width[0] <= value <= width[1]:
            raise TypedPayloadError(
                f"{name!r} does not fit the width its schema declares ({value!r})")
        values.append(value)
    return values
