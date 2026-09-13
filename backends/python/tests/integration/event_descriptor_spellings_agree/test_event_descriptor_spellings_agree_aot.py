# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.12.1: ``wild``, ``wild.`` and ``wild.*`` are one descriptor — Python AOT path.

The clause calls the three spellings "functionally equivalent since they are
token prefixes of exactly the same set of event names", and a descriptor ending
in ``.*`` matches "zero or more tokens", so a bare ``.*`` is an empty token
prefix and matches every event.

No W3C fixture delivers a bare ``foo`` to a ``foo.*`` handler, and the four that
write a bare ``.*`` write it as a catch-all to fail — which an engine that
matches nothing on ``.*`` passes by never taking the transition it must not
take. This fixture puts every case in positive polarity instead.

Fixture: ``integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_event_descriptor_spellings_agree_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem event_descriptor_spellings_agree`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import event_descriptor_spellings_agree_sm as _sm  # noqa: E402 — path inserted above


def test_event_descriptor_spellings_agree_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 2000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine never reached a final state (parked in "
        f"{engine.current_state!s}); every case in this fixture has a literal "
        "fallback, so parking means no transition matched an event that two of "
        "them describe"
    )

    # Each failure final names the case, so this one assertion says which
    # spelling disagreed with the clause:
    #   failSuffixed   `wild.*` did not catch bare `wild`
    #   failDotted     `dot.` did not catch bare `dot`
    #   failBounded    `wild.*` caught `wilder`, across a token boundary
    #   failUniversal  a bare `.*` did not catch `any.token.sequence`
    assert str(engine.current_state) == "pass", (
        f"the machine rested in {engine.current_state!s}: `wild.*` must catch bare "
        "`wild` and `dot.` must catch bare `dot` (the clause calls the spellings "
        "functionally equivalent), a bare `.*` must catch every event, and "
        "`wild.*` must NOT catch `wilder` because the prefix is a whole token"
    )
