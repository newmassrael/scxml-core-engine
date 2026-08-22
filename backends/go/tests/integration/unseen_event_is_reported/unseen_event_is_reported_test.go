// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
// stopped is never looked at, and the host that sent it can find out — Go AOT.
//
// Appendix D's main event loop exits when the machine reaches a top-level final
// state. Refusing what arrives afterwards is the clause; saying nothing about
// it is not. The silence is expensive because it looks like the two outcomes a
// host can already read:
//
//	dequeued, no transition matched            -> DiscardedExternalEvents
//	dequeued, matched, guard said no           -> nothing, correctly
//	never dequeued — the machine had stopped   -> this
//
// Fixture: integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_unseen_event_is_reported_go.sh

package unseen_event_is_reported

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func started(t *testing.T) (*sce.Engine[UnseenEventIsReportedState, UnseenEventIsReportedEvent], *UnseenEventIsReportedPolicy) {
	t.Helper()
	policy := NewUnseenEventIsReportedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts handled deliveries with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[UnseenEventIsReportedState, UnseenEventIsReportedEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

func deliver(engine *sce.Engine[UnseenEventIsReportedState, UnseenEventIsReportedEvent],
	event UnseenEventIsReportedEvent) {
	engine.RaiseExternal(event, "", "")
	engine.Step()
}

// The axis: an event the host queued after the machine stopped is counted.
func TestAnEventDeliveredAfterTheMachineStoppedIsCounted(t *testing.T) {
	engine, policy := started(t)
	if got := engine.UnseenExternalEvents(); got != 0 {
		t.Fatalf("nothing has been refused before the first event, got %d", got)
	}

	deliver(engine, UnseenEventIsReportedEventPoke)
	if pokes, ok := policy.Pokes(); !ok || pokes != 1 {
		t.Fatalf("`poke`'s transition did not run (pokes=%d ok=%v), so nothing below is "+
			"measuring a machine that was working first", pokes, ok)
	}

	deliver(engine, UnseenEventIsReportedEventFinish)
	if !engine.IsInFinalState() {
		t.Fatalf("`finish` should have taken the machine to its top-level final state")
	}
	if got := engine.UnseenExternalEvents(); got != 0 {
		t.Fatalf("`finish` was itself dequeued and handled — the machine stopped BECAUSE of "+
			"it, which is not the same as stopping before it. Count = %d", got)
	}

	deliver(engine, UnseenEventIsReportedEventPoke)

	if got := engine.UnseenExternalEvents(); got != 1 {
		t.Fatalf("the host queued `poke` on a machine that had reached its final state. W3C "+
			"SCXML Appendix D's loop had already ended, so the event was never dequeued; "+
			"before this count the host had no way to learn that. Count = %d", got)
	}
	if pokes, _ := policy.Pokes(); pokes != 1 {
		t.Fatalf("the refused delivery ran the document's transition anyway (pokes=%d) — the "+
			"count would then be reporting something that did not happen", pokes)
	}
}

// A machine can stop two different ways, and each is refused at a different
// door. Both have to answer.
//
// Reaching a top-level final state does NOT clear isRunning — the main event
// loop simply stops draining, so what the host queued is abandoned there.
// Stop() is the other way: ProcessEvent returns before anything is queued, so
// the loop never sees the delivery at all.
//
// Measured 2026-08-23 on the Rust twin of this engine: a round with only the
// final-state assertions left a mutation that deletes the door-side record
// CAUGHT by nothing, because every test stopped the machine the other way.
func TestAMachineStoppedByItsHostRefusesAtTheOtherDoor(t *testing.T) {
	engine, policy := started(t)
	deliver(engine, UnseenEventIsReportedEventPoke)
	if pokes, _ := policy.Pokes(); pokes != 1 {
		t.Fatalf("the machine should have been working first (pokes=%d)", pokes)
	}

	engine.Stop()
	if engine.IsRunning() {
		t.Fatalf("Stop() should have halted the engine")
	}
	if engine.IsInFinalState() {
		t.Fatalf("Stop() halted the machine WITHOUT a final state, which is the point of " +
			"this test: the other assertions reach a final state instead, and that " +
			"leaves isRunning true")
	}
	if got := engine.UnseenExternalEvents(); got != 0 {
		t.Fatalf("stopping is not itself a refused event; count = %d", got)
	}

	engine.ProcessEvent(UnseenEventIsReportedEventPoke)

	if got := engine.UnseenExternalEvents(); got != 1 {
		t.Fatalf("ProcessEvent returned early because the host had stopped the engine, so "+
			"the event never reached the queue the main event loop drains — a no-op "+
			"nobody can count is the silence this axis is about. Count = %d", got)
	}
	if last, ok := engine.LastUnseenEvent(); !ok || last != UnseenEventIsReportedEventPoke {
		t.Fatalf("the door has to name what it refused, exactly as the loop does "+
			"(last=%v ok=%v)", last, ok)
	}
	if pokes, _ := policy.Pokes(); pokes != 1 {
		t.Fatalf("the refused delivery ran the document's transition anyway (pokes=%d)", pokes)
	}
}

// Why the query has to exist at all: every other accessor answers the same
// before and after the refused delivery.
func TestTheRefusalIsNotDerivableFromAnyOtherAccessor(t *testing.T) {
	engine, policy := started(t)
	deliver(engine, UnseenEventIsReportedEventFinish)

	beforeState := engine.GetCurrentState()
	beforeRunning := engine.IsRunning()
	beforeFinal := engine.IsInFinalState()
	beforeDiscarded := engine.DiscardedExternalEvents()
	beforePokes, _ := policy.Pokes()

	deliver(engine, UnseenEventIsReportedEventPoke)

	if got := engine.GetCurrentState(); got != beforeState {
		t.Fatalf("this fixture exists because a refused delivery is indistinguishable through "+
			"the accessors a host had; the state moved (%v -> %v)", beforeState, got)
	}
	if engine.IsRunning() != beforeRunning || engine.IsInFinalState() != beforeFinal {
		t.Fatalf("the run flags moved across a refused delivery")
	}
	if got := engine.DiscardedExternalEvents(); got != beforeDiscarded {
		t.Fatalf("the discard count moved across a refused delivery (%d -> %d)", beforeDiscarded, got)
	}
	if pokes, _ := policy.Pokes(); pokes != beforePokes {
		t.Fatalf("the document's own count moved across a refused delivery (%d -> %d)",
			beforePokes, pokes)
	}

	if got := engine.UnseenExternalEvents(); got != 1 {
		t.Fatalf("the two readings agree on everything else, so this count is the only thing "+
			"that separates `the machine never looked` from `it looked and nothing "+
			"matched`. Count = %d", got)
	}
}

// The distinction the whole axis turns on: a discard and a refusal are
// different facts, and each has its own count.
func TestADiscardAndARefusalAreCountedSeparately(t *testing.T) {
	engine, _ := started(t)

	deliver(engine, UnseenEventIsReportedEventPoke)
	if got := engine.DiscardedExternalEvents(); got != 0 {
		t.Fatalf("`poke` matched a targetless transition; nothing was discarded. Count = %d", got)
	}
	if got := engine.UnseenExternalEvents(); got != 0 {
		t.Fatalf("the machine was running, so nothing was refused either. Count = %d", got)
	}

	deliver(engine, UnseenEventIsReportedEventFinish)
	deliver(engine, UnseenEventIsReportedEventPoke)

	if d, u := engine.DiscardedExternalEvents(), engine.UnseenExternalEvents(); d != 0 || u != 1 {
		t.Fatalf("a refusal must not be reported as a discard: the first says the machine "+
			"looked and nothing matched, the second says it never looked, and a host acts "+
			"differently on each. discarded=%d unseen=%d", d, u)
	}
}

// A count says an event went unlooked-at; a host debugging a supervisor that
// stopped answering needs to know which one.
func TestTheEngineNamesTheEventItNeverLookedAt(t *testing.T) {
	engine, _ := started(t)
	if _, ok := engine.LastUnseenEvent(); ok {
		t.Fatalf("nothing has been refused yet, so there is no last unseen event")
	}

	deliver(engine, UnseenEventIsReportedEventFinish)
	deliver(engine, UnseenEventIsReportedEventPoke)
	if last, ok := engine.LastUnseenEvent(); !ok || last != UnseenEventIsReportedEventPoke {
		t.Fatalf("the engine counted a refusal but cannot say which event it refused "+
			"(last=%v ok=%v)", last, ok)
	}

	deliver(engine, UnseenEventIsReportedEventFinish)
	if got := engine.UnseenExternalEvents(); got != 2 {
		t.Fatalf("the count is a count, not a flag; got %d", got)
	}
	if last, ok := engine.LastUnseenEvent(); !ok || last != UnseenEventIsReportedEventFinish {
		t.Fatalf("the name did not follow the second refusal (last=%v ok=%v)", last, ok)
	}
}
