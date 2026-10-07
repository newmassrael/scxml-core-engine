# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15 — a ``<param>`` of a ``<send>`` the HOST serves whose
value is a string literal, written beside one that reads a field, in a
``datamodel="sce-static"`` machine. Python compile+run gate.

A literal is folded when the document is built, so it has no native code to
lower; a field's ``<param>`` has. The template put every pair of a send that held
any field through the field's line, and the literal came out as
``().to_python()``: the machine stopped at its first send. A document of only literals, or of only
fields, never failed, which is why the two controls below are asked too.

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_static_literal_param.scxml``,
generated WITH ``--host-processor x-sce-host`` by
``scripts/regen_host_processor_python.sh``. The declaration is load-bearing:
without it codegen emits the refusal and every case would measure it.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import HostSendRequest, HostSendResponse  # noqa: E402

import statechart_static_literal_param_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for.
DECLARED_TYPE = "x-sce-host"


def _run(events: List[str]) -> List[HostSendRequest]:
    """The requests the host was handed after ``events`` were driven in turn."""
    engine = _sm.create_engine()
    sends: List[HostSendRequest] = []

    def processor(request: HostSendRequest) -> List[HostSendResponse]:
        sends.append(request)
        return []

    engine.register_event_processor(DECLARED_TYPE, processor)
    engine.initialize()
    for event in events:
        engine.send_external_by_name(event)
        # The macrostep the event starts, run to its end.
        engine.advance_time(0)
    return sends


def _request_of(sends: List[HostSendRequest], event: str) -> HostSendRequest:
    named = [s for s in sends if s.event_name == event]
    assert len(named) == 1, f"one request for `{event}`: {sends}"
    return named[0]


def test_a_literal_beside_a_field_crosses_as_written() -> None:
    sends = _run(["bump", "go"])
    notify = _request_of(sends, "notify")
    assert notify.params == {"id": ["E7401"], "count": ["4"], "unit": ["ms"]}, "the text each <param> crosses as"
    data = json.loads(notify.event_data)
    assert data == {"id": "E7401", "count": 4, "unit": "ms"}, f"typed as the data model holds them: {notify.event_data}"


def test_the_field_is_read_when_the_send_runs_not_at_start_up() -> None:
    # The control for the case above: nothing has written ``count``, so the
    # field still holds what ``<data expr>`` gave it. Without it the case above
    # would pass on a field that is simply always 4.
    notify = _request_of(_run(["go"]), "notify")
    assert json.loads(notify.event_data)["count"] == 3


def test_literals_alone_cross_as_the_same_pairs() -> None:
    # The arm that never failed: a send of only literals is the pairs it
    # shares with ``notify``, as the same text and the same JSON.
    sends = _run(["bump", "go"])
    plain = _request_of(sends, "plain")
    assert plain.params == {"id": ["E7401"], "unit": ["ms"]}
    assert json.loads(plain.event_data) == {"id": "E7401", "unit": "ms"}
    assert len(sends) == 2, "both sends went"
