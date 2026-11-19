# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-6.4 (docs/adr/0098): a value an ``<invoke>`` hands its child is the
initial value of the child's top-level ``<data>`` of that name, in place of the
``<data>``'s own -- Python AOT channel.

The parent hands every name it evaluated; the child's start decides which it
takes, from the ids of its own top-level ``<data>``, and declares each of those
with the handed value without evaluating the ``<data>``'s ``expr``. State-local
``<data>``, early- or late-bound, takes no handed value, and a name that is the
id of no top-level ``<data>`` is not added. The start's order is the top-level
``<data>`` (seeded), the early-bound state ``<data>``, the top-level
``<script>``, then the initial entry.

The parent judges what each child reported and lands a failure in a
``<final>`` named for the sentence that broke; the fixture's header comment
says what each phase asks.

Fixture: ``integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/materialize.sh python-integration``
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_child_takes_a_handed_value_in_place_of_its_data_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime import LuaScriptEngine, ScriptError, ScriptValue  # noqa: E402

#: What reaching each verdict `<final>` says broke.
_WHY = {
    "failOwnValueRan": (
        "the own `expr` of `effectful`, which pushes onto `effects`, was "
        "evaluated although a value was handed in its place"
    ),
    "failHandedValueLost": "a top-level `<data>` kept its own value: the handed one did not arrive",
    "failStateDataTookTheValue": (
        "the early-bound `<data id='kept'>` of the child's state took the value "
        "handed under its name; only a top-level `<data>` takes one"
    ),
    "failUnmatchedNameAdded": "`stranger`, which no `<data>` of the child declares, became a variable",
    "failObjectNotWhole": "an object handed by `<param>` or `namelist` lost part of its structure",
    "failChildSharedTheParentsObject": (
        "the child's write into the object it was handed reached the parent's "
        "`record`: the child was given the parent's object, not a copy"
    ),
    "failScriptReadItsOwnValue": "the child's top-level `<script>` ran before the handed values were declared",
    "failEntryReadItsOwnValue": "the child's initial `<onentry>` ran before the handed values were declared",
    "failChildRaised": (
        "the child saw error.execution: the own `expr` of `raising` "
        "(`undefined.invalidProperty`) was evaluated, or a handed value could not be held"
    ),
    "failNamelistOwnValueRaised": (
        "the own `expr` of `listed`, handed by `namelist`, was evaluated and raised"
    ),
    "failNamelistValueLost": "the value `namelist` handed for `listed` did not arrive",
    "failNamelistNameAdded": "the `namelist` name `stray`, which the child does not declare, became a variable",
    "failLateStateDataTookTheValue": (
        "a late-bound state `<data>` took the value handed under its name when its state was entered"
    ),
    "failParentRaised": "the parent raised error.execution while it evaluated what it hands",
}

#: Where a run that never reached a `<final>` stood, and what that says.
_STRANDED = {
    "byParam": "the child handed `<param>`s never reported",
    "byNamelist": "the child handed `namelist` values never reported",
    "lateBound": "the late-bound child never reported",
    "firstMacrostep": (
        "the child's first macrostep read its own value of `handed`, not the "
        "handed one, and it stands in `missed`"
    ),
}

#: The id the parent is started under in the test that reads its session.
_ROOT = "root"


def test_a_child_takes_a_handed_value_in_place_of_its_data_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 200:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "a_child_takes_a_handed_value_in_place_of_its_data did not reach a "
        f"top-level <final> within 200 ms; last leaf={engine.current_state}: "
        f"{_STRANDED.get(str(engine.current_state), 'no phase names this state')}"
    )
    actual = str(engine.terminal_state)
    assert actual == "pass", (
        f"reached <final id={actual!r}>: {_WHY.get(actual, 'unknown verdict state')}"
    )


def test_a_start_takes_the_values_handed_to_it_once() -> None:
    # The parent document stands in for a child here: it is handed values as an
    # `<invoke>` would hand them, and its own session is read.
    lua = LuaScriptEngine()
    lua.initialize()
    engine = _sm.create_engine(script_engine=lua, session_id=_ROOT)
    engine.policy.hand_values(
        {
            "listed": ScriptValue.of("given by the test"),
            "undeclaredName": ScriptValue.of("leaked"),
        }
    )
    engine.initialize()
    assert lua.evaluate_expression(_ROOT, "listed").string_val == "given by the test", (
        "the top-level `<data id='listed'>` did not take the value handed under its id"
    )
    with pytest.raises(ScriptError):
        # A name that is the id of no top-level `<data>` is not added, so it
        # reads as a variable nothing declares.
        lua.evaluate_expression(_ROOT, "undeclaredName")

    engine.stop()
    engine.initialize()
    assert lua.evaluate_expression(_ROOT, "listed").string_val == "handed listed", (
        "a second start of the same policy took the values the first start was "
        "handed; a start consumes what it is handed"
    )
