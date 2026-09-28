// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.6.2 + 6.2: a <send>'s <content expr> is evaluated when the send
// is, and its value is the event's data — Go AOT path.
//
// Fixture: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_send_content_expr_is_the_payload_go.sh
package a_send_content_expr_is_the_payload

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestASendContentExprIsThePayload(t *testing.T) {
	policy := NewASendContentExprIsThePayloadPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ASendContentExprIsThePayloadState, ASendContentExprIsThePayloadEvent](&policy)
	engine.Initialize()

	if !engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond) {
		t.Fatalf("the machine never completed (parked in %v)", engine.GetCurrentState())
	}
	if ended, ok := engine.TerminalState(); !ok || ended != ASendContentExprIsThePayloadStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"numberOk", policy.NumberOk, 1},
		{"objectOk", policy.ObjectOk, 1},
		{"textOk", policy.TextOk, 1},
		{"errors", policy.Errors, 1},
		{"badArrived", policy.BadArrived, 1},
		{"badEmpty", policy.BadEmpty, 1},
		{"afterBad", policy.AfterBad, 0},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
