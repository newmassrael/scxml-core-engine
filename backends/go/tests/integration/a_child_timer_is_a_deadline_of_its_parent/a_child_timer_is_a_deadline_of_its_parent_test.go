// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A child session's <send delay> is a deadline of the machine that invoked it
// — Go AOT path.
//
// A host that drives a scheduler-owning machine asks the engine when it next
// needs a tick and sleeps that long. The parent's Tick advances every running
// child by the same step, so a moment a child needs is a moment the host must
// not step over. An answer that counts only the parent's own scheduler tells a
// host with nothing of the parent's armed that there is nothing to wait for,
// and the child's timer is never fired.
//
// Driven entirely on ManualClock: no case sleeps, and each move lands exactly
// on the deadline the engine reported, which is the use the answer exists for.
//
// Fixture: tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml.
// It is outside integration_resources/ for the reason
// scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_child_timer_is_a_deadline_of_its_parent_go.sh

package a_child_timer_is_a_deadline_of_its_parent

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

// started builds the machine on host-owned time. The clock is installed
// BEFORE Initialize: the child arms its first timer as it starts, and the
// engine refuses a clock afterwards because deadlines armed against one do not
// compare with another.
func started() *sce.Engine[AChildTimerIsADeadlineOfItsParentState, AChildTimerIsADeadlineOfItsParentEvent] {
	policy := NewAChildTimerIsADeadlineOfItsParentPolicy()
	engine := sce.NewEngine[AChildTimerIsADeadlineOfItsParentState, AChildTimerIsADeadlineOfItsParentEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	return engine
}

// The parent arms nothing, so the only deadline in the run is the child's first
// timer, 200 ms after the child started.
func TestTheEngineNamesItsChildsFirstTimerAsItsOwnNextDeadline(t *testing.T) {
	engine := started()
	if got := engine.GetCurrentState(); got != AChildTimerIsADeadlineOfItsParentStateWaiting {
		t.Fatalf("the parent should be waiting on its child; it is in %v", got)
	}

	due, ok := engine.TimeUntilNextScheduled()
	if !ok {
		t.Fatal("the child armed <send delay=\"200ms\"> when it started and the parent ticks the " +
			"child, so that deadline is the parent's. An answer of \"nothing owed\" tells a host " +
			"there is nothing to wait for while a running child's timer is pending")
	}
	if due != 200*time.Millisecond {
		t.Fatalf("the child's first timer is 200 ms out; the engine answered %v", due)
	}
}

// The child arms its second timer when the first fires, so the whole run is
// two moves of 200 ms and the engine has to name the second one only after the
// first has been taken.
func TestAHostWalkingTimeByTheAnswerReachesTheEndOfTheChild(t *testing.T) {
	engine := started()

	var walked []time.Duration
	for {
		due, ok := engine.TimeUntilNextScheduled()
		if !ok {
			break
		}
		if len(walked) >= 8 {
			t.Fatalf("the engine keeps naming deadlines: %v", walked)
		}
		walked = append(walked, due)
		engine.AdvanceTimeMs(due.Milliseconds())
	}

	want := []time.Duration{200 * time.Millisecond, 200 * time.Millisecond}
	if len(walked) != len(want) || walked[0] != want[0] || walked[1] != want[1] {
		t.Fatalf("each move should land on the child's next timer, the second of which is armed by the "+
			"first firing; the host walked %v, want %v", walked, want)
	}
	if got, ended := engine.TerminalState(); !ended || got != AChildTimerIsADeadlineOfItsParentStateFinished {
		t.Fatalf("the child's last timer ended it, so the parent should have taken done.invoke.kid and "+
			"finished; it is in %v (ended=%v)", engine.GetCurrentState(), ended)
	}
	if due, ok := engine.TimeUntilNextScheduled(); ok {
		t.Fatalf("nothing is armed once the child has ended; the engine answered %v", due)
	}
}
