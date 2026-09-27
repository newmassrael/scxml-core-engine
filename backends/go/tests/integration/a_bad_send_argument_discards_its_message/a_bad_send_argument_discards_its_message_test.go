// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 6.2 + 4.9: a <send> whose own argument cannot be evaluated raises
// error.execution, discards the message, and ends its block — Go AOT path.
//
// Fixture: integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_bad_send_argument_discards_its_message_go.sh

package a_bad_send_argument_discards_its_message

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// On a manual clock advanced a full minute before `finish`: a channel that
// scheduled the message whose `delayexpr` failed, under some default delay,
// delivers it within that minute and moves `sent`.
func TestEachBadArgumentDiscardsItsMessage(t *testing.T) {
	policy := NewABadSendArgumentDiscardsItsMessagePolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ABadSendArgumentDiscardsItsMessageState, ABadSendArgumentDiscardsItsMessageEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	engine.AdvanceTimeMs(60000)
	engine.RaiseExternal(ABadSendArgumentDiscardsItsMessageEventFinish, "", "")
	engine.Step()

	if ended, ok := engine.TerminalState(); !ok || ended != ABadSendArgumentDiscardsItsMessageStateDone {
		t.Errorf("`finish` must carry the run to `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"errors", policy.Errors, 6},
		{"sent", policy.Sent, 0},
		{"after", policy.After, 0},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
