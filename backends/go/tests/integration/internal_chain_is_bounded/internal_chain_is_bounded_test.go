// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
// by NULL AND the internal queue is empty. Appendix D's Principles and
// Constraints then say that end need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." Go AOT path.
//
// eventless_macrostep_is_bounded owns the half of that clause built from
// transitions that need no event. This one owns the other half: a <raise>
// answered by a transition that raises again. Measured 2026-08-20 before the
// ceiling reached this branch, ProcessEvent on the fixture's spin document did
// not return on this engine — the internal drain had no budget at all, and
// checkEventlessTransitions' hundred was spent on the branch that was not
// running.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Kotlin / Python / Rust channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_internal_chain_is_bounded_go.sh

package internal_chain_is_bounded

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

// alternatingLapsAtCeiling: one lap of the alternating chain is two microsteps
// — one internal event, one eventless transition — and only the internal half
// is counted, so a chain run to the shared ceiling records half.
const alternatingLapsAtCeiling int64 = maxMicrosteps / 2

func started(t *testing.T) (*sce.Engine[InternalChainIsBoundedState, InternalChainIsBoundedEvent], *InternalChainIsBoundedPolicy) {
	t.Helper()
	policy := NewInternalChainIsBoundedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts chain links with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[InternalChainIsBoundedState, InternalChainIsBoundedEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// The fixture's <assign>s are the only witness of how far a chain got — every
// outcome leaves the machine in a state the configuration alone cannot tell
// apart from the others.
func counter(t *testing.T, policy *InternalChainIsBoundedPolicy, name string) int64 {
	t.Helper()
	got, ok := sce.ReadDatamodelInt(policy.ScriptEngine, policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares %q in its datamodel", name)
	}
	return got
}

// The axis: a macrostep whose <raise> chain cannot end is stopped, and the host
// is told that it was. This test returning at all is half the assertion.
func TestARaiseChainThatCannotEndIsStopped(t *testing.T) {
	engine, policy := started(t)
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("nothing has been refused before the machine has done anything, got %d", got)
	}

	engine.ProcessEvent(InternalChainIsBoundedEventSpin)

	if got := counter(t, policy, "links"); got != maxMicrosteps {
		t.Fatalf("the chain must run exactly as far as the engine allows: fewer means the "+
			"document was cut off early, more means the ceiling moved; want %d got %d",
			maxMicrosteps, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 1 {
		t.Fatalf("the microstep past the budget was queued and was not taken. Without this "+
			"count the host sees a machine that is running, in a state the document names, "+
			"having returned in microseconds — and no way to learn that the configuration "+
			"it is reading is not a stable one; got %d", got)
	}
	state, ok := engine.LastTruncatedMacrostepState()
	if !ok || state != InternalChainIsBoundedStateSpin {
		t.Fatalf("the count alone says a document somewhere cannot settle; this says where "+
			"to look; got %v (present=%v)", state, ok)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine: the specification allows the document, so " +
			"refusing to run it forever is the engine's decision to report, not a reason to " +
			"stop a machine whose other states still work")
	}
}

// The other half, and the one that makes the count mean something: a chain that
// ends on its own is not refused, however long it is. The fixture's bounded
// chain is exactly maxMicrosteps links for this reason — a ceiling that counted
// loop turns rather than microsteps taken reports this ordinary document as a
// runaway.
func TestARaiseChainThatEndsAtTheCeilingIsNotRefused(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(InternalChainIsBoundedEventBounded)

	if got := counter(t, policy, "laps"); got != maxMicrosteps {
		t.Fatalf("the guard `laps < 999` stops matching at the thousandth link, which raises "+
			"nothing — so the queue empties and the chain stops by itself; want %d got %d",
			maxMicrosteps, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("nothing was refused: the macrostep reached the stable configuration the "+
			"clause describes, using every microstep it was allowed. A long chain is not a "+
			"runaway; got %d", got)
	}
	if _, ok := engine.LastTruncatedMacrostepState(); ok {
		t.Fatal("and nothing names a state, because nothing was stopped")
	}
	if !engine.IsRunning() {
		t.Fatal("a document that settles on its own must not be reported dead by an engine " +
			"that just finished running it correctly")
	}
}

// A dequeue that selected nothing is not a microstep, so it spends no budget.
//
// Appendix D takes a microstep for a transition that was SELECTED; a dequeue
// that matched none is the loop turn the clause does not count. The fixture's
// unanswered chain is `bounded` with one unmatched event added per link, so the
// two differ in exactly that and must cost the same.
//
// Measured 2026-08-21: this claim had no witness in any channel. The mutation
// that spends the budget on every dequeue SURVIVED all five outcomes, because
// every other chain here answers every event it raises — an engine that
// over-counted would report this settling document as a runaway at half its
// length, and nothing could see it.
func TestADequeueThatSelectedNothingSpendsNoBudget(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(InternalChainIsBoundedEventUnanswered)

	if got := counter(t, policy, "ignores"); got != maxMicrosteps {
		t.Fatalf("the chain is the same length as `bounded`; the unmatched events between "+
			"its links are dequeues that selected nothing, and those are not microsteps; "+
			"want %d got %d", maxMicrosteps, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("a thousand microsteps and a thousand discards is a thousand microsteps: an "+
			"engine that counted the discards refuses this document at link five hundred and "+
			"reports a runaway that is not one; got %d", got)
	}
	if _, ok := engine.LastTruncatedMacrostepState(); ok {
		t.Fatal("and nothing names a state, because nothing was stopped")
	}
	if !engine.IsRunning() {
		t.Fatal("the document settled on its own")
	}
}

// The case a per-branch budget lets through: a chain that alternates one
// <raise> with one eventless transition. Neither branch of Appendix D's inner
// loop reaches the ceiling on its own here, so an engine that gives each branch
// a counter of its own runs this document forever with both ceilings half
// spent. One of the seven shipped exactly that pair of counters.
func TestAnAlternatingChainSpendsOneSharedBudget(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(InternalChainIsBoundedEventAlternate)

	if got := counter(t, policy, "alts"); got != alternatingLapsAtCeiling {
		t.Fatalf("the two branches share one budget, so a chain that alternates them gets "+
			"five hundred laps out of a thousand microsteps. A thousand here would mean the "+
			"internal branch had a ceiling of its own; want %d got %d",
			alternatingLapsAtCeiling, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 1 {
		t.Fatalf("and the refusal is reported once, whichever branch was holding the budget "+
			"when it ran out; got %d", got)
	}
	state, ok := engine.LastTruncatedMacrostepState()
	if !ok || state != InternalChainIsBoundedStateAlt {
		t.Fatalf("named the same way as any other chain that could not settle; got %v "+
			"(present=%v)", state, ok)
	}
}

// What the refusal did with the links it would not run: it left them queued.
// The fixture's resume chain is half again as long as the ceiling, so the first
// macrostep is refused with five hundred links still to go and the second one
// finishes them. An engine that dropped the queue stops at a thousand and never
// finishes; one that ran the chain anyway finishes it in the first macrostep.
//
// The event driving the second macrostep is poke, and what it does is
// deliberately not asserted: internal events outrank it here, while the C++ AOT
// engine's processEvent takes the host's event first. That divergence is its
// own debt — the counters below are the same on both.
func TestARefusedChainIsLeftQueuedForTheNextMacrostep(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(InternalChainIsBoundedEventResume)
	if got := counter(t, policy, "beats"); got != maxMicrosteps {
		t.Fatalf("the first macrostep spends the whole budget on the chain; want %d got %d",
			maxMicrosteps, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 1 {
		t.Fatalf("want 1 got %d", got)
	}

	engine.ProcessEvent(InternalChainIsBoundedEventPoke)

	if got := counter(t, policy, "beats"); got != maxMicrosteps+maxMicrosteps/2 {
		t.Fatalf("the second macrostep picked the chain up where the first was cut and ran it "+
			"to its end — the refused links were left on the queue, not dropped; want %d got %d",
			maxMicrosteps+maxMicrosteps/2, got)
	}
	if got := engine.TruncatedMacrosteps(); got != 1 {
		t.Fatalf("and nothing was refused this time: the chain ended on its own inside the "+
			"budget, which is an ordinary macrostep however long the document took to get "+
			"there; got %d", got)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine")
	}
}

// The control: an ordinary document is untouched by any of this. Without it, an
// engine that refused every macrostep would pass the assertions above and fail
// nothing.
func TestAnOrdinaryMacrostepIsNotCounted(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(InternalChainIsBoundedEventPoke)

	if got := counter(t, policy, "pokes"); got != 1 {
		t.Fatalf("the run happened: a counter of zero cannot tell an engine that did nothing "+
			"from one that was never asked; got %d", got)
	}
	if got := engine.TruncatedMacrosteps(); got != 0 {
		t.Fatalf("and one transition is not a chain that cannot end; got %d", got)
	}
	if _, ok := engine.LastTruncatedMacrostepState(); ok {
		t.Fatal("nothing was stopped, so nothing names a state")
	}
	if got := engine.GetCurrentState(); got != InternalChainIsBoundedStateIdle {
		t.Fatalf("the control transition returns to idle; got %v", got)
	}
}
