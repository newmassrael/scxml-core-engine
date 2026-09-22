// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A — the OTHER carrier, the Go twin of the Rust
// `event_schema_native.rs` lifted cases, the Kotlin `EventSchemaLiftedTest`,
// the Python `test_event_schema_lifted.py`, the C11
// `c11_integration_event_schema_lifted` and the C++ `EventSchemaLiftedAotTest`.
//
// The committed SM (statechart_lifted_sm.go) is generated from
// sce-build/tests/fixtures/event_schema/statechart_lifted.scxml
// (regen: scripts/regen_event_schema_native_go.sh).
//
// A schema'd event's typed payload is filled by ONE producer: the generated
// `RaiseJobCompleted` seam. Every other producer — `<send>` with `<param>`, an
// invoke forwarding an event either way, autoforward, BasicHTTP, mesh — fills
// `EventMetadata.Data`, and until 2026-09-22 a natively lowered guard could
// read nothing but the typed carrier. The same guard therefore answered
// differently depending on where its event came from.
//
// What a refusal does is not a policy chosen here: it is what the SCRIPT
// ENGINE answers for the same guard on the same data (W3C SCXML 3.13, measured
// on this document). A native lowering that answered differently would make
// the optimisation observable, which is the one thing it may not be.

package statechart_lifted

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// A machine with no script engine — the fixture lowers its guard natively, so
// there is none to construct.
func newLifted(t *testing.T) (*sce.Engine[StatechartLiftedState, StatechartLiftedEvent], *StatechartLiftedPolicy) {
	t.Helper()
	policy := NewStatechartLiftedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartLiftedState, StatechartLiftedEvent](&policy)
	engine.Initialize()
	if got := engine.GetCurrentState(); got != StatechartLiftedStateWaiting {
		t.Fatalf("initial state = %v, want Waiting", got)
	}
	return engine, &policy
}

// The event as every producer but the inject seam delivers it: its fields on
// the `Data` wire, with no typed payload riding along.
func deliverData(engine *sce.Engine[StatechartLiftedState, StatechartLiftedEvent], data string) {
	engine.RaiseExternalWithMeta(sce.EventWithMetadata[StatechartLiftedEvent]{
		Event: StatechartLiftedEventJobCompleted,
		Metadata: sce.EventMetadata{
			EventType: sce.EventTypeExternal,
			Data:      data,
		},
	})
	engine.Step()
}

func TestPayloadOnTheDataWireFiresTheSameGuard(t *testing.T) {
	engine, _ := newLifted(t)

	deliverData(engine, `{"elapsed_ms": 0}`)

	if got := engine.GetCurrentState(); got != StatechartLiftedStateDone {
		t.Fatalf("state = %v, want Done: a payload that arrived on the Data wire "+
			"must satisfy the same native guard the inject seam's typed payload does", got)
	}
}

func TestInjectSeamStillFiresItsOwnGuard(t *testing.T) {
	engine, _ := newLifted(t)

	RaiseJobCompleted(engine, 0)
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartLiftedStateDone {
		t.Fatalf("state = %v, want Done: the typed inject seam must still fire "+
			"the guard it was built for", got)
	}
}

func TestValueOfAnotherTypeIsRefusedAsTheScriptEngineRefusesIt(t *testing.T) {
	engine, _ := newLifted(t)

	deliverData(engine, `{"elapsed_ms": "nought"}`)

	if got := engine.GetCurrentState(); got != StatechartLiftedStateRefused {
		t.Fatalf("state = %v, want Refused: a text where the schema declares a "+
			"number must raise error.execution and leave the guard unfired", got)
	}
}

func TestEventWithNoDataIsRefusedTheSameWay(t *testing.T) {
	engine, _ := newLifted(t)

	deliverData(engine, "")

	if got := engine.GetCurrentState(); got != StatechartLiftedStateRefused {
		t.Fatalf("state = %v, want Refused: an event carrying no data cannot "+
			"answer a guard that reads a field of it", got)
	}
}

func TestFieldTheDataDoesNotNameIsRefused(t *testing.T) {
	engine, _ := newLifted(t)

	deliverData(engine, `{"other": 0}`)

	if got := engine.GetCurrentState(); got != StatechartLiftedStateRefused {
		t.Fatalf("state = %v, want Refused: data that names none of the schema's "+
			"fields cannot answer the guard", got)
	}
}

func TestPayloadTheGuardRejectsIsNotAnError(t *testing.T) {
	engine, _ := newLifted(t)

	// The payload reads perfectly; the comparison is simply false. Nothing
	// failed, so nothing is raised — the machine waits.
	deliverData(engine, `{"elapsed_ms": 5}`)

	if got := engine.GetCurrentState(); got != StatechartLiftedStateWaiting {
		t.Fatalf("state = %v, want Waiting: a well-typed payload the guard "+
			"rejects must not route the machine to the error handler", got)
	}
}
