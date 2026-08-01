// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: only a top-level <final> ends the session — Go AOT path.
//
// Appendix D enterStates sets running = false for a <final> only when
// isSCXMLElement(s.parent); otherwise it queues done.state.<parent> and the
// machine carries on. IsFinalState is therefore the structural question —
// "is this state a <final> element" — while Engine.IsInFinalState answers
// "has this session ended", and only the latter may gate completion, the
// completion callback, and the done.invoke.<id> a parent emits for this
// machine.
//
// The fixture rests in the nested final rather than passing through it: a
// machine that continues within the same macrostep is only ever sampled at
// the end, where a right and a wrong predicate agree.
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_nested_final_not_terminal_go.sh

package nested_final_not_terminal

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestANestedFinalDoesNotEndTheSession(t *testing.T) {
	policy := NewNestedFinalNotTerminalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[NestedFinalNotTerminalState, NestedFinalNotTerminalEvent](&policy)
	engine.Initialize()

	if got := engine.GetCurrentState(); got != NestedFinalNotTerminalStatePhaseDone {
		t.Fatalf("fixture came to rest in %v, want PhaseDone: it is supposed to stop "+
			"in the nested <final>, so nothing below is testing what it claims", got)
	}
	if engine.IsInFinalState() {
		t.Fatalf("the engine reported completion while resting in `phaseDone`, a " +
			"<final> nested inside `phase`. W3C SCXML Appendix D enterStates ends " +
			"the session only when the final's parent is the <scxml> element — a " +
			"nested one finishes its compound state and queues done.state.phase, " +
			"leaving the machine live. Completion must test the parent, not just " +
			"IsFinalState.")
	}

	engine.RaiseExternal(NestedFinalNotTerminalEventResume, "", "")
	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)

	if !completed {
		t.Fatalf("the machine did not complete after `resume` (parked in %v)",
			engine.GetCurrentState())
	}
	if got := engine.GetCurrentState(); got != NestedFinalNotTerminalStatePass {
		t.Fatalf("machine reached %v, want Pass: `resume` did not carry it out of the "+
			"nested final to the top-level one", got)
	}
}
