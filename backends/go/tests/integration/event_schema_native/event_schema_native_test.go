// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// `TypedPayload` carrier round-trip, and the per-event `RaiseJobCompleted`
// inject seam are really type-checked.
//
// The transition guard `cond="_event.data.elapsed_ms === 0"` lowers to a
// native `p.pendingPayloadTag == … && (…)` tag-checked field comparison with
// NO script engine, so the policy is constructed WITHOUT a `ScriptEngine`
// (the MCU-relevant property: a typed-guard machine links no Lua). The
// per-event `RaiseJobCompleted` seam binds the event name and the payload
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
	RaiseJobCompleted(engine, 0)
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartMinimalStateDone {
		t.Fatalf("after RaiseJobCompleted(0): state = %v, want Done "+
			"(elapsed_ms == 0 must fire the native typed guard)", got)
	}
}

func TestTypedPayloadGuardMissesOnNonzero(t *testing.T) {
	policy := NewStatechartMinimalPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartMinimalState, StatechartMinimalEvent](&policy)
	engine.Initialize()

	// Same event, a payload the guard rejects — the machine stays put.
	RaiseJobCompleted(engine, 5)
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartMinimalStateWaiting {
		t.Fatalf("after RaiseJobCompleted(5): state = %v, want Waiting "+
			"(elapsed_ms == 5 must leave the machine in waiting)", got)
	}
}
