// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's main event loop returns to
// selectEventlessTransitions() after every microstep, and drains the internal
// queue in the same inner loop. It never asks whether the microstep it just
// took moved the machine — it cannot, because W3C SCXML 3.13 defines a
// transition with no target as one that exits and enters nothing and runs its
// content in place. Go AOT path.
//
// Measured 2026-08-20, the two C++ engines end the macrostep at such a
// transition: whatever its content enabled is never walked, and the host is
// handed a configuration the clause says is not stable. This channel is the
// side of that comparison that was already right, and it is here so the
// contract is stated for every backend rather than only for the ones that
// broke it.
//
// eventless_macrostep_is_bounded owns how FAR a chain may run; this one owns
// whether the chain is entered at all.
//
// Fixture: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// (canonical, shared with the C++ / C11 / Kotlin / Python / Rust channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_targetless_transition_completes_macrostep_go.sh

package targetless_transition_completes_macrostep

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func started(t *testing.T) (*sce.Engine[TargetlessTransitionCompletesMacrostepState, TargetlessTransitionCompletesMacrostepEvent], *TargetlessTransitionCompletesMacrostepPolicy) {
	t.Helper()
	policy := NewTargetlessTransitionCompletesMacrostepPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts what the macrostep reached with <assign>, so this is
	// an ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[TargetlessTransitionCompletesMacrostepState, TargetlessTransitionCompletesMacrostepEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// The fixture's <assign>s are the only witness of how far the macrostep got:
// every outcome here leaves the machine in a state the configuration alone
// cannot tell apart from a macrostep that stopped one microstep early.
func counter(t *testing.T, policy *TargetlessTransitionCompletesMacrostepPolicy, name string) int64 {
	t.Helper()
	got, ok := sce.ReadDatamodelInt(policy.ScriptEngine, policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares %q in its datamodel", name)
	}
	return got
}

// The axis: a transition that moves nothing still ends a microstep, so the
// macrostep continues into whatever its content enabled.
//
// chained == 1, polished == 0 is the signature of an engine that resumes the
// chain only after a transition that MOVED the machine: it takes the link that
// moves and stops before the link that does not. chained == 0 is the signature
// of one that never entered the chain at all. Both are failures of the same
// clause, and the two counters are what tell them apart.
func TestATargetlessTransitionDoesNotEndTheMacrostep(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventArm)

	if got := counter(t, policy, "armed"); got != 1 {
		t.Fatalf("the targetless transition must run its content — without that the rest measures a lost event "+
			"rather than a stopped macrostep, got armed=%d", got)
	}
	if got := counter(t, policy, "chained"); got != 1 {
		t.Errorf("the eventless transition that content enabled must be taken in the SAME macrostep, which is "+
			"the whole of what Appendix D's inner loop promises a host, got chained=%d", got)
	}
	if got := counter(t, policy, "polished"); got != 1 {
		t.Errorf("including the chain's last link, which is targetless itself: an engine that walks the chain "+
			"only while the machine keeps moving stops exactly here, got polished=%d", got)
	}
	if got := engine.GetCurrentState(); got != TargetlessTransitionCompletesMacrostepStateSettled {
		t.Errorf("the host must be handed the stable configuration, not the one the machine was passing "+
			"through, got %v", got)
	}
}

// The other side of the same inner loop: what a targetless transition raises is
// answered before the host gets control back.
func TestARaiseFromATargetlessTransitionIsAnsweredInTheSameMacrostep(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventPing)

	if got := counter(t, policy, "answered"); got != 1 {
		t.Errorf("the internal event the targetless transition raised must be dequeued and matched inside this "+
			"macrostep, got answered=%d", got)
	}
	if got := engine.GetCurrentState(); got != TargetlessTransitionCompletesMacrostepStateIdle {
		t.Errorf("neither transition moves the machine, which is the point: the macrostep has to continue "+
			"anyway, got %v", got)
	}
}

// The control, and the reason a zero above means anything: a targetless
// transition that enables nothing leaves the machine exactly where it was, and
// having run is still observable.
func TestATargetlessTransitionThatEnablesNothingChangesNothingElse(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventQuiet)

	if got := counter(t, policy, "quiet"); got != 1 {
		t.Fatalf("the transition must fire, got quiet=%d", got)
	}
	if got := counter(t, policy, "chained"); got != 0 {
		t.Errorf("and nothing else may: the eventless transition's guard is still closed, so an engine that "+
			"walked the chain here would be firing a transition the document did not enable, got chained=%d", got)
	}
	if got := counter(t, policy, "polished"); got != 0 {
		t.Errorf("polished=%d", got)
	}
	if got := counter(t, policy, "answered"); got != 0 {
		t.Errorf("answered=%d", got)
	}
	if got := engine.GetCurrentState(); got != TargetlessTransitionCompletesMacrostepStateIdle {
		t.Errorf("state=%v", got)
	}
	if !engine.IsRunning() {
		t.Errorf("the machine must still be running")
	}
}

// The other microstep that ends where it began: a transition whose target is
// its own source.
//
// It is not targetless — W3C SCXML 3.13 gives it an exit and an entry — but a
// macrostep loop that continues only while the configuration keeps changing
// drops it for the same reason and, in the C++ AOT engine, in the same line of
// code. entries == 1 is that engine: the transition was selected, nothing ran,
// and the chain ended.
func TestAnEventlessSelfTransitionExitsAndReEnters(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventRecycle)

	if got := counter(t, policy, "entries"); got != 2 {
		t.Errorf("the state is entered once by `recycle` and once more by the eventless self transition its "+
			"entry enabled — a self transition exits and re-enters, so <onentry> runs again, got entries=%d", got)
	}
	if got := engine.GetCurrentState(); got != TargetlessTransitionCompletesMacrostepStateRecycled {
		t.Errorf("and the guard closes behind it, so the machine rests here rather than spinning, got %v", got)
	}
}

// A macrostep, not a one-shot: the second targetless transition is followed the
// same way the first was.
func TestTheSecondTargetlessTransitionIsFollowedToo(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventQuiet)
	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventPing)
	if got := counter(t, policy, "answered"); got != 1 {
		t.Fatalf("precondition: this test is about the SECOND raise, got answered=%d", got)
	}

	engine.ProcessEvent(TargetlessTransitionCompletesMacrostepEventPing)

	if got := counter(t, policy, "answered"); got != 2 {
		t.Errorf("the raise in the third macrostep must be answered like the one in the second — the inner loop "+
			"belongs to every macrostep, not to the first, got answered=%d", got)
	}
	if got := counter(t, policy, "quiet"); got != 1 {
		t.Errorf("quiet=%d", got)
	}
	if got := engine.GetCurrentState(); got != TargetlessTransitionCompletesMacrostepStateIdle {
		t.Errorf("state=%v", got)
	}
}
