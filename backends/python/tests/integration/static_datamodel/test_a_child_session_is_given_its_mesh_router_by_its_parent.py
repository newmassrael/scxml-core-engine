# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043) -- Python.

A child session that talks to a Mesh peer is given a router of its own by the
machine that starts it, asked of the factory the host registered, and never the
machine's own. ``static_child_mesh.scxml`` starts `list`, which greets
`#server` as it is entered and ends when the greeting is refused; the machine
counts that end in `refusals`. Its hybrid `peer` starts `static_mesh_relay`,
whose child `inner` answers `inner.ping`. The Rust half is
`a_child_session_is_given_its_mesh_router_by_its_parent.rs`.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import EventMetadata, MeshRaise, mesh_event_metadata  # noqa: E402 -- path inserted above


class _Recorder:
    """A child's router that writes down what it is told and holds what a test
    hands it for the child."""

    def __init__(self, name: str, log: list, inbox: list) -> None:
        self.name = name
        self.log = log
        self.inbox = inbox

    def send(self, request) -> list:
        self.log.append(f"{self.name} sent {request.event_name}")
        return []

    def invoke(self, _event):
        return None

    def take_calls(self) -> list:
        taken = list(self.inbox)
        self.inbox.clear()
        return taken

    def detach(self) -> None:
        self.log.append(f"{self.name} detached")

    def child_router(self, invoke_id: str, document: str) -> "_Recorder":
        self.log.append(f"{self.name} asked for {invoke_id} of '{document}'")
        return _Recorder(f"{self.name}/{invoke_id}", self.log, self.inbox)


def _started(register):
    """The machine, with `register` applied before it starts, and the names of
    what its own router was handed."""
    module = importlib.import_module("integration.static_datamodel.static_child_mesh_sm")
    engine = module.create_engine()
    parent_sends: list = []

    def route(request) -> list:
        parent_sends.append(request.event_name)
        return []

    engine.register_mesh_router(route)
    register(engine)
    engine.initialize()
    return engine, parent_sends


def _send(engine, name: str) -> None:
    engine.send_event(engine.policy.resolve_event_by_name(name), EventMetadata())


def _settle(engine) -> None:
    for _ in range(5):
        engine.advance_time(0)


def test_a_child_given_no_router_has_its_peer_send_refused_and_the_machines_router_is_not_asked() -> None:
    engine, parent_sends = _started(lambda engine: None)
    assert engine.policy.refusals() == 1, (
        "the child's greeting found no router, raised error.execution in the child, and ended it"
    )
    assert parent_sends == [], f"the child's send fell back to the machine's router: {parent_sends}"


def test_a_factory_that_answers_none_leaves_the_child_with_no_router() -> None:
    engine, parent_sends = _started(
        lambda engine: engine.register_child_mesh_routers(lambda _invoke, _document: None)
    )
    assert engine.policy.refusals() == 1
    assert parent_sends == []


def test_each_start_asks_the_factory_and_each_end_detaches_what_it_answered() -> None:
    log: list = []
    inbox: list = []

    def factory(invoke_id: str, document: str) -> _Recorder:
        log.append(f"asked for {invoke_id} of '{document}'")
        return _Recorder(invoke_id, log, inbox)

    engine, parent_sends = _started(lambda engine: engine.register_child_mesh_routers(factory))

    def told() -> list:
        taken = list(log)
        log.clear()
        return taken

    # Asked before the child was entered, since the greeting went through it.
    assert told() == ["asked for list of ''", "list sent list.hello"]
    assert engine.policy.refusals() == 0

    # A peer's event reaches the child when the machine next drives it.
    inbox.append(MeshRaise("list.ping", mesh_event_metadata("", "server", "")))
    _settle(engine)
    assert told() == ["list sent list.pong"]

    # The hybrid candidate is asked for by its stem, and its child of its router.
    _send(engine, "open")
    _settle(engine)
    assert told() == ["asked for peer of 'static_mesh_relay'", "peer asked for inner of ''"]

    # Leaving the invoking states ends every session, each detached once, a
    # grandchild before its parent.
    _send(engine, "again")
    _settle(engine)
    assert told() == ["peer/inner detached", "peer detached", "list detached"]

    # Entered anew, the invoke asks again.
    _send(engine, "back")
    _settle(engine)
    assert told() == ["asked for list of ''", "list sent list.hello"]
    assert parent_sends == []
