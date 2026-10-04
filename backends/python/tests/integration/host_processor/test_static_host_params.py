# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15 — a ``<param expr>`` of a ``<send>`` or an ``<invoke>``
the HOST serves, in a ``datamodel="sce-static"`` machine, carries the value of a
typed expression read from the machine's own attributes when the send or the
invoke happens. Python compile+run gate; the Rust, Go, C++ and Kotlin twins are
``a_static_machines_typed_params_reach_the_host.rs``, ``host_params_test.go``,
``StaticHostParamsAotTest.cpp`` and ``StaticHostParamsTest``.

What this holds is the value on the wire. The run ``bump``, ``go`` changes every
variable before the send and the invoke read it, so a copy taken at start-up
(3, false, "idle") is told from what the fields hold now (4, true, "busy").

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml``,
generated WITH both ``--host-processor x-sce-host`` and ``--host-invoker x-sce-host``
by ``scripts/regen_host_processor_python.sh``. The declarations are load-bearing:
without them codegen emits the refusal and every case would measure it.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Dict, List

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import HostInvokeEvent, HostSendRequest, HostSendResponse  # noqa: E402

import statechart_static_host_params_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for.
DECLARED_TYPE = "x-sce-host"


def _started():
    """A machine with the host's side registered, standing at ``idle``."""
    engine = _sm.create_engine()
    sends: List[HostSendRequest] = []
    starts: List = []

    def processor(request: HostSendRequest) -> List[HostSendResponse]:
        sends.append(request)
        return []

    def invoker(event: HostInvokeEvent):
        if event.start is not None:
            starts.append(event.start)
        return None

    engine.register_event_processor(DECLARED_TYPE, processor)
    engine.register_invoker(DECLARED_TYPE, invoker)
    engine.initialize()
    return engine, sends, starts


def _drive(engine, events: List[str]) -> None:
    for event in events:
        engine.send_external_by_name(event)
        # The macrostep the event starts, run to its end.
        engine.advance_time(0)


def _wanted(count: str, ready: str, label: str, twice: str) -> Dict[str, List[str]]:
    """The text each param crosses as, given what ``count``, ``ready``, ``label``
    and ``twice`` hold. ``delta`` and ``ratio`` never change; ``boom`` is left out,
    because the multiplication that makes it overflows a 32-bit field."""
    return {
        "count": [count],
        "ready": [ready],
        "label": [label],
        "twice": [twice],
        "delta": ["-5"],
        "ratio": ["1.5"],
    }


def _assert_typed_event_data(event_data: str, what: str) -> None:
    """``event_data`` is the pairs as JSON, typed as the data model holds them: a
    number stays a number, a bool a bool, a string a string."""
    value = json.loads(event_data)
    assert value["count"] == 4, f"{what}: {event_data}"
    assert value["ready"] is True, f"{what}: {event_data}"
    assert value["label"] == "busy", f"{what}: {event_data}"
    assert value["twice"] == 8, f"{what}: {event_data}"
    assert value["delta"] == -5, f"{what}: {event_data}"
    assert value["ratio"] == 1.5, f"{what}: {event_data}"
    assert "boom" not in value, f"{what}: a pair whose value failed is left out: {event_data}"


def test_a_send_param_carries_the_value_the_fields_hold_when_it_is_sent() -> None:
    engine, sends, _ = _started()
    _drive(engine, ["bump", "go"])
    assert len(sends) == 1, "one <send>, one request"
    assert sends[0].params == _wanted("4", "true", "busy", "8"), "the text each <param> crosses as"
    _assert_typed_event_data(sends[0].event_data, "send")


def test_an_invoke_param_carries_the_value_the_fields_hold_when_it_starts() -> None:
    engine, _, starts = _started()
    _drive(engine, ["bump", "go"])
    assert len(starts) == 1, "one <invoke>, one start"
    # A copy taken at start-up would say count 3, ready false, label idle.
    assert starts[0].params == _wanted("4", "true", "busy", "8"), "the text each <param> crosses as"
    _assert_typed_event_data(starts[0].event_data, "invoke")


def test_a_param_read_before_any_bump_carries_the_declared_values() -> None:
    # The same machine on the shorter run: nothing has written a variable, so the
    # fields still hold what ``<data expr>`` gave them. The control that keeps the
    # two cases above from passing on a value that is simply always the new one.
    engine, sends, starts = _started()
    _drive(engine, ["go"])
    assert sends[0].params == _wanted("3", "false", "idle", "6")
    assert starts[0].params == _wanted("3", "false", "idle", "6")


def test_a_param_whose_value_cannot_be_computed_is_reported_and_left_out() -> None:
    # W3C SCXML 5.7.1: a ``<param>`` whose value cannot be computed — here a
    # multiplication a 32-bit field cannot hold — is reported with
    # ``error.execution`` and its pair left out, while the message still goes and
    # the invocation still starts. ``errors`` counts the reports the document took,
    # one for the send and one for the invoke, so a pair dropped in silence is told
    # from one reported.
    engine, sends, starts = _started()
    _drive(engine, ["bump", "go"])
    assert len(sends) == 1, "the send still went"
    assert len(starts) == 1, "the invoke still started"
    assert engine.policy.errors() == 2, "one error.execution for the send's `boom` and one for the invoke's"
    assert "boom" not in sends[0].params, "the failed pair is left out of the send"
    assert "boom" not in starts[0].params, "the failed pair is left out of the invoke"
