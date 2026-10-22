# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045, section
8) and "Running host invocations" (section 3) — the Python half of the Rust suite's
``a_saved_tree_holds_every_session_where_it_stood.rs`` and
``a_child_sessions_host_invocation_is_served_by_the_trees_host.rs``: the tree fixture
restored from the shared text and saved again byte for byte, refused for a grandchild
of another shape and a child of a later revision by the path of the session, the
invokers a host registers on the machine it built serving every session of its tree,
and an ending routed to the session that made the request.

Fixture: ``sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml``. The
shared text, ``saved/static_invoke_tree_working.json``, is the tree ``working``
leaves:

* ``worker`` was started twice (``rest``, ``back``), and has taken ``go``: it counted a
  step, holds a delayed ``report`` to its parent, asks its host for the time (a
  host-run invocation that has not answered), and runs a grandchild under the hybrid
  candidate ``static_hybrid_watcher``, whose ``pick`` was changed to the other
  candidate after it started (``swap``); it keeps the id the grandchild was started
  under in ``helper_id`` (``idlocation``).
* The parent holds a delayed ``nudge`` to the child (``arm``) and the child's
  ``answer`` in its queue; the child holds the parent's ``swap`` in its own.

Python drives a child synchronously: the event a parent forwards is taken by the
child within the forward, so the state in which the child's queue still holds it is
reached by a restore of the text and not by a run. What a run reaches, the tests of
the run hold; what the text holds, the tests of the text do.
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

from sce_runtime import (  # noqa: E402 -- path inserted above
    HostInvokeEvent,
    JsonNumber,
    StateRefusal,
    saved_state_from_json,
)

_SAVED_AT_MS = 1_700_000_000_000
_SAVED = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved"
_TREE = importlib.import_module("integration.static_datamodel.static_invoke_tree_sm")

# The session id the shared format-2 fixtures give their root.
_ROOT = "fixture-root"
# The session `worker` is: the root's id and the id its first start is started under
# (`<state>.<start>.<invoke>`).
_WORKER = "fixture-root.working.1.worker"
# The standard host type the child asks.
_CLOCK = "sce-host-clock-now"
# A completion of the clock the child's result record reads.
_NOW = '{"error":"","utc_ms":1700000000000,"monotonic_ms":5}'


def _shared() -> str:
    return (_SAVED / "static_invoke_tree_working.json").read_text(encoding="utf-8").strip()


class _Starts:
    """Every start the host was handed, in order, and what it was told to stop."""

    def __init__(self) -> None:
        self.requests: list = []
        self.cancels: list = []

    def serve(self, engine) -> "_Starts":
        """Register the clock on `engine`, the root, recording each start."""

        def handler(event: HostInvokeEvent):
            if event.start is not None:
                self.requests.append(event.start)
            if event.cancel is not None:
                self.cancels.append(event.cancel)
            return None

        engine.register_invoker(_CLOCK, handler)
        return self

    def only(self):
        assert len(self.requests) == 1, f"one start: {self.requests!r}"
        return self.requests[0]


def _started():
    """A root built under the fixtures' id, served, and initialized."""
    engine = _TREE.create_engine(session_id=_ROOT)
    starts = _Starts().serve(engine)
    engine.initialize()
    return engine, starts


def _raise(engine, name: str) -> None:
    engine.send_event(engine.policy.resolve_event_by_name(name))


def _into_second(engine) -> None:
    """`go`, which the root forwards to the child, which takes it, entering `second`
    and starting the clock."""
    _raise(engine, "go")


def _working(engine):
    """The run the shared text is the save of, as far as a synchronous child lets a
    run reach it: started twice, `go`, `swap`, and a delayed send to the child."""
    for name in ("rest", "back", "go", "swap", "arm"):
        _raise(engine, name)
    return engine


def _restored(text: str):
    engine = _TREE.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS)
    return engine


def _saved(engine):
    return _TREE.save_at(engine, _SAVED_AT_MS)


def _acts_of(sends, kind: str) -> list:
    return [send.act.event for send in sends if send.act.kind == kind]


def test_the_tree_a_run_reaches_holds_every_session_where_it_stood() -> None:
    engine, starts = _started()
    _working(engine)
    root = _saved(engine)
    assert root.session_id == _ROOT and root.current == "working"
    assert [(s.id, s.count) for s in root.invoke_starts] == [("worker", 2)], "the worker was started twice"
    assert _acts_of(root.pending, "child") == ["nudge"], "the parent holds a delayed send to the child"

    assert len(root.invokes.children) == 1, "one running child"
    worker = root.invokes.children[0]
    assert (worker.id, worker.start, worker.invoke_id) == ("worker", 2, "working.2.worker")
    child = worker.session
    assert child.session_id == "fixture-root.working.2.worker"
    assert child.current == "second", "the child took `go`"
    assert child.variable("steps") == JsonNumber("1")
    assert child.variable("pick") == "static_hybrid_holder.scxml", "`pick` was changed after the grandchild started"
    assert _acts_of(child.pending, "parent") == ["report"], "the child holds a delayed send to its parent"
    assert [(h.processor_type, h.invoke_id) for h in child.host_invokes] == [(_CLOCK, "clock")]

    assert len(child.invokes.children) == 1, "one grandchild"
    helper = child.invokes.children[0]
    assert helper.document == "static_hybrid_watcher", "the candidate that was started, whatever `pick` names now"
    assert helper.start == 1 and helper.invoke_id.startswith("second.1."), (helper.start, helper.invoke_id)
    assert child.variable("helper_id") == helper.invoke_id, "the idlocation holds the id the grandchild was started under"
    assert len(starts.requests) == 1, "the child asked its host once, as it took `go`"


def test_the_shared_text_restores_and_saves_again_as_it_was() -> None:
    engine = _restored(_shared())
    assert _saved(engine).to_json() == _shared(), "a restored tree stands where the saved one stood"


def test_the_restored_tree_stands_where_the_text_says() -> None:
    root = _saved(_restored(_shared()))
    assert root.session_id == _ROOT and root.current == "working"
    assert [event.name for event in root.external] == ["answer"], "the child's answer waits in the parent's queue"
    assert _acts_of(root.pending, "child") == ["nudge"]
    child = root.invokes.children[0].session
    assert [event.name for event in child.external] == ["swap"], "the parent's `swap` waits in the child's queue"
    assert _acts_of(child.pending, "parent") == ["report"]


def test_a_grandchild_of_another_shape_refuses_the_tree_by_its_path() -> None:
    shared = _shared()
    shape = "a85ea43d8de628b004074e59c0ca90c198a4f7736073bd953750bca35c174c7a"
    text = shared.replace(shape, "0" * 64, 1)
    assert text != shared, "the change applied"
    with pytest.raises(StateRefusal) as refused:
        _restored(text)
    message = str(refused.value)
    assert "invokes[0].session.invokes[0].session" in message and "shape" in message, message


def test_a_child_of_a_later_revision_refuses_the_tree_by_its_path() -> None:
    # The child holds a record of `schema_revised.scxml`, saved at its third
    # revision; a fourth this build does not know is refused, naming the child.
    shared = _shared()
    assert '"revision":3' in shared, "the child names the revision of its record"
    with pytest.raises(StateRefusal) as refused:
        _restored(shared.replace('"revision":3', '"revision":4', 1))
    message = str(refused.value)
    assert "invokes[0].session" in message and "later than" in message, message


def test_a_save_after_any_host_call_finds_nothing_in_transit() -> None:
    # Half of the Rust suite's `nothing_is_in_transit_when_a_tree_is_saved.rs`: after
    # each call of a run that drives the child every way there is (a send made at
    # once, a delayed one armed, a forwarded event, a tick, a start again), the tree
    # saves, the text it saves restores, and the restored tree saves that text again.
    engine, _ = _started()

    def check(after: str) -> None:
        text = _saved(engine).to_json()
        copied = _restored(text)
        assert _saved(copied).to_json() == text, f"the tree restored after {after} stands where it was saved"

    for name in ("ask", "go", "ask", "swap", "ask", "arm", "ask", "rest", "back", "ask", "go", "ask"):
        _raise(engine, name)
        check(f"raising `{name}`")
        engine.advance_time(0)
        check(f"ticking after `{name}`")


# ── a child's host-run invocation is served by the tree's host ─────────────


def test_a_childs_invocation_reaches_the_roots_invoker_and_is_ended_through_the_root() -> None:
    engine, starts = _started()
    _into_second(engine)
    request = starts.only()
    assert (request.processor_type, request.invoke_id) == (_CLOCK, "clock")
    assert request.session_id == _WORKER, "the request names the child session that made it"
    assert not request.restarted, "a start the document makes is never a restart"

    engine.complete_host_invoke_in(_WORKER, _CLOCK, "clock", request.token, _NOW)
    engine.advance_time(0)
    assert engine.policy.v_stamps == 1, "the child took done.invoke.clock and told its parent"


def test_an_ending_for_a_session_the_tree_does_not_hold_or_a_start_it_does_not_run_is_refused() -> None:
    engine, starts = _started()
    _into_second(engine)
    request = starts.only()
    for nowhere in ("fixture-root.nowhere", "fixture-root.working.1.worker.nowhere", "elsewhere"):
        with pytest.raises(StateRefusal) as refused:
            engine.complete_host_invoke_in(nowhere, _CLOCK, "clock", request.token, _NOW)
        assert "no session" in str(refused.value), nowhere
    # The root runs no clock of its own.
    with pytest.raises(StateRefusal) as refused:
        engine.complete_host_invoke_in(_ROOT, _CLOCK, "clock", request.token, _NOW)
    assert "is not running" in str(refused.value)
    # A start ended once is ended: the second ending is stale.
    engine.complete_host_invoke_in(_WORKER, _CLOCK, "clock", request.token, _NOW)
    with pytest.raises(StateRefusal) as late:
        engine.fail_host_invoke_in(_WORKER, _CLOCK, "clock", request.token, "late")
    assert "is not running" in str(late.value)
    engine.advance_time(0)
    assert engine.policy.v_stamps == 1, "the refused endings raised nothing"


def test_a_resumed_childs_invocation_starts_again_at_its_first_drive() -> None:
    engine, starts = _started()
    _into_second(engine)
    first = starts.only()
    restored = _restored(saved_state_from_json(_saved(engine).to_json()).to_json())
    # The host registers its invokers on the machine the restore handed it, after the
    # restore built the child: nothing has started yet.
    served = _Starts().serve(restored)
    assert served.requests == [], "a restore calls no host"

    # The parent's first drive of the child is the child's first macrostep.
    restored.advance_time(0)
    request = served.only()
    assert request.restarted, "started again from the saved request"
    assert (request.session_id, request.invoke_id) == (_WORKER, "clock")
    assert request.token != first.token, "a new token"

    restored.complete_host_invoke_in(_WORKER, _CLOCK, "clock", request.token, _NOW)
    restored.advance_time(0)
    assert restored.policy.v_stamps == 1


def test_an_invoke_idlocation_keeps_the_id_the_grandchild_is_started_under() -> None:
    # docs/adr/0045: the grandchild is started under `second.1.<its invoke id>`, the
    # `invokeid` a saved tree holds for it, and `helper_id` holds the same.
    engine, _ = _started()
    _into_second(engine)
    worker = _saved(engine).invokes.children[0].session
    assert len(worker.invokes.children) == 1, "the hybrid candidate runs"
    started_under = worker.invokes.children[0].invoke_id
    assert started_under.startswith("second.1."), started_under
    assert worker.variable("helper_id") == started_under
