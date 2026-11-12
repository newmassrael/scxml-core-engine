# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-6.4 (docs/adr/0098): a value an ``<invoke>`` hands its child is the
initial value of the child's top-level ``<data>`` of that name, in place of the
``<data>``'s own -- Python AOT channel.

The parent hands every name it evaluated; the child's start decides which it
takes, from the ids of its own top-level ``<data>``, and declares each of those
with the handed value without evaluating the ``<data>``'s ``expr``. State-local
``<data>`` takes no handed value, and a name that is the id of no top-level
``<data>`` is not added. The start's order is the top-level ``<data>`` (seeded),
the early-bound state ``<data>``, the top-level ``<script>``, then the initial
entry.

Each region of the document's ``<parallel>`` invokes one child and lands in a
``<final>`` named for what the child saw, so each test below reads one region.
The document's header comment says what each child asks.

Fixture: ``a_child_takes_a_handed_value_in_place_of_its_data.scxml``, beside
this file. It is not under ``integration_resources/``: the canonical fixture of
docs/adr/0098 lands there once every channel meets the rule.

Regeneration (after a template or document edit):
  ``scripts/regen_a_child_takes_a_handed_value_in_place_of_its_data_python.sh``
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

#: The regions of `cases`, one child each.
_REGIONS = ("failing", "effect", "local", "strayCase", "namelistCase", "whole", "ordered")

#: The id the parent is started under in the test that reads its session.
_ROOT = "root"


def _settled():
    """The parent, started and run until every region has reached a `<final>`
    (or 200 ms of its clock have passed, which no child here needs)."""
    engine = _sm.create_engine()
    engine.initialize()
    elapsed = 0
    while not _every_region_ended(engine) and elapsed < 200:
        engine.advance_time(10)
        elapsed += 10
    return engine


def _every_region_ended(engine) -> bool:
    policy = engine.policy
    return all(
        engine.is_in_final_state(policy.get_state_from_name(region)) for region in _REGIONS
    )


def _where(engine, region: str) -> str:
    """The name of the active child of `region`: the `<final>` its child's
    report sent it to, or its running state when no report arrived."""
    policy = engine.policy
    region_state = policy.get_state_from_name(region)
    assert region_state is not None, f"the document declares no region {region!r}"
    for state in engine.active_configuration():
        if policy.get_parent(state) == region_state:
            return policy.get_state_name(state)
    return "<no active child>"


def test_a_handed_value_takes_the_place_of_data_whose_own_value_would_fail() -> None:
    engine = _settled()
    where = _where(engine, "failing")
    assert where == "failingHanded", (
        f"the `failing` region ended in {where!r}. `failingRaised` means the child "
        "evaluated `fragile`'s own `expr` (`nothing.deep`) although the parent "
        "handed `fragile` a value, and raised its `error.execution`: §scxml-6.4 "
        "says the value specified in the `<data>` is ignored. `failingLost` means "
        "the handed value did not arrive"
    )


def test_a_handed_value_takes_the_place_of_data_whose_own_value_has_an_effect() -> None:
    engine = _settled()
    where = _where(engine, "effect")
    assert where == "effectAbsent", (
        f"the `effect` region ended in {where!r}. `effectRan` means the child "
        "evaluated `effectful`'s own `expr`, which assigns `effects`, although "
        "the parent handed `effectful` a value. `effectLost` means the handed "
        "value did not arrive"
    )


def test_a_state_local_data_keeps_its_own_value_under_a_handed_name() -> None:
    engine = _settled()
    where = _where(engine, "local")
    assert where == "localOwn", (
        f"the `local` region ended in {where!r}: the child's state-local "
        "`localName` took the value the parent handed under that name, where only "
        "a top-level `<data>` is seeded"
    )


def test_a_handed_param_naming_no_top_level_data_is_not_added() -> None:
    engine = _settled()
    where = _where(engine, "strayCase")
    assert where == "strayAbsent", (
        f"the `strayCase` region ended in {where!r}. `strayLeaked` means the "
        "`<param name='nowhere'>` the child declares no `<data>` for became a "
        "variable of the child. `strayLost` means `declared`, which it does "
        "declare, did not take the handed value"
    )


def test_a_handed_namelist_value_is_taken_whole_and_an_undeclared_one_is_not_added() -> None:
    engine = _settled()
    where = _where(engine, "namelistCase")
    assert where == "namelistTaken", (
        f"the `namelistCase` region ended in {where!r}. `namelistLeaked` means "
        "the namelist's `stray`, which the child declares no `<data>` for, became "
        "a variable of the child. `namelistLost` means `record` did not arrive as "
        "the parent's object"
    )


def test_an_object_handed_value_arrives_whole() -> None:
    engine = _settled()
    where = _where(engine, "whole")
    assert where == "wholeArrived", (
        f"the `whole` region ended in {where!r}: the object the parent handed as "
        "`handedRecord` did not arrive with its array and its nested object as "
        "the parent evaluated them"
    )


def test_the_top_level_script_and_the_initial_onentry_read_the_handed_value() -> None:
    engine = _settled()
    where = _where(engine, "ordered")
    assert where == "orderedInPlace", (
        f"the `ordered` region ended in {where!r}: the child's top-level "
        "`<script>` or its initial `<onentry>` read `greeting` before the handed "
        "value was in place. The handed value is the `<data>`'s initial value, so "
        "it is there before either runs"
    )


def test_a_start_takes_the_values_handed_to_it_once() -> None:
    # The parent document stands in for a child here: it is handed values as an
    # `<invoke>` would hand them, and its own session is read.
    lua = LuaScriptEngine()
    lua.initialize()
    engine = _sm.create_engine(script_engine=lua, session_id=_ROOT)
    engine.policy.hand_values(
        {
            "stray": ScriptValue.of("handed"),
            "undeclaredName": ScriptValue.of("leaked"),
        }
    )
    engine.initialize()
    assert lua.evaluate_expression(_ROOT, "stray").string_val == "handed", (
        "the top-level `<data id='stray'>` did not take the value handed under its id"
    )
    with pytest.raises(ScriptError):
        # A name that is the id of no top-level `<data>` is not added, so it
        # reads as a variable nothing declares.
        lua.evaluate_expression(_ROOT, "undeclaredName")

    engine.stop()
    engine.initialize()
    assert lua.evaluate_expression(_ROOT, "stray").string_val == "stray", (
        "a second start of the same policy took the values the first start was "
        "handed; a start consumes what it is handed"
    )
