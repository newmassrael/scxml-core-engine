// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1: `<donedata>` survives a late completion — Go AOT path.
//
// The sibling `donedata_local_invoke` pins the payload shapes on a child whose
// initial configuration is already its top-level `<final>`. That child is done
// before its first macrostep, so the lift and the raise sit in the same call
// and the fixture cannot see a child that finishes later.
//
// §6.3.1 raises `done.invoke.<id>` whenever the child reaches a final state,
// and §5.5 puts that final state's `<donedata>` on the event. Neither sentence
// is scoped to a child that finalises during start-up, so a backend that lifts
// the stash only there satisfies the sibling and still hands the parent an
// empty `_event.data` for every child that answers an event first — which is
// what an invoked session normally does.
//
// Here the child opens the exchange with `ready`, the parent answers over
// `<send target="#_inv_late">`, and the child reaches `settled` two macrosteps
// in. The payload and the guard are copied from the sibling's `inv_param`
// phase, so a shape the sibling already proves green cannot be what fails
// here — only the timing differs.
//
// Fixture: integration_resources/donedata_late_completion/donedata_late_completion.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_donedata_late_completion_go.sh

package donedata_late_completion

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestDonedataRidesACompletionAfterTheInvokeStarted(t *testing.T) {
	// Engine DI Parity RFC (Path B+): per-test LuaEngine — the parent's
	// `done.invoke` guard and the child's `<donedata>` param both evaluate
	// expressions.
	policy := NewDonedataLateCompletionPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[DonedataLateCompletionState, DonedataLateCompletionEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("donedata_late_completion timed out before reaching a final state — " +
			"the parent never saw `done.invoke.inv_late` at all, so the child was " +
			"not driven to its `<final>`")
	}

	got := engine.GetCurrentState()
	if got != DonedataLateCompletionStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the `done.invoke.inv_late` guard did not "+
				"see `_event.data.result === 42`, so the child's `<donedata>` was "+
				"dropped on a completion that happened after the invoke was started. "+
				"W3C SCXML 6.3.1 raises `done.invoke.<id>` wherever the child reaches "+
				"its final state and 5.5 puts that state's donedata on the event; "+
				"neither is scoped to children that finalise during start-up.",
			got,
		)
	}
}
