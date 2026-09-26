// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises
// error.execution and is ignored; the done events are raised all the same —
// Go AOT path.
//
// Measured 2026-09-27, this channel treated an empty `location` as a
// structural error that withheld done.state.<parent> while still raising the
// <parallel>'s done event.
//
// Fixture: integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_bad_donedata_pair_is_ignored_go.sh

package a_bad_donedata_pair_is_ignored

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

func TestTheBadPairsAreDroppedAndTheDoneEventsStillArrive(t *testing.T) {
	policy := NewABadDonedataPairIsIgnoredPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ABadDonedataPairIsIgnoredState, ABadDonedataPairIsIgnoredEvent](&policy)
	// The run needs nothing from the host.
	engine.Initialize()

	seen := fmt.Sprintf("errors=%s shape=%s (wanted 2 / 1)", record(policy.Errors()), record(policy.Shape()))
	if ended, ok := engine.TerminalState(); !ok || ended != ABadDonedataPairIsIgnoredStateDone {
		t.Errorf("done.state.p must still arrive and carry the run to `done` (%s)", seen)
	}
	if v, ok := policy.Errors(); !ok || v != 2 {
		t.Errorf("each ignored pair raises its own error.execution (%s)", seen)
	}
	if v, ok := policy.Shape(); !ok || v != 1 {
		t.Errorf("done.state.r1 must carry the surviving pair and neither ignored one (%s)", seen)
	}
}
