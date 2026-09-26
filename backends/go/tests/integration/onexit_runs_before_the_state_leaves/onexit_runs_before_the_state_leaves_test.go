// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D exitStates: a state leaves the configuration AFTER its
// own <onexit> has run — Go AOT path.
//
// The procedure is onexit, then cancelInvoke, then configuration.delete(s),
// for each state in exitOrder. Measured 2026-09-26, this channel removed the
// state from its configuration before cancelling its invocations and running
// the <onexit>, so In(s) inside s's own handler answered false. The handlers
// record what they saw and those records are the verdict.
//
// Fixture: integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_onexit_runs_before_the_state_leaves_go.sh

package onexit_runs_before_the_state_leaves

import (
	"fmt"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func active(states []OnexitRunsBeforeTheStateLeavesState, want OnexitRunsBeforeTheStateLeavesState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestAStateIsStillActiveWhileItsOwnOnexitRuns(t *testing.T) {
	policy := NewOnexitRunsBeforeTheStateLeavesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[OnexitRunsBeforeTheStateLeavesState, OnexitRunsBeforeTheStateLeavesEvent](&policy)
	engine.Initialize()

	entry := engine.GetActiveStates()
	if !active(entry, OnexitRunsBeforeTheStateLeavesStateInner) {
		t.Fatalf("fixture came up as %v; the run has to start inside `inner`", entry)
	}

	engine.RaiseExternal(OnexitRunsBeforeTheStateLeavesEventLeave, "", "")
	engine.Step()

	settled := engine.GetActiveStates()
	if !active(settled, OnexitRunsBeforeTheStateLeavesStateSettled) {
		// What the handlers recorded (W3C SCXML 5.3 readers): the final says
		// which clause broke, these say what the handler actually saw.
		read := func(v int64, ok bool) string {
			if !ok {
				return "<unreadable>"
			}
			return fmt.Sprint(v)
		}
		t.Logf("records: selfInInner=%s parentInInner=%s selfInOuter=%s childInOuter=%s exits=%s "+
			"(wanted 1 / 1 / 1 / 0 / 2)",
			read(policy.SelfInInner()), read(policy.ParentInInner()), read(policy.SelfInOuter()),
			read(policy.ChildInOuter()), read(policy.Exits()))
		t.Errorf("`leave` did not carry the machine to `settled` (active: %v). The document "+
			"checks its clauses in document order and lands each in a <final> of its own: "+
			"failExits (a handler did not run), failSelfInInner / failSelfInOuter (a state was "+
			"already out of the configuration during its own <onexit>), failParentInInner (the "+
			"parent left before its child's <onexit>), failChildInOuter (the child was still "+
			"active during its parent's <onexit>)", settled)
	}
}
