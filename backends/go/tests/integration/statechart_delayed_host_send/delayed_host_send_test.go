// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.2.4 + §scxml-6.3 — a `<send delay>` addressed to a HOST-served Event
// I/O Processor waits, and can be cancelled while it waits. Go AOT path.
//
// §scxml-6.2.4 puts the wait before the dispatch and says nothing about which
// processor the send named; §scxml-6.2.5 makes that set open. Put together, a
// host-served send carrying a delay is an ordinary delayed send whose delivery
// happens to be somebody else's. It was not: every backend chose the host
// branch ahead of the delay branch in one `elif` chain per language, so the act
// was performed at the instant the block ran and `delay` was discarded — while
// the manifest went on answering `needs_event_scheduler: true`, telling the
// host to drive with Tick for a wait the engine had already thrown away.
//
// Driven entirely on ManualClock. Nothing here sleeps and nothing here can be
// decided by how loaded the build machine is: the host sets what time it is and
// the engine answers with the configuration that time implies. That matters
// more than usual on this axis, because a wall-clock version of the first case
// would pass on a slow machine for the wrong reason — the handler running
// "early" is only observable against a clock the test controls.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// (canonical, shared with the Rust / C++ / C11 / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_processor.sh

package statechart_delayed_host_send

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

// The type the fixture was compiled for. `scripts/regen_host_processor.sh`
// passes this same string to `--host-processor`.
const declaredType = "x-sce-host"

type harness struct {
	engine *sce.Engine[StatechartDelayedHostSendState, StatechartDelayedHostSendEvent]
	clock  *sce.ManualClock
	// calls holds the engine's own reading of "now" at the moment the handler
	// was asked to perform the act.
	//
	// The engine's clock rather than the test's bookkeeping, because that is
	// the number the contract is about — a handler called at 0 ms for a
	// delay="200ms" send is the defect, and any other witness (a counter, a
	// wall-clock stamp) only says it happened, not when the engine thought it
	// was.
	calls []int64
}

// newHarness builds a machine on host-owned time. The clock is installed and
// the handler registered BEFORE Initialize: the fixture's first send is armed
// on entry to its initial state, and the engine refuses a clock afterwards
// because deadlines armed against one do not compare with another.
func newHarness(withHandler bool) *harness {
	policy := NewStatechartDelayedHostSendPolicy()
	engine := sce.NewEngine[StatechartDelayedHostSendState, StatechartDelayedHostSendEvent](&policy)
	h := &harness{engine: engine, clock: sce.NewManualClock(0)}
	engine.SetClock(h.clock)
	if withHandler {
		engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse {
			h.calls = append(h.calls, h.clock.ElapsedMs())
			return []sce.HostSendResponse{{EventName: "turn.done"}}
		})
	}
	engine.Initialize()
	return h
}

// The axis. `waiting` arms a host-served send for 200 ms and an ordinary one
// for 100 ms; the ordinary one must arrive first, which is only true if the
// host-served one waited.
//
// The `tooEarly` final state is what the document reaches when it did not: the
// handler's reply is on the queue before the machine has been anywhere, so
// `turn.done` wins the race its own `delay` was supposed to lose.
func TestAHostServedSendWaitsForItsDelay(t *testing.T) {
	h := newHarness(true)

	// Nothing is due at 0 ms. This is the whole defect in one assertion: with
	// the host branch chosen ahead of the delay branch, Initialize has already
	// performed the act by the time this line runs.
	if len(h.calls) != 0 {
		t.Fatalf("the handler was asked to perform a delay=\"200ms\" send at %d ms. "+
			"§scxml-6.2.4 makes the delay the wait the document asked for, and §scxml-6.2.5 "+
			"does not exempt a host-served processor from it", h.engine.NowMs())
	}
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStateWaiting {
		t.Fatalf("the machine should be waiting on its two delayed sends; it is in %v", got)
	}

	// 100 ms: the ordinary `probe` is due, the host-served send is not.
	h.engine.AdvanceTimeMs(100)
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStateArmed {
		t.Fatalf("the 100 ms `probe` did not arrive first; the machine is in %v", got)
	}
	if len(h.calls) != 0 {
		t.Fatalf("the host-served send was dispatched before its 200 ms deadline (at %v)", h.calls)
	}

	// 200 ms: now it is due, and the handler's reply moves the machine on.
	h.engine.AdvanceTimeMs(100)
	if len(h.calls) != 1 || h.calls[0] != 200 {
		t.Fatalf("the host-served send did not fire at its 200 ms deadline; calls = %v", h.calls)
	}
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStateCancelling {
		t.Fatalf("the handler's `turn.done` did not reach the document; the machine is in %v", got)
	}
}

// §scxml-6.3: a `<cancel>` drops a delayed send that has not been dispatched. A
// host-served one is not exempt, and the witness is host-side: the handler must
// never be asked to perform the cancelled act at all.
//
// This is the half that says which queue the deferred send is in. An engine
// that honoured the delay by any private means — a side list, a timer goroutine
// — would pass the case above and fail here, because <cancel sendid> reaches
// the scheduler and nothing else.
func TestACancelDropsAPendingHostServedSend(t *testing.T) {
	h := newHarness(true)

	h.engine.AdvanceTimeMs(100) // probe     -> armed
	h.engine.AdvanceTimeMs(100) // turn.done -> cancelling (arms h2 for 400)
	h.engine.AdvanceTimeMs(100) // settle    -> cancelPending (cancels h2)
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStateCancelPending {
		t.Fatalf("the second round did not reach the state that runs <cancel sendid=\"h2\">; it is in %v", got)
	}

	// 400 ms: h2's deadline. It was cancelled at 300, so nothing may happen.
	h.engine.AdvanceTimeMs(100)
	if len(h.calls) != 1 {
		t.Fatalf("the handler was asked to perform `h2` at 400 ms after <cancel sendid=\"h2\"> ran at 300 ms "+
			"(calls = %v). A host-served act that a document cancelled must not reach the host: the side "+
			"effect is the point of the act, and the document cannot take it back", h.calls)
	}
	if got := h.engine.GetCurrentState(); got == StatechartDelayedHostSendStateCancelLost {
		t.Fatal("`turn.done` arrived for the cancelled send")
	}

	// 500 ms: `finish`. The verdict is itself scheduled, so a channel whose
	// tick loop stopped working fails here rather than passing by not moving.
	h.engine.AdvanceTimeMs(100)
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStatePass {
		t.Fatalf("the machine did not reach `pass`; it is in %v", got)
	}
}

// A deferred act whose handler was never registered is still an act nobody
// performed, and §scxml-6.2 reports that as `error.execution` — at the moment it
// was to be performed, not at the moment it was armed.
//
// The immediate path raises this at the send site. The deferred path cannot:
// the send site has already returned by the time the deadline arrives, so the
// engine owes the report. Without this case a wiring mistake on a delayed send
// is perfect silence — the document waits for a reply that no longer has anyone
// to come from.
func TestADeferredSendWithNoHandlerReportsItWhenItComesDue(t *testing.T) {
	h := newHarness(false)

	// At 100 ms the machine is in `armed`, whose `error.execution` transition
	// is the witness. Nothing has reported anything yet: the send was armed,
	// not performed, so there is nothing to report.
	h.engine.AdvanceTimeMs(100)
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStateArmed {
		t.Fatalf("the report arrived before the send was due; error.execution must be raised when the act "+
			"was to be performed, not when it was armed. The machine is in %v", got)
	}

	// 200 ms: the deadline. Nobody is registered, so nobody performs it, and
	// §scxml-6.2 says so.
	h.engine.AdvanceTimeMs(100)
	if got := h.engine.GetCurrentState(); got == StatechartDelayedHostSendStateCancelling {
		t.Fatal("nothing was registered to perform the act, yet `turn.done` arrived")
	}
	if got := h.engine.GetCurrentState(); got != StatechartDelayedHostSendStateUnserved {
		t.Fatalf("the deadline passed with no handler registered and nothing was reported (the machine is "+
			"in %v). The send site that raises this for an immediate send returned when the send was "+
			"armed, so whatever holds the deferred act owes the report — without it a wiring mistake on "+
			"a delayed send is perfect silence", got)
	}
}

// The engine must be able to say when the deferred host send comes due, or a
// host driving on TimeUntilNextScheduled sleeps straight past it.
//
// A deferred act kept anywhere the deadline query cannot see would leave this
// answering "nothing owed" at 0 ms while an act was owed at 200.
func TestTheEngineSaysWhenTheDeferredHostSendIsDue(t *testing.T) {
	h := newHarness(true)

	due, ok := h.engine.TimeUntilNextScheduled()
	if !ok {
		t.Fatal("two delayed sends are armed at 0 ms, so a deadline is owed")
	}
	if due != 100*time.Millisecond {
		t.Fatalf("the nearer of the two armed sends is the 100 ms `probe`; the engine answered %v", due)
	}

	h.engine.AdvanceTimeMs(100)
	due, ok = h.engine.TimeUntilNextScheduled()
	if !ok {
		t.Fatal("the host-served send is still pending at 100 ms, so a deadline is owed")
	}
	if due != 100*time.Millisecond {
		t.Fatalf("at 100 ms the host-served send is 100 ms out; the engine answered %v. A host sleeping on "+
			"this answer must land on the deferred act, not past it", due)
	}
}
