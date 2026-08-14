// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a region that took an external self-transition still holds an
// atomic state, so it answers the next event too — Go AOT.
//
// Its sibling parallel_regions_take_own_transitions owns the microstep axis.
// This one owns what that axis cannot reach: a region can take its transition
// and run its content exactly as required and still be left holding no leaf,
// and the only thing that tells you so is a later event it fails to answer.
//
// Measured 2026-08-14 on the C++ AOT channel, where the defect lived: with the
// mutation `parallel_microstep_owns_exit_and_entry.cases` restoring it, the
// sibling fixture's driver stayed green and this fixture's went red.
//
// Fixture: integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_self_transition_keeps_its_leaf_go.sh

package parallel_self_transition_keeps_its_leaf

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func active(states []ParallelSelfTransitionKeepsItsLeafState, want ParallelSelfTransitionKeepsItsLeafState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestTheSelfTransitionedRegionAnswersTheNextEvent(t *testing.T) {
	policy := NewParallelSelfTransitionKeepsItsLeafPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture's <assign>s make this an ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ParallelSelfTransitionKeepsItsLeafState, ParallelSelfTransitionKeepsItsLeafEvent](&policy)
	engine.Initialize()

	entry := engine.GetActiveStates()
	if !active(entry, ParallelSelfTransitionKeepsItsLeafStateWithin) ||
		!active(entry, ParallelSelfTransitionKeepsItsLeafStateWorking) {
		t.Fatalf("fixture came up as %v; it is supposed to start with the self-transitioning "+
			"region in `within` and the deeper one in `working`, so nothing below is "+
			"testing what it claims", entry)
	}

	engine.RaiseExternal(ParallelSelfTransitionKeepsItsLeafEventE, "", "")
	engine.Step()

	// The symptom, named where it happens. A region holding no atomic state is
	// still "in" the parallel by every ancestor test.
	after := engine.GetActiveStates()
	if !active(after, ParallelSelfTransitionKeepsItsLeafStateWithin) {
		t.Errorf("the self-transitioning region lost its leaf on the first event (active: %v). "+
			"`within` is both the source and the target of its own external "+
			"self-transition, so the microstep exits and re-enters it; anything that "+
			"exits it a second time takes it back out and does not put it back", after)
	}
	if !active(after, ParallelSelfTransitionKeepsItsLeafStateJudging) {
		t.Errorf("the deeper region did not take its own transition on `e` (active: %v)", after)
	}

	// The second event is the one this fixture exists for: `judging` has no `e`
	// transition, so nothing but the self-transitioning region can answer, and
	// it can only answer from a leaf.
	engine.RaiseExternal(ParallelSelfTransitionKeepsItsLeafEventE, "", "")
	engine.Step()

	twice := engine.GetActiveStates()
	if !active(twice, ParallelSelfTransitionKeepsItsLeafStateWithin) {
		t.Errorf("the self-transitioning region is not in `within` after the second `e` "+
			"(active: %v)", twice)
	}

	engine.RaiseExternal(ParallelSelfTransitionKeepsItsLeafEventCheck, "", "")
	engine.Step()

	settled := engine.GetActiveStates()
	if !active(settled, ParallelSelfTransitionKeepsItsLeafStateSettled) {
		t.Errorf("`check` did not carry the machine to `settled` (active: %v), which the "+
			"document guards on `n == 1 && m == 2`. `m` reaches 2 only if the "+
			"self-transitioning region still had a leaf to transition from when the "+
			"second `e` arrived", settled)
	}
}
