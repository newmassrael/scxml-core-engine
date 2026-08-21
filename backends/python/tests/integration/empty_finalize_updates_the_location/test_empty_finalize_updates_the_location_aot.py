# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.5.2 on the Python AOT path — what an EMPTY ``<finalize>`` does.

With no executable content the Processor "MUST update the data model each time
an event is received from the child process ... for each item in the
'namelist' attribute and each such ``<param>`` element ... as if by
``<assign>`` with any return value that has a name that matches", and then:
"Note that the automatic update does not take place if the ``<finalize>``
element is absent as opposed to empty."

The corpus holds two ``<finalize>`` documents (W3C 233/234) and zero empty
ones. Measured 2026-08-22, no channel implemented the automatic update: every
engine gates the finalize step on the content being non-empty, and the AOT
model had no way to tell an empty element from a missing one.

Fixture: ``integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml``.

Regeneration:
  ``scripts/regen_empty_finalize_updates_the_location_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem empty_finalize_updates_the_location`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import empty_finalize_updates_the_location_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "failNotUpdated": (
        "the empty <finalize/> left `tally` at its old value: W3C SCXML 6.5.2 makes "
        "an empty element mean the automatic update — for each namelist item the "
        "Processor updates the location as if by <assign> with the matching return "
        "value."
    ),
    "failUpdatedWithoutFinalize": (
        "`guard` moved with no <finalize> element at all: the clause's note is a "
        "prohibition — \"the automatic update does not take place if the <finalize> "
        "element is absent as opposed to empty\"."
    ),
    "failUnmatchedNameWrote": (
        "an event carrying no matching name still wrote `keeper`: W3C SCXML 6.5.2 says "
        "\"with ANY return value that has a name that matches\", so an unconditional "
        "write blanks the parent's data model on every unrelated answer."
    ),
    "failUnmatchedChildSilent": (
        "the third child never answered, so the guarded-write half was never exercised."
    ),
    "failEmptyChildSilent": (
        "the first child never answered, so the empty-<finalize> half was never "
        "exercised."
    ),
    "failAbsentChildSilent": (
        "the second child never answered, so the absent-<finalize> half was never "
        "exercised."
    ),
}


def test_empty_finalize_updates_the_location_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    # Each phase is settled by a 3 s delayed <send>, so the clock has to
    # advance past both before a verdict exists.
    elapsed = 0
    while not engine.reached_final and elapsed < 15000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "empty_finalize_updates_the_location did not reach a top-level <final> "
        f"within 15 s; last leaf={engine.current_state}"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"empty_finalize_updates_the_location reached <final id={actual!r}>; "
        "expected 'pass'. " + _WHY.get(actual, "That is not a verdict state.")
    )
