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

// ═══════════════════════════════════════════════════════════════════════════
// §scxml-6.2.2 — the clock the deadlines are measured from
//
// Everything above drives the machine on the wall clock, which is what a
// production host does and what the push runs. It is also why this document
// reached a push before it reached a test: the two <send delay>s in `waiting`
// were armed against two separate readings, so a host descheduled between them
// by more than the 100 ms separating their delays got the later send's deadline
// first. The cases below take the clock away from the machine the suite runs on
// and hand it to the test, so the verdict is about the engine.
// ═══════════════════════════════════════════════════════════════════════════

// steppingClock jumps forward on every reading.
//
// This is what a descheduled host looks like from inside the engine: two
// readings taken for what the document calls one instant come back different.
// A real one does it unpredictably and only under load, which is why the defect
// it exposes reached a push before it reached a test; this one does it on every
// reading, so the cases below are a verdict about the engine rather than about
// the machine the suite runs on.
type steppingClock struct {
	nowMs    int64
	stepMs   int64
	readings int
}

func (c *steppingClock) ElapsedMs() int64 {
	c.readings++
	c.nowMs += c.stepMs
	return c.nowMs
}

func startedOn(clock sce.SceClock) *sce.Engine[LateTickHonoursCancelState, LateTickHonoursCancelEvent] {
	policy := NewLateTickHonoursCancelPolicy()
	engine := sce.NewEngine[LateTickHonoursCancelState, LateTickHonoursCancelEvent](&policy)
	engine.SetClock(clock)
	engine.Initialize()
	return engine
}

// The axis of this round: a host descheduled between the fixture's two
// <send delay>s must not change which of them fires first.
//
// Swept rather than pinned to one value. The threshold is arithmetic — the
// stall has to reach the 100 ms separating the two delays before the later
// deadline can overtake the earlier one — and a case pinned at one stall would
// pass for a fix that moved the threshold instead of removing it. Measured on
// the pre-latch engine: 1, 50 and 99 pass, and 100 is the first failure.
func TestAHostDescheduledBetweenTwoSendsKeepsTheirOrder(t *testing.T) {
	for _, stallMs := range []int64{1, 50, 99, 100, 101, 150, 1000} {
		clock := &steppingClock{stepMs: stallMs}
		engine := startedOn(clock)

		if engine.GetCurrentState() == LateTickHonoursCancelStateCancelLost {
			t.Fatalf("a host stalled %d ms between the two <send delay>s of one "+
				"<onentry> reordered them: `settle` (200 ms) came due before `poke` "+
				"(100 ms) because each send took its own reading. §scxml-6.2.2 makes "+
				"a delay the wait the DOCUMENT asks for, and the time the host spent "+
				"descheduled is not part of it", stallMs)
		}

		// One tick is one reading, so time moves stallMs per tick and the
		// smallest stall in the sweep needs a few hundred of them to cross the
		// document's 200 ms of deadlines.
		for i := 0; i < 4096 && !engine.IsInFinalState(); i++ {
			engine.Tick()
		}
		if engine.GetCurrentState() != LateTickHonoursCancelStatePass {
			t.Fatalf("with a %d ms stall per clock reading the machine ended in %v; "+
				"the document's <cancel sendid=\"s1\"> must still drop `settle`",
				stallMs, engine.GetCurrentState())
		}
	}
}

// A tick dispatches what was due when the host called it — not what its own
// slowness made due while it ran.
//
// Counted rather than inferred: the stall here (150 ms) is larger than every
// delay in the document, so an engine re-reading per pass would run the whole
// machine inside one tick.
func TestATickReadsTheClockOnceHoweverMuchItDoes(t *testing.T) {
	clock := &steppingClock{stepMs: 150}
	engine := startedOn(clock)
	if clock.readings != 1 {
		t.Fatalf("Initialize() is one turn and must take one reading; it took %d",
			clock.readings)
	}

	engine.Tick()
	if clock.readings != 2 {
		t.Fatalf("Tick() is one turn and must take one reading; the run has taken %d "+
			"in total. A tick that re-reads the clock while it works extends its own "+
			"window and dispatches entries the host has not yet reached", clock.readings)
	}
}

// The host-owned clock: the same generated machine, driven by AdvanceTimeMs,
// reaches its verdict on the test's schedule.
//
// This is the contract the Python channel has had all along (advance_time /
// now_ms). A machine driven this way has no dependency on the load of the build
// machine at all.
func TestAManualClockDrivesTheMachineToTheSameVerdict(t *testing.T) {
	engine := startedOn(sce.NewManualClock(0))
	if engine.GetCurrentState() != LateTickHonoursCancelStateWaiting {
		t.Fatalf("nothing is due at t=0, so the machine waits on its two delayed "+
			"sends; it is in %v", engine.GetCurrentState())
	}

	// Past both deadlines in one move — the late wake-up the fixture is about.
	engine.AdvanceTimeMs(400)
	if engine.GetCurrentState() == LateTickHonoursCancelStateCancelLost {
		t.Fatal("a single 400 ms advance stepped over both deadlines; `poke` must " +
			"still be dispatched first so `active`'s <cancel sendid=\"s1\"> can drop " +
			"`settle`")
	}

	engine.AdvanceTimeMs(100)
	if engine.GetCurrentState() != LateTickHonoursCancelStatePass {
		t.Fatalf("`finish` is armed for 100 ms after `active` is entered, so the "+
			"machine should be done; it is in %v", engine.GetCurrentState())
	}
	if engine.NowMs() != 500 {
		t.Fatalf("the host moved this clock 400 + 100 ms and nothing else may move "+
			"it; it reads %d", engine.NowMs())
	}
}

// Determinism is the point, so it is asserted as such: the same call sequence
// twice, and the intermediate states compared rather than only the verdict.
//
// The wall-clock cases above cannot make this assertion — they would be
// re-measuring the load on the build machine, which is exactly the dependency
// this seam removes.
func TestAManualClockRunRepeatsExactly(t *testing.T) {
	trace := func() []LateTickHonoursCancelState {
		engine := startedOn(sce.NewManualClock(0))
		seen := []LateTickHonoursCancelState{engine.GetCurrentState()}
		for i := 0; i < 6; i++ {
			engine.AdvanceTimeMs(100)
			seen = append(seen, engine.GetCurrentState())
		}
		return seen
	}

	first, second := trace(), trace()
	if len(first) != len(second) {
		t.Fatalf("two identical sequences produced traces of different lengths: %v vs %v",
			first, second)
	}
	sawPass := false
	for i := range first {
		if first[i] != second[i] {
			t.Fatalf("two identical sequences of AdvanceTimeMs produced different "+
				"traces: %v vs %v. A host-owned clock that is not reproducible is not "+
				"host-owned", first, second)
		}
		if first[i] == LateTickHonoursCancelStateCancelLost {
			t.Fatalf("the trace reached `cancelLost`: %v", first)
		}
		if first[i] == LateTickHonoursCancelStatePass {
			sawPass = true
		}
	}
	if !sawPass {
		t.Fatalf("the trace never reached `pass`: %v", first)
	}
}

// One generated artifact, two kinds of host: the same policy runs on the wall
// clock and on host-owned time and lands in the same configuration.
func TestOneGeneratedMachineServesBothKindsOfHost(t *testing.T) {
	wall := started()
	deadline := time.Now().Add(2 * time.Second)
	for !wall.IsInFinalState() && time.Now().Before(deadline) {
		time.Sleep(10 * time.Millisecond)
		wall.Tick()
	}

	hostOwned := startedOn(sce.NewManualClock(0))
	for i := 0; i < 8 && !hostOwned.IsInFinalState(); i++ {
		hostOwned.AdvanceTimeMs(100)
	}

	if wall.GetCurrentState() != hostOwned.GetCurrentState() {
		t.Fatalf("the same generated machine reached different configurations on the "+
			"wall clock (%v) and on a host-owned clock (%v)",
			wall.GetCurrentState(), hostOwned.GetCurrentState())
	}
	if hostOwned.GetCurrentState() != LateTickHonoursCancelStatePass {
		t.Fatalf("both hosts should reach `pass`; got %v", hostOwned.GetCurrentState())
	}
}

// AdvanceTimeMs on a clock the host does not own is a programming error, not a
// no-op: the caller believes it owns time and it does not, so the events it is
// waiting for would arrive on a schedule it did not choose.
func TestAdvanceTimeMsRefusesAClockTheHostDoesNotOwn(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("AdvanceTimeMs on the monotonic default returned quietly; a host " +
				"that believes it owns time and does not must be told")
		}
	}()
	started().AdvanceTimeMs(100)
}

// The clock is installed before the machine arms anything against it. Swapping
// it afterwards would leave the scheduler holding deadlines computed from two
// incomparable time bases — `waiting`'s <onentry> has already armed both sends
// by the time Initialize returns.
func TestTheClockCannotBeSwappedAfterTheMachineArmedItsDeadlines(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("SetClock after Initialize was accepted; the queue would then hold " +
				"deadlines from two clocks that do not compare")
		}
	}()
	started().SetClock(sce.NewManualClock(0))
}

// A host on the wall clock still gets an absolute reading, so it can correlate
// the engine's deadlines with its own log.
func TestNowMsAnswersOnEveryKindOfClock(t *testing.T) {
	wall := started()
	a := wall.NowMs()
	time.Sleep(20 * time.Millisecond)
	if b := wall.NowMs(); b < a {
		t.Fatalf("the wall clock went backwards between two readings: %d then %d", a, b)
	}

	manual := startedOn(sce.NewManualClock(7))
	if manual.NowMs() != 7 {
		t.Fatalf("a manual clock reads exactly what the host set, and Initialize() "+
			"must not have moved it; it reads %d", manual.NowMs())
	}
}
