// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.1 — Go compile+run gate for an `<invoke type>` the HOST runs.
//
// The clause leaves the invokable set to the platform in the same words
// §scxml-6.2.5 uses for `<send>`, so the set is open by design. SCE implemented
// the SCXML processor and refused everything else with `error.execution`. The
// send half of that gap was repaid across six backends; this one stayed
// Rust-only, and the generator refused `--host-invoker` for Go by name rather
// than emit a start nothing could service.
//
// The refusal was honest, which is what made it a coverage debt rather than a
// silent drop. Now the Go runtime carries the registry and this file is the
// channel that says so.
//
// An invoke is not a send: it has a LIFETIME. Four scenarios hold the four
// outcomes apart, because the configuration alone cannot:
//
//   - a registered invoker is STARTED with what the document wrote;
//   - leaving the state CANCELS it — the half no configuration assertion can
//     see, because the machine looks correct whether or not the host was told
//     to stop;
//   - a cancel is delivered once, and only for an invocation that started;
//   - a declared type with nothing registered raises `error.execution`.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml
// (canonical, shared with the Rust channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_processor.sh

package statechart_host_invoker

import (
	"fmt"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// The type the fixture was compiled for. `scripts/regen_host_processor.sh`
// passes this same string to `--host-invoker`; a test registering a different
// one would measure nothing and pass, so the `refused` counter is asserted
// rather than the registration trusted.
const declaredType = "x-sce-host"

type started struct {
	engine *sce.Engine[StatechartHostInvokerState, StatechartHostInvokerEvent]
	policy *StatechartHostInvokerPolicy
}

func newStarted() started {
	policy := NewStatechartHostInvokerPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[StatechartHostInvokerState, StatechartHostInvokerEvent](&policy)
	return started{engine: engine, policy: &policy}
}

// The fixture's `<assign>`s are the only witness: several of these outcomes
// leave the machine in the same state, so the configuration cannot tell them
// apart.
func (s started) counter(t *testing.T, name string) int64 {
	t.Helper()
	value, ok := sce.ReadDatamodelInt(s.policy.ScriptEngine, s.policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares `%s` in its datamodel and the machine could not read it", name)
	}
	return value
}

// A recording invoker. Answers a completion on Start so the `done.invoke` path
// is exercised too, and records both arms so the ORDER is assertable.
func recordingInvoker(log *[]string) sce.HostInvokeHandler {
	return func(ev sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if ev.Start != nil {
			*log = append(*log, fmt.Sprintf("START id=%s type=%s src=%s within=%v",
				ev.Start.InvokeID, ev.Start.ProcessorType, ev.Start.Src,
				ev.Start.Params["within"]))
			done := "ok"
			return &sce.HostInvokeResponse{DoneData: &done}
		}
		if ev.Cancel != nil {
			*log = append(*log, fmt.Sprintf("CANCEL id=%s", ev.Cancel.InvokeID))
		}
		return nil
	}
}

func TestARegisteredInvokerIsStartedWithWhatTheDocumentWrote(t *testing.T) {
	var log []string
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, recordingInvoker(&log))
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "started"); got != 1 {
		t.Fatalf("done.invoke never reached the document: started = %d", got)
	}
	if got := s.counter(t, "refused"); got != 0 {
		t.Fatalf("a started invocation also raised error.execution: refused = %d", got)
	}
	// The false-positive guard: ordinary entry content must still run. Without
	// it a change that broke the entry chain while leaving the invoke arm
	// working would read as a pass.
	if got := s.counter(t, "entered"); got != 1 {
		t.Fatalf("the entry chain stopped running: entered = %d", got)
	}

	if len(log) != 1 {
		t.Fatalf("invoker calls: %v", log)
	}
	// `src` and `<param>` are how §scxml-6.4.1 lets the document say WHAT to
	// invoke and with what. A request carrying neither would let a document
	// name an invocation it cannot describe.
	want := fmt.Sprintf("START id=probe type=%s src=pane://turn within=[2500]", declaredType)
	if log[0] != want {
		t.Fatalf("the start request lost part of what the document wrote:\n got %q\nwant %q", log[0], want)
	}
}

// The invocation ends with the state that started it. Without this the host is
// told to begin work and never told to stop — which no configuration assertion
// can detect, because the machine looks correct either way.
func TestLeavingTheStateCancelsTheInvocation(t *testing.T) {
	var log []string
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, recordingInvoker(&log))
	s.engine.Initialize()
	s.engine.Step()
	s.engine.ProcessEvent(StatechartHostInvokerEventLeave)

	if got := s.counter(t, "ended"); got != 1 {
		t.Fatalf("the machine never left the invoking state: ended = %d", got)
	}
	if len(log) == 0 || log[len(log)-1] != "CANCEL id=probe" {
		t.Fatalf("no cancel reached the invoker: %v", log)
	}
}

// A cancel is delivered once, and only for an invocation that started.
//
// The engine, not the emitted code, owns that judgement: the exit chain calls
// CancelHostInvoke unconditionally, so if the engine did not track what
// started, a state that exits before its macrostep settles would have the host
// tearing down work it never began.
//
// Asserted at the engine surface rather than through the fixture, for the
// reason the Rust channel records: driving the machine cannot produce the
// "never started" case, because every host call that advances it runs a
// macrostep and the pending invoke executes at the end of that macrostep.
func TestCancelIsNotDeliveredForAnInvocationThatNeverStarted(t *testing.T) {
	var log []string
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, recordingInvoker(&log))

	if s.engine.CancelHostInvoke(declaredType, "probe") {
		t.Fatal("a cancel was reported for an invocation that never started")
	}
	if len(log) != 0 {
		t.Fatalf("the invoker was called for an invocation that never started: %v", log)
	}

	// Now let one start, cancel it, and cancel again: the second call has
	// nothing left to do. A registry that answered twice would have the host
	// tear down the same work twice.
	s.engine.Initialize()
	s.engine.Step()
	if !s.engine.CancelHostInvoke(declaredType, "probe") {
		t.Fatal("a started invocation reported nothing to cancel")
	}
	if s.engine.CancelHostInvoke(declaredType, "probe") {
		t.Fatal("the same invocation was cancelled twice")
	}
	cancels := 0
	for _, e := range log {
		if len(e) >= 6 && e[:6] == "CANCEL" {
			cancels++
		}
	}
	if cancels != 1 {
		t.Fatalf("cancel reached the invoker %d times: %v", cancels, log)
	}
}

// The other half. The build declared the type, so codegen emitted a start —
// but nothing was registered, so no process was run. Same event as an
// unsupported type, because from the document's side it is the same fact.
//
// This is the scenario that keeps the repair honest: without it the feature
// could start nothing and the document would proceed as though its process
// were running.
func TestADeclaredTypeWithNoInvokerStillRaisesErrorExecution(t *testing.T) {
	s := newStarted()
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "refused"); got != 1 {
		t.Fatalf("an unregistered invoker was silently treated as started: refused = %d", got)
	}
	if got := s.counter(t, "started"); got != 0 {
		t.Fatalf("done.invoke arrived for an invocation nobody ran: started = %d", got)
	}
}

// Registering some other type does not run this one. The registry is keyed,
// and a lookup that fell back to "any invoker" would hand a document's process
// to one it never named.
func TestAnInvokerRegisteredForAnotherTypeDoesNotRunThisOne(t *testing.T) {
	var log []string
	s := newStarted()
	s.engine.RegisterInvoker("x-some-other-host", recordingInvoker(&log))
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "started"); got != 0 {
		t.Fatalf("an invoker for a different type ran this one: started = %d", got)
	}
	if got := s.counter(t, "refused"); got != 1 {
		t.Fatalf("the unregistered type was not reported: refused = %d", got)
	}
	if len(log) != 0 {
		t.Fatalf("the other type's invoker was called: %v", log)
	}
}
