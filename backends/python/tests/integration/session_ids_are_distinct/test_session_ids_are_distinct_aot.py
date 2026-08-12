# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10: ``_sessionid`` is the id of a session - Python AOT.

The clause binds ``_sessionid`` to "the system-generated id for the current
SCXML session", and Appendix C.1.1 derives the address a session publishes
from that id. Two live sessions holding one id publish one address, so a
``<send>`` addressed to either reaches both.

No test in the public IRP corpus can ask: every one that reaches
``_sessionid`` runs a single session, so a processor that hands the same
value to every session it starts passes them all.

The fixture runs two children at once, each reporting the id it was issued,
and the parent compares them.

Fixture: ``integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml``
(canonical, shared with every other channel).

Regeneration (after fixture or template edit):
  ``scripts/regen_session_ids_are_distinct_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem session_ids_are_distinct`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import session_ids_are_distinct_sm as _sm  # noqa: E402 - path inserted above


def test_session_ids_are_distinct_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 2000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "session_ids_are_distinct did not reach a top-level <final> within 2 s; "
        f"last leaf={engine.current_state}. only one child reported its `_sessionid`, so the two ids were never compared."
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"session_ids_are_distinct reached <final id={actual!r}>; expected 'pass'. "
        "two live sessions reported the same `_sessionid`. W3C SCXML 5.10 binds it to the id of the current session, and C.1.1 publishes an address derived from it, so one id for two sessions is one address for two sessions."
    )
