// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
// error.* events into the internal queue, and the same paragraph says they
// "are ignored if no transition is found that matches them". Being ignored is
// the clause. Being unable to say it happened is not. Go AOT path.
//
// discarded_event_is_observable asked this for the EXTERNAL queue and stopped
// at its edge on the stated ground that an unmatched internal event is the
// document's own business with both ends inside the document. That is exactly
// right for an author's <raise> and exactly wrong for an error event, whose
// sender is the ENGINE. The host never wrote the document, cannot see the
// failure in the configuration, and is the only party able to act on it.
//
// Four outcomes the fixture separates, all four leaving the configuration on
// the same state:
//
//	poke              handled, no error            control: proves a run fired
//	whisper           author's <raise>, unmatched  NOT counted
//	boom in idle      error, unmatched             COUNTED — the silent failure
//	boom in guarded   error, HANDLED               not counted
//
// boom is one event name routed to two outcomes by state, so a count cannot be
// keyed off the event or the action — only off what the configuration did with
// the error the engine raised.
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_unhandled_error_is_observable_go.sh

package unhandled_error_is_observable

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func started(t *testing.T) (*sce.Engine[UnhandledErrorIsObservableState, UnhandledErrorIsObservableEvent], *UnhandledErrorIsObservablePolicy) {
	t.Helper()
	policy := NewUnhandledErrorIsObservablePolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts handled events with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[UnhandledErrorIsObservableState, UnhandledErrorIsObservableEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// The fixture's <assign>s are the only witness that a transition ran at all.
func counter(t *testing.T, policy *UnhandledErrorIsObservablePolicy, name string) int64 {
	t.Helper()
	got, ok := sce.ReadDatamodelInt(policy.ScriptEngine, policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares %q in its datamodel", name)
	}
	return got
}

// The axis: an error the engine raised that no active state answers is counted.
func TestAnErrorNoTransitionAnsweredIsCounted(t *testing.T) {
	engine, policy := started(t)
	if got := engine.UnhandledErrorEvents(); got != 0 {
		t.Fatalf("no error has gone unhandled before the first event, got %d", got)
	}

	engine.ProcessEvent(UnhandledErrorIsObservableEventBoom)

	if got := counter(t, policy, "booms"); got != 1 {
		t.Fatalf("boom's transition did not run (booms=%d), so nothing below is "+
			"measuring an error raised inside a transition that fired", got)
	}
	if got := engine.UnhandledErrorEvents(); got != 1 {
		t.Fatalf("boom's second <assign> has W3C 5.3's invalid empty location, so the "+
			"engine raised error.execution — and idle declares no transition for it. "+
			"The host driving this machine has no other way to learn its <assign> "+
			"failed; got %d", got)
	}
	if got := engine.GetCurrentState(); got != UnhandledErrorIsObservableStateIdle {
		t.Fatalf("the error must not move the machine on its own, got %v", got)
	}
}

// The other half: an error the DOCUMENT answered must not be counted. A count
// that is always non-zero is as useless as one that is always zero.
func TestAnErrorTheDocumentHandledIsNotCounted(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(UnhandledErrorIsObservableEventGo)
	if got := engine.GetCurrentState(); got != UnhandledErrorIsObservableStateGuarded {
		t.Fatalf("go should have moved the machine to the state that answers errors, got %v", got)
	}

	engine.ProcessEvent(UnhandledErrorIsObservableEventBoom)

	if got := counter(t, policy, "caught"); got != 1 {
		t.Fatalf("guarded's error.execution transition did not run (caught=%d), so this "+
			"test is not measuring a HANDLED error", got)
	}
	if got := engine.UnhandledErrorEvents(); got != 0 {
		t.Fatalf("the same <assign> failed in guarded, where the document does declare a "+
			"transition for error.execution. The document dealt with it, and its handling "+
			"is already visible in the configuration — counting it would report the "+
			"author's own error handling as a silent failure; got %d", got)
	}
	if _, ok := engine.LastUnhandledError(); ok {
		t.Fatal("nothing went unhandled, so there is no last one to name")
	}
}

// The boundary the count is drawn at: an author's own unmatched <raise> is not
// an unhandled error. Both ends of that event are inside the document, which is
// why DiscardedExternalEvents stops at the external queue — and why this count
// does not stop there.
func TestAnAuthorsUnmatchedRaiseIsNotAnUnhandledError(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(UnhandledErrorIsObservableEventWhisper)

	if got := engine.UnhandledErrorEvents(); got != 0 {
		t.Fatalf("whisper raises `unheard` and `retry.error.execution`, neither of which any "+
			"state answers. Both are discarded exactly as an unmatched error is, and neither "+
			"is one: the author wrote the raises and the absent handlers. "+
			"retry.error.execution is the sharper half — it CONTAINS `error.` without starting "+
			"with it, and W3C 3.12.2 reserves the prefix, not the substring; got %d", got)
	}
	if got := counter(t, policy, "heards"); got != 1 {
		t.Fatalf("whisper's third raise, `heard`, does match — and the transition it matches "+
			"did not run (heards=%d). The count above is a byproduct of the internal drain, "+
			"never its job: an implementation that only selects transitions for error events "+
			"stops running the document for everything else", got)
	}
	if got := engine.DiscardedExternalEvents(); got != 0 {
		t.Fatalf("whisper itself was handled, so the external-queue count stays put — "+
			"the internal events it raised are not on that queue at all; got %d", got)
	}
}

// Why the query has to exist: every pre-existing accessor answers the same for
// a run that failed silently and one that did not fail at all.
func TestTheUnhandledErrorIsNotDerivableFromAnyOtherAccessor(t *testing.T) {
	engine, _ := started(t)

	engine.ProcessEvent(UnhandledErrorIsObservableEventPoke)
	cleanState := engine.GetCurrentState()
	cleanRunning := engine.IsRunning()
	cleanDiscarded := engine.DiscardedExternalEvents()

	engine.ProcessEvent(UnhandledErrorIsObservableEventBoom)
	failedState := engine.GetCurrentState()
	failedRunning := engine.IsRunning()
	failedDiscarded := engine.DiscardedExternalEvents()

	if cleanState != failedState || cleanRunning != failedRunning || cleanDiscarded != failedDiscarded {
		t.Fatalf("this fixture exists because these two are indistinguishable through every "+
			"accessor a host had — including layer three's discard count, which never sees "+
			"the internal queue. If they ever differ, the fixture stopped measuring what it "+
			"claims: clean=(%v,%v,%d) failed=(%v,%v,%d)",
			cleanState, cleanRunning, cleanDiscarded, failedState, failedRunning, failedDiscarded)
	}
	if got := engine.UnhandledErrorEvents(); got != 1 {
		t.Fatalf("the two are indistinguishable through every other accessor, so this count "+
			"is the only thing that separates a silent failure from a clean run; got %d", got)
	}
}

// A count says something failed; a host repairing it needs the class of error.
func TestTheEngineNamesTheErrorItDropped(t *testing.T) {
	engine, _ := started(t)
	if _, ok := engine.LastUnhandledError(); ok {
		t.Fatal("nothing has gone unhandled yet")
	}

	engine.ProcessEvent(UnhandledErrorIsObservableEventBoom)

	got, ok := engine.LastUnhandledError()
	if !ok {
		t.Fatal("the engine counted an unhandled error but reports none to name")
	}
	if got != UnhandledErrorIsObservableEventErrorExecution {
		t.Fatalf("error.execution is the document's own executable content failing; "+
			"error.communication would be a <send> that could not reach its target. Two "+
			"different repairs, and a bare count separates neither; got %v", got)
	}
}

// The supervisor's actual failure mode: every round fails the same way and
// nothing in the configuration ever changes.
func TestAMachineFailingEveryRoundIsCountedEveryRound(t *testing.T) {
	engine, policy := started(t)

	for round := uint32(1); round <= 3; round++ {
		engine.ProcessEvent(UnhandledErrorIsObservableEventBoom)
		if got := engine.UnhandledErrorEvents(); got != round {
			t.Fatalf("round %d did not add to the count; a supervisor polling this number "+
				"is exactly who learns the loop is not making progress; got %d", round, got)
		}
		if got := engine.GetCurrentState(); got != UnhandledErrorIsObservableStateIdle {
			t.Fatalf("the machine looks identical on every round, which is the problem; got %v", got)
		}
	}
	if got := counter(t, policy, "booms"); got != 3 {
		t.Fatalf("all three rounds should have run their transition, got %d", got)
	}
}
