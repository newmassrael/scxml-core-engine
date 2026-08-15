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


def test_a_structured_declaration_reads_back_as_json() -> None:
    """W3C SCXML 5.3 + B.2: an array-declared ``<data>`` is readable as JSON.

    The document declares ``readBackProbe`` for the channels rather than for
    itself, because reading the DATAMODEL is a different question from reading
    the configuration and every other fixture asks the second one. That is how
    a whole class of declaration reached consumers with no reader at all: the
    suites were green over documents nobody could ask anything about.
    """
    engine = _sm.create_engine()
    policy = engine._policy

    # Before the machine is initialised there is no session holding a
    # datamodel, so the only honest answer is that it cannot say. A reader
    # backed by a value captured at generation time would answer the
    # document's literal here and for the rest of the run.
    assert policy.read_back_probe() is None, (
        "an uninitialised machine answered a datamodel read"
    )

    engine.initialize()

    answer = policy.read_back_probe()
    assert answer is not None, (
        "`readBackProbe` could not be read. A structured `<data>` must be "
        "readable off the machine, as the JSON its own session serialises it to."
    )
    assert answer.startswith("["), (
        f"an authored array came back as {answer!r}. The first character of JSON "
        "is its type, and this reader answers only for the shape the document "
        "declared."
    )
    assert "first" in answer and "Escape" in answer, (
        f"the answer {answer!r} is missing what the document wrote into "
        "`readBackProbe`. A reader producing well-formed JSON of the wrong value "
        "would pass the check above and fail here."
    )
