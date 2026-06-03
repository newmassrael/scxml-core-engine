// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC rfc-eventschema-bytes-guard.md §6 — Go compile+run gate for the
// bytes-field EventSchema guard, the twin of the Rust / Python / C11
// bytes integration tests.
//
// The committed SM (statechart_bytes_sm.go) is generated from
// sce-build/tests/fixtures/event_schema/statechart_bytes.scxml
// (regen: scripts/regen_event_schema_native_go.sh). Go forbids `==` on a
// `[]byte` slice, so the guard `cond="_event.data.raw === 'ack'"` lowers to
// `string(p.pending….raw) == "ack"` — a conversion this test really exercises
// at runtime (a non-match must NOT fire). No `ScriptEngine` is attached: a
// typed-guard machine links no Lua, the MCU-relevant property.

package statechart_bytes

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestBytesPayloadGuardFiresOnMatch(t *testing.T) {
	policy := NewStatechartBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartBytesState, StatechartBytesEvent](&policy)
	engine.Initialize()

	if got := engine.GetCurrentState(); got != StatechartBytesStateWaiting {
		t.Fatalf("initial state = %v, want Waiting", got)
	}

	RaiseSignalReceived(engine, []byte("ack"))
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartBytesStateDone {
		t.Fatalf("after RaiseSignalReceived([]byte(\"ack\")): state = %v, want Done "+
			"(raw == \"ack\" must fire the native bytes guard)", got)
	}
}

func TestBytesPayloadGuardMissesOnNonmatch(t *testing.T) {
	policy := NewStatechartBytesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartBytesState, StatechartBytesEvent](&policy)
	engine.Initialize()

	RaiseSignalReceived(engine, []byte("no"))
	engine.Step()

	if got := engine.GetCurrentState(); got != StatechartBytesStateWaiting {
		t.Fatalf("after RaiseSignalReceived([]byte(\"no\")): state = %v, want Waiting "+
			"(raw == \"no\" must leave the machine in waiting)", got)
	}
}
