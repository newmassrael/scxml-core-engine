# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A typed host-run request (``sce:request``, SCE Accepted Subset §2.12).

A request crosses to the host as text, like every ``<param>``. What makes it
typed is that each value is held to its field's type where the invocation
starts — `request_field_wire` — so the text a host reads back is always one
its field's type parses. The Python port of
``sce_rust_runtime::host_processor``'s typed-request half; the rule is the same
in every runtime.
"""

from __future__ import annotations

import math
import struct
from dataclasses import dataclass
from typing import Any

from .event_payload import TypedPayloadError
from .host_processor import HostInvokeRequest
from .scripting.i_script_engine import ScriptValue, ScriptValueKind


@dataclass(frozen=True)
class RequestFieldType:
    """The type one field of a typed request declares, as the generated start
    site names it: ``uint8`` … ``int64``, ``float32``, ``float64``, ``bool``,
    ``string``, or ``bytes`` with its ``sce:max-size`` in `cap`."""

    kind: str
    cap: int = 0


_WHOLE_BOUNDS = {
    "uint8": (0, (1 << 8) - 1),
    "uint16": (0, (1 << 16) - 1),
    "uint32": (0, (1 << 32) - 1),
    "uint64": (0, (1 << 64) - 1),
    "int8": (-(1 << 7), (1 << 7) - 1),
    "int16": (-(1 << 15), (1 << 15) - 1),
    "int32": (-(1 << 31), (1 << 31) - 1),
    "int64": (-(1 << 63), (1 << 63) - 1),
}

#: The largest finite float32, past which narrowing a double gives infinity.
_FLOAT32_MAX = struct.unpack("<f", b"\xff\xff\x7f\x7f")[0]


def _narrow_float32(value: float) -> float:
    return struct.unpack("<f", struct.pack("<f", value))[0]


def request_field_wire(value: ScriptValue, name: str, field_type: RequestFieldType) -> str:
    """Hold one evaluated ``<param>`` to the field it supplies, and spell it
    for the request.

    §scxml-6.4.1: an argument that cannot be evaluated starts nothing, and a
    value the record's field cannot hold is such an argument — the host was
    promised that record. The refusal is a `TypedPayloadError`, whose sentence
    the ``error.execution`` event carries, because it is the judgement the
    payload lift makes on a completion that does not fit its record, made on
    the other half of the same invocation.

    The text returned is the value at the field's type — a whole number's
    digits, a fraction as every untyped ``<param>`` spells one — which is what
    `request_field` parses back, so the adapter reading a checked request
    cannot fail. A byte string rides as its byte-exact Latin-1 text, the
    spelling a completion's byte field uses.
    """
    kind = field_type.kind
    if kind in _WHOLE_BOUNDS:
        if value.kind is ScriptValueKind.INT:
            whole = value.int_val
        elif value.kind is ScriptValueKind.DOUBLE:
            f = value.double_val
            if not math.isfinite(f) or f != math.trunc(f):
                raise TypedPayloadError(f"{name!r} is not a whole number")
            whole = int(f)
        else:
            raise TypedPayloadError(f"{name!r} is not a number")
        lo, hi = _WHOLE_BOUNDS[kind]
        if not lo <= whole <= hi:
            raise TypedPayloadError(
                f"{name!r} does not fit the width its schema declares ({whole})")
        return str(whole)
    if kind in ("float32", "float64"):
        if value.kind is ScriptValueKind.INT:
            f = float(value.int_val)
        elif value.kind is ScriptValueKind.DOUBLE:
            f = value.double_val
        else:
            raise TypedPayloadError(f"{name!r} is not a number")
        # JSON, which a completion's record crosses as, has no spelling for
        # these, so a request may not carry one either.
        if not math.isfinite(f):
            raise TypedPayloadError(f"{name!r} is not a finite number")
        if kind == "float32":
            if abs(f) > _FLOAT32_MAX:
                raise TypedPayloadError(
                    f"{name!r} does not fit the width its schema declares")
            f = _narrow_float32(f)
        # Spelled as every untyped `<param>` is (ECMAScript's `String()`), of
        # the value at its declared width.
        return ScriptValue(kind=ScriptValueKind.DOUBLE, double_val=f).to_wire_string()
    if kind == "bool":
        if value.kind is ScriptValueKind.BOOL:
            return "true" if value.bool_val else "false"
        raise TypedPayloadError(f"{name!r} is not a truth value")
    if kind == "string":
        if value.kind is ScriptValueKind.STRING:
            return value.string_val
        raise TypedPayloadError(f"{name!r} is not a text")
    if kind == "bytes":
        if value.kind is not ScriptValueKind.STRING:
            raise TypedPayloadError(f"{name!r} is not a byte string")
        text = value.string_val
        if any(ord(c) > 0xFF for c in text):
            raise TypedPayloadError(
                f"{name!r} carries a character above U+00FF, which no single "
                f"byte spells")
        if len(text) > field_type.cap:
            raise TypedPayloadError(
                f"{name!r} is {len(text)} bytes, past the {field_type.cap} its "
                f"schema declares")
        return text
    raise ValueError(f"request_field_wire: unknown field type {kind!r}")


def request_field(request: HostInvokeRequest, name: str, field_type: RequestFieldType) -> Any:
    """One field of a checked typed request, read back at its declared type
    by the generated adapter.

    Raises `AssertionError` when the request does not carry the field as text
    its type parses. A request that reached a typed adapter was checked field
    by field where it started (`request_field_wire`), so this is a broken
    promise between two halves of generated code, not a value a host or
    document can supply — and one that would otherwise hand the host a record
    the document never sent.
    """
    values = request.params.get(name, [])
    if len(values) != 1:
        raise AssertionError(
            f"typed request {request.invoke_id!r} does not carry {name!r} "
            f"exactly once, though its record declares it")
    text = values[0]
    kind = field_type.kind
    try:
        if kind in _WHOLE_BOUNDS:
            lo, hi = _WHOLE_BOUNDS[kind]
            whole = int(text, 10)
            if not lo <= whole <= hi:
                raise ValueError(text)
            return whole
        if kind in ("float32", "float64"):
            return float(text)
        if kind == "bool":
            if text not in ("true", "false"):
                raise ValueError(text)
            return text == "true"
        if kind == "string":
            return text
        if kind == "bytes":
            return text.encode("latin-1")
    except (ValueError, UnicodeEncodeError) as exc:
        raise AssertionError(
            f"typed request {request.invoke_id!r} carries {name!r} as {text!r}, "
            f"which its type does not parse, though the start site checked it"
        ) from exc
    raise ValueError(f"request_field: unknown field type {kind!r}")
