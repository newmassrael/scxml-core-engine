# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""NL→IR Item C1 Path A — the OTHER carrier, the Python twin of the Rust
``event_schema_native.rs`` lifted cases, the Go ``event_schema_lifted``
package, the Kotlin ``EventSchemaLiftedTest``, the C11
``c11_integration_event_schema_lifted`` and the C++
``EventSchemaLiftedAotTest``.

The committed SM (``statechart_lifted_sm.py``) is generated from
``sce-build/tests/fixtures/event_schema/statechart_lifted.scxml``
(regen: ``scripts/regen_event_schema_native_python.sh``).

A schema'd event's typed payload is filled by ONE producer: the generated
``raise_job_completed`` seam. Every other producer — ``<send>`` with
``<param>``, an invoke forwarding an event either way, autoforward, BasicHTTP,
mesh — fills ``EventMetadata.data``, and until 2026-09-22 a natively lowered
guard could read nothing but the typed carrier. The same guard therefore
answered differently depending on where its event came from, and a typed
payload could not cross an invoke boundary at all.

⚠ What a refusal does is not a policy chosen here: it is what the SCRIPT
ENGINE answers for the same guard on the same data (W3C SCXML 3.13, measured
on this document — no data, a missing field and a value of another type each
give ``error.execution`` and a guard that does not fire). A native lowering
that answered differently would make the optimisation observable, which is the
one thing it may not be.
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import statechart_lifted_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.event import EventMetadata  # noqa: E402


def _make_engine():
    """The fixture lowers its guard natively and carries no executable content,
    so the machine is built with no script engine at all."""
    engine = _sm.create_engine()
    engine.initialize()
    assert str(engine.current_state) == "waiting"
    return engine


def _deliver_data(engine, data) -> None:
    """The event as every producer but the inject seam delivers it: its fields
    on the ``data`` wire, with no typed payload riding along."""
    engine.send_event(
        engine.policy.get_event_from_name("job.completed"),
        EventMetadata(event_type="external", data=data),
    )


def test_a_payload_on_the_data_wire_fires_the_same_guard() -> None:
    engine = _make_engine()

    _deliver_data(engine, {"elapsed_ms": 0})

    assert str(engine.current_state) == "done", (
        f"state = {engine.current_state!s}, want done — a payload that arrived "
        "on the `data` wire must satisfy the same native guard the inject "
        "seam's typed payload does"
    )


def test_the_json_spelling_of_that_payload_reads_the_same() -> None:
    engine = _make_engine()

    # What a payload that crossed a wire looks like: the JSON spelling
    # §scxml-B-2-8-1's second rung reads, which is what an invoke boundary,
    # BasicHTTP or the mesh hands over.
    _deliver_data(engine, '{"elapsed_ms": 0}')

    assert str(engine.current_state) == "done", (
        f"state = {engine.current_state!s}, want done — the JSON spelling of a "
        "payload must read as the same fields the in-process mapping does"
    )


def test_the_inject_seam_still_fires_its_own_guard() -> None:
    engine = _make_engine()

    _sm.raise_job_completed(engine, 0)

    assert str(engine.current_state) == "done", (
        f"state = {engine.current_state!s}, want done — the typed inject seam "
        "must still fire the guard it was built for"
    )


def test_a_value_of_another_type_is_refused_as_the_script_engine_refuses_it() -> None:
    engine = _make_engine()

    _deliver_data(engine, {"elapsed_ms": "nought"})

    assert str(engine.current_state) == "refused", (
        f"state = {engine.current_state!s}, want refused — a text where the "
        "schema declares a number must raise error.execution and leave the "
        "guard unfired"
    )


def test_an_event_with_no_data_is_refused_the_same_way() -> None:
    engine = _make_engine()

    _deliver_data(engine, "")

    assert str(engine.current_state) == "refused", (
        f"state = {engine.current_state!s}, want refused — an event carrying no "
        "data cannot answer a guard that reads a field of it"
    )


def test_a_field_the_data_does_not_name_is_refused() -> None:
    engine = _make_engine()

    _deliver_data(engine, {"other": 0})

    assert str(engine.current_state) == "refused", (
        f"state = {engine.current_state!s}, want refused — data that names none "
        "of the schema's fields cannot answer the guard"
    )


def test_a_payload_the_guard_rejects_is_not_an_error() -> None:
    engine = _make_engine()

    # The payload reads perfectly; the comparison is simply false. Nothing
    # failed, so nothing is raised — the machine waits.
    _deliver_data(engine, {"elapsed_ms": 5})

    assert str(engine.current_state) == "waiting", (
        f"state = {engine.current_state!s}, want waiting — a well-typed payload "
        "the guard rejects must not route the machine to the error handler"
    )
