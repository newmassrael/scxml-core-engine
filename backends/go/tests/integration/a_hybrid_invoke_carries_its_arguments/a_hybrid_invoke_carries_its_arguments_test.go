// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 + 6.4.3: an <invoke> whose child is named by an expression
// carries its arguments as one whose child is fixed does — Go AOT path.
//
// The value always names keeper, and bare declares the one name keeper does
// not, so a pair seeded by the wrong candidate's declarations is a leak keeper
// reports rather than an absence the test has to infer.
//
// Fixture: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_hybrid_invoke_carries_its_arguments_go.sh
package a_hybrid_invoke_carries_its_arguments

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestEachArgumentReachesOnlyTheChildThatDeclaresIt(t *testing.T) {
	policy := NewAHybridInvokeCarriesItsArgumentsPolicy()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[AHybridInvokeCarriesItsArgumentsState, AHybridInvokeCarriesItsArgumentsEvent](&policy)
	engine.Initialize()

	if !engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond) {
		t.Fatalf("the machine never completed (parked in %v); refusedPhase parks "+
			"when its invoke raised nothing", engine.GetCurrentState())
	}
	if got, ended := engine.TerminalState(); !ended || got != AHybridInvokeCarriesItsArgumentsStateDone {
		t.Fatalf("machine reached %v, want Done: FailWrongChild means bare ran, "+
			"FailRefusedChildStarted means an unreadable namelist still started a child", got)
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"errors", policy.Errors, 2},
		{"started", policy.Started, 2},
		{"paramsOk", policy.ParamsOk, 1},
		{"namelistOk", policy.NamelistOk, 1},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
