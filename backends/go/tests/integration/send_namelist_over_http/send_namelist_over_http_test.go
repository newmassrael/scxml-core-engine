// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.2 + 6.2.3 `<send namelist>` over BasicHTTP — Go AOT.
//
// Two claims the IRP corpus states and cannot measure:
//
//   the namelist reaches the form   test518 is titled "namelist values get
//     encoded as POST parameters" and passes as soon as the event comes
//     back, whatever it carried.
//
//   an unreadable item reports and discards   W3C SCXML 5.9.2 puts
//     `error.execution` on the internal queue; 6.2.3 discards the message.
//     `<param>`'s per-item exception (5.7.1, "ignore the name and value")
//     has no counterpart for namelist anywhere in the specification.
//
// This channel evaluated the namelist once for the payload and then read
// the data model AGAIN inside the BasicHTTP arm, where the second reading
// had no error arm at all. The two could only agree while nothing failed.
//
// The two claims reach distinct final states, so a failure names which one
// broke.
//
// Fixture: integration_resources/send_namelist_over_http/send_namelist_over_http.scxml
// (canonical, shared with the other channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_send_namelist_over_http_go.sh
//
// Needs the W3C harness server, like every BasicHTTP fixture here:
//   node tests/w3c/standalone_http_server.js 8080 /test

package send_namelist_over_http

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestSendNamelistReachesTheFormAndABrokenItemDiscardsTheMessage(t *testing.T) {
	policy := NewSendNamelistOverHttpPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	policy.BasicHTTPAccessURI = scegotest.BasicHTTPAccessURI()
	engine := sce.NewEngine[SendNamelistOverHttpState, SendNamelistOverHttpEvent](&policy)
	scegotest.SetupHTTPTest(engine)
	engine.Initialize()

	completed := engine.RunUntilCompletion(15*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("send_namelist_over_http timed out before reaching a final state — " +
			"the delayed `timeoutMap` / `timeoutDiscard` sends that give each phase " +
			"its verdict never fired, so the machine is not being ticked")
	}

	switch got := engine.GetCurrentState(); got {
	case SendNamelistOverHttpStatePass:
	case SendNamelistOverHttpStateFailNamelistNeverArrived:
		t.Fatalf("the BasicHTTP send never came back at all — the harness server did " +
			"not answer, which is a different failure from posting the wrong form.")
	case SendNamelistOverHttpStateFailNamelistNotPosted:
		t.Fatalf("`mapped` arrived without `Var1` in its data — W3C SCXML C.2 requires " +
			"a namelist's variable names and values to be mapped to HTTP POST " +
			"parameters.")
	case SendNamelistOverHttpStateFailMessageNotDiscarded:
		t.Fatalf("`shouldNotArrive` was delivered — W3C SCXML 6.2.3 discards the " +
			"message when the evaluation of a `<send>`'s arguments produces an error. " +
			"`<param>`'s per-item rule (5.7.1) does not reach a namelist item.")
	case SendNamelistOverHttpStateFailNoNamelistError:
		t.Fatalf("no `error.execution` preceded the timeout — W3C SCXML 5.9.2 requires " +
			"it when a location expression yields no valid location, and the wire the " +
			"send would have crossed does not change the answer.")
	default:
		t.Fatalf("send_namelist_over_http settled in %v, which is not a verdict state", got)
	}
}
