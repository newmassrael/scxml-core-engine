// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: an <invoke> naming its target through an expression
// evaluates that expression at invoke-fire time, and a failure to evaluate
// raises error.execution — Go AOT path.
//
// The clause puts two obligations on the Processor and this fixture is about
// the second only. The evaluated string does not select the child on this
// path: codegen fixes the child at build time and writes an immediate-<final>
// stub for it (docs/SCE_ACCEPTED_SUBSET.md §2.13). A fixture built on the
// value would therefore measure nothing here. One built on the FAILURE
// measures what every channel still owes: evaluate, and say so when you
// cannot.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_expression_failure_is_reported_go.sh

package invoke_expression_failure_is_reported

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAnInvokeExpressionThatCannotBeEvaluatedRaisesErrorExecution(t *testing.T) {
	// Engine DI Parity RFC (Path B+): the fixture declares <data> and its
	// <invoke> names an expression, so the policy needs an engine to evaluate
	// both. Per-test, rather than a process-global singleton.
	policy := NewInvokeExpressionFailureIsReportedPolicy()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[InvokeExpressionFailureIsReportedState, InvokeExpressionFailureIsReportedEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)

	if !completed {
		t.Fatalf("the machine never completed (parked in %v). W3C SCXML 6.4.3 requires "+
			"the expression to be evaluated when the <invoke> fires; parking means "+
			"neither the raise nor the child arrived", engine.GetCurrentState())
	}
	if got := engine.GetCurrentState(); got != InvokeExpressionFailureIsReportedStatePass {
		t.Fatalf("machine reached %v, want Pass: reaching fail means the child started "+
			"on an expression that cannot be evaluated, so nothing evaluated it", got)
	}
}
