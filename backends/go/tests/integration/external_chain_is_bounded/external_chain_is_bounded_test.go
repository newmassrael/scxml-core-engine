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
// The numbers (the default budget, each outcome's budget and counters) are read
// from tests/external_event_budget/budget_outcomes.json with encoding/json, the
// one table every engine's test of this contract is held to. They are read from
// that file and not from the engine: a test that asked the engine for its own
// limit would agree with any limit, including one an edit moved by three orders
// of magnitude.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// integration_resources/ for the reason
// scripts/regen_external_chain_is_bounded.sh states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_external_chain_is_bounded_go.sh

package external_chain_is_bounded

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// outcomes is the part of the shared table this driver reads. A key the table
// lacks would decode as zero, so loadOutcomes refuses a table whose default
// budget, host budget, static-zero cut count or timed interval came out zero.
type outcomes struct {
	DefaultBudget int64 `json:"default_budget"`
	Spin          struct {
		Truncated uint32 `json:"truncated"`
	} `json:"spin"`
	Exact struct {
		Budget    int    `json:"budget"`
		Laps      int64  `json:"laps"`
		Truncated uint32 `json:"truncated"`
	} `json:"exact"`
	OneShort struct {
		Budget    int    `json:"budget"`
		Laps      int64  `json:"laps"`
		Truncated uint32 `json:"truncated"`
	} `json:"one_short"`
	Resume struct {
		Budget               int    `json:"budget"`
		BeatsAfterFirstCall  int64  `json:"beats_after_first_call"`
		BeatsAfterSecondCall int64  `json:"beats_after_second_call"`
		Pokes                int64  `json:"pokes"`
		Truncated            uint32 `json:"truncated"`
	} `json:"resume"`
	StaticZero struct {
		Budget              int    `json:"budget"`
		TruncatedAfterEntry uint32 `json:"truncated_after_entry"`
		Truncated           uint32 `json:"truncated"`
	} `json:"static_zero"`
	ZeroExpr struct {
		Budget              int    `json:"budget"`
		Exprs               int64  `json:"exprs"`
		TruncatedAfterEntry uint32 `json:"truncated_after_entry"`
		TruncatedAfterTick  uint32 `json:"truncated_after_tick"`
	} `json:"zero_expr"`
	Timed struct {
		Budget     int    `json:"budget"`
		Pulses     int64  `json:"pulses"`
		IntervalMs int64  `json:"interval_ms"`
		Truncated  uint32 `json:"truncated"`
	} `json:"timed"`
	Host struct {
		Budget  int   `json:"budget"`
		Refused []int `json:"refused"`
	} `json:"host"`
}

// loadOutcomes reads the shared table. The test runs in its own package
// directory, five levels below the repository root.
func loadOutcomes(t *testing.T) outcomes {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "tests", "external_event_budget", "budget_outcomes.json"))
	if err != nil {
		t.Fatalf("the shared budget table: %v", err)
	}
	var table outcomes
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the shared budget table is not JSON: %v", err)
	}
	if table.DefaultBudget < 1 || table.Host.Budget < 1 || len(table.Host.Refused) == 0 ||
		table.StaticZero.Truncated < 1 || table.Timed.IntervalMs < 1 {
		t.Fatalf("the shared budget table was not read: %+v", table)
	}
	return table
}

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
	table := loadOutcomes(t)
	engine, _ := started(t)
	if got := int64(engine.MaxExternalEventsPerCall()); got != table.DefaultBudget {
		t.Fatalf("the default budget is %d, got %d", table.DefaultBudget, got)
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
	table := loadOutcomes(t)
	engine, policy := started(t)

	engine.ProcessEvent(ExternalChainIsBoundedEventSpin)

	if got := engine.TruncatedEventChains(); got != table.Spin.Truncated {
		t.Fatalf("the call handed control back with an event still queued, and said so; without "+
			"the count the host sees a machine that is running and has returned, with no sign "+
			"that anything went wrong; got %d", got)
	}
	// The host's own event is the first of the invocation, so the budget buys
	// the host's event and then budget-1 links.
	if got := counter(t, policy, "links"); got != table.DefaultBudget-1 {
		t.Fatalf("the chain must run exactly as far as the budget allows: fewer means the call "+
			"was cut early, more means the budget moved; want %d got %d", table.DefaultBudget-1, got)
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
	table := loadOutcomes(t)
	exactly, policy := started(t)
	budget(t, exactly, table.Exact.Budget)
	exactly.ProcessEvent(ExternalChainIsBoundedEventBounded)
	if got := counter(t, policy, "laps"); got != table.Exact.Laps {
		t.Fatalf("the finite chain is the host's event and five laps; laps = %d", got)
	}
	if got := exactly.TruncatedEventChains(); got != table.Exact.Truncated {
		t.Fatalf("a call that takes exactly the budget and empties the queue refused nothing: a "+
			"long chain is not a runaway; counted %d", got)
	}
	if _, ok := exactly.LastTruncatedEvent(); ok {
		t.Fatal("nothing was cut, so there is no event to name")
	}

	oneShort, policy := started(t)
	budget(t, oneShort, table.OneShort.Budget)
	oneShort.ProcessEvent(ExternalChainIsBoundedEventBounded)
	if got := counter(t, policy, "laps"); got != table.OneShort.Laps {
		t.Fatalf("one lap was left queued; laps = %d", got)
	}
	if got := oneShort.TruncatedEventChains(); got != table.OneShort.Truncated {
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
	table := loadOutcomes(t)
	engine, policy := started(t)
	budget(t, engine, table.Resume.Budget)

	engine.ProcessEvent(ExternalChainIsBoundedEventResume)
	if got := engine.TruncatedEventChains(); got != table.Resume.Truncated {
		t.Fatalf("the first call was cut; counted %d", got)
	}
	if got := counter(t, policy, "beats"); got != table.Resume.BeatsAfterFirstCall {
		t.Fatalf("the host's event and nineteen beats; beats = %d", got)
	}

	engine.ProcessEvent(ExternalChainIsBoundedEventPoke)
	if got := counter(t, policy, "beats"); got != table.Resume.BeatsAfterSecondCall {
		t.Fatalf("the second call took the beats the first left on the queue, each in a budget "+
			"of its own, and finished; beats = %d", got)
	}
	if got := counter(t, policy, "pokes"); got != table.Resume.Pokes {
		t.Fatalf("and the host's second event was heard; pokes = %d", got)
	}
	if got := engine.TruncatedEventChains(); got != table.Resume.Truncated {
		t.Fatalf("the second call ended the way the clause says: nothing more is counted; got %d", got)
	}
}

// delay="0ms" is due at the instant being processed. This engine hands it to its
// scheduler, so a tick pops the entry, its handler arms another due at the same
// reading, and the tick that is popping finds it. Each pass takes one event, so a
// budget on the drain alone never trips — this is the case ARCHITECTURE.md rule 5
// exists for. This test returning at all is the assertion.
func TestAChainThroughAStaticDelayOfZeroDoesNotKeepTheTickFromReturning(t *testing.T) {
	table := loadOutcomes(t)
	engine, policy := started(t)
	budget(t, engine, table.StaticZero.Budget)

	engine.ProcessEvent(ExternalChainIsBoundedEventZero)
	if got := engine.TruncatedEventChains(); got != table.StaticZero.TruncatedAfterEntry {
		t.Fatalf("entering the state arms one entry and pops none: nothing has been cut yet; got %d", got)
	}

	engine.Tick()

	if got := engine.TruncatedEventChains(); got != table.StaticZero.Truncated {
		t.Fatalf("the tick popped entries due at its own reading until the budget, left the due "+
			"one waiting and said so; counted %d", got)
	}
	if got := counter(t, policy, "blinks"); got != int64(table.StaticZero.Budget) {
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
	table := loadOutcomes(t)
	engine, policy := started(t)
	budget(t, engine, table.ZeroExpr.Budget)

	engine.ProcessEvent(ExternalChainIsBoundedEventZeroExpr)
	engine.Tick()

	if got := engine.TruncatedEventChains(); got != table.ZeroExpr.TruncatedAfterTick {
		t.Fatalf("the tick popped entries due at its own reading until the budget and said so; "+
			"counted %d", got)
	}
	if got := counter(t, policy, "exprs"); got != table.ZeroExpr.Exprs {
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
	table := loadOutcomes(t)
	walked, policy := started(t)
	budget(t, walked, table.Timed.Budget)
	walked.ProcessEvent(ExternalChainIsBoundedEventTimed)
	for i := int64(0); i < table.Timed.Pulses; i++ {
		walked.AdvanceTimeMs(table.Timed.IntervalMs)
	}
	if got := counter(t, policy, "pulses"); got != table.Timed.Pulses {
		t.Fatalf("every pulse was heard; pulses = %d", got)
	}
	if got := walked.TruncatedEventChains(); got != table.Timed.Truncated {
		t.Fatalf("each pulse came in a tick of its own, at an instant of its own; counted %d", got)
	}

	jumped, jumpedPolicy := started(t)
	budget(t, jumped, table.Timed.Budget)
	jumped.ProcessEvent(ExternalChainIsBoundedEventTimed)
	// Each pulse is due the table's interval after the last, so one jump over
	// all of them is that many milliseconds for each pulse.
	jumped.AdvanceTimeMs(table.Timed.Pulses * table.Timed.IntervalMs)
	if got := counter(t, jumpedPolicy, "pulses"); got != table.Timed.Pulses {
		t.Fatalf("every pulse was heard in one jump; pulses = %d", got)
	}
	if got := jumped.TruncatedEventChains(); got != table.Timed.Truncated {
		t.Fatalf("eight entries came due on the way to one reading, seven of them at earlier "+
			"instants: a clock that moved a long way is bounded by how far it moved, and is not "+
			"a chain that does not end; counted %d", got)
	}
}

func TestAHostChoosesTheBudgetAndABudgetThatTakesNoEventIsRefused(t *testing.T) {
	table := loadOutcomes(t)
	engine, _ := started(t)
	budget(t, engine, table.Host.Budget)
	if got := int(engine.MaxExternalEventsPerCall()); got != table.Host.Budget {
		t.Fatalf("the budget the host chose is %d, got %d", table.Host.Budget, got)
	}
	for _, refused := range table.Host.Refused {
		if err := engine.SetMaxExternalEventsPerCall(refused); err == nil {
			t.Fatalf("a budget of %d takes no event and was accepted", refused)
		}
	}
	if got := int(engine.MaxExternalEventsPerCall()); got != table.Host.Budget {
		t.Fatalf("a refused budget changes nothing; got %d", got)
	}
}
