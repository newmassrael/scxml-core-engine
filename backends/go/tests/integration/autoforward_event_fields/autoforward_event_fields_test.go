// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward field preservation — Go AOT local-invoke path.
//
// W3C §6.4 requires the parent to forward an exact copy of every external
// event to an `<invoke autoforward="true">` child. The public IRP suite never
// checks the copy's contents: test229 only asserts the event name crosses, and
// test230 is a manual test whose field comparison is done by a human reading
// two log dumps. A forward stripped down to the bare event name passes both.
//
// Fixture: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
// (canonical, shared with the C++ / Rust / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_event_fields_go.sh

package autoforward_event_fields

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestForwardedCopyKeepsDataOriginAndInvokeid(t *testing.T) {
	policy := NewAutoforwardEventFieldsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AutoforwardEventFieldsState, AutoforwardEventFieldsEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("autoforward_event_fields timed out before reaching a final state — the " +
			"child never received the forwarded `childToParent`, so no " +
			"done.invoke.inv_echo was emitted")
	}

	got := engine.GetCurrentState()
	if got != AutoforwardEventFieldsStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the child reported `stripped`, so the "+
				"autoforwarded copy of `childToParent` lost `_event.data.value`, "+
				"`_event.origin` or `_event.invokeid`. W3C §6.4 requires an exact copy "+
				"— ForwardToAutoforwardChildren must carry the source event's metadata, "+
				"not just its name.",
			got,
		)
	}
}
