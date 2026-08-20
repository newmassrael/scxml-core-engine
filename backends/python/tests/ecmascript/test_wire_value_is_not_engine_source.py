# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-C-2 — a form-encoded `<param>` carries the value, not the sender's language.

The BasicHTTP Event I/O Processor sends each `<param>` as one
`name=value` pair, so the value crosses as text and the receiving end
hands that text to `_event.data`; no script engine reads it at either
end. This channel used to render it with `str(value.to_python())`, which
is *Python's* spelling of the value and not the document's.

Measured 2026-08-21, one field had six answers across six backends:

    value        C++     Rust/Go     Python    Kotlin   C11
    ─────────────────────────────────────────────────────────────
    null         ""      "nil"       "None"    ""       "nil"
    true         true    true        "True"    true     true
    5.0          5       5.0         5.0       5.0      5.0
    [1, 2]       [1,2]   {1, 2}      [1, 2]    [1,2]    table: 0x…

A peer at the far end of a socket cannot be expected to know which
backend compiled the sender, so all six now answer the column C++
`ScriptResultUtils::resultToString` already gave — which is also
ECMAScript's `String(value)`, with absence empty (§scxml-C-1) and a
structured value as JSON.

The sibling that makes the same claim in the other channels:
`backends/rust/runtime/tests/wire_value_is_not_engine_source.rs` and
`backends/go/runtime/wire_value_test.go`. The rows below are theirs.
"""

from __future__ import annotations

import pytest

from sce_runtime.scripting.i_script_engine import ScriptValue, ScriptValueKind


def _object(pairs: dict) -> ScriptValue:
    return ScriptValue(
        kind=ScriptValueKind.OBJECT,
        object_val={k: ScriptValue.of(v) for k, v in pairs.items()},
    )


# Each row's comment is what this channel used to put on the wire.
WIRE_ROWS = [
    (ScriptValue.null(), ""),  # was "None"
    (ScriptValue.undefined(), ""),  # was "None"
    (ScriptValue.of(True), "true"),  # was "True"
    (ScriptValue.of(False), "false"),  # was "False"
    (ScriptValue.of(42), "42"),
    (ScriptValue.of(5.0), "5"),  # was "5.0"
    (ScriptValue.of(2.5), "2.5"),
    (ScriptValue.of("plain"), "plain"),
    # The quotes belong to the value; a rendering that adds its own would
    # have to trim them again, and the trim ate these.
    (ScriptValue.of('"quoted"'), '"quoted"'),
    (ScriptValue.of([1, 2]), "[1,2]"),  # was "[1, 2]"
    (_object({"k": "v"}), '{"k":"v"}'),  # was "{'k': 'v'}"
    (
        ScriptValue(kind=ScriptValueKind.DOM, dom_val="<r><c/></r>"),
        "<r><c/></r>",
    ),
]


@pytest.mark.parametrize("value,expected", WIRE_ROWS)
def test_a_wire_param_reads_the_same_whoever_sent_it(value, expected):
    assert value.to_wire_string() == expected


def test_the_structured_arm_is_the_payload_encoder():
    """Both directions out of the process are read by a parser, so they
    are the same bytes — the wire form does not invent a second JSON."""
    value = ScriptValue.of({"b": 2, "a": [1, "x"]})
    assert value.to_wire_string() == value.to_json_literal()


def test_a_string_is_quoted_as_json_and_bare_on_the_wire():
    """The one case where the difference is easiest to lose."""
    value = ScriptValue.of("v")
    assert value.to_json_literal() == '"v"'
    assert value.to_wire_string() == "v"


def test_the_neutral_value_no_longer_knows_a_language():
    """`ScriptValue.to_lua_literal` was a port of the Rust defect that
    never acquired a caller here — Python's `<invoke>` path hands the
    child ScriptValues rather than source. Its absence is the assertion:
    a value that has met no engine must not be able to spell one."""
    assert not hasattr(ScriptValue.null(), "to_lua_literal")
