# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.4 and B.2: ``<assign location="v" expr="_event.data"/>`` leaves
``v`` holding the DOM a ``<send>``'s XML ``<content>`` arrived as, and so does
a member-path location, on the Python AOT path.

The template runs ``<location> = <expr>`` in the Lua engine for every
location, so the value never leaves the engine; the C++ engines once read a
bare-name location's value out and wrote it back, which lost the document.

Fixture: ``integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_assignment_keeps_the_document_it_is_given_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem an_assignment_keeps_the_document_it_is_given`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import an_assignment_keeps_the_document_it_is_given_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "nameLost": (
        '`<assign location="v" expr="_event.data"/>` did not leave the document in `v`: '
        "the value was read out of the data model and written back."
    ),
    "memberLost": (
        '`<assign location="holder.doc" expr="_event.data"/>` did not leave the document '
        "in `holder.doc`."
    ),
}


def test_an_assignment_keeps_the_document_it_is_given_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "an_assignment_keeps_the_document_it_is_given did not reach a top-level <final> "
        f"within 100 ms; last leaf={engine.current_state} — its own `doc` event never "
        "arrived"
    )
    actual = str(engine.terminal_state)
    assert actual == "kept", (
        f"an_assignment_keeps_the_document_it_is_given reached <final id={actual!r}>; "
        "expected 'kept'. " + _WHY.get(actual, "That is not a verdict state.")
    )
