// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The name an event arrives under (§scxml-5.10, §scxml-3.12.1) — Go AOT
// local-invoke path.
//
// A transition on `request` takes `request.new` by whole-token matching, and
// `_event.name` is then the name the event was sent under, not the descriptor it
// was matched through. The public IRP suite never reads a name the document does
// not write, so a machine that is told the shorter one passes all of it.
//
// Fixture: integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml
// (canonical, shared with the C++ / Rust / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_event_keeps_the_name_it_was_sent_under_go.sh

package an_event_keeps_the_name_it_was_sent_under

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestANameTheDocumentDoesNotWriteIsToldWhole(t *testing.T) {
	policy := NewAnEventKeepsTheNameItWasSentUnderPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnEventKeepsTheNameItWasSentUnderState, AnEventKeepsTheNameItWasSentUnderEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("an_event_keeps_the_name_it_was_sent_under timed out before reaching a final " +
			"state — the child never heard `request.new`, so it never answered")
	}

	got, ended := engine.TerminalState()
	if !ended || got != AnEventKeepsTheNameItWasSentUnderStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the child reported `arrivedShortened`: `request.new` "+
				"took the transition on `request` but `_event.name` told the child `request`. "+
				"W3C §5.10 makes the name the one the event was sent under, and §3.12.1 only "+
				"decides which transition takes it.",
			got,
		)
	}
}
