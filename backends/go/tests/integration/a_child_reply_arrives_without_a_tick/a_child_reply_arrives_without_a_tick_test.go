// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 6.4: a reply an invoked child has already sent is on the
// parent's external queue, so a host that only hands the machine events
// still sees it ahead of its own later events — Go AOT path.
//
// Fixture: integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_child_reply_arrives_without_a_tick_go.sh

package a_child_reply_arrives_without_a_tick

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

func TestTheChildsReplyArrivesBeforeTheHostsNextEvent(t *testing.T) {
	policy := NewAChildReplyArrivesWithoutATickPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handler records with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AChildReplyArrivesWithoutATickState, AChildReplyArrivesWithoutATickEvent](&policy)
	engine.Initialize()
	engine.RaiseExternal(AChildReplyArrivesWithoutATickEventFinish, "", "")
	engine.Step()

	seen := fmt.Sprintf("hellos=%s (wanted 1)", record(policy.Hellos()))
	if ended, ok := engine.TerminalState(); !ok || ended != AChildReplyArrivesWithoutATickStateDone {
		t.Errorf("`finish` must carry the run to `done` (%s)", seen)
	}
	if v, ok := policy.Hellos(); !ok || v != 1 {
		t.Errorf("the child's start-time reply arrives before `finish` (%s)", seen)
	}
}
