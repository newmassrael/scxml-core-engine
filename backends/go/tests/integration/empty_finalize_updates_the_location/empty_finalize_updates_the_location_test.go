// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.5.2 — what an EMPTY `<finalize>` does, and what an absent one
// does not. Go AOT.
//
// With no executable content the Processor "MUST update the data model each
// time an event is received from the child process ... for each item in the
// 'namelist' attribute and each such <param> element ... as if by <assign>
// with any return value that has a name that matches", and then: "Note that
// the automatic update does not take place if the <finalize> element is
// absent as opposed to empty."
//
// The corpus holds two <finalize> documents (W3C 233/234) and zero empty
// ones, and measured 2026-08-22 no channel implemented the automatic update:
// every engine gates the finalize step on the content being non-empty, and
// the AOT model had no way to tell an empty element from a missing one.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
//
// Regeneration:
//   scripts/regen_empty_finalize_updates_the_location_go.sh

package empty_finalize_updates_the_location

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAnEmptyFinalizeUpdatesTheLocationAndAnAbsentOneDoesNot(t *testing.T) {
	policy := NewEmptyFinalizeUpdatesTheLocationPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[EmptyFinalizeUpdatesTheLocationState, EmptyFinalizeUpdatesTheLocationEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(15*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("empty_finalize_updates_the_location timed out before reaching a " +
			"final state — even the delayed timeouts that judge a silent child " +
			"never fired, so the machine is not being ticked")
	}

	switch got := engine.GetCurrentState(); got {
	case EmptyFinalizeUpdatesTheLocationStatePass:
	case EmptyFinalizeUpdatesTheLocationStateFailNotUpdated:
		t.Fatalf("the empty `<finalize/>` left `tally` at its old value: W3C SCXML " +
			"6.5.2 makes an empty element mean the automatic update — for each " +
			"namelist item the Processor updates the location as if by <assign> " +
			"with the matching return value.")
	case EmptyFinalizeUpdatesTheLocationStateFailUpdatedWithoutFinalize:
		t.Fatalf("`guard` moved with no <finalize> element at all: the clause's note " +
			"is a prohibition — \"the automatic update does not take place if the " +
			"<finalize> element is absent as opposed to empty\".")
	case EmptyFinalizeUpdatesTheLocationStateFailUnmatchedNameWrote:
		t.Fatalf("an event carrying no matching name still wrote `keeper`: W3C SCXML " +
			"6.5.2 says \"with ANY return value that has a name that matches\", so an " +
			"unconditional write blanks the parent's data model on every unrelated " +
			"answer the child sends.")
	case EmptyFinalizeUpdatesTheLocationStateFailUnmatchedChildSilent:
		t.Fatalf("the third child never answered, so the guarded-write half was never " +
			"exercised.")
	case EmptyFinalizeUpdatesTheLocationStateFailEmptyChildSilent:
		t.Fatalf("the first child never answered, so the empty-<finalize> half was " +
			"never exercised.")
	case EmptyFinalizeUpdatesTheLocationStateFailAbsentChildSilent:
		t.Fatalf("the second child never answered, so the absent-<finalize> half was " +
			"never exercised.")
	default:
		t.Fatalf("empty_finalize_updates_the_location settled in %v, which is not a "+
			"verdict state", got)
	}
}
