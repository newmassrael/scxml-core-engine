// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the
// SCXML Event I/O Processor delivers is the address its sender published, on
// every route a session sends by, and a reply to it arrives — Go AOT path.
//
// Fixture: integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_an_origin_is_the_address_its_sender_published_go.sh
package an_origin_is_the_address_its_sender_published

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// A self-send, a child's send to `#_parent` and a parent's send to `#_kid`:
// each origin is the sender's published location with the SCXML processor's
// URI, and each reply sent to it arrives.
func TestAnOriginIsTheAddressItsSenderPublished(t *testing.T) {
	policy := NewAnOriginIsTheAddressItsSenderPublishedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnOriginIsTheAddressItsSenderPublishedState, AnOriginIsTheAddressItsSenderPublishedEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}

	if !engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond) {
		t.Fatalf("the run stopped in %v: a reply sent to an origin reached nobody", engine.GetCurrentState())
	}
	if ended, ok := engine.TerminalState(); !ok || ended != AnOriginIsTheAddressItsSenderPublishedStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"selfOk", policy.SelfOk, 1},
		{"selfReplied", policy.SelfReplied, 1},
		{"childOk", policy.ChildOk, 1},
		{"childReplied", policy.ChildReplied, 1},
		{"parentOk", policy.ParentOk, 1},
		{"parentReplied", policy.ParentReplied, 1},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
