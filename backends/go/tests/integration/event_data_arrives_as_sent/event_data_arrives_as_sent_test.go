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
	if active(afterNote, EventDataArrivesAsSentStateGarbled) {
		t.Errorf("the host sent the text `hold the line` and "+
			"`_event.data === 'hold the line'` did not hold, so a payload that is not "+
			"JSON did not arrive as the string it was sent as (active: %v)", afterNote)
	}

	// Text that happens to be a valid expression. §scxml-B-2-8-1 gives the
	// payload three readings and none of them is "evaluate it": a payload is
	// what a host, a peer session or an HTTP sender put there, and running it
	// makes `_event.data` mean whatever the receiver's engine is written in.
	engine.RaiseExternal(EventDataArrivesAsSentEventArith, "2 + 3", "")
	engine.Step()

	afterArith := engine.GetActiveStates()
	if active(afterArith, EventDataArrivesAsSentStateEvaluated) {
		t.Errorf("the host sent the text `2 + 3` and it arrived as 5 — the payload "+
			"was run rather than read (active: %v)", afterArith)
	}
	if !active(afterArith, EventDataArrivesAsSentStateDocumented) {
		t.Errorf("the arithmetic-shaped payload neither matched nor mismatched "+
			"(active: %v)", afterArith)
	}

	// §scxml-B-2-8-1's XML rung, reached through the EVENT path. The `<data>`
	// path is `xml_data_is_a_dom_tree`'s and the two are lowered on separate
	// code in every backend.
	// Leading whitespace on purpose: the reading is chosen by the first
	// NON-blank character, and a pretty-printed document is the ordinary shape
	// of one. The scan past it is small enough to look redundant.
	engine.RaiseExternal(EventDataArrivesAsSentEventDoc, "\n  "+`<books xmlns=""><book title="t1"/></books>`, "")
	engine.Step()

	afterDoc := engine.GetActiveStates()
	if active(afterDoc, EventDataArrivesAsSentStateFlattened) {
		t.Errorf("the host sent a well-formed XML document and "+
			"`_event.data.documentElement.nodeName === 'books'` did not hold, so the "+
			"payload did not become the DOM structure the clause requires (active: %v)", afterDoc)
	}

	// The sentence that closes the clause. Every `error.*` message this
	// repository raises names the SCXML construct that failed, so every one of
	// them has exactly this shape: it opens like a document and is not one.
	engine.RaiseExternal(EventDataArrivesAsSentEventBroken, "<assign>  to  detail failed", "")
	engine.Step()

	afterBroken := engine.GetActiveStates()
	if active(afterBroken, EventDataArrivesAsSentStateSwallowed) {
		t.Errorf("the host sent `<assign>  to  detail failed`, which opens with `<` and "+
			"is not a valid XML document, so §scxml-B-2-8-1's closing MUST applies and "+
			"the reading is the space-normalized string. This backend answered nil until "+
			"2026-08-19 (active: %v)", afterBroken)
	}
	if !active(afterBroken, EventDataArrivesAsSentStateSettled) {
		t.Errorf("the malformed-XML payload neither matched nor mismatched "+
			"(active: %v)", afterBroken)
	}
}
