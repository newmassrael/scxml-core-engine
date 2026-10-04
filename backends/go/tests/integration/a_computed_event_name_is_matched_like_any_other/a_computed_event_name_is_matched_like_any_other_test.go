// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Go AOT.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Rust / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_computed_event_name_is_matched_like_any_other_go.sh

package a_computed_event_name_is_matched_like_any_other

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAComputedNameIsMatchedByTokenAndToldWhole(t *testing.T) {
	policy := NewAComputedEventNameIsMatchedLikeAnyOtherPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AComputedEventNameIsMatchedLikeAnyOtherState, AComputedEventNameIsMatchedLikeAnyOtherEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("a_computed_event_name_is_matched_like_any_other stopped before its final state: " +
			"a computed name was dropped, or matched but told shorter than the name it was sent " +
			"under (`request.new`, `other.thing`, `request.again`, `last.one`)")
	}

	got, ended := engine.TerminalState()
	if !ended || got != AComputedEventNameIsMatchedLikeAnyOtherStatePass {
		t.Fatalf("the machine reached %v, want Pass", got)
	}
}
