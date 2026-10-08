# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE_MESH.md §9.5, §mesh-19 under ``datamodel="sce-static"`` (docs/adr/0005,
decisions 5 and 7): an ``<invoke type="sce:mesh-rpc">`` whose peer and ``<param>``s
are typed expressions over the machine's own attributes reaches the host's Mesh
router through the runtime's mesh-rpc door — Python AOT. The script-model twin is
``test_a_mesh_request_reaches_the_router_aot.py``.

What this holds is the value on the wire. The run ``bump``, ``go`` changes ``load``
and ``label`` before the request reads them, so a copy taken at start-up
(3, "idle") is told from what the fields hold now (4, "busy"): the request carries
8 and "busy".

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml``,
generated with NO host declaration by ``scripts/regen_host_processor_python.sh``: the
build names the request's type itself.
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Optional

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import (  # noqa: E402
    MESH_RPC_INVOKE_TYPE,
    HostInvokeEvent,
    HostInvokeRequest,
    HostInvokeResponse,
)

import statechart_static_mesh_request_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.StatechartStaticMeshRequestState


def _started(requests: Optional[List[HostInvokeRequest]]):
    """The machine standing at ``idle``, with a router recording into
    `requests` when one is given. The router answers none, so the test
    decides how each request ends."""

    def router(event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if event.start is not None and requests is not None:
            requests.append(event.start)
        return None

    engine = _sm.create_engine()
    if requests is not None:
        engine.register_mesh_rpc_invoker(router)
    engine.initialize()
    return engine


def _drive(engine, events: List[str]) -> None:
    for event in events:
        engine.send_external_by_name(event)
        # The macrostep the event starts, run to its end.
        engine.advance_time(0)


def _the_request(requests: List[HostInvokeRequest]) -> HostInvokeRequest:
    """The one request the router was started with, checked against what
    the document wrote and the fields held when it started."""
    assert len(requests) == 1, f"exactly one request reaches the router: {requests}"
    request = requests[0]
    assert request.processor_type == MESH_RPC_INVOKE_TYPE
    assert request.invoke_id == "ask"
    assert request.src == "#motor"
    assert request.params == {
        "_mesh_event": ["service.request.force"],
        "_mesh_deadline_ms": ["250"],
        "force": ["8"],
        "speed": ["busy"],
    }
    # The author's pairs alone, typed: `force` was computed, `speed` is a
    # string, and the envelope fields are not payload.
    assert request.event_data == '{"force":8,"speed":"busy"}'
    return request


def _ended(engine, answered: int, failed: int, refused: int) -> None:
    p = engine.policy
    assert (p.answered(), p.failed(), p.refused()) == (answered, failed, refused)
    assert engine.terminal_state == _State.DONE


def test_a_static_mesh_request_is_answered_through_the_router() -> None:
    """The router answers: done.invoke.ask ends the run."""
    requests: List[HostInvokeRequest] = []
    engine = _started(requests)
    _drive(engine, ["bump", "go"])
    request = _the_request(requests)
    assert engine.complete_host_invoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, '"ok"')
    engine.advance_time(0)
    _ended(engine, answered=1, failed=0, refused=0)


def test_a_static_mesh_request_the_router_fails_is_error_invoke() -> None:
    """The router fails the request: error.invoke.ask, carrying the
    router's data."""
    requests: List[HostInvokeRequest] = []
    engine = _started(requests)
    _drive(engine, ["bump", "go"])
    request = _the_request(requests)
    assert engine.fail_host_invoke(
        MESH_RPC_INVOKE_TYPE, "ask", request.token, '"unreachable"', origin="mesh://motor"
    )
    engine.advance_time(0)
    _ended(engine, answered=0, failed=1, refused=0)


def test_the_request_carries_the_fields_as_they_stand_and_not_a_copy_from_start_up() -> None:
    """The control: nothing has written a field, so the request carries what
    ``<data expr>`` gave them and not the values ``bump`` would have set."""
    requests: List[HostInvokeRequest] = []
    engine = _started(requests)
    _drive(engine, ["go"])
    assert len(requests) == 1, f"one request: {requests}"
    assert requests[0].event_data == '{"force":6,"speed":"idle"}'


def test_the_peer_is_the_string_the_machine_holds_when_the_request_starts() -> None:
    """``ghost`` changes the string ``peer`` holds, so the router is handed
    that name as the request's ``src`` — and the host's router is the one that
    looks it up among its bindings."""
    requests: List[HostInvokeRequest] = []
    engine = _started(requests)
    _drive(engine, ["ghost", "go"])
    assert len(requests) == 1, f"one request: {requests}"
    assert requests[0].src == "#ghost"


def test_a_static_mesh_request_with_no_router_registered_is_error_execution() -> None:
    """With no router registered the invoke names a type nobody runs:
    error.execution (W3C SCXML 6.4.1)."""
    engine = _started(None)
    _drive(engine, ["bump", "go"])
    _ended(engine, answered=0, failed=0, refused=1)
