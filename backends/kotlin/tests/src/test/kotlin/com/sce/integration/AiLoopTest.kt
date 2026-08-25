// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The AI supervision loop, driven through the Kotlin AOT engine.
//
// `examples/ai_loop/ai_loop.scxml` is a worked example: a statechart that
// supervises a long-running session, with <parallel> splitting the turn cycle
// from the liveness watch and the turn budget. The C++, Rust and Go channels
// drive the same document; this is the fourth.
//
// Why a fourth: a clause asserted in one channel is that engine's word for the
// document rather than the document's own, and the parallel defect that shipped
// in `1419a050ed` (a self-transition whose exit set swallowed the parallel root)
// was invisible to every W3C fixture because they are all one region deep. This
// document is three. `sce-build/tests/ai_loop_channel_parity.rs` holds every
// registered channel to the same scenario set by name, so a scenario added here
// without its siblings fails there — which is the moment it is cheapest to fix.
//
// No sprag, no session, no pane: every effect the host would perform is replaced
// by the event that effect would have produced, so what is under test is the
// machine's topology rather than any driver's plumbing.
//
// Because the regions are orthogonal, a scenario asserts on the ACTIVE SET
// rather than on one state — "the cycle is working AND the budget is within" is
// the kind of claim a parallel machine makes, and asserting a single current
// state cannot express it.
//
// This channel runs under whichever engine `sce.script.engine` selects — rhino,
// quickjs or lua — so nothing here may be written in one engine's language.
// Where a scenario has to put a value into the session it goes through
// `parseDataValue`, which takes JSON and hands back the engine's own value.
//
// Fixture: examples/ai_loop/ai_loop.scxml
//
// Regeneration (after example or template edit):
//   scripts/regen_ai_loop_kotlin.sh

package com.sce.integration

import com.sce.integration.ai_loop.AiLoopEvent
import com.sce.integration.ai_loop.AiLoopState
import com.sce.integration.ai_loop.AiLoopStateMachine
import com.sce.runtime.ConfigurationRejection
import com.sce.runtime.EventMetadata
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// The AI supervision loop — the document's clauses, asked of the Kotlin engine.
@DisplayName("AiLoop — examples/ai_loop/ai_loop.scxml")
class AiLoopTest {

    // The processor type the committed machine was generated for.
    // `scripts/regen_ai_loop_kotlin.sh` passes this same string to
    // `--host-processor`, and the other three channels register the same one.
    private val declaredType = "x-sce-host"

    /// The session id a machine allocates, learned the way the machine
    /// announces it.
    ///
    /// `scriptEngine` and `scriptSessionId` are `protected` on the engine base,
    /// so a test cannot ask a running machine which session it owns — and one
    /// scenario has to write into that session to prove a reader follows an
    /// assignment. This delegates every call to the real engine and remembers
    /// the id the machine handed to `createSession`. A recorder rather than a
    /// fake: every answer the machine gets is the engine's own.
    private class SessionRecordingEngine(
        private val inner: ScxmlScriptEngine,
    ) : ScxmlScriptEngine by inner {
        var sessionId: String? = null
            private set

        override fun createSession(sessionId: String) {
            this.sessionId = sessionId
            inner.createSession(sessionId)
        }
    }

    /// A machine wired the way every scenario wires one, stopping short of
    /// booting it.
    ///
    /// The handler is registered before `initialize()` because `priming`
    /// performs its act on entry: a machine booted without one raises
    /// `error.execution` there instead of reaching a host.
    ///
    /// W3C SCXML 6.2.5: the document declares its acts as sends a host serves.
    /// The default handler performs nothing and reports nothing, which is
    /// deliberate — what these scenarios measure is the TOPOLOGY, and each
    /// supplies the events a host would have produced at exactly the point it
    /// wants them. A handler that answered would deliver the same events a
    /// second time. `examples/ai_loop/ai_loop_example.cpp` registers the real
    /// one, and `theDocumentDeclaresItsActsToTheHost` below records.
    private fun notInitialised(
        engine: ScxmlScriptEngine = W3CTestBase.createEngine(),
        handler: (StateMachineEngine.HostSendRequest) -> List<StateMachineEngine.HostSendResponse> = { emptyList() },
    ): AiLoopStateMachine {
        val sm = AiLoopStateMachine(engine)
        sm.registerEventProcessor(declaredType, handler)
        return sm
    }

    /// A booted machine, sitting in `priming` with nothing prompted yet.
    private fun booted(): AiLoopStateMachine {
        val sm = notInitialised()
        sm.initialize()
        return sm
    }

    /// A run whose first prompt has been sent — where every scenario starts.
    private fun started(): AiLoopStateMachine {
        val sm = booted()
        sm.step(AiLoopEvent.Prompt.Sent)
        return sm
    }

    // One event, then a macrostep — the two calls every Kotlin driver makes,
    // and what the sibling channels' own `step` helpers do.
    private fun AiLoopStateMachine.step(event: AiLoopEvent) {
        send(event)
        tick()
    }

    private fun AiLoopStateMachine.holds(state: AiLoopState): Boolean =
        activeConfiguration.contains(state)

    /// The outcome a finished run ended in.
    ///
    /// The document names five top-level finals so a supervisor can read WHICH
    /// way a run ended, and this is where this engine answers that. W3C SCXML
    /// Appendix D `exitInterpreter` empties the configuration as a top-level
    /// <final> is entered, so `activeConfiguration` is already empty by the
    /// time anyone can look; the outcome is in `currentState`. That is not a
    /// concession made here — `integration_resources/
    /// parallel_regions_take_own_transitions` puts its own verdict in a
    /// top-level final for the same reason, and says so in its comment.
    ///
    /// `isInFinalState` is asserted alongside rather than trusted from the
    /// state alone: `currentState` names a leaf whether the run ended there or
    /// merely passed through it, and "the run ended in `cancelled`" is the
    /// claim every terminal scenario below is making.
    private fun AiLoopStateMachine.endedIn(state: AiLoopState): Boolean =
        isInFinalState && currentState.value == state

    /// What a terminal failure prints: where the machine is, whichever half of
    /// the answer is populated.
    private fun AiLoopStateMachine.outcome(): String =
        "active ${where()}, current `${nameOfState(currentState.value)}`, final $isInFinalState"

    // The active set in the document's own words, for a failure a reader can
    // act on: `[working, alive, within]` says where the machine is, the state
    // objects' own toString does not say it as the document spells it.
    private fun AiLoopStateMachine.where(): List<String> =
        activeConfiguration.map { nameOfState(it) }.sorted()

    /// The verdict a completed turn is judged on.
    ///
    /// `judging` branches on `_event.data.done`, so `judge` is one of the two
    /// events this document requires a payload from — the host in
    /// `examples/ai_loop/ai_loop_example.cpp` composes exactly this JSON.
    /// Sending it bare is not a shortcut with the same meaning: `_event.data` is
    /// then absent, reading a field off it fails, and W3C SCXML 5.9.1 has a
    /// failed `cond` raise `error.execution` and be treated as false — so the
    /// run takes the same third transition a `done:false` verdict would while
    /// quietly counting an error per turn. `aVerdictWithoutItsPayloadIsReported`
    /// is that path and `aCorrectlyDrivenRunReportsNoErrors` is its floor.
    private fun AiLoopStateMachine.verdict(done: Boolean) {
        send(AiLoopEvent.Judge.Self, EventMetadata(data = if (done) """{"done":true}""" else """{"done":false}"""))
        tick()
    }

    /// One completed turn: the work finished, and the loop decides what next.
    private fun AiLoopStateMachine.turn() {
        step(AiLoopEvent.Turn.Done)
        verdict(false)
    }

    @Test
    fun allThreeRegionsAreLiveAtOnce() {
        val sm = started()
        assertTrue(
            sm.holds(AiLoopState.Working) && sm.holds(AiLoopState.Alive) && sm.holds(AiLoopState.Within),
            "the cycle, the liveness watch and the budget are orthogonal regions and must " +
                "all be active at once; got ${sm.where()}",
        )
    }

    @Test
    fun reflectionFiresOnSchedule() {
        val sm = started()
        var at = 0
        for (n in 1..10) {
            sm.turn()
            if (sm.holds(AiLoopState.Reflecting)) {
                at = n
                break
            }
        }
        assertEquals(
            8, at,
            "the document sets `reflect_every` to 8, so the eighth completed turn is the " +
                "one that reflects",
        )
    }

    @Test
    fun reflectionGoesThroughARestartAndTheLoopRePrimes() {
        val sm = started()
        repeat(8) { sm.turn() }

        sm.step(AiLoopEvent.Reflect.Applied)
        assertTrue(
            sm.holds(AiLoopState.Restarting),
            "a session reads its context, MCP config and memory once, when it starts, so " +
                "applying a reflection has to REPLACE the session rather than reconfigure " +
                "it; active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Session.Ready)
        assertTrue(
            sm.holds(AiLoopState.Priming),
            "a replaced session starts empty and must be primed with the current prompts " +
                "before it can take a turn; active: ${sm.where()}",
        )
    }

    @Test
    fun theBudgetEndsTheRunFromWhereverTheCycleIs() {
        val sm = started()
        for (n in 1..60) {
            if (sm.holds(AiLoopState.Reflecting)) {
                sm.step(AiLoopEvent.Reflect.None)
            }
            if (sm.isInFinalState) {
                break
            }
            sm.turn()
        }
        assertTrue(
            sm.endedIn(AiLoopState.Exhausted),
            "the budget is its own region precisely so the turn count is not something " +
                "`judging` has to remember to check; ${sm.outcome()}",
        )
    }

    @Test
    fun aStandingInstructionAnswersWithoutWakingAnybody() {
        val sm = started()

        sm.step(AiLoopEvent.Turn.Blocked)
        assertTrue(
            sm.holds(AiLoopState.Screening),
            "a dialog is screened against the rules the person wrote in advance before " +
                "anyone is woken; active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Screen.Matched)
        assertTrue(
            sm.holds(AiLoopState.Working) && !sm.holds(AiLoopState.Paused),
            "a matched rule is a decision the person already made, so the run carries on " +
                "and nobody is woken; active: ${sm.where()}",
        )
    }

    @Test
    fun anUnmatchedDialogWakesThePersonWhoAnswers() {
        val sm = started()

        sm.step(AiLoopEvent.Turn.Blocked)
        sm.step(AiLoopEvent.Screen.None)
        assertTrue(
            sm.holds(AiLoopState.Paused),
            "the loop answers only what the person decided in advance; anything else stops " +
                "it and waits; active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Turn.Done)
        assertTrue(
            sm.holds(AiLoopState.Judging),
            "once the person has answered, the turn completes where it left off; active: " +
                "${sm.where()}",
        )
    }

    /// A person answering does not re-introduce the session to itself.
    ///
    /// `paused` is a sibling of `running`, so answering targets `judging` and
    /// enters `running` on the way — as an ANCESTOR. W3C SCXML Appendix D
    /// addAncestorStatesToEnter adds such a state without its default initial
    /// child, and here the default is `priming`, whose <onentry> sends the
    /// opening prompt. An engine that gives every entered compound state its
    /// default leaves the cycle in two states at once and the host, reading the
    /// configuration, sends the start prompt again — measured 2026-08-15 on both
    /// AOT engines, with every W3C fixture green.
    ///
    /// The clause itself is pinned across all seven channels by
    /// `integration_resources/ancestor_entry_is_not_default_entry/`. This is the
    /// worked example's own stake in it: the document that made the defect
    /// visible asserts the shape it was found in.
    @Test
    fun answeringAQuestionDoesNotRePrimeTheSession() {
        val sm = started()
        sm.step(AiLoopEvent.Turn.Blocked)
        sm.step(AiLoopEvent.Screen.None)
        sm.step(AiLoopEvent.Turn.Done)

        assertTrue(
            sm.holds(AiLoopState.Judging),
            "the answered turn has to land in `judging`; active: ${sm.where()}",
        )
        assertFalse(
            sm.holds(AiLoopState.Priming),
            "`running` has two children active at once: ${sm.where()}. `priming` sends " +
                "`prompt.start`, so a host driving this configuration re-sends the opening " +
                "prompt every time a person answers a dialog",
        )
    }

    @Test
    fun holdAndResumeReturnToExactlyWhereTheCycleWas() {
        val sm = started()
        sm.turn()

        sm.step(AiLoopEvent.Hold)
        assertTrue(
            sm.holds(AiLoopState.Paused),
            "a person looking at the work holds the cycle; active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Resume)
        assertTrue(
            sm.holds(AiLoopState.Working),
            "resuming puts the cycle back to work rather than ending the run; active: " +
                "${sm.where()}",
        )
    }

    /// `<history id="where">` declares `<transition target="working"/>` as its
    /// default, so a hold taken while the cycle is in `working` resumes there
    /// whether history recorded anything or not — the scenario above cannot tell
    /// a working history from one that records nothing. Measured: deleting the
    /// recording filter left it green.
    ///
    /// `priming` is the one place the two answers differ. The machine comes up
    /// there, `hold` is declared above the cycle so it reaches, and the history
    /// default names `working` — so resuming into `priming` is only possible if
    /// the configuration was really recorded.
    @Test
    fun resumeReturnsSomewhereTheHistoryDefaultDoesNot() {
        val sm = booted()
        assertTrue(
            sm.holds(AiLoopState.Priming),
            "the run starts with a session that exists and has not been prompted; active: " +
                "${sm.where()}",
        )

        sm.step(AiLoopEvent.Hold)
        assertTrue(
            sm.holds(AiLoopState.Paused),
            "a person can take over before the first prompt as readily as after one; " +
                "active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Resume)
        assertTrue(
            sm.holds(AiLoopState.Priming) && !sm.holds(AiLoopState.Working),
            "`<history>` must restore the state the cycle was actually in; landing in " +
                "`working` here is the history default answering instead, which is what a " +
                "history that records nothing looks like; active: ${sm.where()}",
        )
    }

    @Test
    fun thePersonInterruptsTheInnerSessionByHand() {
        val sm = started()

        sm.step(AiLoopEvent.Turn.Interrupted)
        assertTrue(
            sm.holds(AiLoopState.Paused) && !sm.holds(AiLoopState.Screening),
            "a person typing into the session directly is not a dialog to screen — the loop " +
                "stops driving and stays out of the way; active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Turn.Interrupted)
        assertTrue(
            sm.holds(AiLoopState.Paused),
            "further interruptions keep it paused rather than fighting the person for the " +
                "session; active: ${sm.where()}",
        )
    }

    @Test
    fun nobodyComes() {
        val sm = started()

        sm.step(AiLoopEvent.Turn.Blocked)
        sm.step(AiLoopEvent.Screen.None)
        sm.step(AiLoopEvent.Unattended)
        assertTrue(
            sm.endedIn(AiLoopState.Blocked),
            "a question nobody answers ends the run in an outcome the document names, " +
                "rather than leaving it prompting into the dark; ${sm.outcome()}",
        )
    }

    @Test
    fun aPaneThatDiesMidTurnIsNoticedAndRebuilt() {
        val sm = started()

        // The cycle is sitting in `working`, waiting for a turn that will never
        // come because the process is gone. `watch` is the region that sees it.
        sm.step(AiLoopEvent.Session.Lost)
        assertTrue(
            sm.holds(AiLoopState.Restarting) && sm.holds(AiLoopState.Rebuilding),
            "a dead session has to be noticed independently of where the turn cycle happens " +
                "to be, which is why the watch is its own region; active: ${sm.where()}",
        )

        sm.step(AiLoopEvent.Session.Ready)
        assertTrue(
            sm.holds(AiLoopState.Priming) && sm.holds(AiLoopState.Alive),
            "both regions recover together: the run re-primes and the watch goes back to " +
                "alive; active: ${sm.where()}",
        )
    }

    /**
     * §scxml-D-getTransitionDomain: the three transitions on the `drive` region root carry
     * `type="internal"`, and the document's own comment calls that load-bearing rather than
     * decorative. Nothing measured it.
     *
     * An internal transition whose target descends from its compound source has that source as its
     * domain, so `drive` is the whole of what exits and the sibling regions are left alone. Read as
     * EXTERNAL — by a document that omits the type, or by an engine that drops it — the domain is
     * the DOCUMENT ROOT, because findLCCA filters the proper ancestors to `<state>` and `<scxml>`
     * and the only ancestor of a region root is the `<parallel>`. Every region would then exit and
     * come back at its default.
     *
     * The two answers are distinguishable only while a sibling region is OFF its default, which is
     * why `session.lost` comes first — it puts `watch` in `rebuilding`. Firing `hold` on a run whose
     * regions all sit at their defaults cannot tell the two apart, and that is why the 27 scenarios
     * written before this one did not.
     */
    @Test
    fun anInternalRegionRootTransitionLeavesTheSiblingRegion() {
        val sm = started()

        // Move `watch` off its default, so that a region restarted by too wide a
        // domain is a state this scenario can see.
        sm.step(AiLoopEvent.Session.Lost)
        assertTrue(
            sm.holds(AiLoopState.Rebuilding),
            "precondition: the liveness watch has to be off its default, or nothing below can " +
                "tell a domain that spared it from one that reset it; active: ${sm.where()}",
        )

        // Written on the region root, `type="internal"`.
        sm.step(AiLoopEvent.Hold)
        assertTrue(
            sm.holds(AiLoopState.Paused),
            "the transition's own target is entered whichever domain the engine resolved, so " +
                "this half failing means it did not fire at all; active: ${sm.where()}",
        )
        assertTrue(
            sm.holds(AiLoopState.Rebuilding) && !sm.holds(AiLoopState.Alive),
            "an internal region-root transition has the region as its domain, so the watch " +
                "keeps what it saw; reading `alive` here means the domain reached the document " +
                "root and every region was restarted underneath the cycle; active: ${sm.where()}",
        )
    }

    @Test
    fun oneCancelReachesEveryRegion() {
        val sm = started()

        sm.step(AiLoopEvent.Cancel)
        assertTrue(
            sm.endedIn(AiLoopState.Cancelled),
            "cancel is one transition on the `<parallel>` itself rather than one per region, " +
                "so a single event ends all three; ${sm.outcome()}",
        )
    }

    /// W3C SCXML 5.3: the machine answers what its own datamodel holds.
    ///
    /// A host supervising this loop has to size its own work against the budget
    /// the document declares. Without an accessor the only readable copy is the
    /// script engine's, reached with an engine handle, a session id and the
    /// variable's name spelled as a string — three things a consumer should not
    /// need, none of them checked by a compiler.
    ///
    /// The half that decides the shape is `turns`: it is authored 0 and assigned
    /// on every completed turn, so an accessor that answered the AUTHORED
    /// literal would keep saying 0 for the whole run.
    @Test
    fun theMachineAnswersWhatItsOwnDatamodelHolds() {
        val sm = started()

        assertEquals(
            40L, sm.maxTurns(),
            "the authored budget must be readable off the machine itself, in the host's own type",
        )
        assertEquals(8L, sm.reflectEvery(), "so must the reflection cadence")
        assertEquals(
            false, sm.screenPermissions(),
            "a standing answer to permission dialogs is a promise about what the loop may do " +
                "unattended, and a host must be able to inspect it",
        )

        assertEquals(
            0L, sm.turns(),
            "no turn has completed yet, so the bookkeeping still reads its authored value",
        )
        sm.turn()
        assertEquals(
            1L, sm.turns(),
            "the accessor must report what the datamodel HOLDS, not what the document " +
                "authored — a value frozen at generation time would still say 0 here",
        )
    }

    /// The strategy a host edits is the strategy it can read back.
    ///
    /// The budget above is the numeric half of the datamodel. This is the half
    /// the example's own comment calls editable: the north star, the milestone,
    /// the prompts built from them, the marker that ends the run. A supervisor
    /// about to send `start_prompt` has to see what it is about to send, and a
    /// UI over this loop has nothing to display without these.
    ///
    /// `start_prompt` is asserted through its parts rather than as one literal,
    /// because it is a concatenation: it exists to prove that a value the
    /// document COMPUTES from its strings is readable too.
    @Test
    fun theStrategyAHostEditsIsTheStrategyItCanReadBack() {
        val sm = started()

        assertEquals(
            "MILESTONE REACHED", sm.doneMarker(),
            "the marker that decides when the run has converged must be readable off the " +
                "machine — a host matching the session's report against it cannot ask the document",
        )
        assertEquals(
            "(edit me) the outcome this loop exists to reach", sm.northStar(),
            "the goal the author edits is the first thing a supervisor displays",
        )
        assertEquals(
            "(edit me) the next checkpoint on the way there", sm.milestone(),
            "so is the checkpoint it is working toward",
        )

        val start = sm.startPrompt()
        assertNotNull(
            start,
            "the prompt the loop sends into a fresh session must be readable before it is sent",
        )
        assertTrue(
            start!!.contains("(edit me) the outcome this loop exists to reach") &&
                start.contains("Report what you did"),
            "the composed prompt must carry the authored strings it was built from, so a host " +
                "reading it sees what the session will receive: $start",
        )
    }

    /// The standing instructions are readable, which is what makes them
    /// standing.
    ///
    /// `screen_rules` is the block that decides when a person is NOT woken. The
    /// document keeps it in the authored half deliberately — its own comment
    /// says the loop is carrying out a decision made in advance and written down
    /// — and a decision written down where nobody can read it back is
    /// indistinguishable from the loop deciding on its own authority.
    ///
    /// The parts asserted are the ones a reader acts on — which question is
    /// matched and what answer it gets — rather than the whole text, so
    /// reformatting the block inside the document does not fail this.
    @Test
    fun theStandingInstructionsCanBeReadBackOffTheMachine() {
        val sm = started()

        val rules = sm.screenRules()
        assertNotNull(
            rules,
            "the standing-instruction table must be readable off the machine — a host that " +
                "cannot list it cannot show anyone which questions are being answered without them",
        )
        assertTrue(
            rules!!.startsWith("["),
            "the block is authored as an array and must come back as one: $rules",
        )
        for (question in listOf("design-decision", "design-proposal", "multiple-choice")) {
            assertTrue(
                rules.contains(question),
                "`$question` is screened by the document but absent from what the machine " +
                    "reports: $rules",
            )
        }
        assertTrue(
            rules.contains("Rethink for the most durable answer"),
            "the reply a screened question receives is the half a person most needs to see, " +
                "and it is what distinguishes carrying out a decision from making one: $rules",
        )
    }

    /// A structured variable answers with what it is holding, not with what it
    /// was declared as.
    ///
    /// The scalar readers refuse a value of another type, and this asserts the
    /// JSON one does too — from both directions. A write into the session must
    /// be visible, because a reader frozen at generation time would answer the
    /// document's literal for the whole run; and a scalar written into a
    /// variable declared structured must read as "cannot answer" rather than as
    /// the scalar's own JSON.
    ///
    /// The value goes in through `parseDataValue`, which takes JSON and hands
    /// back the engine's own value. That is what keeps this scenario honest
    /// under all three engines this suite runs: writing a JS object literal
    /// would assert about Rhino, and a Lua table literal about the Lua binding,
    /// rather than about the reader either way.
    @Test
    fun aStructuredReadFollowsTheAssignmentAndRefusesAnotherType() {
        val recorder = SessionRecordingEngine(W3CTestBase.createEngine())
        val sm = notInitialised(engine = recorder)
        sm.initialize()
        sm.step(AiLoopEvent.Prompt.Sent)

        val sid = recorder.sessionId
        assertNotNull(sid, "a started machine holds a session")

        recorder.setVariable(sid!!, "screen_rules", recorder.parseDataValue(sid, """[{"when":"later"}]"""))
        val after = sm.screenRules()
        assertNotNull(after, "a reassigned structured variable is still readable")
        assertTrue(
            after!!.contains("later") && !after.contains("design-decision"),
            "the reader answered with the authored table after the session was assigned " +
                "another one: $after",
        )

        recorder.setVariable(sid, "screen_rules", recorder.parseDataValue(sid, "5"))
        assertNull(
            sm.screenRules(),
            "a variable declared structured and now holding a number must report that the " +
                "machine cannot answer. `5` is valid JSON, so a reader that forwarded whatever " +
                "the serializer produced would hand a consumer a document shape that no longer exists",
        )
    }

    /// What a reflection writes is what the restarted session is primed with.
    ///
    /// This is the loop's whole reason for having a restart state: `reflecting`
    /// rewrites the prompts and `restarting` replaces the session so a fresh one
    /// reads them. Both halves are invisible to an outcome — a run converges
    /// just the same whether the text it sent afterwards was the reflection's,
    /// the author's, or empty.
    ///
    /// It is asserted because the example was wrong here: its host wrote
    /// `{"start_prompt":"","turn_prompt":"","milestone":"refined"}`, so the
    /// document came back holding two empty strings and the fresh session was
    /// primed with nothing at all, under a scenario titled "restarts into the
    /// improved prompts". Measured 2026-08-15 in the example's own output.
    @Test
    fun whatAReflectionWritesIsWhatTheMachineThenHolds() {
        val sm = started()

        val authored = sm.startPrompt()
        assertNotNull(authored, "a started loop can read its opening prompt")

        repeat(8) { sm.turn() }
        assertTrue(
            sm.holds(AiLoopState.Reflecting),
            "the document sets `reflect_every` to 8, so the eighth completed turn reflects; " +
                "active: ${sm.where()}",
        )

        sm.send(
            AiLoopEvent.Reflect.Applied,
            EventMetadata(
                data = """{"start_prompt":"Resuming. Milestone: refined","turn_prompt":"Continue toward: refined","milestone":"refined"}""",
            ),
        )
        sm.tick()

        assertEquals(
            "refined", sm.milestone(),
            "the reflection's milestone did not reach the datamodel, so the restart it is " +
                "about to pay for improves nothing",
        )
        val after = sm.startPrompt()
        assertNotNull(
            after,
            "the prompt a restarted session is primed with must still be readable",
        )
        assertEquals(
            "Resuming. Milestone: refined", after,
            "the machine is not holding what the reflection wrote",
        )
        assertFalse(
            after == authored,
            "the reflection has to have changed something, or this scenario would pass " +
                "against a machine that ignored it",
        )
        assertFalse(
            after!!.isEmpty(),
            "an empty prompt is what a host sends when reflection erased it, and the run " +
                "still converges — which is why this is asserted rather than watched",
        )
    }

    /// A machine that has not been booted cannot answer, and says so.
    ///
    /// The failure this refuses is the one a default-valued field would produce:
    /// a freshly constructed machine reporting the document's literal as though
    /// a session had been created and initialised it. Nothing has read the
    /// document at this point, so "cannot answer" is the only honest response.
    @Test
    fun anUninitialisedMachineSaysItCannotAnswer() {
        val sm = AiLoopStateMachine(W3CTestBase.createEngine())

        assertNull(
            sm.maxTurns(),
            "before initialize() there is no session holding a datamodel, and answering 40 " +
                "would be a claim about a run that has not started",
        )
    }

    /// The outcome the loop exists to reach, and the report it asks for first.
    ///
    /// The document's opening comment claims the outcomes are enumerated, and
    /// five finals spell them. Measured 2026-08-23: `converged` — the one a
    /// successful run ends in — was reached by no scenario in any channel, and
    /// neither was the `closing` state on the way to it.
    ///
    /// `closing` is asserted separately from the terminal because it is the
    /// whole reason the document does not send `judge` straight to a final: the
    /// session is asked for a closing report, and only the turn that answers it
    /// ends the run. A machine that jumped from the verdict to `converged` would
    /// satisfy a terminal-only check and lose the report.
    @Test
    fun theRunConvergesThroughAClosingReport() {
        val sm = started()
        sm.step(AiLoopEvent.Turn.Done)
        sm.verdict(true)

        assertTrue(
            sm.holds(AiLoopState.Closing),
            "a `done` verdict asks for the closing report before ending the run; active: " +
                "${sm.where()}",
        )

        sm.step(AiLoopEvent.Turn.Done)

        assertTrue(
            sm.endedIn(AiLoopState.Converged),
            "the turn that answers the closing report reaches `reported`, whose <raise> is " +
                "what takes all three regions out at once; ${sm.outcome()}",
        )
    }

    /// W3C SCXML 5.9.1: a host that forgets the verdict can find out.
    ///
    /// `judging` reads `_event.data.done`. A `judge` that carries nothing leaves
    /// `_event.data` absent, reading a field off it fails, and the clause says a
    /// failed `cond` raises `error.execution` and is treated as false — so the
    /// run does exactly what a `done:false` verdict would and heads into another
    /// turn. The two deliveries are indistinguishable from the configuration,
    /// from the datamodel and from the outcome: a loop driven this way never
    /// converges, however finished the session reports itself to be, and nothing
    /// says why.
    ///
    /// What tells them apart is the engine's own count. The behaviour is correct
    /// per the spec; the defect would be that it is unobservable.
    @Test
    fun aVerdictWithoutItsPayloadIsReported() {
        val sm = started()
        sm.step(AiLoopEvent.Turn.Done)

        sm.step(AiLoopEvent.Judge.Self)

        assertTrue(
            sm.holds(AiLoopState.Working),
            "a `cond` that could not be evaluated is treated as false, so the cycle takes " +
                "the unconditional third transition and works another turn; active: ${sm.where()}",
        )
        assertEquals(
            1, sm.unhandledErrorEvents(),
            "the payload-less verdict raised no error a host could count, so a run that will " +
                "never converge looks exactly like one that has not converged yet",
        )
        assertEquals(
            AiLoopEvent.Error.Execution, sm.lastUnhandledError(),
            "the count has to name what it counted; a host reading only a number cannot tell " +
                "a failed `cond` from a failed action",
        )
    }

    /// The floor that makes the count above a measurement.
    ///
    /// A counter asserted only where it is expected to move measures half of
    /// what it claims: `aVerdictWithoutItsPayloadIsReported` would pass just as
    /// well against an engine that raised `error.execution` on every event. So
    /// the same run, driven the way `ai_loop_example.cpp` drives it, has to
    /// raise nothing at all — through the reflection and the restart it pays
    /// for, which is where the document's other payload-carrying event lands.
    @Test
    fun aCorrectlyDrivenRunReportsNoErrors() {
        val sm = started()
        repeat(8) { sm.turn() }
        assertTrue(
            sm.holds(AiLoopState.Reflecting),
            "the eighth completed turn reflects; active: ${sm.where()}",
        )

        sm.send(
            AiLoopEvent.Reflect.Applied,
            EventMetadata(
                data = """{"start_prompt":"Resuming. Milestone: refined","turn_prompt":"Continue toward: refined","milestone":"refined"}""",
            ),
        )
        sm.tick()
        sm.step(AiLoopEvent.Session.Ready)
        sm.step(AiLoopEvent.Prompt.Sent)
        sm.turn()

        assertEquals(
            0, sm.unhandledErrorEvents(),
            "a run driven the way the document's own host drives it raises nothing; an error " +
                "here means the channels are not asking the machine the same thing, and this " +
                "one would be asserting clauses about a path no deployment takes",
        )
    }

    /// Rebuilding more often than the author allowed is a spent budget, not a
    /// broken document.
    ///
    /// `max_restarts` bounds how many times a session may be replaced. Measured
    /// 2026-08-23: no channel named it, so `stuck` — one of the two states that
    /// reach `exhausted` — was reachable only in prose. The budget region's
    /// `max_turns` had a witness; this one had none, and the two are different
    /// mechanisms that happen to share a terminal.
    ///
    /// A lost session is the cheap way in: `drive` answers `session.lost` with a
    /// restart from wherever the cycle is, which is the same door reflection
    /// uses and the one a real deployment hits when a process dies.
    @Test
    fun aSessionReplacedPastItsBudgetReportsStuck() {
        val sm = started()
        val allowed = sm.maxRestarts()
        assertNotNull(allowed, "the document declares a restart budget")

        for (n in 1..allowed!!) {
            sm.step(AiLoopEvent.Session.Lost)
            sm.step(AiLoopEvent.Session.Ready)
            assertTrue(
                sm.holds(AiLoopState.Priming),
                "replacement $n of $allowed is within the budget, so the fresh session is " +
                    "primed with whatever the loop has written by now; active: ${sm.where()}",
            )
        }

        sm.step(AiLoopEvent.Session.Lost)
        sm.step(AiLoopEvent.Session.Ready)

        assertTrue(
            sm.endedIn(AiLoopState.Exhausted),
            "the replacement past `max_restarts` reaches `stuck`, which reports the run as " +
                "exhausted rather than failed; ${sm.outcome()}",
        )
    }

    /// W3C SCXML 6.2.5: the document tells a host what to do, in its own words.
    ///
    /// Every scenario above registers a handler that answers nothing, because
    /// what they measure is the topology and each supplies its own events. That
    /// makes them blind to the thing this scenario asserts: with a silent
    /// handler, a <send> that LOST its `type="x-sce-host"` behaves exactly like
    /// one that kept it — nothing is delivered either way — so the whole
    /// conversion could rot back to targetless sends with every channel green.
    ///
    /// So this one records instead of ignoring. It pins that entering `priming`
    /// asks the host to prompt, and that the prompt text rides ON the act: the
    /// host is TOLD what to send rather than reaching into the datamodel behind
    /// the machine to find out, which is the difference the conversion bought.
    @Test
    fun theDocumentDeclaresItsActsToTheHost() {
        val seen = mutableListOf<Pair<String, String>>()
        val sm = notInitialised(handler = { request ->
            seen.add(request.eventName to (request.params["text"]?.firstOrNull() ?: ""))
            emptyList()
        })
        sm.initialize()

        assertTrue(seen.isNotEmpty(), "entering `priming` asked the host to perform nothing at all")
        assertEquals(
            "prompt.start", seen[0].first,
            "entering `priming` did not ask the host to prompt; the acts seen were $seen",
        )
        assertTrue(
            seen[0].second.contains("North star:"),
            "the act carried no prompt, so a host would have to reach past the machine for " +
                "one: ${seen[0].second}",
        )
    }

    /// The sibling of `oneCancelReachesEveryRegion`.
    ///
    /// The document writes `fail` and `cancel` once each on the <parallel> and
    /// says so in a comment — one transition rather than one per region, because
    /// a run ends as a whole. Only `cancel` was asserted, and the two are not
    /// the same claim: they are separate transitions to separate terminals, and
    /// a consumer distinguishing "the run broke" from "somebody stopped it"
    /// reads which final it ended in.
    @Test
    fun aFailureEndsTheWholeRun() {
        val sm = started()

        sm.step(AiLoopEvent.Fail)

        assertTrue(
            sm.endedIn(AiLoopState.Failed),
            "`fail` is written on the `<parallel>` itself, so one event takes all three " +
                "regions to `failed` — a different outcome from `cancelled`, which is what " +
                "tells a broken run from a stopped one; ${sm.outcome()}",
        )
    }

    // ══════════════════════════════════════════════════════════════════
    // A run that outlived its process
    //
    // `enterAt` takes states, and nothing that crosses a process boundary can
    // carry one: a journal, a wire and a file all carry STRINGS. `nameOfState`
    // writes that record and `stateOfName` reads it back, and until the second
    // existed the door could be called and its argument could not be built — a
    // supervisor coming back had to `initialize()` instead, which is a replay
    // rather than a resume: `priming` performs its prompt on entry, so the
    // restored loop typed the first prompt again.
    //
    // A consumer-side table mapping the names it knows to states would compile
    // and would age silently — the document gains a state, the table does not,
    // the name reads back as null, and the resume quietly becomes a fresh start.
    // Only the generator writes the half that ages with the document, which is
    // why these two scenarios drive a GENERATED machine.
    // ══════════════════════════════════════════════════════════════════

    @Test
    fun aRunJournalledAsNamesResumesWhereItStopped() {
        val ran = started()
        ran.turn()
        ran.turn()

        // Everything a host can persist. Not states, not a configuration: text.
        val journal = ran.activeConfiguration.map { ran.nameOfState(it) }
        val journalledCurrent = ran.nameOfState(ran.currentState.value)
        assertTrue(
            journal.contains("working"),
            "the journal is meant to be taken mid-run, with the cycle at work; it reads $journal",
        )

        // A new process, holding nothing but those strings.
        val acts = mutableListOf<String>()
        val resumed = notInitialised(handler = { request ->
            acts.add(request.eventName)
            emptyList()
        })

        val configuration = journal.map { name ->
            val state = resumed.stateOfName(name)
            assertNotNull(
                state,
                "`$name` is a name this machine published through nameOfState and it did not " +
                    "read back, so a configuration cannot survive its own record",
            )
            state!!
        }
        val current = resumed.stateOfName(journalledCurrent)
        assertNotNull(current, "the current state's own name `$journalledCurrent` did not read back")

        assertEquals(
            ConfigurationRejection.NONE, resumed.enterAt(configuration, current!!),
            "a configuration this document published is one it can be put back into",
        )

        assertEquals(
            configuration.toSet(), resumed.activeConfiguration,
            "the machine came back somewhere other than where the journal said it was",
        )
        assertEquals(current, resumed.currentState.value)
        assertFalse(
            resumed.isInFinalState,
            "a machine put back into a mid-run configuration is not a finished one",
        )
        assertTrue(
            acts.isEmpty(),
            "resuming performed acts, which is the replay enterAt exists to avoid — a host " +
                "would see the run's earlier prompts sent a second time; performed $acts",
        )
    }

    @Test
    fun everyStateARunReachesReadsBackFromItsOwnName() {
        val seen = linkedSetOf<AiLoopState>()

        // The configuration AND the current state, because a run that ended is
        // where half the document's states live: Appendix D `exitInterpreter`
        // empties the configuration on the way into a top-level <final>, so a
        // walk that read only `activeConfiguration` would never record one of
        // the five outcomes this document exists to name. See `endedIn`.
        fun record(machine: AiLoopStateMachine) {
            seen.addAll(machine.activeConfiguration)
            seen.add(machine.currentState.value)
        }

        // Every outcome the document names, walked rather than listed: a state
        // is recorded here only because a run actually stood in it, and a
        // written-out list of states is what stateOfName exists to replace.
        var sm = started()
        record(sm)
        for (n in 1..60) {
            if (sm.holds(AiLoopState.Reflecting)) {
                record(sm)
                sm.step(AiLoopEvent.Reflect.Applied)
                record(sm)
                sm.step(AiLoopEvent.Session.Ready)
            }
            if (sm.isInFinalState) {
                break
            }
            sm.turn()
            record(sm)
        }
        record(sm)

        sm = started()
        sm.step(AiLoopEvent.Turn.Done)
        // Recorded here, before the verdict, because `judging` is where a
        // completed turn WAITS — the only state in the cycle a host reaches by
        // sending nothing. Every other branch of this walk records after driving
        // the machine on, and that is exactly how `judging` stayed unvisited
        // while the floor below read as satisfied.
        record(sm)
        sm.verdict(true)
        record(sm)
        sm.step(AiLoopEvent.Turn.Done)
        record(sm)

        sm = started()
        sm.step(AiLoopEvent.Turn.Blocked)
        record(sm)
        sm.step(AiLoopEvent.Screen.None)
        record(sm)
        sm.step(AiLoopEvent.Unattended)
        record(sm)

        sm = started()
        sm.step(AiLoopEvent.Hold)
        record(sm)
        sm.step(AiLoopEvent.Resume)
        record(sm)

        sm = started()
        sm.step(AiLoopEvent.Session.Lost)
        record(sm)
        sm.step(AiLoopEvent.Session.Ready)
        record(sm)

        sm = started()
        sm.step(AiLoopEvent.Cancel)
        record(sm)

        sm = started()
        sm.step(AiLoopEvent.Fail)
        record(sm)

        // A floor, not a target: without one, a table that had lost every arm
        // but the first would pass this by being asked about a single state.
        //
        // 21 is measured rather than chosen. The document declares 25 states and
        // the four below are unreachable to any reader of the configuration, so
        // a floor of 25 would retire this test and the 20 it used to hold
        // understated the walk by one.
        assertTrue(
            seen.size >= 21,
            "these scenarios are meant to stand in every state a reader can observe; they " +
                "reached ${seen.size} states: ${seen.map { sm.nameOfState(it) }}",
        )

        // The other side of that ratchet, and the reason the number above is a
        // measurement: these four are inner <final>s whose <onentry> is a
        // <raise> that ends the run in the SAME macrostep — `reported` raises
        // `run.converged`, `stuck` and `spent` raise `run.exhausted`,
        // `abandoned` raises `run.blocked` — so a configuration read taken
        // between macrosteps can never stand in one. Nothing else in the
        // document is like that.
        //
        // Asserting their ABSENCE is what keeps 21 honest from above: make one
        // of them observable, or extend the walk to reach it, and this fails
        // until the floor is raised with it.
        for (unobservable in listOf("abandoned", "reported", "spent", "stuck")) {
            val state = sm.stateOfName(unobservable)
            assertNotNull(state, "`$unobservable` is a state this document declares")
            assertFalse(
                seen.contains(state),
                "`$unobservable` was reached, so the ceiling this test documents has moved: " +
                    "raise the floor above to match what the walk now stands in",
            )
        }

        for (state in seen) {
            val name = sm.nameOfState(state)
            assertEquals(
                state, sm.stateOfName(name),
                "`$name` did not read back as the state that published it",
            )
        }

        // The other half of the contract: a name the document does not carry is
        // refused rather than guessed at. A table that answers anyway turns a
        // stale journal into a plausible-looking resume, which is the one
        // outcome a host has no way to detect afterwards.
        for (absent in listOf("no-such-state", "")) {
            assertNull(
                sm.stateOfName(absent),
                "`$absent` is not a state this document carries and it read back as one",
            )
        }
        assertNull(
            sm.stateOfName("turn.done"),
            "an event name is not a state name; the two tables are separate on purpose",
        )
    }
}
