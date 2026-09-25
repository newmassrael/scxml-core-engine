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
	"strings"
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

	// The fixture counts a completion only when its `_event.invokeid` names the
	// invocation (§scxml-5.10.1), so each counter is that assertion too.
	if got := s.counter(t, "started"); got != 1 {
		t.Fatalf("done.invoke.probe never reached the document, or arrived without its invokeid: started = %d", got)
	}
	if got := s.counter(t, "started2"); got != 1 {
		t.Fatalf("done.invoke.probe2 never reached the document, or arrived without its invokeid: started2 = %d", got)
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

	if len(log) != 2 {
		t.Fatalf("invoker calls: %v", log)
	}
	// `src` and `<param>` are how §scxml-6.4.1 lets the document say WHAT to
	// invoke and with what. A request carrying neither would let a document
	// name an invocation it cannot describe. Each invocation is started as
	// itself: `probe2` begins with `probe`, and a dispatch that matched by
	// substring started `probe` twice.
	want := fmt.Sprintf("START id=probe type=%s src=pane://turn within=[2500]", declaredType)
	if log[0] != want {
		t.Fatalf("the start request lost part of what the document wrote:\n got %q\nwant %q", log[0], want)
	}
	want2 := fmt.Sprintf("START id=probe2 type=%s src=pane://other within=[]", declaredType)
	if log[1] != want2 {
		t.Fatalf("the second invocation was not started as itself:\n got %q\nwant %q", log[1], want2)
	}
}

// §scxml-6.4.1: `srcexpr`, `namelist`, `<param expr>` and `<content expr>` are
// read from the data model when the invocation starts. The request used to
// carry the literal params alone, so a document that computed what to invoke
// handed the host an empty description.
func TestWhatTheRequestSaysIsEvaluatedWhenTheInvocationStarts(t *testing.T) {
	var starts []sce.HostInvokeRequest
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, func(ev sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if ev.Start != nil {
			starts = append(starts, *ev.Start)
		}
		return nil
	})
	s.engine.Initialize()
	s.engine.Step()
	starts = nil // `probe` / `probe2`, which the case above already reads
	s.engine.ProcessEvent(StatechartHostInvokerEventEvaluate)

	// `req3`'s srcexpr cannot be evaluated, so it is never started.
	if len(starts) != 2 || starts[0].InvokeID != "req" || starts[1].InvokeID != "req2" {
		t.Fatalf("started: %+v", starts)
	}
	if got := starts[0].Src; got != "pane://dyn" {
		t.Fatalf("srcexpr was not evaluated: %q", got)
	}
	// A repeated name keeps both values in document order; the `<param>` that
	// failed is absent (§scxml-5.7.1) while the invocation still started.
	if got := fmt.Sprint(starts[0].Params); got != "map[n:[7] twice:[a 8]]" {
		t.Fatalf("params: %s", got)
	}
	if got := starts[1].Content; got != "body:7" {
		t.Fatalf("content: %q", got)
	}
	// One error.execution for the dropped `<param>`, one for `req3`.
	if got := s.counter(t, "dropped"); got != 2 {
		t.Fatalf("a failed evaluation was not reported: dropped = %d", got)
	}
}

// The invocation ends with the state that started it. Without this the host is
// told to begin work and never told to stop — which no configuration assertion
// can detect, because the machine looks correct either way.
func TestLeavingTheStateCancelsTheInvocation(t *testing.T) {
	var log []string
	var starts []hostStart
	s := newStarted()
	// Still running when the state exits — a completed invocation has nothing
	// left to cancel (TestACompletedInvocationIsNotCancelled).
	s.engine.RegisterInvoker(declaredType, runningInvoker(&log, &starts))
	s.engine.Initialize()
	s.engine.Step()
	s.engine.ProcessEvent(StatechartHostInvokerEventLeave)

	if got := s.counter(t, "ended"); got != 1 {
		t.Fatalf("the machine never left the invoking state: ended = %d", got)
	}
	// Both invocations end with the state, each told once.
	for _, id := range []string{"probe", "probe2"} {
		cancels := 0
		for _, e := range log {
			if e == "CANCEL id="+id {
				cancels++
			}
		}
		if cancels != 1 {
			t.Fatalf("cancel for %s reached the invoker %d times: %v", id, cancels, log)
		}
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
	var starts []hostStart
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, runningInvoker(&log, &starts))

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

// hostStart is one start the running invoker saw.
type hostStart struct {
	invokeID string
	token    uint64
}

// runningInvoker's work outlives the call: it answers nothing on Start, so each
// invocation stays running until the test completes or cancels it. It records
// the lines recordingInvoker does and every start's token for a later
// CompleteHostInvoke.
func runningInvoker(log *[]string, starts *[]hostStart) sce.HostInvokeHandler {
	return func(ev sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if ev.Start != nil {
			*log = append(*log, "START id="+ev.Start.InvokeID)
			*starts = append(*starts, hostStart{ev.Start.InvokeID, ev.Start.Token})
		}
		if ev.Cancel != nil {
			*log = append(*log, "CANCEL id="+ev.Cancel.InvokeID)
		}
		return nil
	}
}

// tokenOf returns the token of the latest start of invokeID.
func tokenOf(t *testing.T, starts []hostStart, invokeID string) uint64 {
	t.Helper()
	for i := len(starts) - 1; i >= 0; i-- {
		if starts[i].invokeID == invokeID {
			return starts[i].token
		}
	}
	t.Fatalf("`%s` never started", invokeID)
	return 0
}

// cancelsIn lists the CANCEL lines of log, in order.
func cancelsIn(log []string) []string {
	var out []string
	for _, e := range log {
		if strings.HasPrefix(e, "CANCEL") {
			out = append(out, e)
		}
	}
	return out
}

// §scxml-6.4: `done.invoke` says the invoked process is over, so leaving the
// state afterwards has nothing to stop. Both invocations here complete
// synchronously; neither is cancelled.
func TestACompletedInvocationIsNotCancelled(t *testing.T) {
	var log []string
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, recordingInvoker(&log))
	s.engine.Initialize()
	s.engine.Step()
	s.engine.ProcessEvent(StatechartHostInvokerEventLeave)

	if got := s.counter(t, "started"); got != 1 {
		t.Fatalf("started = %d", got)
	}
	if got := cancelsIn(log); len(got) != 0 {
		t.Fatalf("a completed invocation was cancelled: %v", log)
	}
}

// A host that finishes later reports it with the start's token, and the
// completion is taken once: a second report of the same run finds nothing, and
// the state's exit then cancels only the invocation still running.
func TestALateCompletionIsAcceptedExactlyOnce(t *testing.T) {
	var log []string
	var starts []hostStart
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, runningInvoker(&log, &starts))
	s.engine.Initialize()
	s.engine.Step()
	if got := s.counter(t, "started"); got != 0 {
		t.Fatalf("started before any completion: %d", got)
	}

	token := tokenOf(t, starts, "probe")
	if !s.engine.CompleteHostInvoke(declaredType, "probe", token, "ok") {
		t.Fatal("a running invocation's completion was refused")
	}
	s.engine.Step()
	// The fixture counts it only when `_event.invokeid` names the invocation
	// (§scxml-5.10.1), so this is that assertion too.
	if got := s.counter(t, "started"); got != 1 {
		t.Fatalf("started = %d", got)
	}
	if s.engine.CompleteHostInvoke(declaredType, "probe", token, "again") {
		t.Fatal("the same run completed twice")
	}
	s.engine.Step()
	if got := s.counter(t, "started"); got != 1 {
		t.Fatalf("started = %d after a second completion", got)
	}

	s.engine.ProcessEvent(StatechartHostInvokerEventLeave)
	if got := cancelsIn(log); len(got) != 1 || got[0] != "CANCEL id=probe2" {
		t.Fatalf("only the invocation still running is cancelled: %v", got)
	}
}

// §scxml-6.4: once the state has exited, what the cancelled process sends is
// ignored. The host's reply arrives after the cancel and is refused.
func TestACompletionAfterTheCancelIsRefused(t *testing.T) {
	var log []string
	var starts []hostStart
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, runningInvoker(&log, &starts))
	s.engine.Initialize()
	s.engine.Step()
	token := tokenOf(t, starts, "probe")
	s.engine.ProcessEvent(StatechartHostInvokerEventLeave)

	if s.engine.CompleteHostInvoke(declaredType, "probe", token, "late") {
		t.Fatal("a cancelled run's completion was accepted")
	}
	s.engine.Step()
	if got := s.counter(t, "started"); got != 0 {
		t.Fatalf("started = %d", got)
	}
}

// Re-entering the state starts the same `<invoke>` again under the same id. The
// first run's late reply carries the first start's token and is refused; the
// second run's is accepted.
func TestARestartedInvokeRefusesTheFirstRunsReply(t *testing.T) {
	var log []string
	var starts []hostStart
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, runningInvoker(&log, &starts))
	s.engine.Initialize()
	s.engine.Step()
	first := tokenOf(t, starts, "probe")
	s.engine.ProcessEvent(StatechartHostInvokerEventLeave)
	s.engine.ProcessEvent(StatechartHostInvokerEventAgain)
	second := tokenOf(t, starts, "probe")
	if first == second {
		t.Fatal("a restart reused the first start's token")
	}

	if s.engine.CompleteHostInvoke(declaredType, "probe", first, "stale") {
		t.Fatal("the first run's reply was taken for the second run's")
	}
	s.engine.Step()
	if got := s.counter(t, "started"); got != 0 {
		t.Fatalf("started = %d after a stale reply", got)
	}
	if !s.engine.CompleteHostInvoke(declaredType, "probe", second, "ok") {
		t.Fatal("the second run's completion was refused")
	}
	s.engine.Step()
	if got := s.counter(t, "started"); got != 1 {
		t.Fatalf("started = %d", got)
	}
}

// A host-run invocation's `done.invoke` raised through the ordinary
// external-event API skipped the running check, so the engine refuses it and
// counts the refusal. The metadata names the invocation, so without the
// refusal the fixture's guarded transition would take it.
func TestADoneInvokeRaisedTheOldWayIsRefusedAndCounted(t *testing.T) {
	var log []string
	var starts []hostStart
	s := newStarted()
	s.engine.RegisterInvoker(declaredType, runningInvoker(&log, &starts))
	s.engine.Initialize()
	s.engine.Step()

	s.engine.RaiseExternalByNameWithMeta("done.invoke.probe", sce.EventMetadata{InvokeID: "probe"})
	s.engine.Step()

	if got := s.counter(t, "started"); got != 0 {
		t.Fatalf("a completion that skipped the running check reached the document: started = %d", got)
	}
	if got := s.engine.RefusedHostInvokeCompletions(); got != 1 {
		t.Fatalf("refused completions = %d", got)
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

	// One error.execution per invocation nobody ran.
	if got := s.counter(t, "refused"); got != 2 {
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
	if got := s.counter(t, "refused"); got != 2 {
		t.Fatalf("the unregistered type was not reported: refused = %d", got)
	}
	if len(log) != 0 {
		t.Fatalf("the other type's invoker was called: %v", log)
	}
}
