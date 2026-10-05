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
from typing import Any, Sequence

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


def lift(data: Any, fields: Sequence[tuple]) -> list:
    """Every field of this event's schema, in declaration order.

    A field is `(name, declared)`, or `(name, int, low, high)` for an integer of
    the width its schema declares: a value past that width is a payload that does
    not fit the schema, refused as every other engine refuses it — Python's `int`
    has no width of its own to refuse it by. A `float32` is `(name, Single)`: the
    value is the binary32 nearest the number the payload carried, and a number
    past the single's range is refused the same way.

    Raises `TypedPayloadError` naming the first thing that is wrong, which the
    caller reports as `error.execution` — the answer W3C SCXML 3.13 gives for
    a guard that cannot be evaluated, and the one the script engine gives for
    the same guard on the same data.
    """
    decoded = _decode(data)
    values = []
    for name, declared, *width in fields:
        if name not in decoded:
            raise TypedPayloadError(f"the event's data has no {name!r}")
        value = _as(name, declared, decoded[name])
        if width and not width[0] <= value <= width[1]:
            raise TypedPayloadError(
                f"{name!r} does not fit the width its schema declares ({value!r})")
        values.append(value)
    return values
