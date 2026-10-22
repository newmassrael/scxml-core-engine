# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring": a Mesh request
(``<invoke type="sce:mesh-rpc">``) has one request in flight, and the peer it reached
may already have acted on it. A saved state holds the call as it was sent; a restore
does NOT send it again, because the peer would act on it twice, and the router that
carried it is gone with the process that saved it. The document is told on the first
macrostep that the call was interrupted (``error.invoke.<id>`` carrying
``"interrupted"``) and sends a new request if it means to ask again.

``statechart_static_mesh_request.scxml`` asks from ``asking``; ``done.invoke.ask`` and
``error.invoke.ask`` each leave it, counting in ``answered`` and ``failed``. The Rust,
Kotlin and Go halves are ``a_saved_mesh_request_is_not_sent_again_after_a_restore.rs``,
``ASavedMeshRequestIsNotSentAgainAfterARestoreTest.kt`` and
``a_saved_mesh_request_is_not_sent_again_after_a_restore_test.go``.

Driven entirely on the engine's virtual clock: no case sleeps. The shared instance
``saved/statechart_static_mesh_request_asking.json`` is the format-2 text every backend
writes (docs/adr/0045); ``saved/format1/`` holds the format-1 text of the same run.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import MESH_RPC_INVOKE_TYPE, saved_state_from_json  # noqa: E402 -- path inserted above

import statechart_static_mesh_request_sm as _sm  # noqa: E402 -- path inserted above

_State = _sm.StatechartStaticMeshRequestState
# The wall-clock moment the shared instance was saved at.
_SAVED_AT_MS = 1_700_000_000_000
_SHARED = (
    _HERE.parents[4]
    / "sce-build"
    / "tests"
    / "fixtures"
    / "host_processor"
    / "saved"
    / "statechart_static_mesh_request_asking.json"
)


def _shared() -> str:
    return _SHARED.read_text(encoding="utf-8").strip()


class _Router:
    def __init__(self) -> None:
        self.log: list = []

    def register(self, engine) -> "_Router":
        def router(event):
            self.log.append(event)
            return None

        engine.register_mesh_rpc_invoker(router)
        return self


def _asking():
    """A machine whose request to `#motor` is in flight, the router having been handed
    it, given the session id the shared format-2 fixtures name their root by."""
    engine = _sm.create_engine(session_id="fixture-root")
    router = _Router().register(engine)
    engine.initialize()
    engine.send_event(engine.policy.resolve_event_by_name("go"))
    engine.advance_time(0)
    return engine, router


def _restored(text: str, elapsed_ms: int):
    """`text` restored `elapsed_ms` after it was saved, the router registered on the
    engine the restore returned, the way a host does it."""
    engine = _sm.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS + elapsed_ms)
    return engine, _Router().register(engine)


def test_a_request_in_flight_is_saved_as_the_call_it_was() -> None:
    engine, router = _asking()
    assert len(router.log) == 1, "the router was handed the request"
    saved = _sm.save_at(engine, _SAVED_AT_MS)
    assert len(saved.host_invokes) == 1, "one call in flight"
    entry = saved.host_invokes[0]
    assert (entry.processor_type, entry.invoke_id, entry.src) == (MESH_RPC_INVOKE_TYPE, "ask", "#motor")
    # The router owns the deadline of a Mesh request, so the engine holds none.
    assert entry.due is None
    assert saved.pending == []
    # The text every backend writes for the same machine, byte for byte.
    assert saved.to_json() == _shared()


def test_a_restore_asks_nothing_of_the_router_until_the_machine_is_driven() -> None:
    engine, router = _restored(_shared(), 1000)
    assert router.log == [], "the restore itself calls nobody"
    assert set(engine.active_configuration()) == {_State.ASKING}


def test_a_restored_request_is_not_sent_again_and_the_document_is_told_it_was_interrupted() -> None:
    engine, router = _restored(_shared(), 1000)
    engine.advance_time(0)
    assert router.log == [], "the peer may have acted on the first request already, so no second one leaves"
    policy = engine.policy
    assert (policy.v_failed, policy.v_answered, policy.v_refused) == (1, 0, 0), "error.invoke.ask arrived"
    assert engine.terminal_state == _State.DONE, "the run ended"


@pytest.mark.parametrize("elapsed_ms", [0, 250, 60_000])
def test_a_call_is_interrupted_whenever_the_machine_comes_back(elapsed_ms: int) -> None:
    # There is no moment after which the call is still worth sending again.
    engine, router = _restored(_shared(), elapsed_ms)
    engine.advance_time(0)
    assert router.log == []
    assert engine.policy.v_failed == 1, f"{elapsed_ms} ms after the save"


def test_a_machine_owing_the_interruption_says_it_needs_a_tick_now() -> None:
    # The interruption is told in the first macrostep, so a host driving the machine by
    # its wake-up query must not be told there is nothing to wait for.
    engine, _ = _restored(_shared(), 1000)
    assert engine.time_until_next_scheduled_ms() == 0


def test_a_restored_machine_saved_again_before_it_was_driven_keeps_the_call() -> None:
    engine, _ = _restored(_shared(), 1000)
    again = _sm.save_at(engine, _SAVED_AT_MS + 1000)
    assert [(h.processor_type, h.invoke_id) for h in again.host_invokes] == [(MESH_RPC_INVOKE_TYPE, "ask")], (
        "the call is still held"
    )
