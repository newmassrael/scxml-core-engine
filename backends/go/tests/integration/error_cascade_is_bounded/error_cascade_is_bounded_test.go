// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
// nothing about an error event something DOES match, answered by a handler
// that fails the same way every time: the failure raises error.execution, the
// same transition answers it, and the drain never empties. Go AOT path.
//
// That is not a hang, which is what makes it worth an accessor. Measured
// 2026-08-19 on the Python engine and a two-line document: 37,000 links a
// second, configuration unmoved, IsRunning true — the reading an unattended
// supervisor takes as a healthy idle machine while a core is pinned.
// unhandled_error_is_observable owns the error nobody answered; this owns the
// error answered by a handler that cannot handle it.
//
// The fixture separates a chain that STOPS by itself (settle, three links,
// then its guard stops matching) from one that cannot (spin). Both are runs of
// errors, and only the second is a defect.
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_error_cascade_is_bounded_go.sh

package error_cascade_is_bounded

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// maxLinks is the ceiling the engine applies, spelled here rather than read
// back from it. A test that asked the engine for its own limit would agree
// with any limit, including one an edit moved by three orders of magnitude.
const maxLinks int64 = 100

func started(t *testing.T) (*sce.Engine[ErrorCascadeIsBoundedState, ErrorCascadeIsBoundedEvent], *ErrorCascadeIsBoundedPolicy) {
	t.Helper()
	policy := NewErrorCascadeIsBoundedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts handler runs with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ErrorCascadeIsBoundedState, ErrorCascadeIsBoundedEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

// The fixture's <assign>s are the only witness that a handler ran at all —
// every outcome here leaves the configuration where it was.
func counter(t *testing.T, policy *ErrorCascadeIsBoundedPolicy, name string) int64 {
	t.Helper()
	got, ok := sce.ReadDatamodelInt(policy.ScriptEngine, policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares %q in its datamodel", name)
	}
	return got
}

// The axis: a handler that answers its own failure with the same failure is
// stopped, and the host is told.
//
// This test returning at all is half the assertion. Before the ceiling existed
// it did not: the same call ran until the harness was killed.
func TestAHandlerThatCannotHandleItsErrorIsStopped(t *testing.T) {
	engine, policy := started(t)
	if got := engine.ErrorCascadeEvents(); got != 0 {
		t.Fatalf("nothing has been refused before the machine has done anything, got %d", got)
	}

	engine.ProcessEvent(ErrorCascadeIsBoundedEventSpin)

	if got := counter(t, policy, "runs"); got != maxLinks {
		t.Fatalf("runaway's handler must run exactly as many times as the engine allows "+
			"links in a chain: fewer means the document was cut off early, more means "+
			"the ceiling moved; want %d got %d", maxLinks, got)
	}
	if got := counter(t, policy, "ticks"); got != maxLinks {
		t.Fatalf("every link's handler also raises the author's own tick, and every one of "+
			"them must be delivered. An engine that counted those as links would refuse at "+
			"half the depth; one that let them end the chain would never refuse at all — and "+
			"a handler that logs before it fails is an ordinary document; want %d got %d",
			maxLinks, got)
	}
	if got := engine.ErrorCascadeEvents(); got != 1 {
		t.Fatalf("the handler's <assign> failed again on the last allowed link, and the "+
			"error it raised is the one the engine refused to queue. Without that count "+
			"the host sees a machine that is running, in a plausible state, with nothing "+
			"to say about the core it is burning; got %d", got)
	}
	last, ok := engine.LastErrorCascadeEvent()
	if !ok || last != ErrorCascadeIsBoundedEventErrorExecution {
		t.Fatalf("a count alone does not name the repair: error.execution is a handler "+
			"whose own content fails, error.communication one that answers an unreachable "+
			"target by talking to it again; got %v (present=%v)", last, ok)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine — refusing to feed a broken handler " +
			"is not a reason to stop running a document whose other states still work")
	}
	if got := engine.GetCurrentState(); got != ErrorCascadeIsBoundedStateRunaway {
		t.Fatalf("the handler is targetless, so nothing here may move the machine, got %v", got)
	}
}

// The other half, and the one that makes the count mean something: a chain
// that ends by itself must pass through untouched.
func TestAChainThatEndsOnItsOwnIsNotRefused(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(ErrorCascadeIsBoundedEventSettle)

	if got := counter(t, policy, "repairs"); got != 3 {
		t.Fatalf("settling's handler repairs three times and then its `repairs < 3` guard "+
			"stops matching. Three links is what a real repair strategy looks like, and "+
			"the engine must not have interrupted it; got %d", got)
	}
	if got := engine.ErrorCascadeEvents(); got != 0 {
		t.Fatalf("nothing was refused: the chain ended on the document's own terms. A "+
			"ceiling that fired here would report every document that fails often as one "+
			"that cannot stop failing; got %d", got)
	}
	if _, ok := engine.LastErrorCascadeEvent(); ok {
		t.Fatal("nothing was refused, so there is no last one to name")
	}
	if got := engine.UnhandledErrorEvents(); got != 1 {
		t.Fatalf("the fourth error found no matching transition once the guard closed, "+
			"which is the ordinary clause — the two counts answer different questions "+
			"and this document produces exactly one of each; got %d", got)
	}
}

// A single failure with nobody to answer it is not a chain. The chain is
// measured handler-to-handler, not failure-to-failure.
func TestOneErrorNobodyAnsweredIsNotAChain(t *testing.T) {
	engine, _ := started(t)

	for i := 0; i < 5; i++ {
		engine.ProcessEvent(ErrorCascadeIsBoundedEventBoom)
	}

	if got := engine.UnhandledErrorEvents(); got != 5 {
		t.Fatalf("five failures, none of them answered — the clause's own case; got %d", got)
	}
	if got := engine.ErrorCascadeEvents(); got != 0 {
		t.Fatalf("no handler ran, so no handler raised anything: a count keyed off how "+
			"OFTEN a document fails would already be at five here; got %d", got)
	}
}

// The machine is still a machine afterwards. Cutting the chain must not cost
// the document the states that work.
func TestTheMachineStillAnswersAfterItsChainIsCut(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(ErrorCascadeIsBoundedEventSpin)
	if got := engine.ErrorCascadeEvents(); got != 1 {
		t.Fatalf("precondition: this test is about what happens AFTER a refusal, got %d", got)
	}

	engine.ProcessEvent(ErrorCascadeIsBoundedEventPoke)

	if got := counter(t, policy, "pokes"); got != 1 {
		t.Fatalf("runaway answers poke with a targetless transition, and it ran — an "+
			"engine that stopped the machine to end the chain would leave the host with "+
			"a dead document instead of a bounded one; got %d", got)
	}
	if got := engine.ErrorCascadeEvents(); got != 1 {
		t.Fatalf("poke raises nothing, so the count that was already there is all there "+
			"is: the refusal is a fact about the past, not a mode; got %d", got)
	}
}

// A second chain starts from zero. The depth is a property of the chain, not
// of the machine's whole life: an engine that never reset it would refuse the
// second chain on its first link, and the host would read a machine that has
// stopped trying rather than one that is still failing.
func TestASecondChainStartsFromZero(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(ErrorCascadeIsBoundedEventSpin)
	engine.ProcessEvent(ErrorCascadeIsBoundedEventReset)
	if got := engine.GetCurrentState(); got != ErrorCascadeIsBoundedStateIdle {
		t.Fatalf("reset is the fixture's way back out of the chain, got %v", got)
	}

	engine.ProcessEvent(ErrorCascadeIsBoundedEventSpin)

	if got := counter(t, policy, "runs"); got != 2*maxLinks {
		t.Fatalf("the second entry into runaway must buy the document a full chain again. "+
			"A depth carried across the drains would stop this one at its first link and "+
			"leave the counter at %d; want %d got %d", maxLinks, 2*maxLinks, got)
	}
	if got := engine.ErrorCascadeEvents(); got != 2 {
		t.Fatalf("two chains, two refusals — a count that saturates at one would read as "+
			"a machine that recovered; got %d", got)
	}
}
