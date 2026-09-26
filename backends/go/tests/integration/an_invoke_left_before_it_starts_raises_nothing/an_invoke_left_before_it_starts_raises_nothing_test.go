// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never
// attempted, so the §6.4.1 error.execution for a type no processor runs is
// never raised — Go AOT path.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Fixture: integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_an_invoke_left_before_it_starts_raises_nothing_go.sh

package an_invoke_left_before_it_starts_raises_nothing

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestTheLeftStatesInvokeIsNeverAttempted(t *testing.T) {
	policy := NewAnInvokeLeftBeforeItStartsRaisesNothingPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnInvokeLeftBeforeItStartsRaisesNothingState, AnInvokeLeftBeforeItStartsRaisesNothingEvent](&policy)
	engine.Initialize()
	if engine.GetCurrentState() != AnInvokeLeftBeforeItStartsRaisesNothingStateS1 {
		t.Fatalf("`s0` leaves on an eventless transition within the first macrostep")
	}

	engine.RaiseExternal(AnInvokeLeftBeforeItStartsRaisesNothingEventFinish, "", "")
	engine.Step()

	if ended, ok := engine.TerminalState(); !ok || ended != AnInvokeLeftBeforeItStartsRaisesNothingStateDone {
		t.Errorf("`finish` must carry the run to `done`")
	}
	if v, ok := policy.Errors(); !ok || v != 0 {
		t.Errorf("`s0` left before its macrostep ended, yet its invoke was attempted and raised "+
			"error.execution (errors=%d, readable=%v); W3C SCXML 6.4 cancels an invoke whose state has left", v, ok)
	}
}
