# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2.4 + 6.3 — a delayed ``<send>`` a HOST serves waits, and can be
cancelled while it waits. Python AOT path.

W3C SCXML 6.2.4 puts the wait before the dispatch and says nothing about which
processor the send named; 6.2.5 makes that set open. Put together, a host-served
send carrying a delay is an ordinary delayed send whose delivery happens to be
somebody else's. It was not: every backend chose the host branch ahead of the
delay branch in one ``elif`` chain per language, so the act was performed at the
instant the block ran and ``delay`` was discarded — while the manifest went on
answering ``needs_event_scheduler: true``, telling the host to drive the machine
for a wait the engine had already thrown away.

This backend drives virtual time by construction (``advance_time``), so nothing
here sleeps and nothing here can be decided by how loaded the build machine is.
That matters more than usual on this axis: the handler running "early" is only
observable against a clock the test controls.

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml``,
the same document the other five channels drive, generated WITH
``--host-processor x-sce-host``. The declaration is load-bearing: without it
codegen emits the refusal and every case below would measure the refusal
instead of the feature.

Regeneration (after fixture or template edit):
  ``scripts/regen_host_processor_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import HostSendRequest, HostSendResponse  # noqa: E402

import statechart_delayed_host_send_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for. ``scripts/regen_host_processor_python.sh``
# passes this same string to ``--host-processor``.
DECLARED_TYPE = "x-sce-host"

State = _sm.StatechartDelayedHostSendState


def _armed(with_handler: bool = True):
    """A machine with the handler each case decides on, already initialized.

    Registration happens BEFORE ``initialize()``: the fixture's first send is
    armed on entry to its initial state, so a handler registered afterwards
    would be measuring a run that had already made its decision.

    ``calls`` collects the engine's own reading of "now" at the moment the
    handler was asked to perform the act — the number the contract is about. A
    counter alone would say it happened, not when the engine thought it was.
    """
    engine = _sm.create_engine()
    calls: List[int] = []
    if with_handler:

        def handler(_request: HostSendRequest) -> List[HostSendResponse]:
            calls.append(engine.now_ms)
            return [HostSendResponse(event_name="turn.done", event_data="")]

        engine.register_event_processor(DECLARED_TYPE, handler)
    engine.initialize()
    return engine, calls


def test_a_host_served_send_waits_for_its_delay() -> None:
    """The axis. ``waiting`` arms a host-served send for 200 ms and an ordinary
    one for 100 ms; the ordinary one must arrive first, which is only true if
    the host-served one waited.

    The ``tooEarly`` final state is what the document reaches when it did not:
    the handler's reply is on the queue before the machine has been anywhere,
    so ``turn.done`` wins the race its own ``delay`` was supposed to lose."""
    engine, calls = _armed()

    # Nothing is due at 0 ms. This is the whole defect in one assertion: with
    # the host branch chosen ahead of the delay branch, initialize() has
    # already performed the act by the time this line runs.
    assert calls == [], (
        "the handler was asked to perform a delay=\"200ms\" send at 0 ms. W3C SCXML "
        "6.2.4 makes the delay the wait the document asked for, and 6.2.5 does not "
        "exempt a host-served processor from it"
    )
    assert engine.current_state == State.WAITING

    # 100 ms: the ordinary `probe` is due, the host-served send is not.
    engine.advance_time(100)
    assert engine.current_state == State.ARMED, (
        f"the 100 ms `probe` did not arrive first; the machine is in {engine.current_state}"
    )
    assert calls == [], "the host-served send was dispatched before its 200 ms deadline"

    # 200 ms: now it is due, and the handler's reply moves the machine on.
    engine.advance_time(100)
    assert calls == [200], f"the host-served send did not fire at its 200 ms deadline: {calls}"
    assert engine.current_state == State.CANCELLING, (
        "the handler's `turn.done` did not reach the document"
    )


def test_a_cancel_drops_a_pending_host_served_send() -> None:
    """W3C SCXML 6.3: a ``<cancel>`` drops a delayed send that has not been
    dispatched. A host-served one is not exempt, and the witness is host-side:
    the handler must never be asked to perform the cancelled act at all.

    This is the half that says which queue the deferred send is in. An engine
    that honoured the delay by any private means — a side list, a timer thread —
    would pass the case above and fail here, because ``<cancel sendid>`` reaches
    the scheduler and nothing else."""
    engine, calls = _armed()

    engine.advance_time(100)  # probe     -> armed
    engine.advance_time(100)  # turn.done -> cancelling (arms h2 for 400)
    engine.advance_time(100)  # settle    -> cancelPending (cancels h2)
    assert engine.current_state == State.CANCEL_PENDING, (
        'the second round did not reach the state that runs <cancel sendid="h2">; '
        f"the machine is in {engine.current_state}"
    )

    # 400 ms: h2's deadline. It was cancelled at 300, so nothing may happen.
    engine.advance_time(100)
    assert calls == [200], (
        'the handler was asked to perform `h2` at 400 ms after <cancel sendid="h2"> ran '
        f"at 300 ms (calls = {calls}). A host-served act that a document cancelled must "
        "not reach the host: the side effect is the point of the act, and the document "
        "cannot take it back"
    )
    assert engine.current_state != State.CANCEL_LOST, (
        "`turn.done` arrived for the cancelled send"
    )

    # 500 ms: `finish`. The verdict is itself scheduled, so a channel whose tick
    # loop stopped working fails here rather than passing by not moving.
    engine.advance_time(100)
    assert engine.current_state == State.PASS, (
        f"the machine did not reach `pass`; it is in {engine.current_state}"
    )


def test_a_deferred_send_with_no_handler_reports_it_when_it_comes_due() -> None:
    """A deferred act whose handler was never registered is still an act nobody
    performed, and W3C SCXML 6.2 reports that as ``error.execution`` — at the
    moment it was to be performed, not at the moment it was armed.

    The immediate path raises this at the send site. The deferred path cannot:
    that site has already returned by the time the deadline arrives, so the
    engine owes the report. Without this case a wiring mistake on a delayed send
    is perfect silence — the document waits for a reply that no longer has
    anyone to come from."""
    engine, _calls = _armed(with_handler=False)

    # At 100 ms the machine is in `armed`, whose `error.execution` transition is
    # the witness. Nothing has reported anything yet: the send was armed, not
    # performed, so there is nothing to report.
    engine.advance_time(100)
    assert engine.current_state == State.ARMED, (
        "the report arrived before the send was due; error.execution must be raised when "
        f"the act was to be performed, not when it was armed. In {engine.current_state}"
    )

    # 200 ms: the deadline. Nobody is registered, so nobody performs it, and
    # W3C SCXML 6.2 says so.
    engine.advance_time(100)
    assert engine.current_state != State.CANCELLING, (
        "nothing was registered to perform the act, yet `turn.done` arrived"
    )
    assert engine.current_state == State.UNSERVED, (
        "the deadline passed with no handler registered and nothing was reported (the "
        f"machine is in {engine.current_state}). The send site that raises this for an "
        "immediate send returned when the send was armed, so whatever holds the deferred "
        "act owes the report — without it a wiring mistake on a delayed send is perfect "
        "silence"
    )


def test_the_engine_says_when_the_deferred_host_send_is_due() -> None:
    """The engine must be able to say when the deferred host send comes due, or
    a host driving on ``time_until_next_scheduled_ms`` sleeps straight past it.

    A deferred act kept anywhere the deadline query cannot see would leave this
    answering ``None`` at 0 ms — "nothing is owed" — while an act was owed at
    200."""
    engine, _calls = _armed()

    due = engine.time_until_next_scheduled_ms()
    assert due == 100, (
        f"the nearer of the two armed sends is the 100 ms `probe`; the engine answered {due}"
    )

    engine.advance_time(100)
    due = engine.time_until_next_scheduled_ms()
    assert due == 100, (
        f"at 100 ms the host-served send is 100 ms out; the engine answered {due}. A host "
        "sleeping on this answer must land on the deferred act, not past it"
    )
