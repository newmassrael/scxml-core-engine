# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
delivered has no origin — Python AOT.

An event the host posts itself and the reply a host processor answers a
``<send>`` with, both naming no party, find ``_event.origin`` and
``_event.origintype`` blank, which the ECMAScript data model reads as
undefined (nil in the Lua it lowers to). No engine invents an origin for them: neither
the receiving session's own address nor the SCXML processor's type.
``Engine.send_external`` used to give a host-posted event the SCXML processor's
type with an empty address.

Fixture: ``integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml``,
generated with ``--host-processor x-sce-host``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_event_no_processor_delivered_has_no_origin_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import HostSendRequest, HostSendResponse  # noqa: E402

import an_event_no_processor_delivered_has_no_origin_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AnEventNoProcessorDeliveredHasNoOriginState
_Event = _sm.AnEventNoProcessorDeliveredHasNoOriginEvent

# The type the fixture was compiled for: the regen script passes this same
# string to `--host-processor`.
_HOST_TYPE = "x-sce-host"


def test_an_event_no_processor_delivered_has_no_origin() -> None:
    engine = _sm.create_engine()
    served: List[HostSendRequest] = []

    def processor(request: HostSendRequest) -> List[HostSendResponse]:
        served.append(request)
        # A reply that names no party: no origin.
        return [HostSendResponse(event_name="host.reply")]

    # Registered before the machine starts, as every host registers.
    engine.register_event_processor(_HOST_TYPE, processor)
    engine.initialize()

    # The host raises an event of its own, naming no origin.
    engine.send_external(_Event.POSTED)
    engine.advance_time(0)

    p = engine.policy
    assert len(served) == 1, "the host's processor was not handed the document's send once"
    assert engine.terminal_state == _State.DONE, (
        f"the run must end in `done`; it stopped in {engine.current_state}"
    )
    assert p.posted_blank() == 1, (
        f"postedBlank = {p.posted_blank()}: the host-posted event read an origin it was not given"
    )
    assert p.reply_blank() == 1, (
        f"replyBlank = {p.reply_blank()}: the host's reply read an origin it was not given"
    )
