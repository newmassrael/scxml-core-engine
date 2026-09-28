// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — Go AOT path.
//
// Fixture: integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_delayed_send_reaches_what_its_target_names_go.sh
package a_delayed_send_reaches_what_its_target_names

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestADelayedSendReachesWhatItsTargetNames(t *testing.T) {
	policy := NewADelayedSendReachesWhatItsTargetNamesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ADelayedSendReachesWhatItsTargetNamesState, ADelayedSendReachesWhatItsTargetNamesEvent](&policy)
	engine.Initialize()

	if !engine.RunUntilCompletion(3*time.Second, 5*time.Millisecond) {
		t.Fatalf("the machine never completed (parked in %v)", engine.GetCurrentState())
	}
	if ended, ok := engine.TerminalState(); !ok || ended != ADelayedSendReachesWhatItsTargetNamesStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"order", policy.Order, 31},
		{"innerInternal", policy.InnerInternal, 1},
		{"lateOk", policy.LateOk, 1},
		{"lateCount", policy.LateCount, 1},
		{"pongOk", policy.PongOk, 1},
		{"commErrors", policy.CommErrors, 2},
		{"lostArrived", policy.LostArrived, 0},
		{"afterStranger", policy.AfterStranger, 0},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
