// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as the same value written in
// `target` is, at once or after a delay — Go AOT path.
//
// Fixture: integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_target_expression_is_routed_as_its_literal_is_go.sh
package a_target_expression_is_routed_as_its_literal_is

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestATargetExpressionIsRoutedAsItsLiteralIs(t *testing.T) {
	policy := NewATargetExpressionIsRoutedAsItsLiteralIsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ATargetExpressionIsRoutedAsItsLiteralIsState, ATargetExpressionIsRoutedAsItsLiteralIsEvent](&policy)
	engine.Initialize()

	if !engine.RunUntilCompletion(3*time.Second, 5*time.Millisecond) {
		t.Fatalf("the machine never completed (parked in %v)", engine.GetCurrentState())
	}
	if ended, ok := engine.TerminalState(); !ok || ended != ATargetExpressionIsRoutedAsItsLiteralIsStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"internalNow", policy.InternalNow, 1},
		{"internalLater", policy.InternalLater, 1},
		{"kidNow", policy.KidNow, 1},
		{"kidLater", policy.KidLater, 1},
		{"sessNow", policy.SessNow, 1},
		{"sessLater", policy.SessLater, 1},
		{"commErrors", policy.CommErrors, 4},
		{"execErrors", policy.ExecErrors, 2},
		{"afterStranger", policy.AfterStranger, 0},
		{"afterStrangerLater", policy.AfterStrangerLater, 0},
		{"afterOrphan", policy.AfterOrphan, 0},
		{"afterOrphanLater", policy.AfterOrphanLater, 0},
		{"afterBogus", policy.AfterBogus, 0},
		{"afterBogusLater", policy.AfterBogusLater, 0},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
