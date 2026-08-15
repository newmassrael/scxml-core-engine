// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: a state entered only because the target lies
// inside it takes no default child — Go AOT.
//
// Appendix D asks two different questions with two functions.
// addDescendantStatesToEnter gives a compound state its default child and is
// called for the transition's TARGET; addAncestorStatesToEnter walks the states
// between the target and the LCCA and adds them WITHOUT defaults. Its one
// exception is a parallel ancestor, whose other regions do get theirs.
//
// Measured 2026-08-15 on the worked example examples/ai_loop/ai_loop.scxml,
// where the wrongly-entered state's <onentry> sends a prompt: the supervised
// session was re-sent its opening prompt every time a person answered a dialog.
//
// The document is driven twice on purpose. `cross` enters the <parallel>
// itself, so `run` is a parallel ancestor and `drive`/`outer` are compound
// ones; `again` runs with the parallel already active, so only `outer` is
// entered. Those are different branches of the generated entry walk.
//
// Fixture: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_ancestor_entry_is_not_default_entry_go.sh

package ancestor_entry_is_not_default_entry

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func active(states []AncestorEntryIsNotDefaultEntryState, want AncestorEntryIsNotDefaultEntryState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestAnAncestorEnteredOnTheWayToATargetTakesNoDefaultChild(t *testing.T) {
	policy := NewAncestorEntryIsNotDefaultEntryPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture counts entries with <assign>, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AncestorEntryIsNotDefaultEntryState, AncestorEntryIsNotDefaultEntryEvent](&policy)
	engine.Initialize()

	entry := engine.GetActiveStates()
	if !active(entry, AncestorEntryIsNotDefaultEntryStateAway) {
		t.Fatalf("fixture came up as %v; the run has to start OUTSIDE the <parallel> for the "+
			"first pass to be testing anything — a source already inside it leaves the "+
			"ancestors active and the entry chain never reaches their defaults", entry)
	}

	// Pass one: the parallel is not active, so `run` is entered as a parallel
	// ancestor and `drive` and `outer` as compound ones.
	engine.RaiseExternal(AncestorEntryIsNotDefaultEntryEventCross, "", "")
	engine.Step()

	crossed := engine.GetActiveStates()
	if !active(crossed, AncestorEntryIsNotDefaultEntryStateChosen) {
		t.Errorf("the transition named `chosen` and the machine did not enter it (active: %v)", crossed)
	}
	if active(crossed, AncestorEntryIsNotDefaultEntryStateByDefault) {
		t.Errorf("`outer` has two children active at once (active: %v). `by_default` is what "+
			"`initial` names, and nothing targeted it — it was entered because the engine "+
			"gave `outer` its default child while entering `outer` merely as an ancestor "+
			"of `chosen`", crossed)
	}
	if !active(crossed, AncestorEntryIsNotDefaultEntryStateIdle) {
		t.Errorf("the region no entering state is inside must still be entered with its "+
			"default (active: %v) — Appendix D's one exception for a parallel ancestor", crossed)
	}

	// Pass two: the parallel is already active now, so `run` and `drive` are
	// skipped and only `outer` is entered. That is a different branch of the
	// entry walk, and it is the one a running machine takes.
	engine.RaiseExternal(AncestorEntryIsNotDefaultEntryEventBack, "", "")
	engine.Step()
	engine.RaiseExternal(AncestorEntryIsNotDefaultEntryEventAgain, "", "")
	engine.Step()

	again := engine.GetActiveStates()
	if active(again, AncestorEntryIsNotDefaultEntryStateByDefault) {
		t.Errorf("`outer` took its default child on the second pass (active: %v), where the "+
			"<parallel> was already active and only `outer` itself was entered — the shape "+
			"the worked example hits every time a person answers a dialog", again)
	}

	engine.RaiseExternal(AncestorEntryIsNotDefaultEntryEventCheck, "", "")
	engine.Step()

	settled := engine.GetActiveStates()
	if !active(settled, AncestorEntryIsNotDefaultEntryStateSettled) {
		t.Errorf("`check` did not carry the machine to `settled` (active: %v). The document "+
			"checks its four clauses in document order and lands each in a <final> of its "+
			"own, so the configuration above names which one broke: failDefaulted, "+
			"failLobbied, failIdled, failTargeted", settled)
	}
}
