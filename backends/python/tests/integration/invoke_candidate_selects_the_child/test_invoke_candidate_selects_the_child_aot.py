# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4.3 on the Python AOT path — the value selects the child.

The sibling fixture asks whether the expression is evaluated. This one asks
whether its value means anything: until ``sce:candidates`` existed the child
was fixed at build time and one stub answered whatever was computed
(docs/SCE_ACCEPTED_SUBSET.md §2.13).

The two candidates announce themselves, so the three outcomes are distinct:

    ran ``chosen``  -> ``from.chosen``     -> ``pass``
    ran ``other``   -> ``from.other``      -> ``wrongChild``
    loaded nothing  -> ``error.execution`` -> ``noChild``
    ran a stub      -> no event at all     -> parked in ``probe``

⚠ This channel found the first defect in the selection itself. The stem is
cut out of the evaluated value, and the first version read ``str(value)`` on
a ``ScriptValue`` — a dataclass, so the stem came out of its ``repr`` and the
fixture reached ``fail`` on its very first run. A fixture whose candidates
answered alike would have passed.

Fixture: ``integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml``.

Regeneration:
  ``scripts/regen_invoke_candidate_selects_the_child_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem invoke_candidate_selects_the_child`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import invoke_candidate_selects_the_child_sm as _sm  # noqa: E402 — path inserted above


def test_invoke_candidate_selects_the_child_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 300:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine never completed; last leaf="
        f"{engine.current_state!s}. Parking means no child spoke: a stub ran, "
        "or nothing did"
    )
    assert str(engine.terminal_state) == "pass", (
        "`wrongChild` means the value selected the other candidate, `noChild` "
        f"means nothing was loaded at all; the machine reached {engine.terminal_state!s}"
    )
