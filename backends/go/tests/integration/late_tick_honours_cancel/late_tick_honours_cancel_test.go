// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.3: a <cancel> still lands when the host ticked late —
// Go AOT path.
//
// The scheduler queue is ordered by fire time and Engine.Tick drains it.
// Draining it to exhaustion before running a macrostep is the defect: a host
// that wakes after two fire times have passed holds both entries, and putting
// both on the external queue makes the second undroppable before the first
// one's transitions have run. The <cancel> then executes against a queue the
// event has already left.
//
// The host below sleeps past BOTH fire times before its first tick, because
// that is the only condition under which the two dispatch orders differ. A
// host that wakes between them passes either way, which is why every existing
// suite was blind to this.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_late_tick_honours_cancel_go.sh

package late_tick_honours_cancel

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

// pastBothDeadlines is long enough that both <send delay>s in `waiting`
// (100 ms and 200 ms) are past due when the first tick runs, with margin for
// a loaded machine.
const pastBothDeadlines = 400 * time.Millisecond

func started() *sce.Engine[LateTickHonoursCancelState, LateTickHonoursCancelEvent] {
	policy := NewLateTickHonoursCancelPolicy()
	engine := sce.NewEngine[LateTickHonoursCancelState, LateTickHonoursCancelEvent](&policy)
	engine.Initialize()
	return engine
}

// The fixture is only meaningful on a scheduler-driven machine, and the policy
// is where a consumer reads that without running anything.
func TestFixtureIsSchedulerDriven(t *testing.T) {
	policy := NewLateTickHonoursCancelPolicy()
	if !policy.NeedsEventScheduler() {
		t.Fatal("the fixture arms two delayed <send>s; a policy that does not report " +
			"NeedsEventScheduler means the document lost them, and every assertion " +
			"below would then be measuring the wrong machine")
	}
}

// The axis: one tick, taken after both deadlines passed, must still deliver
// `poke` first and let `active`'s <cancel sendid="s1"> drop `settle`.
func TestCancelSurvivesATickAfterBothDeadlines(t *testing.T) {
	engine := started()
	if engine.GetCurrentState() != LateTickHonoursCancelStateWaiting {
		t.Fatalf("the machine should be waiting on its two delayed sends, got %v",
			engine.GetCurrentState())
	}

	time.Sleep(pastBothDeadlines)
	engine.Tick()

	if engine.GetCurrentState() == LateTickHonoursCancelStateCancelLost {
		t.Fatal("`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran " +
			"first. Both entries were past due when this tick started, so the scheduler " +
			"drain put them on the external queue together and the cancel found nothing " +
			"left to drop. W3C SCXML 6.3 cancels a send that has not been dispatched — " +
			"dispatch is one entry per macrostep, not one queue-flush per tick")
	}

	// The verdict is itself scheduler-driven, so a channel whose tick loop
	// stopped working fails here rather than passing by never moving.
	deadline := time.Now().Add(2 * time.Second)
	for !engine.IsInFinalState() && time.Now().Before(deadline) {
		time.Sleep(20 * time.Millisecond)
		engine.Tick()
	}
	if engine.GetCurrentState() != LateTickHonoursCancelStatePass {
		t.Fatalf("the machine did not reach `pass` after the cancel; it is in %v",
			engine.GetCurrentState())
	}
}

// A host that wakes between the two deadlines is the easy case, and it must
// keep working — the fix is about the late wake-up, not about changing what a
// punctual one does.
func TestPunctualHostReachesTheSameVerdict(t *testing.T) {
	engine := started()
	deadline := time.Now().Add(2 * time.Second)
	for !engine.IsInFinalState() && time.Now().Before(deadline) {
		time.Sleep(10 * time.Millisecond)
		engine.Tick()
	}
	if engine.GetCurrentState() != LateTickHonoursCancelStatePass {
		t.Fatalf("a 10 ms tick loop, which wakes between the 100 ms and 200 ms deadlines, "+
			"must reach `pass`; got %v", engine.GetCurrentState())
	}
}

// The deadline the host would have to guess is one the engine can state.
// RunUntilCompletion uses it, so an interval far coarser than the document's
// delays no longer decides the outcome.
func TestEngineSaysWhenItIsNextDue(t *testing.T) {
	engine := started()

	due, ok := engine.TimeUntilNextScheduled()
	if !ok {
		t.Fatal("two delayed sends are armed, so a deadline is owed")
	}
	if due > 100*time.Millisecond {
		t.Fatalf("the nearer of the two armed sends is 100 ms out; the engine answered %v, "+
			"which would send a host past the earlier deadline", due)
	}
	// The lower bound is the half that catches an answer of "due now", which
	// reads as a working query and costs the caller a spin that never sleeps.
	if due <= 0 {
		t.Fatal("the nearer send is 100 ms out and nothing is due yet, but the engine " +
			"answered 0. A host sleeping on that answer does not sleep at all")
	}

	// A poll interval coarser than either delay: with the deadline in hand this
	// is a ceiling on the wait, not the wait itself.
	startedAt := time.Now()
	if !engine.RunUntilCompletion(3*time.Second, 500*time.Millisecond) {
		t.Fatal("the machine did not complete within 3 s")
	}
	took := time.Since(startedAt)
	if engine.GetCurrentState() != LateTickHonoursCancelStatePass {
		t.Fatalf("a 500 ms poll interval decided the verdict (%v) — the wait must be "+
			"shortened to the scheduler's own next deadline, or a coarse interval "+
			"silently steps over the deadlines the document distinguishes between",
			engine.GetCurrentState())
	}
	// Correctness is not the whole of it: the document's own deadlines are
	// 100 ms + 100 ms, so an engine that sleeps the caller's interval regardless
	// finishes no sooner than 1 s. Timeliness is what the deadline query buys
	// once the dispatch order has made the verdict safe either way.
	if took > 450*time.Millisecond {
		t.Fatalf("the machine's own deadlines total 200 ms, and it took %v — the poll "+
			"interval was slept in full rather than shortened to the next deadline, so "+
			"every delayed event lands as late as the caller's guess", took)
	}

	if _, stillOwed := engine.TimeUntilNextScheduled(); stillOwed {
		t.Fatal("nothing is scheduled once the machine is finished, so no wake-up is owed")
	}
}
