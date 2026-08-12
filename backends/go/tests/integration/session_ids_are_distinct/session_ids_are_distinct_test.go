// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10: `_sessionid` is the id of a session - Go AOT.
//
// The clause binds `_sessionid` to "the system-generated id for the current
// SCXML session", and Appendix C.1.1 derives the address a session publishes
// from that id. Two live sessions holding one id publish one address, so a
// `<send>` addressed to either reaches both.
//
// No test in the public IRP corpus can ask: every one that reaches
// `_sessionid` runs a single session, so a processor that hands the same
// value to every session it starts passes them all.
//
// The fixture runs two children at once, each reporting the id it was
// issued, and the parent compares them.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// (canonical, shared with every other channel).
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_session_ids_are_distinct_go.sh
package session_ids_are_distinct

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestTwoLiveSessionsAreIssuedDifferentIds(t *testing.T) {
	policy := NewSessionIdsAreDistinctPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[SessionIdsAreDistinctState, SessionIdsAreDistinctEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf(
			"session_ids_are_distinct timed out parked in %v: only one child reported its `_sessionid`, so the two ids were never compared.",
			engine.GetCurrentState(),
		)
	}

	switch got := engine.GetCurrentState(); got {
	case SessionIdsAreDistinctStatePass:
	case SessionIdsAreDistinctStateFail:
		t.Fatalf("two live sessions reported the same `_sessionid`. W3C SCXML 5.10 binds it to the id of the current session, and C.1.1 publishes an address derived from it, so one id for two sessions is one address for two sessions.")
	default:
		t.Fatalf("session_ids_are_distinct settled in %v, which is not a verdict state", got)
	}
}
