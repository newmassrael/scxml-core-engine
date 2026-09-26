// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: the value a srcexpr computes is the child that runs — Go
// AOT path, over the candidate set sce:candidates declares.
//
// The sibling fixture asks whether the expression is evaluated at all. This
// one asks whether its value means anything: until candidates existed, the
// child was fixed at build time and one stub answered whatever was computed
// (docs/SCE_ACCEPTED_SUBSET.md §2.13).
//
//	ran chosen     -> from.chosen     -> pass
//	ran other      -> from.other      -> wrongChild
//	loaded nothing -> error.execution -> noChild
//	ran a stub     -> no event at all -> parked in probe
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_invoke_candidate_selects_the_child_go.sh
package invoke_candidate_selects_the_child

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestTheEvaluatedValueSelectsWhichCandidateRuns(t *testing.T) {
	policy := NewInvokeCandidateSelectsTheChildPolicy()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[InvokeCandidateSelectsTheChildState, InvokeCandidateSelectsTheChildEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)

	if !completed {
		t.Fatalf("the machine never completed (parked in %v). Parking means no child "+
			"spoke: a stub ran, or nothing did", engine.GetCurrentState())
	}
	if got, ended := engine.TerminalState(); !ended || got != InvokeCandidateSelectsTheChildStatePass {
		t.Fatalf("machine reached %v, want Pass: WrongChild means the value selected "+
			"the other candidate, NoChild means nothing was loaded at all", got)
	}
}
