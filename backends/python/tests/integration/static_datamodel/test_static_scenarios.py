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
import inspect
import json
import re
import sys
from pathlib import Path

import pytest

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
    # A byte string as its byte-exact Latin-1 text, each byte the character of
    # that code point (docs/adr/0005, decision 2).
    if isinstance(value, bytes):
        return value.decode("latin-1")
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {f.name: _as_json(getattr(value, f.name)) for f in dataclasses.fields(value)}
    if isinstance(value, (list, tuple)):
        return [_as_json(item) for item in value]
    return value


class _ScenarioHost:
    """What a machine asked of its host, as a scenario's ``host_calls`` writes it:
    the name the document gives the action, and its arguments in the order the
    document gives them."""

    def __init__(self) -> None:
        self._calls: list[dict] = []

    def show_attempts(self, count: int, exhausted: bool) -> None:
        self._calls.append({"action": "showAttempts", "args": [count, exhausted]})

    def take(self) -> list[dict]:
        """The calls made since this was last called, oldest first."""
        taken, self._calls = self._calls, []
        return taken


def replay(name: str, machine: str | None = None) -> None:
    """Run scenario ``name`` against the machine it names. Every step names an
    event (or none, for the machine as started) and what it must hold
    afterwards. A name no event of the machine matches is a misspelt step unless
    the step says it expects the drop (``"dropped": true``), which is then what
    is held. A name matches an event of the machine as the policy resolves it
    (``resolve_event_by_name``, §scxml-3.12.1): the document need not write it."""
    scenario = json.loads((_SCENARIOS / f"{name}.json").read_text())
    steps = scenario["steps"]
    assert steps, f"scenario {name} has no steps, which judges nothing"
    module = importlib.import_module(
        f"integration.static_datamodel.{machine or scenario['machine']}_sm"
    )
    # A machine with `<sce:action>`s is built with the host that performs them, and
    # the generated engine says so by taking it: that host is the one that records.
    host = _ScenarioHost() if "actions" in inspect.signature(module.create_engine).parameters else None
    engine = module.create_engine() if host is None else module.create_engine(host)
    engine.initialize()
    policy = engine.policy
    readers = module.SCE_HOST_NAMES["readers"]
    for n, step in enumerate(steps):
        where = f"{name} step {n}: {step.get('note', '')}"
        # A step that moves the machine's time on, for a scenario of a delayed
        # send: the engine never reads a clock of its own, so a wait is the one
        # the step names.
        if "advance_ms" in step:
            engine.advance_time(step["advance_ms"])
        event_name = step.get("event")
        if event_name is not None:
            event = policy.resolve_event_by_name(event_name)
            dropped = step.get("dropped", False)
            assert (event is None) == dropped, (
                f"{where}: the machine's events {'match' if dropped else 'do not match'} {event_name!r}"
            )
            if event is not None:
                data = json.dumps(step["data"]) if "data" in step else ""
                engine.send_event(event, EventMetadata(data=data))
        expect = step["expect"]
        # What the machine asked of its host in this step. Read on every step so
        # that a step which states nothing does not hand its calls to the next, and
        # a step that states some needs a driver that records them.
        asked = host.take() if host is not None else None
        if "host_calls" in expect:
            assert asked is not None, (
                f"{where}: this driver records no host calls, so it cannot hold a step to them"
            )
            assert asked == expect["host_calls"], (
                f"{where}: the machine asked its host {asked!r}, not {expect['host_calls']!r}"
            )
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
        # Every active state, a compound or a parallel one with the atomic ones
        # below it. A set: the order a machine lists them in is not part of the
        # answer.
        if "configuration" in expect:
            got = {str(state) for state in engine.active_configuration()}
            assert got == set(expect["configuration"]), (
                f"{where}: the active states are {sorted(got)!r}, "
                f"not {sorted(expect['configuration'])!r}"
            )
        for variable, want in expect.get("variables", {}).items():
            got = _as_json(_read(policy, readers, variable))
            assert got == want, f"{where}: variable {variable!r} is {got!r}, not {want!r}"


# A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and
# `<elseif>` are native, and `In()` asks the machine's own configuration.
def test_the_counter_counts_to_its_flag_and_lets_go() -> None:
    replay("static_counter")


def test_the_counter_stops_at_its_bound_and_refuses_go() -> None:
    replay("static_counter_bound")


# An event arrives by name from outside the document, so the names it can arrive
# under are open (§scxml-3.12.1): a name the document never writes reaches the
# transition whose descriptor is a token prefix of it, and one no descriptor
# matches is dropped.
def test_an_event_arrives_under_a_name_the_document_does_not_write() -> None:
    replay("static_event_arrival")


# ...and where the document listens with `event="*"`, a name no descriptor it
# writes extends is delivered as the wildcard event instead of being dropped.
def test_an_event_arrives_under_a_name_only_the_wildcard_takes() -> None:
    replay("static_event_wildcard")


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


# An event's payload carries an enum field, the variant's declared name: a guard
# compares it to a variant and an assignment stores it in a variable of the enum.
def test_an_events_payload_carries_an_enum_field() -> None:
    replay("static_payload_enum")


# The payload of the event a transition is on is carried on as the <param>s of a
# <send>: an enum field as the name its enum declares and an integer, read where
# the send runs.
def test_the_payload_of_an_event_is_carried_on_as_params() -> None:
    replay("static_payload_relay")


# A delivery that carried no payload runs none of the content that reads it: the
# machine sends nothing, so nothing is relayed. The scenarios cannot say so for
# every engine -- the Interpreter fails the expression that reads the field,
# where a generated machine does not run the block at all -- so it is stated here.
def test_a_delivery_without_the_payload_sends_nothing() -> None:
    module = importlib.import_module("integration.static_datamodel.static_payload_relay_sm")
    engine = module.create_engine()
    engine.initialize()
    policy = engine.policy
    engine.send_event(policy.resolve_event_by_name("view.shown"), EventMetadata(data=""))
    assert policy.v_relays == 0, "no payload was read, so nothing was carried on"
    engine.send_event(
        policy.resolve_event_by_name("view.shown"),
        EventMetadata(data=json.dumps({"layout": "week", "zoom": 2})),
    )
    assert policy.v_relays == 1, "a delivery that carried the payload is relayed as any other"


# An enum variable holds a variant of its enum, read back by the name the enum
# document declares.
def test_an_enum_variable_holds_a_variant_of_its_enum() -> None:
    replay("static_enum")


# A list is filled to its bound and emptied.
def test_a_list_is_filled_to_its_bound_and_emptied() -> None:
    replay("static_list")


# An element is read by its index, and an index outside the list is a failure.
def test_a_list_element_is_read_by_its_index() -> None:
    replay("static_list_index")


# A `<foreach>` walks a list variable.
def test_a_foreach_walks_a_list_variable() -> None:
    replay("static_foreach")


# A 64-bit real is a native binary64 field: a product and a sum, a quotient, a
# guard comparing it with a literal, and a `<foreach>` summing a list of reals.
def test_a_real_is_a_native_binary64_field() -> None:
    replay("static_real")


# A 32-bit real is a native binary32 field: every operation on it is rounded to
# binary32 where it is made, though Python holds it in a double.
def test_a_real_is_a_native_binary32_field() -> None:
    replay("static_real32")


# An append that fails ends its block, the list as it was.
def test_an_append_that_fails_ends_its_block() -> None:
    replay("static_block_ends_list")


# A record is built whole from its `<sce:set>`s and updated a field at a time,
# from the machine's own value and from a typed event payload.
def test_a_record_is_built_whole_and_updated_a_field_at_a_time() -> None:
    replay("static_record_fields")


# A record with a 64-bit real field is built whole, written a field at a time,
# and replaced from a typed payload without losing a bit of the real it carried.
def test_a_record_holds_a_real_field_to_the_bit() -> None:
    replay("static_record_real")


# A record with a 32-bit real field: the double a payload carries lands as the
# single nearest it, and every operation on the field is a single's.
def test_a_record_holds_a_single_field_as_the_single_nearest_the_payload() -> None:
    replay("static_record_real32")


# A record's bytes field is held to the bytes its schema declares: an assignment
# past the bound — from a literal or a bytes variable — writes nothing, raises
# error.execution and ends its block, and a list of such records holds copies with
# their bytes. A scenario states a byte string as its byte-exact Latin-1 text.
def test_a_record_holds_a_bytes_field_within_the_bound_its_schema_declares() -> None:
    replay("static_record_bytes")


# A record is a frozen dataclass of immutable `bytes`, so what a host is handed is the
# value itself and cannot be written into, and two records of the same bytes are equal.
def test_a_record_of_the_same_bytes_is_equal_and_cannot_be_written_into() -> None:
    module = importlib.import_module("integration.static_datamodel.static_record_bytes_sm")
    engine = module.create_engine()
    engine.initialize()
    last = getattr(engine.policy, module.SCE_HOST_NAMES["readers"]["last"])()
    assert isinstance(last.frame, bytes)
    assert last == dataclasses.replace(last, frame=bytes(last.frame))
    assert last != dataclasses.replace(last, frame=b"zz")
    with pytest.raises(dataclasses.FrozenInstanceError):
        last.frame = b"zz"  # type: ignore[misc]


# The bytes an event's payload carries are read into a bytes variable, into a record's
# field and into a whole record, each held to its own bound: a value past it writes
# nothing, raises error.execution and ends its block. The wire spells a byte string as
# its byte-exact Latin-1 text, which a scenario states it as.
def test_the_payload_of_an_event_carries_bytes_held_within_their_bounds() -> None:
    replay("static_payload_bytes")


# A Python `bytes` cannot be written into, so the machine may keep the very object a
# host raised the event with: the host has no way to change what the machine read.
def test_a_host_cannot_change_the_bytes_an_event_it_raised_carried() -> None:
    module = importlib.import_module("integration.static_datamodel.static_payload_bytes_sm")
    engine = module.create_engine()
    engine.initialize()
    carried = b"wxyz"
    module.raise_framed_taken(engine, 100, carried)
    held = getattr(engine.policy, module.SCE_HOST_NAMES["readers"]["held"])()
    assert isinstance(held, bytes)
    assert held == b"wxyz"
    with pytest.raises(TypeError):
        carried[0] = ord("X")  # type: ignore[index]


# A record's string field is held to the UTF-8 bytes its schema declares: an
# assignment past the bound — from a literal, a string variable or a payload —
# writes nothing, raises error.execution and ends its block, and a list of such
# records holds copies with their text.
def test_a_record_holds_a_string_field_within_the_bound_its_schema_declares() -> None:
    replay("static_record_string")


# A list of records is filled by name from a record variable or a loop's item,
# walked by a `<foreach>`, and a record is taken whole.
def test_a_list_holds_records_and_a_foreach_walks_them() -> None:
    replay("static_record_list")


# The payload of an event is a record of its schema taken whole: it replaces a
# record variable in one assignment and is appended whole to a list.
def test_the_payload_of_an_event_is_taken_whole_as_a_record() -> None:
    replay("static_whole_payload")


# An enum value as a `<param>` crosses as the name its enum declares for it: a
# variable, a field of a record variable and a conditional, sent and read back
# through the schema.
def test_an_enum_value_crosses_as_the_name_its_enum_declares() -> None:
    replay("static_wire_enum")


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


# The `eventexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, and names the event the send delivers.
def test_a_sends_event_is_named_when_it_runs() -> None:
    replay("static_send_event")


# The `targetexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, held to the routes the document declares as `sce:targets`
# (docs/adr/0005, decision 3): the send goes by the entry that matches, and a value
# in none of them is error.communication with nothing sent.
def test_a_sends_target_is_chosen_among_the_declared_routes() -> None:
    replay("static_send_target")


# The `typeexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, held to the processors the document declares as `sce:types`
# (docs/adr/0005, decision 3): the send is delivered by the processor the matching
# entry names, and a value in none of them is error.execution with nothing sent.
def test_a_sends_type_is_chosen_among_the_declared_processors() -> None:
    replay("static_send_type")


# The `delayexpr` of a `<send>` is a string computed from the machine's fields when
# the send runs, and read as the CSS2 time it must be; the scenario's
# `advance_ms` steps move the engine's time on.
def test_a_sends_delay_is_computed_when_it_runs() -> None:
    replay("static_send_delay")


# The `sendidexpr` of a `<cancel>` is a string computed from the machine's fields
# when the cancel runs, the id of the delayed send it removes; the scenario's
# `advance_ms` steps move the engine's time on.
def test_a_cancel_removes_the_send_its_id_names() -> None:
    replay("static_cancel_expr")


# A native host action takes the machine's variables as typed arguments, each
# read when the call is made, and the host the scenario's `host_calls` are read
# from records them: one call per entry of `idle`, with the datamodel as it stood.
def test_a_host_action_tells_its_host_what_the_datamodel_held() -> None:
    replay("static_host_call")


# A child session an `<invoke>` started is driven through its parent by
# autoforward, takes the events it waits for in order, and its end reaches the
# parent as `done.invoke`, which the parent counts.
def test_an_invoked_child_is_driven_and_counted() -> None:
    replay("static_invoke")


# An `<invoke>` hands its child the values its `<param>`s and `namelist` name, as
# they stand when the invoke executes, after the entry actions, and once.
def test_an_invoke_hands_its_child_its_values_once() -> None:
    replay("static_invoke_params")


# A string an `<invoke>` hands its child is held to the bound the child declared,
# in bytes: a value past it is left out and raises `error.execution`, and the
# child still starts.
def test_a_string_handed_to_a_child_is_held_to_its_bound_in_bytes() -> None:
    replay("static_invoke_string")


# Leaving the state that holds an `<invoke>` cancels the child, which then ends
# nothing and counts nothing.
def test_leaving_the_state_of_an_invoke_cancels_the_child() -> None:
    replay("static_invoke_abort")


# A `<history>` remembers what its parent held when it was left, and entering it
# brings that back: the shallow one the child that was active, the deep one the
# atomic states below, and a deep one of a `<parallel>` both regions at once. The
# scenario states the whole active configuration of each step.
def test_a_history_brings_back_what_its_parent_held() -> None:
    replay("static_history")


# Four delayed sends armed on entering a state are delivered when each is due, two
# due the same moment in the order they were sent, and the last takes the machine
# to its final state; the scenario's `advance_ms` steps move the engine's time on.
def test_timers_deliver_each_send_when_it_is_due() -> None:
    replay("static_timers")


# The same machine stopped: a `<cancel>` by the id of the longest send removes that
# one and no other, so the machine never ends.
def test_stopping_cancels_the_send_its_id_names_and_no_other() -> None:
    replay("static_timers_stop")


# The `idlocation` of a `<send>` names a string variable the machine writes the id
# it generates for the send to, which a later `<cancel sendidexpr>` names; the
# scenario's `advance_ms` steps move the engine's time on.
def test_a_send_hands_the_document_an_id_a_cancel_can_name() -> None:
    replay("static_send_idlocation")


# The `<content expr>` of a `<send>` names a record, which crosses as the pairs of
# its fields: a record variable and the payload of the event the transition is
# on, taken whole.
def test_a_send_carries_the_record_its_content_names() -> None:
    replay("static_send_content")


# The `namelist` of a `<send>` names variables the machine holds, each carried as
# the pair `<param name="x" expr="x"/>` it abbreviates, an enum value among them
# as the name its enum declares.
def test_a_send_carries_the_variables_its_namelist_names() -> None:
    replay("static_send_namelist")


# A `<send>` hands its event the pairs of its `<param>`s, each read from the
# machine's fields when the send runs; a pair whose value failed is left out, the
# message still goes, and the receiver finds the field missing.
def test_a_send_hands_its_event_the_pairs_of_its_params() -> None:
    replay("static_send_params")


# A string variable is held to the UTF-8 bytes it declares, not to the characters
# a Python string counts: an assignment past the bound writes nothing, raises
# error.execution and ends its block.
def test_a_string_is_held_to_its_bytes() -> None:
    replay("static_string_capacity")


# A bytes variable is held to the bytes it declares, as a string is to its UTF-8
# bytes: an assignment past the bound writes nothing, raises error.execution and
# ends its block. A scenario states a byte string as its byte-exact Latin-1 text.
def test_a_byte_string_is_held_to_its_bound() -> None:
    replay("static_bytes")


# A byte string crosses a `<param>` and a `<donedata>` as its byte-exact Latin-1 text:
# sent to itself and read back through the typed payload, and carried by the done
# event's data.
def test_a_byte_string_crosses_a_param_as_its_latin1_text() -> None:
    replay("static_bytes_wire")


# A top-level final hands its done event the pairs of its `<donedata>`, each read
# from the machine's fields when the state is entered; a pair whose value failed
# is left out and the others cross.
def test_a_final_hands_its_done_event_the_pairs_of_its_donedata() -> None:
    replay("static_donedata")


# A top-level final whose `<donedata>` names a record in its `<content expr>` hands
# its done event the pairs of the record's fields, read when the state is entered.
def test_a_final_hands_its_done_event_the_record_its_content_names() -> None:
    replay("static_donedata_record")


# A top-level final whose `<donedata>` is inline `<content>` hands its done event
# the text as the string it spells, with no script engine to read it as a number.
def test_a_final_hands_its_done_event_the_text_its_content_spells() -> None:
    replay("static_donedata_content")


# A top-level final whose `<donedata>` carries a `<content expr>` that names one
# value hands its done event that value as its whole data: a number as its
# digits, a string quoted, and one that cannot be computed as the empty string.
def test_a_final_hands_its_done_event_the_value_its_content_names() -> None:
    replay("static_donedata_content_value")
    replay("static_donedata_content_text")
    replay("static_donedata_content_lost")
