// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: done.state.<parallel> is delivered, not merely
// declared — Go AOT.
//
// The sibling fixture parallel_completion_raises_done_state carries no
// listener, deliberately: a transition's `event` attribute is itself a
// registration site, so a listener there would register the completion event
// no matter what the <final> walk does and leave that fixture unable to fail
// for the defect it exists to catch. What it proves is that the event is
// DECLARED.
//
// Declared is not delivered. A backend that names the event and never raises
// it — or raises it where nothing selects from — passes there. This document
// listens, and `settled` is a top-level <final> no other route reaches.
//
// Fixture: integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_done_state_is_delivered_go.sh

package parallel_done_state_is_delivered

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

func active(states []ParallelDoneStateIsDeliveredState, want ParallelDoneStateIsDeliveredState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestCompletionCarriesTheMachineToATopLevelFinal(t *testing.T) {
	policy := NewParallelDoneStateIsDeliveredPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[ParallelDoneStateIsDeliveredState, ParallelDoneStateIsDeliveredEvent](&policy)
	engine.Initialize()

	entry := engine.GetActiveStates()
	if !active(entry, ParallelDoneStateIsDeliveredStateA1) ||
		!active(entry, ParallelDoneStateIsDeliveredStateB1) {
		t.Fatalf("fixture came up as %v; it is supposed to start with both regions "+
			"inside the <parallel>, so nothing below is testing what it claims", entry)
	}

	engine.RaiseExternal(ParallelDoneStateIsDeliveredEventGo, "", "")
	engine.Step()

	// One check, with the configuration in its message, because the two ways
	// this can fail are not separately observable: completion is selected
	// within the SAME macrostep as the regions' finals, so once Step returns
	// the parallel has been exited and A2/B2 are gone. Measured — checking
	// them as a precondition failed against engines that had already done the
	// right thing.
	//
	// The remaining states tell the two apart: A1/B1 means `go` moved nothing;
	// A2/B2 means the parallel completed and the event went nowhere.
	after := engine.GetActiveStates()
	if !active(after, ParallelDoneStateIsDeliveredStateSettled) {
		t.Errorf("every region reaching its <final> completes the parallel, so "+
			"done.state.run had to be raised AND selected — `settled` is reachable "+
			"by nothing else (active: %v)", after)
	}
}
