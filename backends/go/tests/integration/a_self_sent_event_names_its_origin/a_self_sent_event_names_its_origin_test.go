// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends to itself names its origin, and a
// target expression that evaluates to nothing reaches no one — Go AOT path.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_self_sent_event_names_its_origin_go.sh
package a_self_sent_event_names_its_origin

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestASelfSentEventNamesItsOrigin(t *testing.T) {
	policy := NewASelfSentEventNamesItsOriginPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ASelfSentEventNamesItsOriginState, ASelfSentEventNamesItsOriginEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	engine.AdvanceTimeMs(1000)
	engine.Step()

	if ended, ok := engine.TerminalState(); !ok || ended != ASelfSentEventNamesItsOriginStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"immediateOk", policy.ImmediateOk, 1},
		{"replied", policy.Replied, 1},
		{"delayedOk", policy.DelayedOk, 1},
		{"unreachable", policy.Unreachable, 1},
		{"strayed", policy.Strayed, 0},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
