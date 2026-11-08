// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A (EventSchema native lowering) — Go compile+run gate,
// the twin of the Rust `tests/event_schema_native.rs` and the C11
// `c11_integration_event_schema_native` tests.
//
// The committed SM (statechart_minimal_sm.go) is generated from
// sce-build/tests/fixtures/event_schema/statechart_minimal.scxml
// (regen: scripts/regen_event_schema_native_go.sh). Because it compiles as
// part of this package, the generated payload struct, the type-erased
// `TypedPayload` carrier round-trip, and the per-event `StatechartMinimalRaiseJobCompleted`
// inject seam are really type-checked.
//
// The transition guard `cond="_event.data.elapsed_ms === 0"` lowers to a
// native `p.pendingPayloadTag == … && (…)` tag-checked field comparison with
// NO script engine, so the policy is constructed WITHOUT a `ScriptEngine`
// (the MCU-relevant property: a typed-guard machine links no Lua). The
// per-event `StatechartMinimalRaiseJobCompleted` seam binds the event name and the payload
// field value in one call.

package statechart_minimal

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestTypedPayloadGuardFiresNatively(t *testing.T) {
	policy := NewStatechartMinimalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// No policy.ScriptEngine — needs_script_engine is false for a typed guard.
	engine := sce.NewEngine[StatechartMinimalState, StatechartMinimalEvent](&policy)
	engine.Initialize()

	if got := engine.GetCurrentState(); got != StatechartMinimalStateWaiting {
		t.Fatalf("initial state = %v, want Waiting", got)
	}

	// Per-event typed inject. elapsed_ms == 0 satisfies the native guard.
	StatechartMinimalRaiseJobCompleted(engine, 0)
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartMinimalStateDone {
		t.Fatalf("after StatechartMinimalRaiseJobCompleted(0): state = %v, want Done "+
			"(elapsed_ms == 0 must fire the native typed guard)", got)
	}
}

func TestTypedPayloadGuardMissesOnNonzero(t *testing.T) {
	policy := NewStatechartMinimalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartMinimalState, StatechartMinimalEvent](&policy)
	engine.Initialize()

	// Same event, a payload the guard rejects — the machine stays put.
	StatechartMinimalRaiseJobCompleted(engine, 5)
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartMinimalStateWaiting {
		t.Fatalf("after StatechartMinimalRaiseJobCompleted(5): state = %v, want Waiting "+
			"(elapsed_ms == 5 must leave the machine in waiting)", got)
	}
}

// A delivery the door refuses in a document that declares no error.execution
// has nothing taken in its place, and is not dropped unseen: it is counted as a
// discarded external event and named the last one, the hook every lane shares
// (docs/adr/0086, section 4.2). statechart_minimal declares no error.execution.
func TestARefusedDeliveryWithNothingInItsPlaceIsCountedAsDiscarded(t *testing.T) {
	policy := NewStatechartMinimalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartMinimalState, StatechartMinimalEvent](&policy)
	engine.Initialize()

	engine.RaiseExternalByName("job.completed", `{"elapsed_ms": "nought"}`)
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartMinimalStateWaiting {
		t.Fatalf("state = %v, want Waiting: a refused delivery selects nothing", got)
	}
	if got := engine.DiscardedExternalEvents(); got != 1 {
		t.Fatalf("DiscardedExternalEvents() = %d, want 1: a refused delivery with "+
			"nothing in its place was dropped unseen", got)
	}
	if got, ok := engine.LastDiscardedEvent(); !ok || got != StatechartMinimalEventJobCompleted {
		t.Fatalf("LastDiscardedEvent() = %v, %v, want JobCompleted: the count names "+
			"the delivery it refused", got, ok)
	}
}
