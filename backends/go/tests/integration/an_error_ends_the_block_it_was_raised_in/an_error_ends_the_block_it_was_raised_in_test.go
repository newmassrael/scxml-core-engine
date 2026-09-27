// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 4.9: an error ends the block it was raised in — whichever element
// raised it — and no other block — Go AOT path.
//
// Fixture: integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_an_error_ends_the_block_it_was_raised_in_go.sh

package an_error_ends_the_block_it_was_raised_in

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestEachErrorEndsOnlyItsOwnBlock(t *testing.T) {
	policy := NewAnErrorEndsTheBlockItWasRaisedInPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnErrorEndsTheBlockItWasRaisedInState, AnErrorEndsTheBlockItWasRaisedInEvent](&policy)
	engine.Initialize()
	for _, event := range []AnErrorEndsTheBlockItWasRaisedInEvent{
		AnErrorEndsTheBlockItWasRaisedInEventT,
		AnErrorEndsTheBlockItWasRaisedInEventFinish,
	} {
		engine.RaiseExternal(event, "", "")
		engine.Step()
	}

	if ended, ok := engine.TerminalState(); !ok || ended != AnErrorEndsTheBlockItWasRaisedInStateDone {
		t.Errorf("`finish` must carry the run to `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"errors", policy.Errors, 7},
		{"afterAssign", policy.AfterAssign, 0},
		{"afterScript", policy.AfterScript, 0},
		{"afterLog", policy.AfterLog, 0},
		{"afterCancel", policy.AfterCancel, 0},
		{"afterIfInner", policy.AfterIfInner, 0},
		{"afterIf", policy.AfterIf, 0},
		{"afterSingle", policy.AfterSingle, 0},
		{"afterTrans", policy.AfterTrans, 0},
		{"initRan", policy.InitRan, 1},
		{"pairs", policy.Pairs, 4},
		{"sum", policy.Sum, 90},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
