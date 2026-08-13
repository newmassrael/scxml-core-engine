// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: every region of a <parallel> takes its own enabled
// transition in the same microstep — Go AOT.
//
// The fixture is asymmetric on purpose. One region's transition on the event
// is an external self-transition, whose domain Appendix D resolves through
// findLCCA over the proper ancestors — candidates that never include the state
// itself. Answering with the state left the exit-set walk without a stopping
// point, so it ran to the document root, the exit set named the enclosing
// <parallel>, and conflict resolution preempted the deeper region's transition
// on that same event.
//
// The observable is `settled`, which the document reaches only when both
// regions' assignments have run — a configuration check alone would still pass
// for a region that moved without executing its transition content.
//
// Fixture: integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_regions_take_own_transitions_go.sh

package parallel_regions_take_own_transitions

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func active(states []ParallelRegionsTakeOwnTransitionsState, want ParallelRegionsTakeOwnTransitionsState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestEveryRegionTakesItsOwnTransition(t *testing.T) {
	policy := NewParallelRegionsTakeOwnTransitionsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture's <assign>s make this an ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ParallelRegionsTakeOwnTransitionsState, ParallelRegionsTakeOwnTransitionsEvent](&policy)
	engine.Initialize()

	entry := engine.GetActiveStates()
	if !active(entry, ParallelRegionsTakeOwnTransitionsStateWorking) ||
		!active(entry, ParallelRegionsTakeOwnTransitionsStateWithin) {
		t.Fatalf("fixture came up as %v; it is supposed to start with the deeper region "+
			"in `working` and the shallower one in `within`, so nothing below is "+
			"testing what it claims", entry)
	}

	engine.RaiseExternal(ParallelRegionsTakeOwnTransitionsEventE, "", "")
	engine.Step()

	after := engine.GetActiveStates()
	if !active(after, ParallelRegionsTakeOwnTransitionsStateJudging) {
		t.Errorf("the deeper region lost its leaf (active: %v). W3C SCXML 3.4 has every "+
			"region take its own enabled transition on `e`; the sibling region's "+
			"external self-transition must not preempt this one", after)
	}
	if !active(after, ParallelRegionsTakeOwnTransitionsStateWithin) {
		t.Errorf("the shallower region left `within`, which is both the source and the "+
			"target of its own external self-transition (active: %v)", after)
	}

	engine.RaiseExternal(ParallelRegionsTakeOwnTransitionsEventCheck, "", "")
	engine.Step()

	settled := engine.GetActiveStates()
	if !active(settled, ParallelRegionsTakeOwnTransitionsStateSettled) {
		t.Errorf("`check` did not carry the machine to `settled` (active: %v), which the "+
			"document guards on both regions' assignments having run. Reaching "+
			"`judging` without `n == 1 && m == 1` means a region changed state while "+
			"its transition content was skipped", settled)
	}
}
