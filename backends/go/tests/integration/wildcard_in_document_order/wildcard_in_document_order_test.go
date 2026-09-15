// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard keeps its own guard and its own type — Go AOT path.
//
// W3C SCXML 3.12.1 lets a "*" descriptor match every event, and that is all it
// changes: W3C SCXML 3.13 still asks the transition's cond whether it is
// enabled, and a type="internal" transition whose target is a proper
// descendant of its compound source still does not exit that source.
//
// No W3C document writes a wildcard with a cond or with type="internal", so a
// generator that moved the wildcard into a hand-written fallback dropped both
// without a suite noticing — the Kotlin one did until 2026-09-13.
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
// (canonical, shared with the C++ / C11 / Kotlin / Python / Rust channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_wildcard_in_document_order_go.sh

package wildcard_in_document_order

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAWildcardKeepsItsGuardAndItsType(t *testing.T) {
	policy := NewWildcardInDocumentOrderPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The guards and the entry counters need a datamodel, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[WildcardInDocumentOrderState, WildcardInDocumentOrderEvent](&policy)
	engine.Initialize()

	if completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond); !completed {
		t.Fatalf("the machine never reached a final state (parked in %v); resting in "+
			"GuardedFrom or SealedFrom means an internal wildcard was not taken at all",
			engine.GetCurrentState())
	}

	// Each failure final names the case, so this one check says which property
	// of the wildcard was lost:
	//   FailGuardIgnored              a wildcard fired with its guard false
	//   FailGuardNeverFired           a wildcard did not fire with its guard true
	//   FailGuardedInternalReentered  a guarded internal wildcard exited its source
	//   FailSealedInternalReentered   an unguarded internal wildcard exited its source
	if got := engine.GetCurrentState(); got != WildcardInDocumentOrderStatePass {
		t.Fatalf("the machine rested in %v: a wildcard is enabled only when its guard is "+
			"true, and an internal wildcard targeting a descendant of its compound source "+
			"must not exit and re-enter that source", got)
	}
}
