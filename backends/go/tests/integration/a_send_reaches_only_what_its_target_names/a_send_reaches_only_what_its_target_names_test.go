// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a <send> reaches what its target names, carries
// its payload there, and a target that names nothing reachable is reported —
// Go AOT path.
//
// Fixture: integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_send_reaches_only_what_its_target_names_go.sh
package a_send_reaches_only_what_its_target_names

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// W3C SCXML 6.2.4: this document sends to #_parent, so a host that runs
// machines as roots and asks for the refusal gets it — and nothing starts.
// The plain Initialize below runs the same machine; the refusal is opt-in.
func TestARootStartOfAMachineThatNeedsAParentIsRefused(t *testing.T) {
	policy := NewASendReachesOnlyWhatItsTargetNamesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ASendReachesOnlyWhatItsTargetNamesState, ASendReachesOnlyWhatItsTargetNamesEvent](&policy)
	if refusal := engine.RootStartRefusal(); refusal != sce.RootStartNeedsParent {
		t.Fatalf("RootStartRefusal = %v, want RootStartNeedsParent", refusal)
	}
	if err := engine.InitializeAsRoot(); err != sce.RootStartNeedsParent {
		t.Fatalf("InitializeAsRoot = %v, want RootStartNeedsParent", err)
	}
	if engine.IsRunning() {
		t.Errorf("a refused root start must start nothing")
	}
	if got := sce.RootStartNeedsParent.Error(); got != "the machine sends to #_parent and was started with no parent session" {
		t.Errorf("reason = %q", got)
	}
}

func TestASendReachesOnlyWhatItsTargetNames(t *testing.T) {
	policy := NewASendReachesOnlyWhatItsTargetNamesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ASendReachesOnlyWhatItsTargetNamesState, ASendReachesOnlyWhatItsTargetNamesEvent](&policy)
	engine.Initialize()

	if !engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond) {
		t.Fatalf("the machine never completed (parked in %v)", engine.GetCurrentState())
	}
	if ended, ok := engine.TerminalState(); !ok || ended != ASendReachesOnlyWhatItsTargetNamesStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"execErrors", policy.ExecErrors, 1},
		{"commErrors", policy.CommErrors, 5},
		{"afterRefused", policy.AfterRefused, 0},
		{"afterNobody", policy.AfterNobody, 0},
		{"afterStranger", policy.AfterStranger, 0},
		{"afterOrphan", policy.AfterOrphan, 0},
		{"afterOrphanExpr", policy.AfterOrphanExpr, 0},
		{"afterOrphanExprLater", policy.AfterOrphanExprLater, 0},
		{"bareArrived", policy.BareArrived, 1},
		{"pongOk", policy.PongOk, 1},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
