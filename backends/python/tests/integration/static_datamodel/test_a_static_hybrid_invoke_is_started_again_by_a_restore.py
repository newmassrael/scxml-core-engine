# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid ``<invoke>``
(§2.13). A format-2 saved state records the candidate that was running, with its whole
state, and a restore resumes it whatever ``srcexpr`` reads now: the arguments were
evaluated when the invocation started (§scxml-6.4), and a resumed child did not start
(docs/adr/0045). A format-1 state named the invocation alone, and its restore starts it
again from its beginning, reading ``srcexpr`` and the arguments from the restored
variables as entering the state would: the one case in which a child restarts.

``static_invoke_hybrid_saved.scxml`` invokes ``watch``, whose ``srcexpr`` is ``pick``
among ``static_hybrid_watcher`` (ends on the 8 it is handed) and
``static_hybrid_holder`` (ends on the 7). ``base`` is 7 when the child starts, and
``bump`` makes it 8 without telling the child; ``swap`` makes ``pick`` name the other
candidate. The Rust, Kotlin and Go halves are
``a_static_hybrid_invoke_is_started_again_by_a_restore.rs``,
``AStaticHybridInvokeIsStartedAgainByARestoreTest.kt`` and
``a_static_hybrid_invoke_is_started_again_by_a_restore_test.go``.
"""
from __future__ import annotations

import dataclasses
import importlib
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import SavedInvokes, saved_state_from_json  # noqa: E402 -- path inserted above

_SAVED_AT_MS = 1_700_000_000_000
_HYBRID = importlib.import_module("integration.static_datamodel.static_invoke_hybrid_saved_sm")


def _started():
    engine = _HYBRID.create_engine()
    engine.initialize()
    return engine


def _settle(engine) -> None:
    """Let the child run and report to its parent."""
    for _ in range(5):
        engine.advance_time(0)


def _raise(engine, name: str) -> None:
    engine.send_event(engine.policy.resolve_event_by_name(name))


def _saved(engine):
    return _HYBRID.save_at(engine, _SAVED_AT_MS)


def _restored_from(saved):
    return _HYBRID.restore(saved_state_from_json(saved.to_json()), wall_now_ms=_SAVED_AT_MS)


def _restored(engine):
    """The machine saved and restored, as a host carries it through its own storage."""
    return _restored_from(_saved(engine))


def _as_format_one(saved):
    """The same moment as a format-1 state, which names the running invocation by its id
    alone. The document holds no enum, so its format-1 shape is the format-2 one the
    save wrote."""
    return dataclasses.replace(
        saved,
        invokes=SavedInvokes(resumed=False, ids=saved.invokes.invoke_ids()),
        session_id="",
        invoke_starts=[],
    )


def _documents(saved) -> list:
    """The document each running child of `saved` runs."""
    assert saved.invokes.resumed, "a format-2 state names each child's document"
    return [child.document for child in saved.invokes.children]


def _working(engine) -> bool:
    return _HYBRID.State.WORKING in engine.active_configuration()


def test_a_restored_hybrid_child_keeps_the_values_it_was_handed() -> None:
    engine = _started()
    _settle(engine)
    # The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and it is not
    # told.
    _raise(engine, "bump")
    _settle(engine)

    # Resumed, the watcher holds the 7 it was handed: nothing is evaluated again, so it
    # still waits.
    restored = _restored(engine)
    _settle(restored)
    assert restored.policy.v_completed == 0 and _working(restored)


def test_a_hybrid_restore_resumes_the_candidate_that_was_running_whatever_srcexpr_reads_now() -> None:
    engine = _started()
    _settle(engine)
    # The watcher is the child that is running when the machine is saved, though `pick`
    # now names the holder.
    _raise(engine, "swap")
    _raise(engine, "bump")
    _settle(engine)
    assert engine.policy.v_completed == 0
    saved = _saved(engine)
    assert _documents(saved) == ["static_hybrid_watcher"], "the running child"

    restored = _restored_from(saved)
    _settle(restored)
    assert _documents(_saved(restored)) == ["static_hybrid_watcher"], (
        "the watcher was resumed, not the holder `pick` names now"
    )
    assert _working(restored), "the watcher waits for an 8 it was not handed"


def test_a_hybrid_child_nothing_has_changed_for_waits_as_it_did() -> None:
    engine = _started()
    _settle(engine)
    restored = _restored(engine)
    _settle(restored)
    # `base` is still 7: the watcher waits for an 8, as it did.
    assert restored.policy.v_completed == 0 and _working(restored)


def test_a_hybrid_child_that_ended_is_not_started_again() -> None:
    # In format 1 the watcher started again is handed the 8 `base` holds, and ends.
    engine = _started()
    _settle(engine)
    _raise(engine, "bump")
    _settle(engine)
    restarted = _restored_from(_as_format_one(_saved(engine)))
    _settle(restarted)
    assert restarted.policy.v_completed == 1

    # Saved once its `done.invoke` was taken, the machine is past `working`, and a
    # restore names no invocation to resume or start.
    assert not _working(restarted), "the machine is past `working`"
    again = _restored(restarted)
    _settle(again)
    assert again.policy.v_completed == 1, "the child ran twice"
    assert not _working(again), "the restored machine is past `working`"


# ── format 1: the invocation named by its id is started again ───────────────


def test_a_format_one_restore_hands_the_child_the_values_the_restored_machine_holds() -> None:
    engine = _started()
    _settle(engine)
    _raise(engine, "bump")
    _settle(engine)
    assert engine.policy.v_completed == 0

    # Started again from its beginning, the watcher is handed 8, and ends.
    restarted = _restored_from(_as_format_one(_saved(engine)))
    _settle(restarted)
    assert restarted.policy.v_completed == 1


def test_a_format_one_restore_reads_the_srcexpr_again() -> None:
    engine = _started()
    _settle(engine)
    _raise(engine, "swap")
    _raise(engine, "bump")
    _settle(engine)
    assert engine.policy.v_completed == 0

    # The restored fields name the holder, which is handed 8 and waits for 7. The
    # watcher, which would have ended on that 8, is not what the restore starts.
    restarted = _restored_from(_as_format_one(_saved(engine)))
    _settle(restarted)
    assert restarted.policy.v_completed == 0, "the watcher was started"
    assert _working(restarted), "the holder waits for a 7 it is not handed"
    # The holder keeps its child in a slot of its own: the invocation is still running,
    # and a save names it.
    assert _documents(_saved(restarted)) == ["static_hybrid_holder"]
