# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE_MESH.md §mesh-19: a ``targetexpr`` that evaluates to a Mesh peer reaches
the host's Mesh router — Python AOT.

Fixture: ``integration_resources/a_peer_named_at_run_time_reaches_the_router/
a_peer_named_at_run_time_reaches_the_router.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_peer_named_at_run_time_reaches_the_router_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import MESH_PROCESSOR_TYPE, HostSendRequest  # noqa: E402

import a_peer_named_at_run_time_reaches_the_router_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.APeerNamedAtRunTimeReachesTheRouterState


def test_a_peer_named_at_run_time_is_sent_to_the_router() -> None:
    """With a router registered, the peer send is the router's — processor
    ``sce:mesh``, the evaluated target, the event — and the non-peer target is
    still this session's."""
    routed: List[HostSendRequest] = []
    engine = _sm.create_engine()
    engine.register_mesh_router(lambda request: routed.append(request) or [])
    engine.initialize()

    assert len(routed) == 1, f"exactly the peer send reaches the router: {routed}"
    assert routed[0].processor_type == MESH_PROCESSOR_TYPE
    assert routed[0].target == "#hmi"
    assert routed[0].event_name == "ping"
    p = engine.policy
    assert (p.refused(), p.looped()) == (0, 1)
    assert engine.terminal_state == _State.DONE


def test_a_peer_with_no_router_registered_is_error_execution() -> None:
    """With no router registered, the peer send is a send to a host processor
    nobody serves: one error.execution, and the non-peer target is
    unaffected."""
    engine = _sm.create_engine()
    engine.initialize()

    p = engine.policy
    assert (p.refused(), p.looped()) == (1, 1)
    assert engine.terminal_state == _State.DONE
