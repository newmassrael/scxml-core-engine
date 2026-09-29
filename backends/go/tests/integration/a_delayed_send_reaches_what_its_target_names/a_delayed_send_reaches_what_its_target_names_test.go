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
	assertTheRoutes(t, engine, &policy)
}

// A host that ticks late — every 150ms of its own time, past the 100ms the
// delayed `lost` waits — gets the same answers. That tick brings `lost` due
// before it ticks the children, so `gone` must already have taken the `stop`
// sent to it at once: an event delivered to an invoked session is taken when
// it is delivered (W3C SCXML 6.4), and `lost` then finds `gone` ended and is
// reported (C.1). Measured 2026-09-29: before the child took its delivery at
// once, `lost` joined `gone`'s queue behind `stop` and vanished — commErrors 1,
// which the wall-clock run above showed only under heavy load.
func TestADelayedSendReachesWhatItsTargetNamesOnALateHost(t *testing.T) {
	policy := NewADelayedSendReachesWhatItsTargetNamesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ADelayedSendReachesWhatItsTargetNamesState, ADelayedSendReachesWhatItsTargetNamesEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()

	for i := 0; i < 20 && !engine.IsInFinalState(); i++ {
		engine.AdvanceTimeMs(150)
	}
	if !engine.IsInFinalState() {
		t.Fatalf("the machine never completed (parked in %v)", engine.GetCurrentState())
	}
	assertTheRoutes(t, engine, &policy)
}

func assertTheRoutes(
	t *testing.T,
	engine *sce.Engine[ADelayedSendReachesWhatItsTargetNamesState, ADelayedSendReachesWhatItsTargetNamesEvent],
	policy *ADelayedSendReachesWhatItsTargetNamesPolicy,
) {
	t.Helper()
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
