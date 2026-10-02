// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty — Go AOT path.
//
// Every macrostep of such a machine ends, so maxMacrostepMicrosteps never
// applies, and the main event loop takes the next external event whenever the
// queue is not empty: a host call that drains it did not return. The Go
// runtime's runMainEventLoop had that shape until it took the budget
// ARCHITECTURE.md "External-Event Budget" states as one contract for every
// engine. This driver holds this engine to it: the same outcomes, the same
// arithmetic, the same document as the Python and Rust ones.
//
// The delayed outcomes run on ManualClock, so nothing here sleeps and the clock
// moves only where a case moves it. This engine hands a STATIC zero delay to its
// scheduler (Rust reads it as undelayed), so `zero` and `zero_expr` both reach
// the same-instant bound here.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// integration_resources/ for the reason
// scripts/regen_external_chain_is_bounded.sh states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_external_chain_is_bounded_go.sh

package external_chain_is_bounded

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// defaultBudget is the default the contract states, spelled here rather than
// read back from the engine: a test that asked the engine for its own limit
// would agree with any limit, including one an edit moved by three orders of
// magnitude.
const defaultBudget int64 = 10000

type machine = *sce.Engine[ExternalChainIsBoundedState, ExternalChainIsBoundedEvent]

func started(t *testing.T) (machine, *ExternalChainIsBoundedPolicy) {
	t.Helper()
	policy := NewExternalChainIsBoundedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts chain links with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ExternalChainIsBoundedState, ExternalChainIsBoundedEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	return engine, &policy
}

func budget(t *testing.T, engine machine, events int) {
	t.Helper()
	if err := engine.SetMaxExternalEventsPerCall(events); err != nil {
		t.Fatalf("a budget of %d events was refused: %v", events, err)
	}
}

// The fixture's <assign>s are the only witness of how far a chain got: every
// outcome leaves the machine in a state the configuration alone cannot tell
// apart from the others.
func counter(t *testing.T, policy *ExternalChainIsBoundedPolicy, name string) int64 {
	t.Helper()
	got, ok := sce.ReadDatamodelInt(policy.ScriptEngine, policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares %q in its datamodel", name)
	}
	return got
}

func TestTheDefaultBudgetIsTheDocumentedOne(t *testing.T) {
	engine, _ := started(t)
	if got := int64(engine.MaxExternalEventsPerCall()); got != defaultBudget {
		t.Fatalf("the default budget is %d, got %d", defaultBudget, got)
	}
	if got := engine.TruncatedEventChains(); got != 0 {
		t.Fatalf("nothing has been cut before the machine has done anything, got %d", got)
	}
	if _, ok := engine.LastTruncatedEvent(); ok {
		t.Fatal("no call has been handed back, so there is no event to name")
	}
}

// This test returning at all is half the assertion: before the budget the call
// did not.
func TestAChainThatCannotEndIsCutAtTheBudgetAndTheCallReturns(t *testing.T) {
	engine, policy := started(t)

	engine.ProcessEvent(ExternalChainIsBoundedEventSpin)

	if got := engine.TruncatedEventChains(); got != 1 {
		t.Fatalf("the call handed control back with an event still queued, and said so; without "+
			"the count the host sees a machine that is running and has returned, with no sign "+
			"that anything went wrong; got %d", got)
	}
	// The host's own event is the first of the invocation, so the budget buys
	// the host's event and then budget-1 links.
	if got := counter(t, policy, "links"); got != defaultBudget-1 {
		t.Fatalf("the chain must run exactly as far as the budget allows: fewer means the call "+
			"was cut early, more means the budget moved; want %d got %d", defaultBudget-1, got)
	}
	if event, ok := engine.LastTruncatedEvent(); !ok || event != ExternalChainIsBoundedEventLink {
		t.Fatalf("the count says a call did not reach quiet; this says what it was still taking; "+
			"got %v (present=%v)", event, ok)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine: the document is legal, and refusing to run " +
			"it forever is the engine's decision to report, not a reason to stop a machine whose " +
			"other states still work")
	}
}

// The half that makes the count mean something: a chain that ends on its own is
// not refused, however close to the budget it comes. `bounded` is the host's
// event and five laps, six in all.
func TestTheBudgetIsExactForAChainThatEndsByItself(t *testing.T) {
	exactly, policy := started(t)
	budget(t, exactly, 6)
	exactly.ProcessEvent(ExternalChainIsBoundedEventBounded)
	if got := counter(t, policy, "laps"); got != 5 {
		t.Fatalf("the finite chain is the host's event and five laps; laps = %d", got)
	}
	if got := exactly.TruncatedEventChains(); got != 0 {
		t.Fatalf("a call that takes exactly the budget and empties the queue refused nothing: a "+
			"long chain is not a runaway; counted %d", got)
	}
	if _, ok := exactly.LastTruncatedEvent(); ok {
		t.Fatal("nothing was cut, so there is no event to name")
	}

	oneShort, policy := started(t)
	budget(t, oneShort, 5)
	oneShort.ProcessEvent(ExternalChainIsBoundedEventBounded)
	if got := counter(t, policy, "laps"); got != 4 {
		t.Fatalf("one lap was left queued; laps = %d", got)
	}
	if got := oneShort.TruncatedEventChains(); got != 1 {
		t.Fatalf("the call was one event short and said so; counted %d", got)
	}
	if event, ok := oneShort.LastTruncatedEvent(); !ok || event != ExternalChainIsBoundedEventLap {
		t.Fatalf("the cut left a lap queued; got %v (present=%v)", event, ok)
	}
}

// What the refusal did with the events it would not take: it left them queued.
// An engine that dropped the queue stops short and never finishes; one that ran
// the chain anyway finishes it in the first call.
func TestARefusedCallLeavesTheQueueSoTheNextCallFinishesTheChain(t *testing.T) {
	engine, policy := started(t)
	budget(t, engine, 20)

	engine.ProcessEvent(ExternalChainIsBoundedEventResume)
	if got := engine.TruncatedEventChains(); got != 1 {
		t.Fatalf("the first call was cut; counted %d", got)
	}
	if got := counter(t, policy, "beats"); got != 19 {
		t.Fatalf("the host's event and nineteen beats; beats = %d", got)
	}

	engine.ProcessEvent(ExternalChainIsBoundedEventPoke)
	if got := counter(t, policy, "beats"); got != 30 {
		t.Fatalf("the second call took the beats the first left on the queue, each in a budget "+
			"of its own, and finished; beats = %d", got)
	}
	if got := counter(t, policy, "pokes"); got != 1 {
		t.Fatalf("and the host's second event was heard; pokes = %d", got)
	}
	if got := engine.TruncatedEventChains(); got != 1 {
		t.Fatalf("the second call ended the way the clause says: nothing more is counted; got %d", got)
	}
}

// delay="0ms" is due at the instant being processed. This engine hands it to its
// scheduler, so a tick pops the entry, its handler arms another due at the same
// reading, and the tick that is popping finds it. Each pass takes one event, so a
// budget on the drain alone never trips — this is the case ARCHITECTURE.md rule 5
// exists for. This test returning at all is the assertion.
func TestAChainThroughAStaticDelayOfZeroDoesNotKeepTheTickFromReturning(t *testing.T) {
	engine, policy := started(t)
	budget(t, engine, 50)

	engine.ProcessEvent(ExternalChainIsBoundedEventZero)
	if got := engine.TruncatedEventChains(); got != 0 {
		t.Fatalf("entering the state arms one entry and pops none: nothing has been cut yet; got %d", got)
	}

	engine.Tick()

	if got := engine.TruncatedEventChains(); got != 1 {
		t.Fatalf("the tick popped entries due at its own reading until the budget, left the due "+
			"one waiting and said so; counted %d", got)
	}
	if got := counter(t, policy, "blinks"); got != 50 {
		t.Fatalf("the budget of pops at one reading, no more and no fewer; blinks = %d", got)
	}
	if event, ok := engine.LastTruncatedEvent(); !ok || event != ExternalChainIsBoundedEventBlink {
		t.Fatalf("the cut left a blink due; got %v (present=%v)", event, ok)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine")
	}
}

// The same chain through delayexpr="'0ms'", which has no static value an engine
// could read as undelayed.
func TestAChainThroughADelayExpressionThatIsZeroDoesNotKeepTheTickFromReturning(t *testing.T) {
	engine, policy := started(t)
	budget(t, engine, 50)

	engine.ProcessEvent(ExternalChainIsBoundedEventZeroExpr)
	engine.Tick()

	if got := engine.TruncatedEventChains(); got != 1 {
		t.Fatalf("the tick popped entries due at its own reading until the budget and said so; "+
			"counted %d", got)
	}
	if got := counter(t, policy, "exprs"); got != 50 {
		t.Fatalf("the budget of pops at one reading, no more and no fewer; exprs = %d", got)
	}
	if !engine.IsRunning() {
		t.Fatal("the chain was cut, not the machine")
	}
}

// Eight pulses, each due one millisecond after the last. They are due at later
// instants, so a legitimate time-driven workload is not a runaway however small
// the budget: three here, against eight events.
func TestAChainThatIsFiniteBecauseTheClockIsIsNotRefused(t *testing.T) {
	walked, policy := started(t)
	budget(t, walked, 3)
	walked.ProcessEvent(ExternalChainIsBoundedEventTimed)
	for i := 0; i < 8; i++ {
		walked.AdvanceTimeMs(1)
	}
	if got := counter(t, policy, "pulses"); got != 8 {
		t.Fatalf("every pulse was heard; pulses = %d", got)
	}
	if got := walked.TruncatedEventChains(); got != 0 {
		t.Fatalf("each pulse came in a tick of its own, at an instant of its own; counted %d", got)
	}

	jumped, jumpedPolicy := started(t)
	budget(t, jumped, 3)
	jumped.ProcessEvent(ExternalChainIsBoundedEventTimed)
	jumped.AdvanceTimeMs(8)
	if got := counter(t, jumpedPolicy, "pulses"); got != 8 {
		t.Fatalf("every pulse was heard in one jump; pulses = %d", got)
	}
	if got := jumped.TruncatedEventChains(); got != 0 {
		t.Fatalf("eight entries came due on the way to one reading, seven of them at earlier "+
			"instants: a clock that moved a long way is bounded by how far it moved, and is not "+
			"a chain that does not end; counted %d", got)
	}
}

func TestAHostChoosesTheBudgetAndABudgetThatTakesNoEventIsRefused(t *testing.T) {
	engine, _ := started(t)
	budget(t, engine, 7)
	if got := engine.MaxExternalEventsPerCall(); got != 7 {
		t.Fatalf("the budget the host chose is 7, got %d", got)
	}
	for _, refused := range []int{0, -1, -100} {
		if err := engine.SetMaxExternalEventsPerCall(refused); err == nil {
			t.Fatalf("a budget of %d takes no event and was accepted", refused)
		}
	}
	if got := engine.MaxExternalEventsPerCall(); got != 7 {
		t.Fatalf("a refused budget changes nothing; got %d", got)
	}
}
