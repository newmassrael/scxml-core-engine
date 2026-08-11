# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix C.1 ``_event.origin`` is an address — Python AOT.

The clause has two halves. The origin of a delivered event must match the
``location`` field the sending session published for the SCXML Event I/O
Processor in its ``_ioprocessors``, and that location is what a peer sends
back to. A machine that puts a bare session id — or an invoke-instance path
— there satisfies neither: the value matches nothing the sender published,
and it names no target.

The public IRP suite cannot separate the two spellings. Test 336 and test
350 both check ``_event.origin`` by sending to it with the sender and the
receiver being the same session, so any value at all round-trips. Nothing in
the corpus sends across sessions, which is the only arrangement where a bare
id and a location differ.

The fixture puts a second session on the other end, so the two halves
separate and each has its own signal:

mismatch
    The parent lands in ``fail`` — ``_event.origin`` did not equal the
    location the child published for itself.

routing
    The parent parks in ``await_reply`` and the run times out — a target
    that resolves nowhere delivers no event to fail on.

Fixture: ``integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml``
(canonical, shared with the C++ / Rust / Go / Kotlin / C11 channels).

Regeneration (after fixture or template edit):
  ``scripts/regen_event_origin_is_a_location_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem event_origin_is_a_location`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import event_origin_is_a_location_sm as _sm  # noqa: E402 — path inserted above


def test_event_origin_is_a_location_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 2000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "event_origin_is_a_location did not reach a top-level <final> within 2 s; "
        f"last leaf={engine.current_state}. The parent accepted `_event.origin` as "
        "an address and sent `reply` to it, and nothing came back: Appendix C.1 "
        "requires the published location to be a usable <send> target, so an origin "
        "that routes nowhere fails the half a self-addressed test cannot exercise."
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"event_origin_is_a_location reached <final id={actual!r}>; expected 'pass'. "
        "`_event.origin` did not carry the sender's published `_ioprocessors` "
        "location: Appendix C.1 requires the origin to match that location, which is "
        "what makes it an address a peer can answer; a bare session id or an "
        "invoke-instance path matches nothing the sender published."
    )
