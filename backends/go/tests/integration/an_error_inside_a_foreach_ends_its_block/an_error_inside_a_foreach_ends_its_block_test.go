// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 4.9 + 4.6: an error raised inside a <foreach> body ends the
// block that contains the <foreach>, and it is the only error raised —
// Go AOT path.
//
// Fixture: integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_an_error_inside_a_foreach_ends_its_block_go.sh

package an_error_inside_a_foreach_ends_its_block

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestTheErrorEndsTheBlockAndIsTheOnlyOne(t *testing.T) {
	policy := NewAnErrorInsideAForeachEndsItsBlockPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnErrorInsideAForeachEndsItsBlockState, AnErrorInsideAForeachEndsItsBlockEvent](&policy)
	engine.Initialize()
	for _, event := range []AnErrorInsideAForeachEndsItsBlockEvent{
		AnErrorInsideAForeachEndsItsBlockEventGo,
		AnErrorInsideAForeachEndsItsBlockEventT,
		AnErrorInsideAForeachEndsItsBlockEventFinish,
	} {
		engine.RaiseExternal(event, "", "")
		engine.Step()
	}

	if ended, ok := engine.TerminalState(); !ok || ended != AnErrorInsideAForeachEndsItsBlockStateDone {
		t.Errorf("`finish` must carry the run to `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"iters1", policy.Iters1, 1},
		{"after1", policy.After1, 0},
		{"iters2", policy.Iters2, 1},
		{"after2", policy.After2, 0},
		{"iters3", policy.Iters3, 1},
		{"after3", policy.After3, 0},
		{"errors", policy.Errors, 3},
		{"sent", policy.Sent, 2},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
