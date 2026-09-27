// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 5.10 + 6.2: a <send>'s payload is the data of the event it sends,
// and a data-less event dequeued before it carries none — Go AOT path.
//
// Fixture: integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_payload_rides_on_its_own_event_go.sh

package a_payload_rides_on_its_own_event

import (
	"fmt"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func record(v int64, ok bool) string {
	if !ok {
		return "<unreadable>"
	}
	return fmt.Sprint(v)
}

func TestEachPayloadArrivesOnItsOwnEvent(t *testing.T) {
	policy := NewAPayloadRidesOnItsOwnEventPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[APayloadRidesOnItsOwnEventState, APayloadRidesOnItsOwnEventEvent](&policy)
	engine.Initialize()
	engine.RaiseExternal(APayloadRidesOnItsOwnEventEventFinish, "", "")
	engine.Step()

	seen := fmt.Sprintf("got=%s stolen=%s plains=%s (wanted 4 / 0 / 4)",
		record(policy.Got()), record(policy.Stolen()), record(policy.Plains()))
	if ended, ok := engine.TerminalState(); !ok || ended != APayloadRidesOnItsOwnEventStateDone {
		t.Errorf("`finish` must carry the run to `done` (%s)", seen)
	}
	if v, ok := policy.Got(); !ok || v != 4 {
		t.Errorf("each payload event arrives carrying its own payload (%s)", seen)
	}
	if v, ok := policy.Stolen(); !ok || v != 0 {
		t.Errorf("no data-less event arrives carrying a payload (%s)", seen)
	}
	if v, ok := policy.Plains(); !ok || v != 4 {
		t.Errorf("every data-less event arrives, and arrives empty (%s)", seen)
	}
}
