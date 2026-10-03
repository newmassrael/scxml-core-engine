# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated Python.

A variable is an attribute of the policy, every expression was lowered to Python
at build time, and no datamodel lives in a script engine.

The scenarios here are the ones Kotlin, Rust, C++, Go and the Interpreter
replay -- ``sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json``,
whose expected values are derived from the document, not observed from a
backend -- so the engines are held to one answer. A scenario is a list of steps,
each an external event with its payload and what the machine must hold after it
runs to quiescence: its current state and any of its published variables.

The machines are the ones ``scripts/regen_static_datamodel_python.sh`` generates,
into this directory and out of version control, with the algorithms they call as
modules beside them: the directory is a package, and a machine is imported as
one of its members so that its ``from . import <algorithm>`` resolves. Those
with no scenario here are generated for their effect: a machine nobody built is
a machine nobody type-checked.

Regeneration (after a fixture or template edit):
  ``scripts/regen_static_datamodel_python.sh`` (local)
"""
from __future__ import annotations

import dataclasses
import importlib
import json
import re
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import EventMetadata  # noqa: E402 -- path inserted above

# Where the scenarios live, from this file's directory.
_SCENARIOS = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "scenarios"


def _atomic_state(engine) -> str:
    """The current state is the atomic one: a compound state is active for as
    long as one of its children is, and is no more where the machine is."""
    leaves = engine.active_leaves
    if len(leaves) != 1:
        return f"<{len(leaves)} active atomic states>"
    return str(leaves[0])


def _read(policy, readers: dict, variable: str):
    """What the machine holds in ``variable``: through the reader it publishes,
    or -- for a variable the machine keeps to itself, which has none, so that
    renaming it never changes what a host was written against -- from its
    attribute. Reading is all this does."""
    if variable in readers:
        return getattr(policy, readers[variable])()
    attribute = "v_" + re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", variable).lower()
    assert hasattr(policy, attribute), f"the machine holds no variable {variable!r}"
    return getattr(policy, attribute)


def _as_json(value):
    """``value`` as the scenario writes it: an enum member by the name its
    document declares, a record by the schema's field ids, a list element by
    element -- what a host reads back, in the shape JSON states."""
    if hasattr(value, "sce_name"):
        return value.sce_name
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {f.name: _as_json(getattr(value, f.name)) for f in dataclasses.fields(value)}
    if isinstance(value, (list, tuple)):
        return [_as_json(item) for item in value]
    return value


def replay(name: str, machine: str | None = None) -> None:
    """Run scenario ``name`` against the machine it names. Every step names an
    event (or none, for the machine as started) and what it must hold
    afterwards."""
    scenario = json.loads((_SCENARIOS / f"{name}.json").read_text())
    steps = scenario["steps"]
    assert steps, f"scenario {name} has no steps, which judges nothing"
    module = importlib.import_module(
        f"integration.static_datamodel.{machine or scenario['machine']}_sm"
    )
    engine = module.create_engine()
    engine.initialize()
    policy = engine.policy
    readers = module.SCE_HOST_NAMES["readers"]
    for n, step in enumerate(steps):
        where = f"{name} step {n}: {step.get('note', '')}"
        event_name = step.get("event")
        if event_name is not None:
            event = policy.get_event_from_name(event_name)
            assert event is not None, f"{where}: the machine declares no event {event_name!r}"
            data = json.dumps(step["data"]) if "data" in step else ""
            engine.send_event(event, EventMetadata(data=data))
        expect = step["expect"]
        if expect.get("ended"):
            assert engine.reached_final, f"{where}: the machine ended in a top-level <final>"
            if "donedata" in expect:
                # What the final's `<donedata>` left for an invoking parent, as
                # the JSON text every producer writes.
                got = json.loads(engine.done_data)
                assert got == expect["donedata"], (
                    f"{where}: the donedata is {got!r}, not {expect['donedata']!r}"
                )
            continue
        if "state" in expect:
            assert _atomic_state(engine) == expect["state"], where
        for variable, want in expect.get("variables", {}).items():
            got = _as_json(_read(policy, readers, variable))
            assert got == want, f"{where}: variable {variable!r} is {got!r}, not {want!r}"


# A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and
# `<elseif>` are native, and `In()` asks the machine's own configuration.
def test_the_counter_counts_to_its_flag_and_lets_go() -> None:
    replay("static_counter")


def test_the_counter_stops_at_its_bound_and_refuses_go() -> None:
    replay("static_counter_bound")


# A checked integer operation that overflows fails instead of wrapping.
def test_an_overflowing_operation_fails_instead_of_wrapping() -> None:
    replay("static_overflow")


# An error ends the block it stands in (W3C SCXML 4.9): the statements after a
# failed `<assign>`, after a failed `<if>` cond, or inside a branch that failed
# do not run, while the next block does.
def test_an_error_ends_the_block_it_stands_in() -> None:
    replay("static_block_ends")


# An event's typed payload is read in a guard and in assignments.
def test_an_events_typed_payload_is_read_in_a_guard_and_in_content() -> None:
    replay("static_payload")


# An enum variable holds a variant of its enum, read back by the name the enum
# document declares.
def test_an_enum_variable_holds_a_variant_of_its_enum() -> None:
    replay("static_enum")


# A list is filled to its bound and emptied.
def test_a_list_is_filled_to_its_bound_and_emptied() -> None:
    replay("static_list")


# A `<foreach>` walks a list variable.
def test_a_foreach_walks_a_list_variable() -> None:
    replay("static_foreach")


# An append that fails ends its block, the list as it was.
def test_an_append_that_fails_ends_its_block() -> None:
    replay("static_block_ends_list")


# A record is built whole from its `<sce:set>`s and updated a field at a time,
# from the machine's own value and from a typed event payload.
def test_a_record_is_built_whole_and_updated_a_field_at_a_time() -> None:
    replay("static_record_fields")


# A list of records is filled by name from a record variable or a loop's item,
# walked by a `<foreach>`, and a record is taken whole.
def test_a_list_holds_records_and_a_foreach_walks_them() -> None:
    replay("static_record_list")


# A record may hold an enum: its field is read back by the name the enum
# document declares.
def test_a_record_holds_an_enum_field() -> None:
    replay("static_record_enum")


# A guard calls an imported algorithm with the record's own fields, and the
# call is the module the algorithm's own generation put beside the machine.
def test_a_guard_calls_an_imported_algorithm() -> None:
    replay("static_record")


# A sync run composed of the standard sync rules: each rule an imported
# algorithm, called from a guard or an assignment, and the run's payloads the
# standard event schemas'.
def test_a_sync_run_is_composed_of_the_standard_sync_rules() -> None:
    replay("sync_client")


# A `<send>` hands its event the pairs of its `<param>`s, each read from the
# machine's fields when the send runs; a pair whose value failed is left out, the
# message still goes, and the receiver finds the field missing.
def test_a_send_hands_its_event_the_pairs_of_its_params() -> None:
    replay("static_send_params")


# A top-level final hands its done event the pairs of its `<donedata>`, each read
# from the machine's fields when the state is entered; a pair whose value failed
# is left out and the others cross.
def test_a_final_hands_its_done_event_the_pairs_of_its_donedata() -> None:
    replay("static_donedata")
