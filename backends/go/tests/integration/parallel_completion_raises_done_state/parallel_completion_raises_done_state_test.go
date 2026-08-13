// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: a <parallel> completing raises done.state.<id> — Go AOT.
//
// A <parallel> owns no <final> of its own; its finals sit one level down,
// inside the regions. A rule that registers the completion event by walking
// from a <final> to its direct parent therefore never reaches the parallel,
// while an emitter that raises it from the grandparent does — which is how the
// C++ and C11 channels ended up naming an enumerator nothing had declared.
//
// This channel is asked the behavioural half of the same question: both
// regions reaching their <final> on one event, in one microstep.
//
// Fixture: integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_completion_raises_done_state_go.sh

package parallel_completion_raises_done_state

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

func active(states []ParallelCompletionRaisesDoneStateState, want ParallelCompletionRaisesDoneStateState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestEveryRegionFinalCompletesTheParallel(t *testing.T) {
	policy := NewParallelCompletionRaisesDoneStatePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[ParallelCompletionRaisesDoneStateState, ParallelCompletionRaisesDoneStateEvent](&policy)
	engine.Initialize()

	entry := engine.GetActiveStates()
	if !active(entry, ParallelCompletionRaisesDoneStateStateA1) ||
		!active(entry, ParallelCompletionRaisesDoneStateStateB1) {
		t.Fatalf("fixture came up as %v; it is supposed to start with both regions "+
			"inside the <parallel>, so nothing below is testing what it claims", entry)
	}

	engine.RaiseExternal(ParallelCompletionRaisesDoneStateEventGo, "", "")
	engine.Step()

	after := engine.GetActiveStates()
	if !active(after, ParallelCompletionRaisesDoneStateStateA2) {
		t.Errorf("region `a` did not reach its <final> on `go` (active: %v)", after)
	}
	if !active(after, ParallelCompletionRaisesDoneStateStateB2) {
		t.Errorf("region `b` did not reach its <final> on `go` (active: %v)", after)
	}
}
