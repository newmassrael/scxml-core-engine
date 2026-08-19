// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: "If no transition matches in any state, the event is
// discarded" — and the host that fed it in can find out. Go AOT path.
//
// Three outcomes leave the configuration identical, so no accessor that
// existed before this fixture separates them:
//
//	poke    self transition       handled (exits and re-enters idle)
//	nudge   targetless internal   handled (actions only, no exit/entry)
//	settle  no matching           DISCARDED — the host's event went nowhere
//
// The C++ Interpreter answers all three (processEvent's TransitionResult and
// getStatistics().failedTransitions); the generated engines computed the same
// fact at the same point of Appendix D's mainEventLoop and dropped it.
//
// nudge is in the fixture because the engine's own "did anything happen" bool
// is a different fact: it reports whether the configuration changed, and a
// targetless internal transition answers false after running its actions.
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_discarded_event_is_observable_go.sh

package discarded_event_is_observable

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func started(t *testing.T) (*sce.Engine[DiscardedEventIsObservableState, DiscardedEventIsObservableEvent], *DiscardedEventIsObservablePolicy) {
	t.Helper()
	policy := NewDiscardedEventIsObservablePolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts handled events with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[DiscardedEventIsObservableState, DiscardedEventIsObservableEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// The axis: an event the machine knows but no active state answers is counted.
func TestAnEventNoActiveStateAnsweredIsCounted(t *testing.T) {
	engine, _ := started(t)
	if got := engine.DiscardedExternalEvents(); got != 0 {
		t.Fatalf("nothing has been discarded before the first event, got %d", got)
	}

	// `settle` is declared in `busy`, so it is in the machine's vocabulary and
	// the host can name it — it just matches nothing while the machine is in
	// `idle`.
	engine.ProcessEvent(DiscardedEventIsObservableEventSettle)

	if got := engine.DiscardedExternalEvents(); got != 1 {
		t.Fatalf("`settle` came off the external queue in `idle`, where no transition "+
			"matches it. W3C SCXML 3.1.2 discards it; the host that queued it has no "+
			"other way to learn its event went nowhere. Count = %d", got)
	}
	if got := engine.GetCurrentState(); got != DiscardedEventIsObservableStateIdle {
		t.Fatalf("a discarded event must not move the machine, now in %v", got)
	}
}

// The other half: a handled event must NOT be counted, including the one that
// changes nothing.
func TestAHandledEventIsNotCounted(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(DiscardedEventIsObservableEventPoke)
	if pokes, ok := policy.Pokes(); !ok || pokes != 1 {
		t.Fatalf("`poke`'s self transition did not run (pokes=%d ok=%v), so nothing "+
			"below is measuring a handled event", pokes, ok)
	}
	if got := engine.DiscardedExternalEvents(); got != 0 {
		t.Fatalf("`poke` matched a self transition — handled, and the configuration is "+
			"unchanged only because the transition returns to its own source. Count = %d", got)
	}

	engine.ProcessEvent(DiscardedEventIsObservableEventNudge)
	if nudges, ok := policy.Nudges(); !ok || nudges != 1 {
		t.Fatalf("`nudge`'s targetless transition did not run (nudges=%d ok=%v)", nudges, ok)
	}
	if got := engine.DiscardedExternalEvents(); got != 0 {
		t.Fatalf("`nudge` matched a targetless internal transition: its actions ran and no "+
			"state was exited or entered. The engine's own configuration-changed bool is "+
			"false here, which is why the count cannot be keyed off it. Count = %d", got)
	}
}

// Why the query has to exist at all: every pre-existing accessor answers the
// same for a handled event and a discarded one.
func TestTheDiscardIsNotDerivableFromAnyOtherAccessor(t *testing.T) {
	engine, _ := started(t)

	engine.ProcessEvent(DiscardedEventIsObservableEventPoke)
	handledState := engine.GetCurrentState()
	handledRunning := engine.IsRunning()
	handledFinal := engine.IsInFinalState()

	engine.ProcessEvent(DiscardedEventIsObservableEventSettle)
	if engine.GetCurrentState() != handledState ||
		engine.IsRunning() != handledRunning ||
		engine.IsInFinalState() != handledFinal {
		t.Fatal("this fixture exists because a handled event and a discarded one are " +
			"indistinguishable through the accessors a host had; if they ever differ, " +
			"the fixture stopped measuring what it claims")
	}
	if got := engine.DiscardedExternalEvents(); got != 1 {
		t.Fatalf("the two are indistinguishable through every other accessor, so the "+
			"count is the only thing that separates them. Count = %d", got)
	}
}

// A count says something went nowhere; a host debugging a stalled supervisor
// needs to know which event did.
func TestTheEngineNamesTheEventItDiscarded(t *testing.T) {
	engine, _ := started(t)
	if _, ok := engine.LastDiscardedEvent(); ok {
		t.Fatal("nothing has been discarded yet, so there is no event to name")
	}

	engine.ProcessEvent(DiscardedEventIsObservableEventSettle)

	last, ok := engine.LastDiscardedEvent()
	if !ok || last != DiscardedEventIsObservableEventSettle {
		t.Fatalf("the engine counted a discard but named %v (ok=%v)", last, ok)
	}
}

// The supervisor's actual failure mode: the machine moved on, and the events
// the host keeps sending no longer match anything.
func TestAnEventTheMachineHasMovedPastIsCounted(t *testing.T) {
	engine, _ := started(t)
	engine.ProcessEvent(DiscardedEventIsObservableEventGo)
	if got := engine.GetCurrentState(); got != DiscardedEventIsObservableStateBusy {
		t.Fatalf("`go` should have moved the machine out of `idle`, now in %v", got)
	}

	engine.ProcessEvent(DiscardedEventIsObservableEventPoke)
	if got := engine.DiscardedExternalEvents(); got != 1 {
		t.Fatalf("the machine left `idle`, so `poke` no longer matches — the host that "+
			"kept sending it is exactly who the count is for. Count = %d", got)
	}
	if last, ok := engine.LastDiscardedEvent(); !ok || last != DiscardedEventIsObservableEventPoke {
		t.Fatalf("the discarded event should be `poke`, got %v (ok=%v)", last, ok)
	}
}
