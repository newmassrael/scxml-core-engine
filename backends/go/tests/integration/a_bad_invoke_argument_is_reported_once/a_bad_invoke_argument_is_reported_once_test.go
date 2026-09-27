// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 5.7.1 + 6.4: what each argument of an <invoke> costs when it
// cannot be read, and what a readable one delivers — Go AOT path.
//
// Fixture: integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_bad_invoke_argument_is_reported_once_go.sh

package a_bad_invoke_argument_is_reported_once

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestEachArgumentCostsWhatItsClauseSays(t *testing.T) {
	policy := NewABadInvokeArgumentIsReportedOncePolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ABadInvokeArgumentIsReportedOnceState, ABadInvokeArgumentIsReportedOnceEvent](&policy)
	engine.Initialize()
	for _, event := range []ABadInvokeArgumentIsReportedOnceEvent{
		ABadInvokeArgumentIsReportedOnceEventGo,
		ABadInvokeArgumentIsReportedOnceEventFinish,
	} {
		engine.RaiseExternal(event, "", "")
		engine.Step()
	}

	if ended, ok := engine.TerminalState(); !ok || ended != ABadInvokeArgumentIsReportedOnceStateDone {
		t.Errorf("`finish` must carry the run to `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"errors", policy.Errors, 5},
		{"started", policy.Started, 1},
		{"fromLocOk", policy.FromLocOk, 1},
		{"emptyLocLeftOut", policy.EmptyLocLeftOut, 1},
		{"brokenLeftOut", policy.BrokenLeftOut, 1},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
