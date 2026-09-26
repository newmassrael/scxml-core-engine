// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is
// bound on that state's FIRST entry and never again — Go AOT path.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its
// <onentry> records the `v` it sees each time. Measured 2026-09-26, this
// channel never bound a state's <data> under late binding at all.
//
// Fixture: integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_late_data_binds_on_first_entry_go.sh

package late_data_binds_on_first_entry

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

func TestAStateBindsItsDataOnlyOnItsFirstEntry(t *testing.T) {
	policy := NewLateDataBindsOnFirstEntryPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[LateDataBindsOnFirstEntryState, LateDataBindsOnFirstEntryEvent](&policy)
	engine.Initialize()

	// Enter `s`, change its `v`, leave it, enter it again — one step each.
	for _, event := range []LateDataBindsOnFirstEntryEvent{
		LateDataBindsOnFirstEntryEventGo, LateDataBindsOnFirstEntryEventBump,
		LateDataBindsOnFirstEntryEventBack, LateDataBindsOnFirstEntryEventGo,
	} {
		engine.RaiseExternal(event, "", "")
		engine.Step()
	}

	seen := fmt.Sprintf("entries=%s seen=%s contentSeen=%s (wanted 2 / 15 / 7)",
		record(policy.Entries()), record(policy.Seen()), record(policy.ContentSeen()))
	if engine.GetCurrentState() != LateDataBindsOnFirstEntryStateS {
		t.Fatalf("the second `go` has to leave the machine in `s` (%s)", seen)
	}
	if v, ok := policy.Entries(); !ok || v != 2 {
		t.Errorf("both entries of `s` must have run (%s)", seen)
	}
	if v, ok := policy.Seen(); !ok || v != 15 {
		t.Errorf("`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: "+
			"11 is a processor that binds late data on every entry, and no value at all one that "+
			"never binds it (%s)", seen)
	}
	if v, ok := policy.ContentSeen(); !ok || v != 7 {
		t.Errorf("`c` is bound from inline content, not an expr, and must be bound too (%s)", seen)
	}
}
