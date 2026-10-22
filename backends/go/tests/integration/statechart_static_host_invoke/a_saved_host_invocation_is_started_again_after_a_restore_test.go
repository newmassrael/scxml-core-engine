// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring": an `<invoke>` the HOST runs
// is part of what a machine is doing (§scxml-6.4.1), and the process that ran it
// is gone with the process that saved it. A saved state holds each running
// host-run invocation with the request it was started with and the deadline it
// had left, and a restore starts it again from that request, never evaluating
// the element a second time, with `Restarted` set, a new token and what was left
// of the deadline.
//
// `statechart_static_host_invoke.scxml` hands a job to the host from `working`.
// `done.invoke.h` and `error.invoke.h` each leave it and record a digit in
// `seen`, so the number says which arrived. The Rust and Kotlin halves are
// `a_saved_host_invocation_is_started_again_after_a_restore.rs` and
// `ASavedHostInvocationIsStartedAgainAfterARestoreTest.kt`.
//
// Driven entirely on a manual clock: no case sleeps, and none can be decided by
// how loaded the build machine is. The shared instance
// `saved/statechart_static_host_invoke_running.json` is the format-2 text every
// backend writes (docs/adr/0045); the format-1 text of the same run, under
// `saved/format1/`, still restores.

package statechart_static_host_invoke

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

const (
	hostType = "x-sce-host"
	// savedAtMs is the wall-clock moment the shared instance was saved at.
	savedAtMs int64 = 1_700_000_000_000
)

type Engine = sce.Engine[StatechartStaticHostInvokeState, StatechartStaticHostInvokeEvent]

func sharedText(t *testing.T, format1 bool) string {
	t.Helper()
	root := filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "host_processor", "saved")
	if format1 {
		root = filepath.Join(root, "format1")
	}
	raw, err := os.ReadFile(filepath.Join(root, "statechart_static_host_invoke_running.json"))
	if err != nil {
		t.Fatalf("the shared saved state: %v", err)
	}
	return strings.TrimSpace(string(raw))
}

// told is what the host was told, in order: a start or a cancel.
type told struct {
	start  *sce.HostInvokeRequest
	cancel *sce.HostInvokeCancel
}

type host struct{ log []told }

func (h *host) register(engine *Engine) {
	engine.RegisterInvoker(hostType, func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		switch {
		case event.Start != nil:
			h.log = append(h.log, told{start: event.Start})
		case event.Cancel != nil:
			h.log = append(h.log, told{cancel: event.Cancel})
		}
		return nil
	})
}

func (h *host) starts() []sce.HostInvokeRequest {
	var starts []sce.HostInvokeRequest
	for _, entry := range h.log {
		if entry.start != nil {
			starts = append(starts, *entry.start)
		}
	}
	return starts
}

// sameRequest is whether two requests say the same thing: everything but the token
// the engine assigns and the mark a restart carries.
func sameRequest(a, b sce.HostInvokeRequest) bool {
	return a.ProcessorType == b.ProcessorType && a.InvokeID == b.InvokeID && a.Src == b.Src &&
		reflect.DeepEqual(a.Params, b.Params) && a.EventData == b.EventData && a.Content == b.Content
}

// working is a machine standing in `working`, its invocation started with token 0,
// given the session id the shared format-2 fixtures name their root by.
func working(t *testing.T) (*Engine, *StatechartStaticHostInvokePolicy, *host) {
	t.Helper()
	policy := NewStatechartStaticHostInvokePolicy()
	if err := policy.SetSessionID("fixture-root"); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[StatechartStaticHostInvokeState, StatechartStaticHostInvokeEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	log := &host{}
	log.register(engine)
	engine.Initialize()
	engine.RaiseExternalByName("start", "")
	engine.Tick()
	return engine, &policy, log
}

func tryRestore(text string, elapsedMs int64) (*Engine, *StatechartStaticHostInvokePolicy, *host, error) {
	saved, err := sce.SavedStateFromJSON(text)
	if err != nil {
		return nil, nil, nil, err
	}
	policy := NewStatechartStaticHostInvokePolicy()
	engine, err := StatechartStaticHostInvokeRestoreAt(&policy, saved, sce.NewManualClock(0), savedAtMs+elapsedMs)
	return engine, &policy, &host{}, err
}

// restored is text restored elapsedMs of wall-clock time after it was saved, its
// host invoker registered the way a Go host does it: on the engine the restore
// returned.
func restored(t *testing.T, text string, elapsedMs int64) (*Engine, *StatechartStaticHostInvokePolicy, *host) {
	t.Helper()
	engine, policy, log, err := tryRestore(text, elapsedMs)
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	log.register(engine)
	return engine, policy, log
}

func standingIn(engine *Engine) []StatechartStaticHostInvokeState {
	return engine.GetActiveStates()
}

func TestAMachineIsSavedWithTheRequestItsInvocationWasStartedWith(t *testing.T) {
	engine, _, log := working(t)
	first := log.starts()[0]
	if first.Restarted {
		t.Error("the document's own start is not a restart")
	}
	if first.Token != 0 {
		t.Errorf("the first token is %d", first.Token)
	}

	saved, err := StatechartStaticHostInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatalf("saves: %v", err)
	}
	if len(saved.HostInvokes) != 1 {
		t.Fatalf("one running invocation: %+v", saved.HostInvokes)
	}
	entry := saved.HostInvokes[0]
	if entry.ProcessorType != hostType || entry.InvokeID != "h" || entry.Src != first.Src ||
		entry.Content != first.Content || entry.Data != first.EventData {
		t.Errorf("the request the invocation was started with: %+v against %+v", entry, first)
	}
	// The deadline is the invocation's own, not a send waiting to be delivered.
	if entry.Due == nil || *entry.Due != savedAtMs+5000 {
		t.Errorf("due is %v", entry.Due)
	}
	if len(saved.Pending) != 0 {
		t.Errorf("%+v", saved.Pending)
	}
	// One start has been made, so the next token is 1.
	if saved.HostInvokeToken != 1 {
		t.Errorf("the next token is %d", saved.HostInvokeToken)
	}
	// The text every backend writes for the same machine, byte for byte.
	if got, want := saved.ToJSON(), sharedText(t, false); got != want {
		t.Errorf("saved as\n%s\nthe shared text is\n%s", got, want)
	}
}

func TestARestoreAsksNothingOfTheHostUntilTheMachineIsDriven(t *testing.T) {
	// From the format-2 text, and from the format-1 text of the same run.
	for _, text := range []string{sharedText(t, false), sharedText(t, true)} {
		engine, _, log := restored(t, text, 1000)
		if len(log.log) != 0 {
			t.Error("the restore itself calls nobody")
		}
		if !reflect.DeepEqual(standingIn(engine), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateWorking}) {
			t.Errorf("standing in %v", standingIn(engine))
		}
		// The first macrostep starts it, where an invocation the document entered
		// would have started.
		engine.Tick()
		if len(log.starts()) != 1 {
			t.Errorf("one start: %d", len(log.starts()))
		}
	}
}

func TestARestoredInvocationIsStartedAgainFromTheRequestItWasSavedWith(t *testing.T) {
	_, _, original := working(t)
	first := original.starts()[0]

	engine, _, log := restored(t, sharedText(t, false), 1000)
	engine.Tick()
	all := log.starts()
	if len(all) != 1 {
		t.Fatalf("one start: %+v", all)
	}
	again := all[0]
	if !again.Restarted {
		t.Error("the host is told this is a resumption")
	}
	if again.Token != 1 {
		t.Errorf("the next token the saved state carried on is 1, not %d", again.Token)
	}
	if !sameRequest(again, first) {
		t.Errorf("the same request, not one evaluated again:\n%+v\n%+v", again, first)
	}
}

func TestARestoredInvocationKeepsTheDeadlineItHadLeft(t *testing.T) {
	// Saved with 5000 ms to run, restored 1000 ms later: 4000 ms are left.
	engine, policy, log := restored(t, sharedText(t, false), 1000)
	engine.Tick()
	engine.AdvanceTimeMs(3999)
	if policy.Seen() != 0 {
		t.Errorf("not due yet: seen is %d", policy.Seen())
	}
	if !reflect.DeepEqual(standingIn(engine), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateWorking}) {
		t.Errorf("standing in %v", standingIn(engine))
	}
	engine.AdvanceTimeMs(1)
	if policy.Seen() != 2 {
		t.Errorf("error.invoke.h, at 4000 ms: seen is %d", policy.Seen())
	}
	if !reflect.DeepEqual(standingIn(engine), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateFailed}) {
		t.Errorf("standing in %v", standingIn(engine))
	}
	// The host is told to stop what it was given, with the token it was given.
	last := log.log[len(log.log)-1]
	if last.cancel == nil || last.cancel.Token != 1 || last.cancel.InvokeID != "h" {
		t.Errorf("the host is told to stop: %+v", log.log)
	}
}

func TestADeadlineThatPassedWhileTheMachineWasAwayEndsTheInvocationUnstarted(t *testing.T) {
	// 6 s after the save, and the deadline was at 5 s.
	engine, policy, log := restored(t, sharedText(t, false), 6000)
	engine.Tick()
	if len(log.log) != 0 {
		t.Errorf("the host is not asked to begin what it would be told to stop: %+v", log.log)
	}
	if policy.Seen() != 2 {
		t.Errorf("the document hears error.invoke.h: seen is %d", policy.Seen())
	}
	if !reflect.DeepEqual(standingIn(engine), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateFailed}) {
		t.Errorf("standing in %v", standingIn(engine))
	}
}

// A host drives a machine by asking when it next needs a tick and sleeping that
// long (Engine.TimeUntilNextScheduled). A restored invocation owes the machine a
// tick before anything else: it is started in the first macrostep, and the
// deadline it keeps is armed there. A machine that answered "nothing" would be
// left asleep for good, holding an invocation no one had started.
func TestARestoredMachineOwingAStartSaysItNeedsATickNow(t *testing.T) {
	for _, elapsedMs := range []int64{1000, 6000} {
		engine, _, _ := restored(t, sharedText(t, false), elapsedMs)
		if wait, ok := engine.TimeUntilNextScheduled(); !ok || wait != 0 {
			t.Errorf("%d ms after the save, before any tick: %v %v", elapsedMs, wait, ok)
		}
		engine.Tick()
	}
}

func TestWhatTheFirstRunHandedAHostCannotAnswerForTheSecond(t *testing.T) {
	// Token 0 belonged to the process that saved the machine. A late reply
	// carrying it is a different process's, and is ignored (§scxml-6.4).
	engine, policy, log := restored(t, sharedText(t, false), 1000)
	engine.Tick()
	token := log.starts()[0].Token
	if engine.CompleteHostInvoke(hostType, "h", 0, "late") {
		t.Error("a reply of the first run is accepted")
	}
	engine.Tick()
	if policy.Seen() != 0 || !reflect.DeepEqual(standingIn(engine), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateWorking}) {
		t.Errorf("seen %d, standing in %v", policy.Seen(), standingIn(engine))
	}
	if !engine.CompleteHostInvoke(hostType, "h", token, "done") {
		t.Error("the reply of this run is refused")
	}
	engine.Tick()
	if policy.Seen() != 1 || !reflect.DeepEqual(standingIn(engine), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateAfter}) {
		t.Errorf("seen %d, standing in %v", policy.Seen(), standingIn(engine))
	}
}

func TestACompletionStillQueuedWhenTheMachineWasSavedIsAcceptedAfterTheRestore(t *testing.T) {
	engine, _, log := working(t)
	// The host answers; the completion is on the external queue, stamped with the
	// start's token, and the machine has not been driven through it.
	if !engine.CompleteHostInvoke(hostType, "h", 0, "done") {
		t.Fatal("the completion is accepted")
	}
	saved, err := StatechartStaticHostInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if len(saved.HostInvokes) != 0 {
		t.Errorf("the invocation ended: %+v", saved.HostInvokes)
	}
	if len(saved.External) != 1 || saved.External[0].Name != "done.invoke.h" ||
		saved.External[0].HostInvokeToken == nil || *saved.External[0].HostInvokeToken != 0 {
		t.Fatalf("one queued completion carrying its token: %+v", saved.External)
	}
	if len(log.log) != 1 {
		t.Errorf("started once, never cancelled: %+v", log.log)
	}

	again, policy, againLog := restored(t, saved.ToJSON(), 0)
	again.Tick()
	if policy.Seen() != 1 {
		t.Errorf("taken once, not refused: seen is %d", policy.Seen())
	}
	if !reflect.DeepEqual(standingIn(again), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateAfter}) {
		t.Errorf("standing in %v", standingIn(again))
	}
	if len(againLog.log) != 0 {
		t.Errorf("an ended invocation is not started again: %+v", againLog.log)
	}
	if again.RefusedHostInvokeCompletions() != 0 {
		t.Error("the completion was refused")
	}
}

func TestACompletionTheEngineWasAboutToRefuseIsStillRefusedAfterTheRestore(t *testing.T) {
	engine, _, _ := working(t)
	// Raised through the ordinary door, so it carries no token: the engine refuses
	// it when it is dequeued, because it may be a cancelled run's reply.
	engine.RaiseExternalByName("done.invoke.h", "")
	saved, err := StatechartStaticHostInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if saved.External[0].HostInvokeToken != nil {
		t.Fatal("the event carries no token")
	}

	again, policy, _ := restored(t, saved.ToJSON(), 0)
	again.Tick()
	if again.RefusedHostInvokeCompletions() != 1 || policy.Seen() != 0 ||
		!reflect.DeepEqual(standingIn(again), []StatechartStaticHostInvokeState{StatechartStaticHostInvokeStateWorking}) {
		t.Errorf("refused %d, seen %d, standing in %v", again.RefusedHostInvokeCompletions(), policy.Seen(), standingIn(again))
	}
}

func TestAnInvocationCancelledBeforeItWasStartedAgainIsNeverStarted(t *testing.T) {
	engine, _, log := restored(t, sharedText(t, false), 1000)
	// Nothing was started, so there is nothing to tell the host to stop.
	if engine.CancelHostInvoke(hostType, "h") {
		t.Error("there is no run to cancel")
	}
	engine.Tick()
	if len(log.log) != 0 {
		t.Errorf("%+v", log.log)
	}
}

func TestAMachineRestoredAndSavedAgainBeforeItWasDrivenKeepsItsInvocation(t *testing.T) {
	engine, _, _ := restored(t, sharedText(t, false), 1000)
	again, err := StatechartStaticHostInvokeSaveAt(engine, savedAtMs+1000)
	if err != nil {
		t.Fatal(err)
	}
	// The same request, due at the same wall-clock moment, and the same counter.
	if got, want := again.ToJSON(), sharedText(t, false); got != want {
		t.Errorf("saved as\n%s\nwant\n%s", got, want)
	}
}

func TestARestartNobodyCanRunIsTheErrorAStartNobodyCanRunIs(t *testing.T) {
	engine, _, _, err := tryRestore(sharedText(t, false), 1000)
	if err != nil {
		t.Fatal(err)
	}
	// No invoker registered by the first macrostep.
	engine.Tick()
	if got := engine.UnhandledErrorEvents(); got != 1 {
		t.Errorf("error.execution: %d unhandled", got)
	}
}

func TestASavedStateNamingAHostInvocationTheDocumentCannotHaveRunIsRefused(t *testing.T) {
	shared := sharedText(t, false)
	swap := func(from, to string) string { return strings.Replace(shared, from, to, 1) }
	for _, c := range []struct{ what, text, expected string }{
		{"an id the document does not invoke", swap(`"id":"h"`, `"id":"stranger"`),
			"which the document does not have a host invoker run"},
		{"a type the document does not hand that id to", swap(`"type":"x-sce-host"`, `"type":"x-other"`),
			"which the document does not have a host invoker run"},
		{"an invocation whose state the configuration is not in",
			strings.Replace(swap(`"configuration":["working"]`, `"configuration":["idle"]`),
				`"current":"working"`, `"current":"idle"`, 1),
			"whose state the saved configuration does not stand in"},
	} {
		if _, _, _, err := tryRestore(c.text, 0); err == nil || !strings.Contains(err.Error(), c.expected) {
			t.Errorf("%s: refused as %v, want it to contain %q", c.what, err, c.expected)
		}
	}
	// An invocation named twice.
	start := strings.Index(shared, `"hostinvokes":[`) + len(`"hostinvokes":[`)
	end := start + strings.Index(shared[start:], `],"hostinvoketoken"`)
	one := shared[start:end]
	twice := strings.Replace(shared, one, one+","+one, 1)
	if _, _, _, err := tryRestore(twice, 0); err == nil || !strings.Contains(err.Error(), "which an earlier entry already names") {
		t.Errorf("an invocation named twice: %v", err)
	}
}

func TestTheHostInvokeFieldsAreRequiredAndMustBeWhatTheySay(t *testing.T) {
	shared := sharedText(t, false)
	swap := func(from, to string) string { return strings.Replace(shared, from, to, 1) }
	for _, c := range []struct{ what, text, expected string }{
		{"no hostinvokes", swap(`"hostinvokes":[`, `"hostinvokes_":[`), "has no 'hostinvokes'"},
		{"no hostinvoketoken", swap(`"hostinvoketoken":"1",`, ``), "has no 'hostinvoketoken'"},
		{"a token that is a number", swap(`"hostinvoketoken":"1"`, `"hostinvoketoken":1`),
			"'hostinvoketoken' is not a text"},
		{"a token past a signed 64-bit count", swap(`"hostinvoketoken":"1"`, `"hostinvoketoken":"9223372036854775808"`),
			"is not a whole number a token can be"},
		{"a deadline that is not a moment", swap(`"due":"1700000005000"`, `"due":"soon"`),
			"is not a whole number of milliseconds"},
		{"a deadline that is a number", swap(`"due":"1700000005000"`, `"due":1700000005000`),
			"'hostinvokes[0].due' is neither a text nor null"},
	} {
		if _, err := sce.SavedStateFromJSON(c.text); err == nil || !strings.Contains(err.Error(), c.expected) {
			t.Errorf("%s: refused as %v, want it to contain %q", c.what, err, c.expected)
		}
	}
}
