// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + 6.3 — a `<send delay>` addressed to a HOST-served Event I/O
// Processor waits, and can be cancelled while it waits. Kotlin AOT path.
//
// W3C SCXML 6.2.4 puts the wait before the dispatch and says nothing about
// which processor the send named; 6.2.5 makes that set open. Put together, a
// host-served send carrying a delay is an ordinary delayed send whose delivery
// happens to be somebody else's. It was not: every backend chose the host
// branch ahead of the delay branch in one `elif` chain per language, so the act
// was performed at the instant the block ran and `delay` was discarded — while
// the manifest went on answering `needs_event_scheduler: true`, telling the
// host to drive with `tick()` for a wait the engine had already thrown away.
//
// Driven entirely on ManualClock. Nothing here sleeps and nothing here can be
// decided by how loaded the build machine is: the host sets what time it is and
// the engine answers with the configuration that time implies. That matters
// twice over on this suite, which has been measured timing out under load on
// wall-clock cases that this one has no equivalent of.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// (canonical, shared with the Rust / C++ / C11 / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_processor_kotlin.sh

package com.sce.integration

import com.sce.integration.statechart_delayed_host_send.StatechartDelayedHostSendState
import com.sce.integration.statechart_delayed_host_send.StatechartDelayedHostSendStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.StateMachineEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.2.4 + 6.3 — a delayed `<send type>` the host serves.
@DisplayName("DelayedHostSend — W3C SCXML 6.2.4 + 6.3")
class DelayedHostSendTest {

    // The type the fixture was compiled for.
    // `scripts/regen_host_processor_kotlin.sh` passes this same string to
    // `--host-processor`.
    private val declaredType = "x-sce-host"

    /// A machine on host-owned time with the handler each case decides on.
    ///
    /// The clock is installed and the handler registered BEFORE `initialize()`:
    /// the fixture's first send is armed on entry to its initial state, and a
    /// clock swapped afterwards would be compared against deadlines armed on
    /// the previous one.
    ///
    /// `calls` collects the engine's own reading of "now" at the moment the
    /// handler was asked to perform the act — the number the contract is about.
    /// A counter alone would say it happened, not when the engine thought it
    /// was.
    private inner class Harness(withHandler: Boolean) {
        val clock = ManualClock(0L)
        val calls = mutableListOf<Long>()
        // No engine argument: the fixture declares no datamodel, so codegen
        // emits a machine with no script engine at all. That is the fixture's
        // single-axis discipline showing up in the constructor.
        val sm = StatechartDelayedHostSendStateMachine()

        init {
            sm.clock = clock
            if (withHandler) {
                sm.registerEventProcessor(declaredType) {
                    calls.add(clock.elapsedMs())
                    listOf(StateMachineEngine.HostSendResponse("turn.done"))
                }
            }
            sm.initialize()
        }
    }

    // The axis. `waiting` arms a host-served send for 200 ms and an ordinary
    // one for 100 ms; the ordinary one must arrive first, which is only true if
    // the host-served one waited.
    //
    // The `tooEarly` final state is what the document reaches when it did not:
    // the handler's reply is on the queue before the machine has been anywhere,
    // so `turn.done` wins the race its own `delay` was supposed to lose.
    @Test
    fun aHostServedSendWaitsForItsDelay() {
        val h = Harness(withHandler = true)
        try {
            // Nothing is due at 0 ms. This is the whole defect in one
            // assertion: with the host branch chosen ahead of the delay branch,
            // initialize() has already performed the act by the time this runs.
            assertEquals(
                emptyList<Long>(), h.calls,
                "the handler was asked to perform a delay=\"200ms\" send at 0 ms. W3C SCXML 6.2.4 " +
                    "makes the delay the wait the document asked for, and 6.2.5 does not exempt a " +
                    "host-served processor from it"
            )
            assertEquals(StatechartDelayedHostSendState.Waiting, h.sm.currentState.value)

            // 100 ms: the ordinary `probe` is due, the host-served send is not.
            h.sm.advanceTimeMs(100)
            assertEquals(
                StatechartDelayedHostSendState.Armed, h.sm.currentState.value,
                "the 100 ms `probe` did not arrive first"
            )
            assertEquals(
                emptyList<Long>(), h.calls,
                "the host-served send was dispatched before its 200 ms deadline"
            )

            // 200 ms: now it is due, and the reply moves the machine on.
            h.sm.advanceTimeMs(100)
            assertEquals(
                listOf(200L), h.calls,
                "the host-served send did not fire at its 200 ms deadline"
            )
            assertEquals(
                StatechartDelayedHostSendState.Cancelling, h.sm.currentState.value,
                "the handler's `turn.done` did not reach the document"
            )
        } finally {
            h.sm.stop()
        }
    }

    // W3C SCXML 6.3: a `<cancel>` drops a delayed send that has not been
    // dispatched. A host-served one is not exempt, and the witness is
    // host-side: the handler must never be asked to perform the cancelled act.
    //
    // This is the half that says which queue the deferred send is in. An engine
    // that honoured the delay by any private means — a side list, a coroutine
    // timer — would pass the case above and fail here, because `<cancel sendid>`
    // reaches the scheduler and nothing else.
    @Test
    fun aCancelDropsAPendingHostServedSend() {
        val h = Harness(withHandler = true)
        try {
            h.sm.advanceTimeMs(100) // probe     -> armed
            h.sm.advanceTimeMs(100) // turn.done -> cancelling (arms h2 for 400)
            h.sm.advanceTimeMs(100) // settle    -> cancelPending (cancels h2)
            assertEquals(
                StatechartDelayedHostSendState.CancelPending, h.sm.currentState.value,
                "the second round did not reach the state that runs <cancel sendid=\"h2\">"
            )

            // 400 ms: h2's deadline. It was cancelled at 300, so nothing may
            // happen.
            h.sm.advanceTimeMs(100)
            assertEquals(
                listOf(200L), h.calls,
                "the handler was asked to perform `h2` at 400 ms after <cancel sendid=\"h2\"> ran at " +
                    "300 ms. A host-served act that a document cancelled must not reach the host: " +
                    "the side effect is the point of the act, and the document cannot take it back"
            )
            assertNotEquals(
                StatechartDelayedHostSendState.CancelLost, h.sm.currentState.value,
                "`turn.done` arrived for the cancelled send"
            )

            // 500 ms: `finish`. The verdict is itself scheduled, so a channel
            // whose tick loop stopped working fails here rather than passing by
            // not moving.
            h.sm.advanceTimeMs(100)
            assertEquals(
                StatechartDelayedHostSendState.Pass, h.sm.currentState.value,
                "the machine did not reach `pass`"
            )
        } finally {
            h.sm.stop()
        }
    }

    // A deferred act whose handler was never registered is still an act nobody
    // performed, and W3C SCXML 6.2 reports that as `error.execution` — at the
    // moment it was to be performed, not at the moment it was armed.
    //
    // The immediate path raises this at the send site. The deferred path
    // cannot: that site has already returned by the time the deadline arrives,
    // so the engine owes the report. Without this case a wiring mistake on a
    // delayed send is perfect silence — the document waits for a reply that no
    // longer has anyone to come from.
    @Test
    fun aDeferredSendWithNoHandlerReportsItWhenItComesDue() {
        val h = Harness(withHandler = false)
        try {
            // At 100 ms the machine is in `armed`, whose `error.execution`
            // transition is the witness. Nothing has reported anything yet: the
            // send was armed, not performed, so there is nothing to report.
            h.sm.advanceTimeMs(100)
            assertEquals(
                StatechartDelayedHostSendState.Armed, h.sm.currentState.value,
                "the report arrived before the send was due; error.execution must be raised when " +
                    "the act was to be performed, not when it was armed"
            )

            // 200 ms: the deadline. Nobody is registered, so nobody performs
            // it, and W3C SCXML 6.2 says so.
            h.sm.advanceTimeMs(100)
            assertNotEquals(
                StatechartDelayedHostSendState.Cancelling, h.sm.currentState.value,
                "nothing was registered to perform the act, yet `turn.done` arrived"
            )
            assertEquals(
                StatechartDelayedHostSendState.Unserved, h.sm.currentState.value,
                "the deadline passed with no handler registered and nothing was reported. The send " +
                    "site that raises this for an immediate send returned when the send was armed, " +
                    "so whatever holds the deferred act owes the report — without it a wiring " +
                    "mistake on a delayed send is perfect silence"
            )
        } finally {
            h.sm.stop()
        }
    }

    // The engine must be able to say when the deferred host send comes due, or
    // a host driving on `timeUntilNextScheduledMs()` sleeps straight past it.
    //
    // A deferred act kept anywhere the deadline query cannot see would leave
    // this answering null at 0 ms — "nothing is owed" — while an act was owed
    // at 200.
    @Test
    fun theEngineSaysWhenTheDeferredHostSendIsDue() {
        val h = Harness(withHandler = true)
        try {
            assertEquals(
                100L, h.sm.timeUntilNextScheduledMs(),
                "the nearer of the two armed sends is the 100 ms `probe`"
            )

            h.sm.advanceTimeMs(100)
            assertEquals(
                100L, h.sm.timeUntilNextScheduledMs(),
                "at 100 ms the host-served send is 100 ms out. A host sleeping on this answer must " +
                    "land on the deferred act, not past it"
            )
        } finally {
            h.sm.stop()
        }
    }
}
