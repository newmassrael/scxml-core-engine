// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 6.2: a <send> delay is read as one CSS2 time — Go AOT path.
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_delay_is_a_css2_time_go.sh

package a_delay_is_a_css2_time

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// On a manual clock advanced a full minute: both valid delays fire in the order
// their milliseconds give, and a refused message, had it been scheduled under
// some default wait, would have arrived and moved `bad`.
func TestEachDelayIsReadAsOneCSS2Time(t *testing.T) {
	policy := NewADelayIsACss2TimePolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ADelayIsACss2TimeState, ADelayIsACss2TimeEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	engine.AdvanceTimeMs(60000)
	engine.Step()

	if ended, ok := engine.TerminalState(); !ok || ended != ADelayIsACss2TimeStateDone {
		t.Errorf("`a` must carry the run to `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"errors", policy.Errors, 2},
		{"after", policy.After, 0},
		{"bad", policy.Bad, 0},
		{"aAfterB", policy.AAfterB, 1},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
