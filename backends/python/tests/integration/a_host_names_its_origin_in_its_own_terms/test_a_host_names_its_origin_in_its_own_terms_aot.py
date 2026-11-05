# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event
it delivers in its own terms — Python AOT.

An ending of a host-run invocation whose origin is {``x-sce-host``, ``motor``}
reads back ``motor`` and ``x-sce-host``. The address has no scheme, so it is no
SCXML location, and the type is the host's: the engine neither wraps the one as
``sce://scxml/motor`` nor defaults the other to the SCXML processor's.

Fixture: ``integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml``,
generated with ``--host-invoker x-sce-host``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_host_names_its_origin_in_its_own_terms_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Optional

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import EventOrigin, HostInvokeEvent, HostInvokeResponse  # noqa: E402

import a_host_names_its_origin_in_its_own_terms_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AHostNamesItsOriginInItsOwnTermsState

# The type the fixture was compiled for: the regen script passes this same
# string to `--host-invoker`.
_HOST_TYPE = "x-sce-host"


def test_a_host_names_its_origin_in_its_own_terms() -> None:
    engine = _sm.create_engine()
    tokens: List[int] = []

    def invoker(event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        # The work outlives the start: the test ends it, naming the party it
        # stands for.
        if event.start is not None:
            tokens.append(event.start.token)
        return None

    engine.register_invoker(_HOST_TYPE, invoker)
    engine.initialize()
    assert len(tokens) == 1, "the host was not asked to run `job` once"

    assert engine.complete_host_invoke(
        _HOST_TYPE, "job", tokens[0], "{}", origin=EventOrigin(_HOST_TYPE, "motor")
    ), "the running invocation's completion was refused"
    engine.advance_time(0)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, (
        f"the run must end in `done`; it stopped in {engine.current_state}"
    )
    assert p.host_named() == 1, (
        f"hostNamed = {p.host_named()}: the completion did not read back the address "
        "and the type the host named"
    )
