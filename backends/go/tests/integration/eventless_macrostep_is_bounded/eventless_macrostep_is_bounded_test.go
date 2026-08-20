// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
// configuration where nothing is enabled by NULL. Appendix D's Principles and
// Constraints then say the chain need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." Go AOT
// path.
//
// So a cyclic eventless document is not malformed, and an engine that runs it
// to the letter never returns. This one does not run it to the letter — and
// that decision was invisible from every other reading. Measured 2026-08-20 on
// a two-state document, this engine stopped the chain, returned in 1.6ms,
// reported IsRunning() true and a state the document names, and said nothing
// anywhere a program could read: the truncation went to a log line.
//
// error_cascade_is_bounded owns the chain built from errors; this one owns the
// chain built from transitions that need no event at all. The fixture
// separates a chain that stops on its own — a HUNDRED microsteps, exactly the
// ceiling, which is where an off-by-one lands — from one that cannot stop.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Kotlin / Python / Rust channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_eventless_macrostep_is_bounded_go.sh

package eventless_macrostep_is_bounded

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// maxMicrosteps is the ceiling the engine applies, spelled here rather than
// read back from it. A test that asked the engine for its own limit would
// agree with any limit, including one an edit moved by three orders of
// magnitude.
const maxMicrosteps int64 = 1000

// lapsAtCeiling: one lap of either chain is two microsteps (_a to _b, then
// back) and only the _a edge counts, so a chain run to the ceiling records
// half.
const lapsAtCeiling int64 = maxMicrosteps / 2

func started(t *testing.T) (*sce.Engine[EventlessMacrostepIsBoundedState, EventlessMacrostepIsBoundedEvent], *EventlessMacrostepIsBoundedPolicy) {
	t.Helper()
	policy := NewEventlessMacrostepIsBoundedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts chain laps with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[EventlessMacrostepIsBoundedState, EventlessMacrostepIsBoundedEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// The fixture's <assign>s are the only witness of how far a chain got — the
// configuration alone cannot tell a chain that stopped from one that was
// stopped.
func counter(t *testing.T, policy *EventlessMacrostepIsBoundedPolicy, name string) int64 {
	t.Helper()
	got, ok := sce.ReadDatamodelInt(policy.ScriptEngine, policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares %q in its datamodel", name)
	}
	return got
}

// The axis: a macrostep whose eventless chain cannot end is stopped, and the
// host is told that it was.
func TestAMacrostepThatCannotEndIsStopped(t *testing.T) {
	engine, policy := started(t)
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("nothing has been refused before the machine has done anything, got %d", got)
	}

	engine.ProcessEvent(EventlessMacrostepIsBoundedEventSpin)

	if got := counter(t, policy, "spins"); got != lapsAtCeiling {
		t.Fatalf("the chain must run exactly as far as the engine allows: fewer means the "+
			"document was cut off early, more means the ceiling moved; want %d got %d",
			lapsAtCeiling, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 1 {
		t.Fatalf("the microstep past the budget was enabled and was not taken. Without "+
			"this count the host sees a machine that is running, in a state the document "+
			"names, having returned in microseconds — and no way to learn that the "+
			"configuration it is reading is not a stable one; got %d", got)
	}
	last, ok := engine.LastTruncatedMacrostepState()
	if !ok || last != EventlessMacrostepIsBoundedStateSpinA {
		t.Fatalf("an eventless cycle is a closed walk through the state graph, and the "+
			"count alone does not say which walk. This names a state on it, which is "+
			"where an author looks first; got %v (present=%v)", last, ok)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine. §scxml-D allows the document; " +
			"refusing to run it forever is the engine's decision to report, not a reason " +
			"to stop a machine whose other states still work")
	}
}

// The other half, and the one that makes the count mean something: a chain
// that ends on its own is not refused, however long it is.
//
// The fixture's bounded chain is exactly maxMicrosteps microsteps for this
// reason. A ceiling that counted loop turns rather than microsteps taken, or
// that tested >= where it meant >, reports this ordinary document as a
// runaway.
func TestAChainThatEndsAtTheCeilingIsNotRefused(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(EventlessMacrostepIsBoundedEventBounded)

	if got := counter(t, policy, "laps"); got != lapsAtCeiling {
		t.Fatalf("the guard `laps < 500` closes after five hundred laps, so the chain is a "+
			"thousand microsteps long and then stops by itself; want %d got %d", lapsAtCeiling, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("nothing was refused: the macrostep reached the stable configuration "+
			"§scxml-3.13 describes, using every microstep it was allowed. A long chain is "+
			"not a runaway; got %d", got)
	}
	if _, ok := engine.LastTruncatedMacrostepState(); ok {
		t.Fatal("and nothing names a state, because nothing was stopped")
	}
	if !engine.IsRunning() {
		t.Fatal("a document that settles on its own must not be reported dead by an " +
			"engine that just finished running it correctly")
	}
	if got := engine.GetCurrentState(); got != EventlessMacrostepIsBoundedStateBoundedA {
		t.Fatalf("the chain rests where its guard closed, got %v", got)
	}
}

// A count, not a flag: a second unbounded macrostep is refused the same way
// the first was.
func TestASecondTruncatedMacrostepCountsAgain(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(EventlessMacrostepIsBoundedEventSpin)
	if got := engine.TruncatedMacrosteps(); got != 1 {
		t.Fatalf("precondition: this test is about the SECOND refusal, got %d", got)
	}

	// reset is the fixture's way back out of the cycle, and it moves the
	// machine on purpose: the two C++ engines complete a macrostep only after
	// a transition that does.
	engine.ProcessEvent(EventlessMacrostepIsBoundedEventReset)
	if got := engine.GetCurrentState(); got != EventlessMacrostepIsBoundedStateIdle {
		t.Fatalf("reset is the way back out of the chain, got %v", got)
	}

	engine.ProcessEvent(EventlessMacrostepIsBoundedEventSpin)

	if got := engine.TruncatedMacrosteps(); got != 2 {
		t.Fatalf("the second macrostep hit the same ceiling and must be counted again; got %d", got)
	}
	if got := counter(t, policy, "spins"); got != 2*lapsAtCeiling {
		t.Fatalf("and it really bought the document a full budget again rather than refusing on "+
			"sight — the ceiling bounds a macrostep, it does not condemn a machine; want %d got %d",
			2*lapsAtCeiling, got)
	}
}

// The control: an ordinary document is untouched by any of this. Without it,
// an engine that refused every macrostep would pass the assertions above and
// fail nothing.
func TestAnOrdinaryMacrostepIsNotCounted(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(EventlessMacrostepIsBoundedEventPoke)

	if got := counter(t, policy, "pokes"); got != 1 {
		t.Fatalf("the run fired, got %d", got)
	}
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("a macrostep of one microstep ends the way the clause says it does; got %d", got)
	}
	if _, ok := engine.LastTruncatedMacrostepState(); ok {
		t.Fatal("nothing was stopped, so there is no state to name")
	}
	if got := engine.GetCurrentState(); got != EventlessMacrostepIsBoundedStateIdle {
		t.Fatalf("poke is a self transition on idle, got %v", got)
	}
}
