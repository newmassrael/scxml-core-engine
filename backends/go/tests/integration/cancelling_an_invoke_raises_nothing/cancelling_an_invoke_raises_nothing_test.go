// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 6.4: cancelling an invocation raises no event in the invoking
// session — Go AOT path.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Fixture: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_cancelling_an_invoke_raises_nothing_go.sh

package cancelling_an_invoke_raises_nothing

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestLeavingTheInvokingStateRaisesNoCancelEvent(t *testing.T) {
	policy := NewCancellingAnInvokeRaisesNothingPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[CancellingAnInvokeRaisesNothingState, CancellingAnInvokeRaisesNothingEvent](&policy)
	engine.Initialize()
	if engine.GetCurrentState() != CancellingAnInvokeRaisesNothingStateP {
		t.Fatalf("the run has to start in `p`, with its child invoked")
	}

	for _, event := range []CancellingAnInvokeRaisesNothingEvent{
		CancellingAnInvokeRaisesNothingEventLeave, CancellingAnInvokeRaisesNothingEventFinish,
	} {
		engine.RaiseExternal(event, "", "")
		engine.Step()
	}

	if ended, ok := engine.TerminalState(); !ok || ended != CancellingAnInvokeRaisesNothingStateDone {
		t.Errorf("`finish` must carry the run to `done`")
	}
	if v, ok := policy.Spurious(); !ok || v != 0 {
		t.Errorf("leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking "+
			"session (spurious=%d, readable=%v); W3C SCXML 6.4 defines no such event", v, ok)
	}
}
