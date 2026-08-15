# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.3 + Appendix D: an ancestor takes no default child — Python AOT.

Appendix D asks two different questions with two functions.
``addDescendantStatesToEnter`` gives a compound state its default child and is
called for the transition's TARGET; ``addAncestorStatesToEnter`` walks the
states between the target and the LCCA and adds them WITHOUT defaults. Its one
exception is a parallel ancestor, whose other regions do get theirs.

Measured 2026-08-15 on the worked example ``examples/ai_loop/ai_loop.scxml``,
where the wrongly-entered state's ``<onentry>`` sends a prompt: the supervised
session was re-sent its opening prompt every time a person answered a dialog.

The document is driven twice on purpose. ``cross`` enters the ``<parallel>``
itself, so ``run`` is a parallel ancestor and ``drive``/``outer`` are compound
ones; ``again`` runs with the parallel already active, so only ``outer`` is
entered. Those are different branches of the generated entry walk.

Fixture: ``integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_ancestor_entry_is_not_default_entry_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import ancestor_entry_is_not_default_entry_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AncestorEntryIsNotDefaultEntryState
_Event = _sm.AncestorEntryIsNotDefaultEntryEvent


def test_ancestor_entry_is_not_default_entry_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.AWAY in entry, (
        f"fixture came up as {entry}; the run has to start OUTSIDE the `<parallel>` for the "
        "first pass to be testing anything — a source already inside it leaves the ancestors "
        "active and the entry chain never reaches their defaults"
    )

    # Pass one: the parallel is not active, so `run` is entered as a parallel
    # ancestor and `drive` and `outer` as compound ones.
    engine.send_event(_Event.CROSS)

    crossed = engine.active_configuration()
    assert _State.CHOSEN in crossed, (
        f"the transition named `chosen` and the machine did not enter it (active: {crossed})"
    )
    assert _State.BY_DEFAULT not in crossed, (
        f"`outer` has two children active at once (active: {crossed}). `by_default` is what "
        "`initial` names, and nothing targeted it — it was entered because the engine gave "
        "`outer` its default child while entering `outer` merely as an ancestor of `chosen`"
    )
    assert _State.IDLE in crossed, (
        f"the region no entering state is inside must still be entered with its default "
        f"(active: {crossed}) — Appendix D's one exception for a parallel ancestor"
    )

    # Pass two: the parallel is already active now, so `run` and `drive` are
    # skipped and only `outer` is entered. That is a different branch of the
    # entry walk, and it is the one a running machine takes.
    engine.send_event(_Event.BACK)
    engine.send_event(_Event.AGAIN)

    again = engine.active_configuration()
    assert _State.BY_DEFAULT not in again, (
        f"`outer` took its default child on the second pass (active: {again}), where the "
        "`<parallel>` was already active and only `outer` itself was entered — the shape the "
        "worked example hits every time a person answers a dialog"
    )

    engine.send_event(_Event.CHECK)

    settled = engine.active_configuration()
    assert _State.SETTLED in settled, (
        f"`check` did not carry the machine to `settled` (active: {settled}). The document "
        "checks its four clauses in document order and lands each in a `<final>` of its own, "
        "so the configuration above names which one broke: failDefaulted, failLobbied, "
        "failIdled, failTargeted"
    )
