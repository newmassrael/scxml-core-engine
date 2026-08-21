// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 under 6.4 — Go AOT.
//
// A `<param>` of an `<invoke>` whose expression will not evaluate is the one
// place two clauses meet: §6.4.2 terminates the element when "the evaluation
// of its arguments produces an error", and §5.7.1 says a failing `<param>`
// costs `error.execution` and "MUST ignore the name and value" — then
// delegates only the SUCCESSFUL name and value to the context, naming
// `<donedata>`, `<send>` and `<invoke>` in that sentence.
//
// 5.7.1 governs: it has already said what a failed `<param>` costs, in this
// context by name. W3C test343 settles the same clause from the `<donedata>`
// side; no IRP document asks it of `<invoke>`.
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml
// (canonical, shared with the other channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_param_error_starts_the_child_go.sh

package invoke_param_error_starts_the_child

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAnInvokeParamThatWillNotEvaluateCostsItsPairAndNothingElse(t *testing.T) {
	policy := NewInvokeParamErrorStartsTheChildPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[InvokeParamErrorStartsTheChildState, InvokeParamErrorStartsTheChildEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(10*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("invoke_param_error_starts_the_child timed out before reaching a " +
			"final state — even the `timeout` that judges a never-started child " +
			"never fired, so the machine is not being ticked")
	}

	switch got := engine.GetCurrentState(); got {
	case InvokeParamErrorStartsTheChildStatePass:
	case InvokeParamErrorStartsTheChildStateFailNoParamError:
		t.Fatalf("`childUp` arrived with no `error.execution` before it: W3C SCXML " +
			"5.7.1 puts that error on the internal queue while the <invoke> is " +
			"being evaluated, so it is dequeued before the child's first word.")
	case InvokeParamErrorStartsTheChildStateFailInvokeNotStarted:
		t.Fatalf("the child never started: this channel read W3C SCXML 6.4.2's " +
			"\"terminate the processing of the element\" over 5.7.1's per-item " +
			"rule. One <param> that will not evaluate costs its own pair, not " +
			"the session.")
	case InvokeParamErrorStartsTheChildStateFailGoodParamLost:
		t.Fatalf("the child's `kept` did not arrive as 'here': W3C SCXML 6.4.3 seeds " +
			"the child's matching <data> from the param's value, and one sibling " +
			"that failed does not cost the others.")
	case InvokeParamErrorStartsTheChildStateFailBrokenParamSeeded:
		t.Fatalf("the child found the empty string under `broken`: 5.7.1 says ignore " +
			"the name AND the value, so the child must find its own declaration " +
			"untouched rather than a placeholder the author never wrote.")
	default:
		t.Fatalf("invoke_param_error_starts_the_child settled in %v, which is not a "+
			"verdict state", got)
	}
}
