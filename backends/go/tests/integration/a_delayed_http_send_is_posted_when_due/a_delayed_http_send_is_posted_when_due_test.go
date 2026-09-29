// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the
// processor it names — a delayed BasicHTTP send is POSTed when due, and
// <cancel> reaches it while it waits — Go AOT path.
//
// Fixture: integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_delayed_http_send_is_posted_when_due_go.sh
package a_delayed_http_send_is_posted_when_due

import (
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestADelayedHttpSendIsPostedWhenDue(t *testing.T) {
	policy := NewADelayedHttpSendIsPostedWhenDuePolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ADelayedHttpSendIsPostedWhenDueState, ADelayedHttpSendIsPostedWhenDueEvent](&policy)

	// The host stands in for the HTTP transport: it keeps what it was handed.
	var posted []sce.HttpSendRequest
	engine.SetHTTPSendCallback(func(request sce.HttpSendRequest) *sce.HttpSendResponse {
		posted = append(posted, request)
		return nil
	})
	events := func() []string {
		names := make([]string, 0, len(posted))
		for _, request := range posted {
			names = append(names, request.EventName)
		}
		return names
	}
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()

	if got, want := events(), []string{"now"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("at once: POSTed %v, want %v (only the undelayed send)", got, want)
	}

	engine.AdvanceTimeMs(99)
	if got, want := events(), []string{"now"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("at 99ms: POSTed %v, want %v (a send delayed 100ms is not due)", got, want)
	}

	engine.AdvanceTimeMs(1)
	if got, want := events(), []string{"now", "later"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("at 100ms: POSTed %v, want %v (due now; the cancelled one never is)", got, want)
	}
	if got := posted[1]; got.Target != "http://127.0.0.1:18081/later" || got.SendID != "later" {
		t.Errorf("the delayed send carried target %q sendid %q", got.Target, got.SendID)
	}

	engine.AdvanceTimeMs(100)
	if got, want := events(), []string{"now", "later", "dynamic"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("at 200ms: POSTed %v, want %v", got, want)
	}
	dynamic := posted[2]
	if dynamic.Target != "http://127.0.0.1:18081/dynamic" {
		t.Errorf("a targetexpr is read when the send is made: target = %q", dynamic.Target)
	}
	if got, want := dynamic.Params["k"], []string{"v"}; !reflect.DeepEqual(got, want) {
		t.Errorf("the <param> the send carried: k = %v, want %v", got, want)
	}

	engine.AdvanceTimeMs(100)
	if ended, ok := engine.TerminalState(); !ok || ended != ADelayedHttpSendIsPostedWhenDueStateDone {
		t.Errorf("the run must end in `done`")
	}
	if got, want := events(), []string{"now", "later", "dynamic"}; !reflect.DeepEqual(got, want) {
		t.Errorf("at 300ms: POSTed %v, want %v", got, want)
	}
}
