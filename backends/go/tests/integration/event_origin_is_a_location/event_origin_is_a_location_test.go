// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix C.1 `_event.origin` is an address — Go AOT.
//
// The clause has two halves. The origin of a delivered event must match the
// `location` field the sending session published for the SCXML Event I/O
// Processor in its `_ioprocessors`, and that location is what a peer sends
// back to. A machine that puts a bare session id — or an invoke-instance
// path — there satisfies neither: the value matches nothing the sender
// published, and it names no target.
//
// The public IRP suite cannot separate the two spellings. Test 336 and test
// 350 both check `_event.origin` by sending to it with the sender and the
// receiver being the same session, so any value at all round-trips. Nothing
// in the corpus sends across sessions, which is the only arrangement where
// a bare id and a location differ.
//
// The fixture puts a second session on the other end, so the two halves
// separate and each has its own signal:
//
//	mismatch  the parent lands in `fail` — `_event.origin` did not equal
//	          the location the child published for itself
//	routing   the parent parks in `await_reply` and the run times out — a
//	          target that resolves nowhere delivers no event to fail on
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// (canonical, shared with the C++ / Rust / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_event_origin_is_a_location_go.sh
package event_origin_is_a_location

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestOriginIsTheSendersPublishedLocationAndRoutesBack(t *testing.T) {
	policy := NewEventOriginIsALocationPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[EventOriginIsALocationState, EventOriginIsALocationEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf(
			"event_origin_is_a_location timed out parked in %v. The parent accepted "+
				"`_event.origin` as an address and sent `reply` to it, and nothing came "+
				"back: Appendix C.1 requires the published location to be a usable "+
				"<send> target, so an origin that routes nowhere fails the half a "+
				"self-addressed test cannot exercise.",
			engine.GetCurrentState(),
		)
	}

	switch got := engine.GetCurrentState(); got {
	case EventOriginIsALocationStatePass:
	case EventOriginIsALocationStateFail:
		t.Fatalf("`_event.origin` did not carry the sender's published `_ioprocessors` " +
			"location. Appendix C.1 requires the origin to match that location, which " +
			"is what makes it an address a peer can answer; a bare session id or an " +
			"invoke-instance path matches nothing the sender published.")
	default:
		t.Fatalf("event_origin_is_a_location settled in %v, which is not a verdict state", got)
	}
}
