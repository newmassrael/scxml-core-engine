# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The AI supervision loop, driven through the Python AOT engine.

``examples/ai_loop/ai_loop.scxml`` is a worked example: a statechart that
supervises a long-running session, with ``<parallel>`` splitting the turn cycle
from the liveness watch and the turn budget. The C++, Rust, Go and Kotlin
channels drive the same document; this is the fifth.

Why a fifth: a clause asserted in one channel is that engine's word for the
document rather than the document's own, and the parallel defect that shipped in
``1419a050ed`` (a self-transition whose exit set swallowed the parallel root) was
invisible to every W3C fixture because they are all one region deep. This
document is three. ``sce-build/tests/ai_loop_channel_parity.rs`` holds every
registered channel to the same scenario set by name, so a scenario added here
without its siblings fails there — which is the moment it is cheapest to fix.

No sprag, no session, no pane: every effect the host would perform is replaced by
the event that effect would have produced, so what is under test is the machine's
topology rather than any driver's plumbing.

Because the regions are orthogonal, a scenario asserts on the ACTIVE SET rather
than on one state — "the cycle is working AND the budget is within" is the kind
of claim a parallel machine makes, and asserting a single current state cannot
express it. This engine makes that unavoidable rather than merely advisable: it
does not store a current state at all, and ``current_state`` answers the
document-order-earliest active leaf.

Fixture: ``examples/ai_loop/ai_loop.scxml``

Regeneration (after example or template edit):
  ``scripts/regen_ai_loop_python.sh``
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import List, Tuple

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import ConfigurationRejection, EventMetadata  # noqa: E402
from sce_runtime import HostSendRequest, HostSendResponse  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine, ScriptValue  # noqa: E402

import ai_loop_sm as _sm  # noqa: E402 — path inserted above

State = _sm.AiLoopState
Event = _sm.AiLoopEvent

# The processor type the committed machine was generated for.
# ``scripts/regen_ai_loop_python.sh`` passes this same string to
# ``--host-processor``, and the other four channels register the same one.
DECLARED_TYPE = "x-sce-host"


def _silent(_request: HostSendRequest) -> List[HostSendResponse]:
    return []


def _not_initialised(handler=_silent, script_engine=None):
    """A machine wired the way every scenario wires one, stopping short of
    booting it.

    The handler is registered before ``initialize()`` because ``priming``
    performs its act on entry: a machine booted without one raises
    ``error.execution`` there instead of reaching a host.

    W3C SCXML 6.2.5: the document declares its acts as sends a host serves. The
    default handler performs nothing and reports nothing, which is deliberate —
    what these scenarios measure is the TOPOLOGY, and each supplies the events a
    host would have produced at exactly the point it wants them. A handler that
    answered would deliver the same events a second time.
    ``examples/ai_loop/ai_loop_example.cpp`` registers the real one, and
    ``test_the_document_declares_its_acts_to_the_host`` below records.
    """
    engine = _sm.create_engine(script_engine)
    engine.register_event_processor(DECLARED_TYPE, handler)
    return engine


def _booted():
    """A booted machine, sitting in ``priming`` with nothing prompted yet."""
    engine = _not_initialised()
    engine.initialize()
    return engine


def _started():
    """A run whose first prompt has been sent — where every scenario starts."""
    engine = _booted()
    _step(engine, Event.PROMPT_SENT)
    return engine


def _step(engine, event: Event, data: str = "") -> None:
    """One event, driven to stability.

    ``send_event`` queues and runs the macrostep loop, which is what the sibling
    channels' own ``step`` helpers do in two calls.
    """
    engine.send_event(event, EventMetadata(data=data) if data else None)


def _active(engine):
    return engine.active_configuration()


def _holds(engine, state: State) -> bool:
    return state in engine.active_configuration()


def _where(engine) -> List[str]:
    """The active set in the document's own words, for a failure a reader can
    act on: ``['alive', 'within', 'working']`` says where the machine is and a
    set of ints does not."""
    return sorted(engine.policy.get_state_name(s) for s in engine.active_configuration())


def _verdict(engine, done: bool) -> None:
    """The verdict a completed turn is judged on.

    ``judging`` branches on ``_event.data.done``, so ``judge`` is one of the two
    events this document requires a payload from — the host in
    ``examples/ai_loop/ai_loop_example.cpp`` composes exactly this JSON. Sending
    it bare is not a shortcut with the same meaning: ``_event.data`` is then
    absent, reading a field off it fails, and W3C SCXML 5.9.1 has a failed
    ``cond`` raise ``error.execution`` and be treated as false — so the run takes
    the same third transition a ``done:false`` verdict would while quietly
    counting an error per turn. ``test_a_verdict_without_its_payload_is_reported``
    is that path and ``test_a_correctly_driven_run_reports_no_errors`` its floor.
    """
    _step(engine, Event.JUDGE, json.dumps({"done": done}))


def _turn(engine) -> None:
    """One completed turn: the work finished, and the loop decides what next."""
    _step(engine, Event.TURN_DONE)
    _verdict(engine, False)


def test_all_three_regions_are_live_at_once() -> None:
    engine = _started()
    assert (
        _holds(engine, State.WORKING)
        and _holds(engine, State.ALIVE)
        and _holds(engine, State.WITHIN)
    ), (
        "the cycle, the liveness watch and the budget are orthogonal regions and "
        f"must all be active at once; got {_where(engine)}"
    )


def test_reflection_fires_on_schedule() -> None:
    engine = _started()
    at = 0
    for n in range(1, 11):
        _turn(engine)
        if _holds(engine, State.REFLECTING):
            at = n
            break
    assert at == 8, (
        "the document sets `reflect_every` to 8, so the eighth completed turn is "
        f"the one that reflects; reflection fired at turn {at}"
    )


def test_reflection_goes_through_a_restart_and_the_loop_re_primes() -> None:
    engine = _started()
    for _ in range(8):
        _turn(engine)

    _step(engine, Event.REFLECT_APPLIED)
    assert _holds(engine, State.RESTARTING), (
        "a session reads its context, MCP config and memory once, when it starts, "
        "so applying a reflection has to REPLACE the session rather than "
        f"reconfigure it; active: {_where(engine)}"
    )

    _step(engine, Event.SESSION_READY)
    assert _holds(engine, State.PRIMING), (
        "a replaced session starts empty and must be primed with the current "
        f"prompts before it can take a turn; active: {_where(engine)}"
    )


def test_the_budget_ends_the_run_from_wherever_the_cycle_is() -> None:
    engine = _started()
    for _ in range(60):
        if _holds(engine, State.REFLECTING):
            _step(engine, Event.REFLECT_NONE)
        if _holds(engine, State.EXHAUSTED):
            break
        _turn(engine)
    assert _holds(engine, State.EXHAUSTED), (
        "the budget is its own region precisely so the turn count is not something "
        f"`judging` has to remember to check; active: {_where(engine)}"
    )


def test_a_standing_instruction_answers_without_waking_anybody() -> None:
    engine = _started()

    _step(engine, Event.TURN_BLOCKED)
    assert _holds(engine, State.SCREENING), (
        "a dialog is screened against the rules the person wrote in advance "
        f"before anyone is woken; active: {_where(engine)}"
    )

    _step(engine, Event.SCREEN_MATCHED)
    assert _holds(engine, State.WORKING) and not _holds(engine, State.PAUSED), (
        "a matched rule is a decision the person already made, so the run carries "
        f"on and nobody is woken; active: {_where(engine)}"
    )


def test_an_unmatched_dialog_wakes_the_person_who_answers() -> None:
    engine = _started()

    _step(engine, Event.TURN_BLOCKED)
    _step(engine, Event.SCREEN_NONE)
    assert _holds(engine, State.PAUSED), (
        "the loop answers only what the person decided in advance; anything else "
        f"stops it and waits; active: {_where(engine)}"
    )

    _step(engine, Event.TURN_DONE)
    assert _holds(engine, State.JUDGING), (
        "once the person has answered, the turn completes where it left off; "
        f"active: {_where(engine)}"
    )


def test_answering_a_question_does_not_re_prime_the_session() -> None:
    """A person answering does not re-introduce the session to itself.

    ``paused`` is a sibling of ``running``, so answering targets ``judging`` and
    enters ``running`` on the way — as an ANCESTOR. W3C SCXML Appendix D
    addAncestorStatesToEnter adds such a state without its default initial child,
    and here the default is ``priming``, whose ``<onentry>`` sends the opening
    prompt. An engine that gives every entered compound state its default leaves
    the cycle in two states at once and the host, reading the configuration,
    sends the start prompt again — measured 2026-08-15 on both AOT engines, with
    every W3C fixture green.

    The clause itself is pinned across all seven channels by
    ``integration_resources/ancestor_entry_is_not_default_entry/``. This is the
    worked example's own stake in it.
    """
    engine = _started()
    _step(engine, Event.TURN_BLOCKED)
    _step(engine, Event.SCREEN_NONE)
    _step(engine, Event.TURN_DONE)

    assert _holds(engine, State.JUDGING), (
        f"the answered turn has to land in `judging`; active: {_where(engine)}"
    )
    assert not _holds(engine, State.PRIMING), (
        f"`running` has two children active at once: {_where(engine)}. `priming` "
        "sends `prompt.start`, so a host driving this configuration re-sends the "
        "opening prompt every time a person answers a dialog"
    )


def test_hold_and_resume_return_to_exactly_where_the_cycle_was() -> None:
    engine = _started()
    _turn(engine)

    _step(engine, Event.HOLD)
    assert _holds(engine, State.PAUSED), (
        f"a person looking at the work holds the cycle; active: {_where(engine)}"
    )

    _step(engine, Event.RESUME)
    assert _holds(engine, State.WORKING), (
        "resuming puts the cycle back to work rather than ending the run; "
        f"active: {_where(engine)}"
    )


def test_resume_returns_somewhere_the_history_default_does_not() -> None:
    """``<history id="where">`` declares ``<transition target="working"/>`` as
    its default, so a hold taken while the cycle is in ``working`` resumes there
    whether history recorded anything or not — the scenario above cannot tell a
    working history from one that records nothing. Measured: deleting the
    recording filter left it green.

    ``priming`` is the one place the two answers differ. The machine comes up
    there, ``hold`` is declared above the cycle so it reaches, and the history
    default names ``working`` — so resuming into ``priming`` is only possible if
    the configuration was really recorded.
    """
    engine = _booted()
    assert _holds(engine, State.PRIMING), (
        "the run starts with a session that exists and has not been prompted; "
        f"active: {_where(engine)}"
    )

    _step(engine, Event.HOLD)
    assert _holds(engine, State.PAUSED), (
        "a person can take over before the first prompt as readily as after one; "
        f"active: {_where(engine)}"
    )

    _step(engine, Event.RESUME)
    assert _holds(engine, State.PRIMING) and not _holds(engine, State.WORKING), (
        "`<history>` must restore the state the cycle was actually in; landing in "
        "`working` here is the history default answering instead, which is what a "
        f"history that records nothing looks like; active: {_where(engine)}"
    )


def test_the_person_interrupts_the_inner_session_by_hand() -> None:
    engine = _started()

    _step(engine, Event.TURN_INTERRUPTED)
    assert _holds(engine, State.PAUSED) and not _holds(engine, State.SCREENING), (
        "a person typing into the session directly is not a dialog to screen — "
        f"the loop stops driving and stays out of the way; active: {_where(engine)}"
    )

    _step(engine, Event.TURN_INTERRUPTED)
    assert _holds(engine, State.PAUSED), (
        "further interruptions keep it paused rather than fighting the person for "
        f"the session; active: {_where(engine)}"
    )


def test_nobody_comes() -> None:
    engine = _started()

    _step(engine, Event.TURN_BLOCKED)
    _step(engine, Event.SCREEN_NONE)
    _step(engine, Event.UNATTENDED)
    assert _holds(engine, State.BLOCKED), (
        "a question nobody answers ends the run in an outcome the document names, "
        f"rather than leaving it prompting into the dark; active: {_where(engine)}"
    )


def test_a_pane_that_dies_mid_turn_is_noticed_and_rebuilt() -> None:
    engine = _started()

    # The cycle is sitting in `working`, waiting for a turn that will never come
    # because the process is gone. `watch` is the region that sees it.
    _step(engine, Event.SESSION_LOST)
    assert _holds(engine, State.RESTARTING) and _holds(engine, State.REBUILDING), (
        "a dead session has to be noticed independently of where the turn cycle "
        "happens to be, which is why the watch is its own region; active: "
        f"{_where(engine)}"
    )

    _step(engine, Event.SESSION_READY)
    assert _holds(engine, State.PRIMING) and _holds(engine, State.ALIVE), (
        "both regions recover together: the run re-primes and the watch goes back "
        f"to alive; active: {_where(engine)}"
    )


def test_an_internal_region_root_transition_leaves_the_sibling_region() -> None:
    """The three transitions on the ``drive`` region root carry ``type="internal"``.

    §scxml-D-getTransitionDomain. The document's own comment calls that
    load-bearing rather than decorative, and nothing measured it.

    An internal transition whose target descends from its compound source has
    that source as its domain, so ``drive`` is the whole of what exits and the
    sibling regions are left alone. Read as EXTERNAL -- by a document that omits
    the type, or by an engine that drops it -- the domain is the DOCUMENT ROOT,
    because findLCCA filters the proper ancestors to ``<state>`` and ``<scxml>``
    and the only ancestor of a region root is the ``<parallel>``. Every region
    would then exit and come back at its default.

    The two answers are distinguishable only while a sibling region is OFF its
    default, which is why ``session.lost`` comes first -- it puts ``watch`` in
    ``rebuilding``. Firing ``hold`` on a run whose regions all sit at their
    defaults cannot tell the two apart, and that is why the 27 scenarios written
    before this one did not.
    """
    engine = _started()

    # Move `watch` off its default, so that a region restarted by too wide a
    # domain is a state this scenario can see.
    _step(engine, Event.SESSION_LOST)
    assert _holds(engine, State.REBUILDING), (
        "precondition: the liveness watch has to be off its default, or nothing "
        f"below can tell a domain that spared it from one that reset it; active: {_where(engine)}"
    )

    # Written on the region root, `type="internal"`.
    _step(engine, Event.HOLD)
    assert _holds(engine, State.PAUSED), (
        "the transition's own target is entered whichever domain the engine resolved, "
        f"so this half failing means it did not fire at all; active: {_where(engine)}"
    )
    assert _holds(engine, State.REBUILDING) and not _holds(engine, State.ALIVE), (
        "an internal region-root transition has the region as its domain, so the watch "
        "keeps what it saw; reading `alive` here means the domain reached the document "
        f"root and every region was restarted underneath the cycle; active: {_where(engine)}"
    )


def test_one_cancel_reaches_every_region() -> None:
    engine = _started()

    _step(engine, Event.CANCEL)
    assert _holds(engine, State.CANCELLED), (
        "cancel is one transition on the `<parallel>` itself rather than one per "
        f"region, so a single event ends all three; active: {_where(engine)}"
    )


def test_the_machine_answers_what_its_own_datamodel_holds() -> None:
    """W3C SCXML 5.3: the machine answers what its own datamodel holds.

    A host supervising this loop has to size its own work against the budget the
    document declares. Without an accessor the only readable copy is the script
    engine's, reached with an engine handle, a session id and the variable's name
    spelled as a string — three things a consumer should not need, none of them
    checked by a type.

    The half that decides the shape is ``turns``: it is authored 0 and assigned on
    every completed turn, so an accessor that answered the AUTHORED literal would
    keep saying 0 for the whole run.
    """
    engine = _started()

    assert engine.policy.max_turns() == 40, (
        "the authored budget must be readable off the machine itself, in the "
        f"host's own type; got {engine.policy.max_turns()}"
    )
    assert engine.policy.reflect_every() == 8, "so must the reflection cadence"
    assert engine.policy.screen_permissions() is False, (
        "a standing answer to permission dialogs is a promise about what the loop "
        "may do unattended, and a host must be able to inspect it; got "
        f"{engine.policy.screen_permissions()}"
    )

    assert engine.policy.turns() == 0, (
        "no turn has completed yet, so the bookkeeping still reads its authored value"
    )
    _turn(engine)
    assert engine.policy.turns() == 1, (
        "the accessor must report what the datamodel HOLDS, not what the document "
        f"authored — a value frozen at generation time would still say 0 here; got "
        f"{engine.policy.turns()}"
    )


def test_the_strategy_a_host_edits_is_the_strategy_it_can_read_back() -> None:
    """The strategy a host edits is the strategy it can read back.

    The budget above is the numeric half of the datamodel. This is the half the
    example's own comment calls editable: the north star, the milestone, the
    prompts built from them, the marker that ends the run. A supervisor about to
    send ``start_prompt`` has to see what it is about to send, and a UI over this
    loop has nothing to display without these.

    ``start_prompt`` is asserted through its parts rather than as one literal,
    because it is a concatenation: it exists to prove that a value the document
    COMPUTES from its strings is readable too.
    """
    engine = _started()

    assert engine.policy.done_marker() == "MILESTONE REACHED", (
        "the marker that decides when the run has converged must be readable off "
        "the machine — a host matching the session's report against it cannot ask "
        f"the document; got {engine.policy.done_marker()!r}"
    )
    assert engine.policy.north_star() == "(edit me) the outcome this loop exists to reach", (
        "the goal the author edits is the first thing a supervisor displays; got "
        f"{engine.policy.north_star()!r}"
    )
    assert engine.policy.milestone() == "(edit me) the next checkpoint on the way there", (
        f"so is the checkpoint it is working toward; got {engine.policy.milestone()!r}"
    )

    start = engine.policy.start_prompt()
    assert start is not None, (
        "the prompt the loop sends into a fresh session must be readable before it "
        "is sent"
    )
    assert "(edit me) the outcome this loop exists to reach" in start and (
        "Report what you did" in start
    ), (
        "the composed prompt must carry the authored strings it was built from, so "
        f"a host reading it sees what the session will receive: {start!r}"
    )


def test_the_standing_instructions_can_be_read_back_off_the_machine() -> None:
    """The standing instructions are readable, which is what makes them standing.

    ``screen_rules`` is the block that decides when a person is NOT woken. The
    document keeps it in the authored half deliberately — its own comment says the
    loop is carrying out a decision made in advance and written down — and a
    decision written down where nobody can read it back is indistinguishable from
    the loop deciding on its own authority.

    The parts asserted are the ones a reader acts on — which question is matched
    and what answer it gets — rather than the whole text, so reformatting the
    block inside the document does not fail this.
    """
    engine = _started()

    rules = engine.policy.screen_rules()
    assert rules is not None, (
        "the standing-instruction table must be readable off the machine — a host "
        "that cannot list it cannot show anyone which questions are being answered "
        "without them"
    )
    assert rules.startswith("["), (
        f"the block is authored as an array and must come back as one: {rules!r}"
    )
    for question in ("design-decision", "design-proposal", "multiple-choice"):
        assert question in rules, (
            f"`{question}` is screened by the document but absent from what the "
            f"machine reports: {rules!r}"
        )
    assert "Rethink for the most durable answer" in rules, (
        "the reply a screened question receives is the half a person most needs to "
        "see, and it is what distinguishes carrying out a decision from making "
        f"one: {rules!r}"
    )


def test_a_structured_read_follows_the_assignment_and_refuses_another_type() -> None:
    """A structured variable answers with what it is holding, not with what it
    was declared as.

    The scalar readers refuse a value of another type, and this asserts the JSON
    one does too — from both directions. A write into the session must be visible,
    because a reader frozen at generation time would answer the document's literal
    for the whole run; and a scalar written into a variable declared structured
    must read as "cannot answer" rather than as the scalar's own JSON.

    The writes go through ``set_variable``, which takes a value rather than source
    text. That is the half of the engine interface that is the same whichever
    engine a deployment injected — ``evaluate_expression`` takes the ENGINE's
    language — so a test written in either language would be asserting about the
    injection rather than about the reader. The engine is constructed here rather
    than taken from ``create_engine``'s default so the test holds the same handle
    the machine does.
    """
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _not_initialised(script_engine=script_engine)
    engine.initialize()
    _step(engine, Event.PROMPT_SENT)

    session_id = engine.policy._session_id
    assert session_id, "a started machine holds a session"

    script_engine.set_variable(
        session_id, "screen_rules", ScriptValue.of([{"when": "later"}])
    )
    after = engine.policy.screen_rules()
    assert after is not None, "a reassigned structured variable is still readable"
    assert "later" in after and "design-decision" not in after, (
        "the reader answered with the authored table after the session was assigned "
        f"another one: {after!r}"
    )

    script_engine.set_variable(session_id, "screen_rules", ScriptValue.of(5))
    assert engine.policy.screen_rules() is None, (
        "a variable declared structured and now holding a number must report that "
        "the machine cannot answer. `5` is valid JSON, so a reader that forwarded "
        "whatever the serializer produced would hand a consumer a document shape "
        f"that no longer exists; got {engine.policy.screen_rules()!r}"
    )


def test_what_a_reflection_writes_is_what_the_machine_then_holds() -> None:
    """What a reflection writes is what the restarted session is primed with.

    This is the loop's whole reason for having a restart state: ``reflecting``
    rewrites the prompts and ``restarting`` replaces the session so a fresh one
    reads them. Both halves are invisible to an outcome — a run converges just the
    same whether the text it sent afterwards was the reflection's, the author's,
    or empty.

    It is asserted because the example was wrong here: its host wrote
    ``{"start_prompt":"","turn_prompt":"","milestone":"refined"}``, so the document
    came back holding two empty strings and the fresh session was primed with
    nothing at all, under a scenario titled "restarts into the improved prompts".
    Measured 2026-08-15 in the example's own output.
    """
    engine = _started()

    authored = engine.policy.start_prompt()
    assert authored is not None, "a started loop can read its opening prompt"

    for _ in range(8):
        _turn(engine)
    assert _holds(engine, State.REFLECTING), (
        "the document sets `reflect_every` to 8, so the eighth completed turn "
        f"reflects; active: {_where(engine)}"
    )

    _step(
        engine,
        Event.REFLECT_APPLIED,
        json.dumps(
            {
                "start_prompt": "Resuming. Milestone: refined",
                "turn_prompt": "Continue toward: refined",
                "milestone": "refined",
            }
        ),
    )

    assert engine.policy.milestone() == "refined", (
        "the reflection's milestone did not reach the datamodel, so the restart it "
        f"is about to pay for improves nothing; got {engine.policy.milestone()!r}"
    )
    after = engine.policy.start_prompt()
    assert after is not None, (
        "the prompt a restarted session is primed with must still be readable"
    )
    assert after == "Resuming. Milestone: refined", (
        f"the machine is not holding what the reflection wrote; got {after!r}"
    )
    assert after != authored, (
        "the reflection has to have changed something, or this scenario would pass "
        "against a machine that ignored it"
    )
    assert after != "", (
        "an empty prompt is what a host sends when reflection erased it, and the run "
        "still converges — which is why this is asserted rather than watched"
    )


def test_an_uninitialised_machine_says_it_cannot_answer() -> None:
    """A machine that has not been booted cannot answer, and says so.

    The failure this refuses is the one a default-valued field would produce: a
    freshly constructed machine reporting the document's literal as though a
    session had been created and initialised it. Nothing has read the document at
    this point, so "cannot answer" is the only honest response.
    """
    engine = _sm.create_engine()

    assert engine.policy.max_turns() is None, (
        "before initialize() there is no session holding a datamodel, and answering "
        f"40 would be a claim about a run that has not started; got "
        f"{engine.policy.max_turns()}"
    )


def test_the_run_converges_through_a_closing_report() -> None:
    """The outcome the loop exists to reach, and the report it asks for first.

    The document's opening comment claims the outcomes are enumerated, and five
    finals spell them. Measured 2026-08-23: ``converged`` — the one a successful
    run ends in — was reached by no scenario in any channel, and neither was the
    ``closing`` state on the way to it.

    ``closing`` is asserted separately from the terminal because it is the whole
    reason the document does not send ``judge`` straight to a final: the session is
    asked for a closing report, and only the turn that answers it ends the run. A
    machine that jumped from the verdict to ``converged`` would satisfy a
    terminal-only check and lose the report.
    """
    engine = _started()
    _step(engine, Event.TURN_DONE)
    _verdict(engine, True)

    assert _holds(engine, State.CLOSING), (
        "a `done` verdict asks for the closing report before ending the run; "
        f"active: {_where(engine)}"
    )

    _step(engine, Event.TURN_DONE)

    assert _holds(engine, State.CONVERGED), (
        "the turn that answers the closing report reaches `reported`, whose "
        f"`<raise>` is what takes all three regions out at once; active: {_where(engine)}"
    )


def test_a_verdict_without_its_payload_is_reported() -> None:
    """W3C SCXML 5.9.1: a host that forgets the verdict can find out.

    ``judging`` reads ``_event.data.done``. A ``judge`` that carries nothing leaves
    ``_event.data`` absent, reading a field off it fails, and the clause says a
    failed ``cond`` raises ``error.execution`` and is treated as false — so the run
    does exactly what a ``done:false`` verdict would and heads into another turn.
    The two deliveries are indistinguishable from the configuration, from the
    datamodel and from the outcome: a loop driven this way never converges, however
    finished the session reports itself to be, and nothing says why.

    What tells them apart is the engine's own count. The behaviour is correct per
    the spec; the defect would be that it is unobservable.
    """
    engine = _started()
    _step(engine, Event.TURN_DONE)

    _step(engine, Event.JUDGE)

    assert _holds(engine, State.WORKING), (
        "a `cond` that could not be evaluated is treated as false, so the cycle "
        "takes the unconditional third transition and works another turn; active: "
        f"{_where(engine)}"
    )
    assert engine.unhandled_error_events() == 1, (
        "the payload-less verdict raised no error a host could count, so a run that "
        "will never converge looks exactly like one that has not converged yet; "
        f"unhandled errors = {engine.unhandled_error_events()}"
    )
    assert engine.last_unhandled_error() == Event.ERROR_EXECUTION, (
        "the count has to name what it counted; a host reading only a number cannot "
        f"tell a failed `cond` from a failed action; got {engine.last_unhandled_error()}"
    )


def test_a_correctly_driven_run_reports_no_errors() -> None:
    """The floor that makes the count above a measurement.

    A counter asserted only where it is expected to move measures half of what it
    claims: ``test_a_verdict_without_its_payload_is_reported`` would pass just as
    well against an engine that raised ``error.execution`` on every event. So the
    same run, driven the way ``ai_loop_example.cpp`` drives it, has to raise
    nothing at all — through the reflection and the restart it pays for, which is
    where the document's other payload-carrying event lands.
    """
    engine = _started()
    for _ in range(8):
        _turn(engine)
    assert _holds(engine, State.REFLECTING), (
        f"the eighth completed turn reflects; active: {_where(engine)}"
    )

    _step(
        engine,
        Event.REFLECT_APPLIED,
        json.dumps(
            {
                "start_prompt": "Resuming. Milestone: refined",
                "turn_prompt": "Continue toward: refined",
                "milestone": "refined",
            }
        ),
    )
    _step(engine, Event.SESSION_READY)
    _step(engine, Event.PROMPT_SENT)
    _turn(engine)

    assert engine.unhandled_error_events() == 0, (
        "a run driven the way the document's own host drives it raises nothing; an "
        "error here means the channels are not asking the machine the same thing, "
        "and this one would be asserting clauses about a path no deployment takes; "
        f"unhandled errors = {engine.unhandled_error_events()}"
    )


def test_a_session_replaced_past_its_budget_reports_stuck() -> None:
    """Rebuilding more often than the author allowed is a spent budget, not a
    broken document.

    ``max_restarts`` bounds how many times a session may be replaced. Measured
    2026-08-23: no channel named it, so ``stuck`` — one of the two states that
    reach ``exhausted`` — was reachable only in prose. The budget region's
    ``max_turns`` had a witness; this one had none, and the two are different
    mechanisms that happen to share a terminal.

    A lost session is the cheap way in: ``drive`` answers ``session.lost`` with a
    restart from wherever the cycle is, which is the same door reflection uses and
    the one a real deployment hits when a process dies.
    """
    engine = _started()
    allowed = engine.policy.max_restarts()
    assert allowed is not None, "the document declares a restart budget"

    for n in range(1, allowed + 1):
        _step(engine, Event.SESSION_LOST)
        _step(engine, Event.SESSION_READY)
        assert _holds(engine, State.PRIMING), (
            f"replacement {n} of {allowed} is within the budget, so the fresh "
            "session is primed with whatever the loop has written by now; active: "
            f"{_where(engine)}"
        )

    _step(engine, Event.SESSION_LOST)
    _step(engine, Event.SESSION_READY)

    assert _holds(engine, State.EXHAUSTED), (
        "the replacement past `max_restarts` reaches `stuck`, which reports the run "
        f"as exhausted rather than failed; active: {_where(engine)}"
    )


def test_the_document_declares_its_acts_to_the_host() -> None:
    """W3C SCXML 6.2.5: the document tells a host what to do, in its own words.

    Every scenario above registers a handler that answers nothing, because what
    they measure is the topology and each supplies its own events. That makes them
    blind to the thing this scenario asserts: with a silent handler, a ``<send>``
    that LOST its ``type="x-sce-host"`` behaves exactly like one that kept it —
    nothing is delivered either way — so the whole conversion could rot back to
    targetless sends with every channel green.

    So this one records instead of ignoring. It pins that entering ``priming`` asks
    the host to prompt, and that the prompt text rides ON the act: the host is TOLD
    what to send rather than reaching into the datamodel behind the machine to find
    out, which is the difference the conversion bought.
    """
    seen: List[Tuple[str, str]] = []

    def recorder(request: HostSendRequest) -> List[HostSendResponse]:
        values = request.params.get("text") if request.params else None
        seen.append((request.event_name, values[0] if values else ""))
        return []

    engine = _not_initialised(handler=recorder)
    engine.initialize()

    assert seen, "entering `priming` asked the host to perform nothing at all"
    assert seen[0][0] == "prompt.start", (
        f"entering `priming` did not ask the host to prompt; the acts seen were {seen}"
    )
    assert "North star:" in seen[0][1], (
        "the act carried no prompt, so a host would have to reach past the machine "
        f"for one: {seen[0][1]!r}"
    )


def test_a_failure_ends_the_whole_run() -> None:
    """The sibling of ``test_one_cancel_reaches_every_region``.

    The document writes ``fail`` and ``cancel`` once each on the ``<parallel>`` and
    says so in a comment — one transition rather than one per region, because a run
    ends as a whole. Only ``cancel`` was asserted, and the two are not the same
    claim: they are separate transitions to separate terminals, and a consumer
    distinguishing "the run broke" from "somebody stopped it" reads which final it
    ended in.
    """
    engine = _started()

    _step(engine, Event.FAIL)

    assert _holds(engine, State.FAILED), (
        "`fail` is written on the `<parallel>` itself, so one event takes all three "
        "regions to `failed` — a different outcome from `cancelled`, which is what "
        f"tells a broken run from a stopped one; active: {_where(engine)}"
    )


# ══════════════════════════════════════════════════════════════════
# A run that outlived its process
#
# `enter_at` takes states, and nothing that crosses a process boundary can carry
# one: a journal, a wire and a file all carry STRINGS. `get_state_name` writes
# that record and `get_state_from_name` reads it back, and until the second
# existed the door could be called and its argument could not be built — a
# supervisor coming back had to `initialize()` instead, which is a replay rather
# than a resume: `priming` performs its prompt on entry, so the restored loop
# typed the first prompt again.
#
# A consumer-side table mapping the names it knows to states would compile and
# would age silently — the document gains a state, the table does not, the name
# reads back as None, and the resume quietly becomes a fresh start. Only the
# generator writes the half that ages with the document, which is why these two
# scenarios drive a GENERATED policy.
# ══════════════════════════════════════════════════════════════════


def test_a_run_journalled_as_names_resumes_where_it_stopped() -> None:
    ran = _started()
    _turn(ran)
    _turn(ran)

    # Everything a host can persist. Not states, not a configuration: text.
    journal = [ran.policy.get_state_name(s) for s in ran.active_configuration()]
    journalled_current = ran.policy.get_state_name(ran.current_state)
    assert "working" in journal, (
        "the journal is meant to be taken mid-run, with the cycle at work; it reads "
        f"{sorted(journal)}"
    )

    # A new process, holding nothing but those strings.
    acts: List[str] = []

    def recorder(request: HostSendRequest) -> List[HostSendResponse]:
        acts.append(request.event_name)
        return []

    resumed = _not_initialised(handler=recorder)

    configuration = []
    for name in journal:
        state = resumed.policy.get_state_from_name(name)
        assert state is not None, (
            f"`{name}` is a name this policy published through `get_state_name` and "
            "it did not read back, so a configuration cannot survive its own record"
        )
        configuration.append(state)
    current = resumed.policy.get_state_from_name(journalled_current)
    assert current is not None, (
        f"the current state's own name `{journalled_current}` did not read back"
    )

    assert resumed.enter_at(configuration, current) is ConfigurationRejection.NONE, (
        "a configuration this document published is one it can be put back into"
    )

    assert resumed.active_configuration() == set(configuration), (
        "the machine came back somewhere other than where the journal said it was: "
        f"{sorted(resumed.policy.get_state_name(s) for s in resumed.active_configuration())} "
        f"against {sorted(journal)}"
    )
    # This engine derives `current_state` from its leaves rather than storing one,
    # so what the door owes a host is that the recorded leaf is among the leaves it
    # came back with — a resume that dropped the region a host was watching would
    # otherwise pass.
    assert current in resumed.active_leaves, (
        f"the journalled leaf `{journalled_current}` is not among the restored "
        f"leaves {[resumed.policy.get_state_name(s) for s in resumed.active_leaves]}"
    )
    assert resumed.is_running, "an accepted entry left the machine stopped"
    assert not acts, (
        "resuming performed acts, which is the replay `enter_at` exists to avoid — a "
        f"host would see the run's earlier prompts sent a second time; performed {acts}"
    )


def test_every_state_a_run_reaches_reads_back_from_its_own_name() -> None:
    seen: List[State] = []

    def record(engine) -> None:
        for state in engine.active_configuration():
            if state not in seen:
                seen.append(state)

    # Every outcome the document names, walked rather than listed: a state is
    # recorded here only because a run actually stood in it, and a written-out list
    # of states is what `get_state_from_name` exists to replace.
    engine = _started()
    record(engine)
    for _ in range(60):
        if _holds(engine, State.REFLECTING):
            record(engine)
            _step(engine, Event.REFLECT_APPLIED)
            record(engine)
            _step(engine, Event.SESSION_READY)
        if _holds(engine, State.EXHAUSTED):
            break
        _turn(engine)
        record(engine)
    record(engine)

    engine = _started()
    _step(engine, Event.TURN_DONE)
    # Recorded here, before the verdict, because `judging` is where a completed
    # turn WAITS — the only state in the cycle a host reaches by sending
    # nothing. Every other branch of this walk records after driving the machine
    # on, and that is exactly how `judging` stayed unvisited while the floor
    # below read as satisfied.
    record(engine)
    _verdict(engine, True)
    record(engine)
    _step(engine, Event.TURN_DONE)
    record(engine)

    engine = _started()
    _step(engine, Event.TURN_BLOCKED)
    record(engine)
    _step(engine, Event.SCREEN_NONE)
    record(engine)
    _step(engine, Event.UNATTENDED)
    record(engine)

    engine = _started()
    _step(engine, Event.HOLD)
    record(engine)
    _step(engine, Event.RESUME)
    record(engine)

    engine = _started()
    _step(engine, Event.SESSION_LOST)
    record(engine)
    _step(engine, Event.SESSION_READY)
    record(engine)

    engine = _started()
    _step(engine, Event.CANCEL)
    record(engine)

    engine = _started()
    _step(engine, Event.FAIL)
    record(engine)

    # A floor, not a target: without one, a table that had lost every arm but the
    # first would pass this by being asked about a single state.
    #
    # 21 is measured rather than chosen. The document declares 25 states and the
    # four below are unreachable to any reader of the configuration, so a floor
    # of 25 would retire this test and the 20 it used to hold understated the
    # walk by one.
    assert len(seen) >= 21, (
        "these scenarios are meant to stand in every state a reader can observe; they "
        f"reached {len(seen)} states: {[engine.policy.get_state_name(s) for s in seen]}"
    )

    # The other side of that ratchet, and the reason the number above is a
    # measurement: these four are inner ``<final>``s whose ``<onentry>`` is a
    # ``<raise>`` that ends the run in the SAME macrostep — ``reported`` raises
    # ``run.converged``, ``stuck`` and ``spent`` raise ``run.exhausted``,
    # ``abandoned`` raises ``run.blocked`` — so a configuration read taken
    # between macrosteps can never stand in one. Nothing else in the document is
    # like that.
    #
    # Asserting their ABSENCE is what keeps 21 honest from above: make one of
    # them observable, or extend the walk to reach it, and this fails until the
    # floor is raised with it.
    for unobservable in ("abandoned", "reported", "spent", "stuck"):
        state = engine.policy.get_state_from_name(unobservable)
        assert state is not None, f"`{unobservable}` is a state this document declares"
        assert state not in seen, (
            f"`{unobservable}` was reached, so the ceiling this test documents has moved: "
            "raise the floor above to match what the walk now stands in"
        )

    for state in seen:
        name = engine.policy.get_state_name(state)
        assert engine.policy.get_state_from_name(name) == state, (
            f"`{name}` did not read back as the state that published it"
        )

    # The other half of the contract: a name the document does not carry is refused
    # rather than guessed at. A table that answers anyway turns a stale journal into
    # a plausible-looking resume, which is the one outcome a host has no way to
    # detect afterwards.
    for absent in ("no-such-state", ""):
        assert engine.policy.get_state_from_name(absent) is None, (
            f"`{absent}` is not a state this document carries and it read back as one"
        )
    assert engine.policy.get_state_from_name("turn.done") is None, (
        "an event name is not a state name; the two tables are separate on purpose"
    )
