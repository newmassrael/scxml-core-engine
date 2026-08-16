// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel as a
// value — Go AOT.
//
// The edge nothing measured. Every other integration fixture drives its
// machine with RaiseExternal(event, "", ""); the payload argument was the
// empty string in every call on every channel until this one, so the
// host-to-datamodel boundary was covered by no test at all. The W3C suite
// does not reach it either — its payloads originate inside the document
// (<send><content>, <param>, <donedata>), a separate path in every backend.
//
// Fixture: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_data_arrives_as_sent_go.sh

package event_data_arrives_as_sent

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func active(states []EventDataArrivesAsSentState, want EventDataArrivesAsSentState) bool {
	for _, s := range states {
		if s == want {
			return true
		}
	}
	return false
}

func TestAHostsJSONPayloadIsAddressableAndItsTextStaysText(t *testing.T) {
	policy := NewEventDataArrivesAsSentPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture reads `_event.data` in its guards, so this is an
	// ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[EventDataArrivesAsSentState, EventDataArrivesAsSentEvent](&policy)
	engine.Initialize()

	if entry := engine.GetActiveStates(); !active(entry, EventDataArrivesAsSentStateWaiting) {
		t.Fatalf("fixture came up as %v; it is supposed to start in `waiting`, so "+
			"nothing below is testing what it claims", entry)
	}

	// A JSON object, the shape an embedder has when it holds structured data
	// and a state machine to give it to.
	engine.RaiseExternal(EventDataArrivesAsSentEventPayload, `{"milestone":"refined","turns":2}`, "")
	engine.Step()

	afterPayload := engine.GetActiveStates()
	if active(afterPayload, EventDataArrivesAsSentStateMangled) {
		t.Fatalf("the host sent a JSON object and the guard "+
			"`_event.data.milestone === 'refined' && _event.data.turns === 2` did not "+
			"hold, so the payload did not arrive as an object with those properties "+
			"(active: %v)", afterPayload)
	}
	if !active(afterPayload, EventDataArrivesAsSentStateHeard) {
		t.Fatalf("the payload guard neither matched nor mismatched — the machine is "+
			"not in `heard` (active: %v)", afterPayload)
	}

	// Text that is not JSON. The same call, and it must NOT be parsed into
	// something else: `hold the line` is the value the document compares
	// against, character for character.
	engine.RaiseExternal(EventDataArrivesAsSentEventNote, "hold the line", "")
	engine.Step()

	afterNote := engine.GetActiveStates()
	if !active(afterNote, EventDataArrivesAsSentStateSettled) {
		t.Errorf("the host sent the text `hold the line` and "+
			"`_event.data === 'hold the line'` did not hold, so a payload that is not "+
			"JSON did not arrive as the string it was sent as (active: %v)", afterNote)
	}
}
