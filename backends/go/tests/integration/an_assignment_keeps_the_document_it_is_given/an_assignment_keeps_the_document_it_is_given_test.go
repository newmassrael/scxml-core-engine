// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.4 and B.2: `<assign location="v" expr="_event.data"/>` leaves
// `v` holding the DOM a `<send>`'s XML `<content>` arrived as, and so does a
// member-path location — Go AOT.
//
// The template runs `<location> = <expr>` in the Lua engine for every
// location, so the value never leaves the engine; the C++ engines once read a
// bare-name location's value out and wrote it back, which lost the document.
//
// Fixture: integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_assignment_keeps_the_document_it_is_given_go.sh

package an_assignment_keeps_the_document_it_is_given

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestBothLocationsHoldTheDocumentTheEventCarried(t *testing.T) {
	policy := NewAnAssignmentKeepsTheDocumentItIsGivenPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnAssignmentKeepsTheDocumentItIsGivenState, AnAssignmentKeepsTheDocumentItIsGivenEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("an_assignment_keeps_the_document_it_is_given timed out before reaching a " +
			"final state: its own `doc` event never arrived")
	}

	got, ended := engine.TerminalState()
	if !ended {
		t.Fatalf("an_assignment_keeps_the_document_it_is_given did not end in a top-level "+
			"<final> (parked in %v)", engine.GetCurrentState())
	}
	switch got {
	case AnAssignmentKeepsTheDocumentItIsGivenStateKept:
	case AnAssignmentKeepsTheDocumentItIsGivenStateNameLost:
		t.Fatalf("`<assign location=\"v\" expr=\"_event.data\"/>` did not leave the document " +
			"in `v`: the value was read out of the data model and written back.")
	case AnAssignmentKeepsTheDocumentItIsGivenStateMemberLost:
		t.Fatalf("`<assign location=\"holder.doc\" expr=\"_event.data\"/>` did not leave the " +
			"document in `holder.doc`.")
	default:
		t.Fatalf("an_assignment_keeps_the_document_it_is_given ended in %v, which is not a verdict state", got)
	}
}
