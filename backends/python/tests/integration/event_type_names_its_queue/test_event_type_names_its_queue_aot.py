# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10.1: ``_event.type`` names the queue an event was taken from — Python AOT.

The document queues an external event first and two internal ones after it.
Measured 2026-09-26, this channel typed a ``<send target="#_internal">`` that
carried a payload as "external": the metadata it was built with kept the
dataclass default, and the internal queue never corrected it.

Fixture: ``integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_event_type_names_its_queue_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import event_type_names_its_queue_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.EventTypeNamesItsQueueState


def test_each_event_is_typed_by_the_queue_it_was_taken_from() -> None:
    engine = _sm.create_engine()
    # The document queues its own events; the run needs nothing from the host.
    engine.initialize()

    p = engine.policy
    seen = (
        f"intCode={p.int_code()} sendCode={p.send_code()} extCode={p.ext_code()} "
        "(1 internal, 2 external, 3 other; wanted 1 / 1 / 2)"
    )
    assert engine.terminal_state == _State.DONE, f"`ext` must carry the run to `done`. {seen}"
    assert p.int_code() == 1, (
        f"`int` came off the internal queue while `ext` waited on the external one. {seen}"
    )
    assert p.send_code() == 1, (
        f'a `<send target="#_internal">` with a payload rides the internal queue too. {seen}'
    )
    assert p.ext_code() == 2, f"`ext` came off the external queue. {seen}"
