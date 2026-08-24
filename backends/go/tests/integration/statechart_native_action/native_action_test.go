// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML G.7 `<sce:action>` — Go compile+run gate for native host dispatch.
//
// The committed machine beside this file is generated from
// `sce-build/tests/fixtures/event_schema/statechart_native_action.scxml`
// (regen: `scripts/regen_native_action.sh`), the same document the Rust, C++,
// C11, Kotlin and Python channels drive. Because the tree is part of the
// module it is REALLY compiled: the generated policy takes a
// `StatechartNativeActionActions` implementation at construction and carries
// no script engine at all, so this gate proves the engine-free dispatch
// surface compiles AND that the effects actually fire — the runtime behaviour
// a byte-golden layer cannot give.
//
// What each scenario measures:
//
//   - `append_fragment_payload` reads two typed `_event.data` fields (a
//     `bytes` payload lowered to `[]byte`, a `uint32` offset lowered to
//     `uint32`) bound from the event's typed payload;
//   - `reset_slot` takes no arguments;
//   - `on_idle_entry` and `on_assembling_exit` appear in NO transition, so
//     they prove the engine-free entry/exit path and that an eventless-only
//     action still gets a generated interface method;
//   - an event raised BY NAME carries no typed payload, and the arg-bearing
//     action must not fire against a zero value it would take for data. That
//     last one is the half a configuration assertion cannot see: the machine
//     moves either way.

package statechart_native_action

import (
	"bytes"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// recorder is the host implementation of the generated operations. It records
// each dispatch so a test can assert the engine-free call path fired with the
// arguments the event carried.
type recorder struct {
	appended        [][]byte
	offsets         []uint32
	resets          int
	idleEntries     int
	assemblingExits int
}

func (r *recorder) AppendFragmentPayload(payload []byte, offset uint32) {
	// Copied: the payload the machine hands over is the one in its own
	// storage, and a test that kept the slice would be asserting on whatever
	// the next event wrote there.
	r.appended = append(r.appended, append([]byte(nil), payload...))
	r.offsets = append(r.offsets, offset)
}

func (r *recorder) ResetSlot()        { r.resets++ }
func (r *recorder) OnIdleEntry()      { r.idleEntries++ }
func (r *recorder) OnAssemblingExit() { r.assemblingExits++ }

type started struct {
	engine *sce.Engine[StatechartNativeActionState, StatechartNativeActionEvent]
	host   *recorder
}

func newStarted() started {
	host := &recorder{}
	policy := NewStatechartNativeActionPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartNativeActionState, StatechartNativeActionEvent](&policy)
	engine.Initialize()
	return started{engine: engine, host: host}
}

func TestNativeActionDispatchesTypedPayloadToHostInterface(t *testing.T) {
	s := newStarted()

	if got := s.engine.GetCurrentState(); got != StatechartNativeActionStateIdle {
		t.Fatalf("initial state = %v, want Idle", got)
	}
	// `<onentry>` of the initial state fires on entry — the engine-free
	// entry-effect path, with no transition having to carry the action.
	if s.host.idleEntries != 1 {
		t.Fatalf("on_idle_entry fired %d times on the initial entry, want 1", s.host.idleEntries)
	}

	// Per-event typed inject: deliver `fragment.received` with a bytes payload
	// and an offset. The transition fires `append_fragment_payload`.
	RaiseFragmentReceived(s.engine, []byte("abc"), 7)
	s.engine.Step()

	if got := s.engine.GetCurrentState(); got != StatechartNativeActionStateAssembling {
		t.Fatalf("state after fragment.received = %v, want Assembling", got)
	}
	if len(s.host.appended) != 1 || !bytes.Equal(s.host.appended[0], []byte("abc")) {
		t.Fatalf("append_fragment_payload received %q, want one call with \"abc\"", s.host.appended)
	}
	if len(s.host.offsets) != 1 || s.host.offsets[0] != 7 {
		t.Fatalf("append_fragment_payload offsets = %v, want [7]", s.host.offsets)
	}

	// `reset` fires the no-argument `reset_slot` and returns to idle. Exiting
	// `assembling` fires its `<onexit>` effect; re-entering `idle` fires
	// `<onentry>` a second time.
	s.engine.RaiseExternalByName("reset", "")
	s.engine.Step()

	if got := s.engine.GetCurrentState(); got != StatechartNativeActionStateIdle {
		t.Fatalf("state after reset = %v, want Idle", got)
	}
	if s.host.resets != 1 {
		t.Fatalf("reset_slot fired %d times, want 1", s.host.resets)
	}
	if s.host.assemblingExits != 1 {
		t.Fatalf("on_assembling_exit fired %d times, want 1", s.host.assemblingExits)
	}
	if s.host.idleEntries != 2 {
		t.Fatalf("on_idle_entry fired %d times after returning to idle, want 2", s.host.idleEntries)
	}
}

// An event raised by NAME carries no typed payload. The transition still
// fires — the guard is the event name — but the arg-bearing action has
// nothing to read, and handing the host a zeroed buffer it would take for
// data is the one outcome this seam must never produce.
//
// Asserted on the host's record AND on the configuration: the record says the
// host was not handed a value it would take for data, and `faulted` says the
// machine reported it rather than swallowing it. §scxml-3.12.2 makes the second
// half a contract — `error.execution` covers errors "arising from expression
// evaluation", and the processor MUST place it on the internal event queue.
func TestNativeActionDoesNotFireWithoutItsTypedPayload(t *testing.T) {
	s := newStarted()

	s.engine.RaiseExternalByName("fragment.received", "")
	s.engine.Step()

	if len(s.host.appended) != 0 {
		t.Fatalf("append_fragment_payload fired %d times without a typed payload, want 0", len(s.host.appended))
	}
	if got := s.engine.GetCurrentState(); got != StatechartNativeActionStateFaulted {
		t.Fatalf("state after untyped fragment.received = %v, want Faulted", got)
	}
	// The eventless effects still ran: they read no payload, so nothing about
	// this delivery should have stopped them.
	if s.host.idleEntries != 1 {
		t.Fatalf("on_idle_entry fired %d times, want 1", s.host.idleEntries)
	}
	if s.host.assemblingExits != 1 {
		t.Fatalf("on_assembling_exit fired %d times, want 1", s.host.assemblingExits)
	}
}

// The same arm, reached with NO host mistake at all: `<raise
// event="fragment.received"/>` is legal SCXML this generator accepts, and a
// raise carries no typed payload. The host's only act is delivering
// `selftest`; everything after it is the document's own doing.
func TestNativeActionAnswersADocumentRaisedEventWithErrorExecution(t *testing.T) {
	s := newStarted()

	s.engine.RaiseExternalByName("selftest", "")
	s.engine.Step()

	if len(s.host.appended) != 0 {
		t.Fatalf("append_fragment_payload fired %d times on a document raise, want 0", len(s.host.appended))
	}
	if got := s.engine.GetCurrentState(); got != StatechartNativeActionStateFaulted {
		t.Fatalf("state after a document-raised fragment.received = %v, want Faulted", got)
	}
	if s.host.idleEntries != 1 {
		t.Fatalf("on_idle_entry fired %d times, want 1", s.host.idleEntries)
	}
}
