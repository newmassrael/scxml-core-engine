# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE_MESH.md §9.5, §mesh-19: an ``<invoke type="sce:mesh-rpc">`` reaches the
host's Mesh router through the runtime's mesh-rpc door — Python AOT.

Fixture: ``integration_resources/a_mesh_request_reaches_the_router/
a_mesh_request_reaches_the_router.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_mesh_request_reaches_the_router_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Optional

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import (  # noqa: E402
    MESH_RPC_INVOKE_TYPE,
    HostInvokeEvent,
    HostInvokeRequest,
    HostInvokeResponse,
)

import a_mesh_request_reaches_the_router_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AMeshRequestReachesTheRouterState


def _started(requests: Optional[List[HostInvokeRequest]]):
    """The machine run into ``asking``, with a router recording into
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


def _the_request(requests: List[HostInvokeRequest]) -> HostInvokeRequest:
    """The one request the router was started with, checked against what
    the document wrote."""
    assert len(requests) == 1, f"exactly one request reaches the router: {requests}"
    request = requests[0]
    assert request.processor_type == MESH_RPC_INVOKE_TYPE
    assert request.invoke_id == "ask"
    assert request.src == "#motor"
    assert request.params == {
        "_mesh_event": ["service.request.force"],
        "_mesh_deadline_ms": ["250"],
        "force": ["3"],
        "speed": ["3"],
    }
    # The author's pairs alone, typed: `force` was computed, `speed` was
    # written as a string, and the envelope fields are not payload.
    assert request.event_data == '{"force":3,"speed":"3"}'
    return request


def _ended(engine, answered: int, failed: int, refused: int) -> None:
    p = engine.policy
    assert (p.answered(), p.failed(), p.refused()) == (answered, failed, refused)
    assert engine.terminal_state == _State.DONE


def test_a_mesh_request_is_answered_through_the_router() -> None:
    """The router answers: done.invoke.ask ends the run."""
    requests: List[HostInvokeRequest] = []
    engine = _started(requests)
    request = _the_request(requests)
    assert engine.complete_host_invoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, '"ok"')
    engine.advance_time(0)
    _ended(engine, answered=1, failed=0, refused=0)


def test_a_mesh_request_the_router_fails_is_error_invoke() -> None:
    """The router fails the request: error.invoke.ask, carrying the
    router's data."""
    requests: List[HostInvokeRequest] = []
    engine = _started(requests)
    request = _the_request(requests)
    assert engine.fail_host_invoke(
        MESH_RPC_INVOKE_TYPE, "ask", request.token, '"unreachable"', origin="mesh://motor"
    )
    engine.advance_time(0)
    _ended(engine, answered=0, failed=1, refused=0)


def test_a_mesh_request_with_no_router_registered_is_error_execution() -> None:
    """With no router registered the invoke names a type nobody runs:
    error.execution (W3C SCXML 6.4.1)."""
    engine = _started(None)
    _ended(engine, answered=0, failed=0, refused=1)
