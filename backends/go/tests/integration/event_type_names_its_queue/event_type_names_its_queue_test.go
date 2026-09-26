// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 5.10.1: _event.type names the queue an event was taken from — Go
// AOT path.
//
// The document queues an external event first and two internal ones after
// it. Measured 2026-09-26, this channel already typed events from their
// metadata; the enqueue-time flag it also carried was read by nothing and is
// gone. The fixture pins the behaviour.
//
// Fixture: integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_event_type_names_its_queue_go.sh

package event_type_names_its_queue

import (
	"fmt"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// record reads one value the handlers wrote (W3C SCXML 5.3 readers).
func record(v int64, ok bool) string {
	if !ok {
		return "<unreadable>"
	}
	return fmt.Sprint(v)
}

func TestEachEventIsTypedByTheQueueItWasTakenFrom(t *testing.T) {
	policy := NewEventTypeNamesItsQueuePolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[EventTypeNamesItsQueueState, EventTypeNamesItsQueueEvent](&policy)
	// The document queues its own events; the run needs nothing from the host.
	engine.Initialize()

	seen := fmt.Sprintf("intCode=%s sendCode=%s extCode=%s (1 internal, 2 external, 3 other; wanted 1 / 1 / 2)",
		record(policy.IntCode()), record(policy.SendCode()), record(policy.ExtCode()))
	if ended, ok := engine.TerminalState(); !ok || ended != EventTypeNamesItsQueueStateDone {
		t.Errorf("`ext` must carry the run to `done` (%s)", seen)
	}
	if v, ok := policy.IntCode(); !ok || v != 1 {
		t.Errorf("`int` came off the internal queue while `ext` waited on the external one (%s)", seen)
	}
	if v, ok := policy.SendCode(); !ok || v != 1 {
		t.Errorf("a `<send target=\"#_internal\">` with a payload rides the internal queue too (%s)", seen)
	}
	if v, ok := policy.ExtCode(); !ok || v != 2 {
		t.Errorf("`ext` came off the external queue (%s)", seen)
	}
}
