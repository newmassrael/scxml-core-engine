# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045): a child
session an ``<invoke type="scxml">`` started is part of what a machine is doing, so
a saved state holds each running child inside its parent, with its whole state, and
a restore resumes it where it stood. Nothing a child did at its start happens again,
and it raises its ``done.invoke`` once, when it ends.

``static_invoke.scxml`` invokes ``worker``, a child that takes ``a`` and then ``b``
and ends; the parent forwards what it is sent (``autoforward``), so the host drives
the child through the parent and every case is read from the parent:
``done.invoke.worker`` counts the run in ``completed`` and leaves the parent in
``working``. A child that had taken ``a`` is resumed at ``second``, so ``b`` alone
ends it.

The shared instance ``saved/static_invoke_working.json`` is the format-2 text of the
machine just started, its root given the id the fixtures name it by. A format-1 text
holds the child's id alone, and the build that wrote it promised to start the child
again from its beginning: ``saved/format1/static_invoke_working.json`` is the one
case in which a restore does (docs/adr/0045, section 3). The Rust, Kotlin and Go
halves are ``a_saved_child_session_is_resumed_after_a_restore.rs``,
``ASavedChildSessionIsResumedAfterARestoreTest.kt`` and
``a_saved_child_session_is_resumed_after_a_restore_test.go``.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import StateRefusal, saved_state_from_json  # noqa: E402 -- path inserted above

_SAVED_AT_MS = 1_700_000_000_000
_SAVED = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved"
_STATIC_INVOKE = importlib.import_module("integration.static_datamodel.static_invoke_sm")


def _shared(format1: bool = False) -> str:
    directory = _SAVED / "format1" if format1 else _SAVED
    return (directory / "static_invoke_working.json").read_text(encoding="utf-8").strip()


def _started():
    """The machine just started, its root given the id the shared fixtures name it by."""
    engine = _STATIC_INVOKE.create_engine(session_id="fixture-root")
    engine.initialize()
    return engine


def _restored(text: str):
    return _STATIC_INVOKE.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS)


def _saved(engine):
    return _STATIC_INVOKE.save_at(engine, _SAVED_AT_MS)


def _send(engine, name: str) -> None:
    """Send `name` to the parent, which forwards it to its child, and let the child
    run."""
    engine.send_event(engine.policy.resolve_event_by_name(name))
    for _ in range(3):
        engine.advance_time(0)


def _working(engine) -> bool:
    return _STATIC_INVOKE.State.WORKING in engine.active_configuration()


def test_a_machine_is_saved_with_the_whole_state_of_the_child_it_is_running() -> None:
    saved = _saved(_started())
    assert saved.invokes.resumed and len(saved.invokes.children) == 1
    child = saved.invokes.children[0]
    assert (child.id, child.start) == ("worker", 1)
    assert [(s.id, s.count) for s in saved.invoke_starts] == [("worker", 1)]
    # Its id is this session's and the id it was started under: the tree's alone.
    assert child.invoke_id == "working.1.worker"
    assert child.session.session_id == "fixture-root.working.1.worker"
    assert child.session.current == "first"
    # The text every backend writes for the same machine, byte for byte.
    assert saved.to_json() == _shared()


def test_a_child_that_had_taken_an_event_is_resumed_with_it_taken() -> None:
    before = _started()
    _send(before, "a")
    saved = _saved(before)

    # Resumed at `second`: `b` alone ends it, and its `done.invoke` arrives once.
    after = _restored(saved.to_json())
    assert _working(after), "the restored machine stands in `working`"
    _send(after, "b")
    assert after.policy.v_completed == 1, "the child resumed where it stood and ended on `b`"
    _send(after, "b")
    assert after.policy.v_completed == 1, "it ended once"


def test_a_restored_tree_saves_the_text_it_was_restored_from() -> None:
    engine = _restored(_shared())
    assert _saved(engine).to_json() == _shared(), "the child kept its id, its start and where it stood"


def test_the_count_of_starts_is_carried_through_a_restore() -> None:
    # The resumed child was not started again, and when it ends the count the
    # restore carried on is still its one start.
    before = _started()
    _send(before, "a")
    after = _restored(_saved(before).to_json())
    _send(after, "b")
    assert after.policy.v_completed == 1
    again = _saved(after)
    assert [(s.id, s.count) for s in again.invoke_starts] == [("worker", 1)]
    assert again.invokes.invoke_ids() == [], "the child ended, and is not listed"


def test_a_child_that_has_ended_is_not_resumed_or_started_again() -> None:
    # `done.invoke.worker` leaves the parent in `working`, so a machine saved after
    # its child ended stands in the state that invokes it with nothing running. The
    # invocation is complete: it is not listed, and a restore does not run it a
    # second time.
    before = _started()
    _send(before, "a")
    _send(before, "b")
    assert before.policy.v_completed == 1 and _working(before)
    saved = _saved(before)
    assert saved.invokes.invoke_ids() == [], "the child ended, and is not listed"

    after = _restored(saved.to_json())
    assert after.policy.v_completed == 1, "the run is kept"
    _send(after, "a")
    _send(after, "b")
    assert after.policy.v_completed == 1, "a child started again would have ended again"


def test_a_machine_that_left_the_invoking_state_saves_no_invocation() -> None:
    engine = _started()
    _send(engine, "abort")
    assert not _working(engine), "`abort` leaves `working`"
    saved = _saved(engine)
    assert saved.invokes.invoke_ids() == [], "cancelled with its state"

    # And nothing is started when it comes back: no child is there to take `a` and
    # `b`, and `idle` invokes nothing.
    again = _restored(saved.to_json())
    _send(again, "a")
    _send(again, "b")
    assert not again.reached_final


def test_an_event_saved_in_the_childs_queue_is_taken_by_the_resumed_child() -> None:
    # `a` reached the child's own queue and the child had not taken it when the tree
    # was saved: it is in the child's `external`, and the resumed child takes it, then
    # ends on `b`.
    shared = _shared()
    empty = (
        '"configuration":["first"],"current":"first","variables":{},"history":{},"pending":[],"invokes":[],'
        '"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]'
    )
    queued = empty.replace(
        '"external":[]',
        '"external":[{"name":"a","data":"","type":"external","sendid":"","origin":"","origintype":"",'
        '"invokeid":"","hostinvoketoken":null}]',
        1,
    )
    text = shared.replace(empty, queued, 1)
    assert text != shared, "the child's queue was changed"
    engine = _restored(text)
    for _ in range(3):
        engine.advance_time(0)
    _send(engine, "b")
    assert engine.policy.v_completed == 1, "the queued `a` reached the resumed child"


@pytest.mark.parametrize(
    "what, old, new, expected",
    [
        ("a child the document does not invoke", '"id":"worker"', '"id":"stranger"', "which the document does not invoke"),
        ("a start that is not the count its invoke holds", '"start":"1"', '"start":"2"', "the session counts 1 starts of it"),
        (
            "a child of a document its invoke does not start",
            '"document":"static_invoke__sce_synth_invoke__worker"',
            '"document":"static_hybrid_holder"',
            "does not start",
        ),
        (
            "a child whose id is not its parent's and the id it was started under",
            '"sessionid":"fixture-root.working.1.worker"',
            '"sessionid":"elsewhere"',
            "a child of 'fixture-root' started under 'working.1.worker'",
        ),
        (
            "a child of another shape",
            "688ad00d954ad9f35bdaa5eae06768a788173e0f8a8236e670efa68038781a6d",
            "0" * 64,
            "the session at invokes[0].session: the saved state is of a document of shape",
        ),
        (
            "a child that stands in no state of its document",
            '"configuration":["first"],"current":"first"',
            '"configuration":["nowhere"],"current":"nowhere"',
            "the session at invokes[0].session: the document has no state 'nowhere'",
        ),
    ],
)
def test_a_tree_whose_child_is_not_one_the_document_could_run_is_refused_whole(
    what: str, old: str, new: str, expected: str
) -> None:
    shared = _shared()
    text = shared.replace(old, new, 1)
    assert text != shared, f"{what}: the change did not apply"
    with pytest.raises(StateRefusal) as refused:
        _restored(text)
    assert expected in str(refused.value), f"{what}: refused as {refused.value!s}"


# ── format 1: a child named by its id is started again ─────────────────────


def test_a_format_one_state_starts_its_child_again_from_the_beginning() -> None:
    # The only case in which a child restarts: the text holds nothing of it.
    engine = _restored(_shared(format1=True))
    assert _working(engine), "the restored machine stands in `working`"

    # The child is at `first`: `b` is not what it waits for.
    _send(engine, "b")
    assert _working(engine) and engine.policy.v_completed == 0, "`b` alone does not end a child that starts over"

    _send(engine, "a")
    _send(engine, "b")
    assert engine.policy.v_completed == 1, "`a` then `b` ends it, and `done.invoke.worker` arrived once"


@pytest.mark.parametrize(
    "what, old, new, expected",
    [
        ("an id the document does not invoke", '"invokes":["worker"]', '"invokes":["stranger"]', "which the document does not invoke"),
        ("an id named twice", '"invokes":["worker"]', '"invokes":["worker","worker"]', "which an earlier entry already names"),
    ],
)
def test_a_format_one_state_naming_an_invocation_the_document_cannot_have_run_is_refused(
    what: str, old: str, new: str, expected: str
) -> None:
    shared = _shared(format1=True)
    assert old in shared, what
    with pytest.raises(StateRefusal) as refused:
        _restored(shared.replace(old, new, 1))
    assert expected in str(refused.value), f"{what}: refused as {refused.value!s}"


def test_a_format_one_invocation_whose_state_the_configuration_is_not_in_is_refused() -> None:
    shared = _shared(format1=True)
    text = shared.replace('"configuration":["working"]', '"configuration":["idle"]', 1).replace(
        '"current":"working"', '"current":"idle"', 1
    )
    assert text != shared
    with pytest.raises(StateRefusal) as refused:
        _restored(text)
    assert "whose state the saved configuration does not stand in" in str(refused.value)
