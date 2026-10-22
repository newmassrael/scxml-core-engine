// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045): a child
// session an `<invoke type="scxml">` started is part of what a machine is doing,
// so a saved state holds each running child inside its parent, with its whole
// state, and a restore resumes it where it stood. Nothing a child did at its
// start happens again, and it raises its `done.invoke` once, when it ends.
//
// `static_invoke.scxml` invokes `worker`, a child that takes `a` and then `b`
// and ends; the parent forwards what it is sent (`autoforward`), so the host
// drives the child through the parent and every case is read from the parent:
// `done.invoke.worker` counts the run in `completed` and leaves the parent in
// `working`. A child that had taken `a` is resumed at `second`, so `b` alone
// ends it.
//
// The shared instance `saved/static_invoke_working.json` is the format-2 text of
// the machine just started, its root given the id the fixtures name it by. A
// format-1 text holds the child's id alone, and the build that wrote it promised
// to start the child again from its beginning:
// `saved/format1/static_invoke_working.json` is the one case in which a restore
// does (docs/adr/0045, section 3). The Rust and Kotlin halves are
// `a_saved_child_session_is_resumed_after_a_restore.rs` and
// `ASavedChildSessionIsResumedAfterARestoreTest.kt`.

package static_datamodel

import (
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke"
)

type invokeEngine = sce.Engine[static_invoke.StaticInvokeState, static_invoke.StaticInvokeEvent]

func sharedInvokeText(t *testing.T, format1 bool) string {
	t.Helper()
	root := filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "static_datamodel", "saved")
	if format1 {
		root = filepath.Join(root, "format1")
	}
	raw, err := os.ReadFile(filepath.Join(root, "static_invoke_working.json"))
	if err != nil {
		t.Fatalf("the shared saved state: %v", err)
	}
	return strings.TrimSpace(string(raw))
}

// invokeStarted is the machine just started, its root given the id the shared
// fixtures name it by.
func invokeStarted(t *testing.T) (*invokeEngine, *static_invoke.StaticInvokePolicy) {
	t.Helper()
	policy := static_invoke.NewStaticInvokePolicy()
	if err := policy.SetSessionID("fixture-root"); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[static_invoke.StaticInvokeState, static_invoke.StaticInvokeEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	return engine, &policy
}

func tryInvokeRestore(text string) (*invokeEngine, *static_invoke.StaticInvokePolicy, error) {
	saved, err := sce.SavedStateFromJSON(text)
	if err != nil {
		return nil, nil, err
	}
	policy := static_invoke.NewStaticInvokePolicy()
	engine, err := static_invoke.StaticInvokeRestoreAt(&policy, saved, sce.NewManualClock(0), savedAtMs)
	return engine, &policy, err
}

func invokeRestored(t *testing.T, text string) (*invokeEngine, *static_invoke.StaticInvokePolicy) {
	t.Helper()
	engine, policy, err := tryInvokeRestore(text)
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	return engine, policy
}

// sendToInvoke sends event to the parent, which forwards it to its child, and
// lets the child run.
func sendToInvoke(engine *invokeEngine, event string) {
	engine.RaiseExternalByName(event, "")
	for i := 0; i < 3; i++ {
		engine.Tick()
	}
}

func invokeWorking(engine *invokeEngine) bool {
	return slices.Contains(engine.GetActiveStates(), static_invoke.StaticInvokeStateWorking)
}

func invokeSaved(t *testing.T, engine *invokeEngine) *sce.SavedState {
	t.Helper()
	saved, err := static_invoke.StaticInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatalf("saves: %v", err)
	}
	return saved
}

func TestAMachineIsSavedWithTheWholeStateOfTheChildItIsRunning(t *testing.T) {
	engine, _ := invokeStarted(t)
	saved := invokeSaved(t, engine)
	if !saved.Invokes.Resumed || len(saved.Invokes.Children) != 1 {
		t.Fatalf("a format-2 state with one running child: %+v", saved.Invokes)
	}
	child := saved.Invokes.Children[0]
	if child.ID != "worker" || child.Start != 1 {
		t.Errorf("the child is %q, start %d", child.ID, child.Start)
	}
	if len(saved.InvokeStarts) != 1 || saved.InvokeStarts[0] != (sce.SavedInvokeStart{ID: "worker", Count: 1}) {
		t.Errorf("the starts are %+v", saved.InvokeStarts)
	}
	// Its id is this session's and the id it was started under: the tree's alone.
	if child.InvokeID != "working.1.worker" || child.Session.SessionID != "fixture-root.working.1.worker" {
		t.Errorf("the child was started under %q as %q", child.InvokeID, child.Session.SessionID)
	}
	if child.Session.Current != "first" {
		t.Errorf("the child stands in %q", child.Session.Current)
	}
	// The text every backend writes for the same machine, byte for byte.
	if got, want := saved.ToJSON(), sharedInvokeText(t, false); got != want {
		t.Errorf("saved as\n%s\nthe shared text is\n%s", got, want)
	}
}

func TestAChildThatHadTakenAnEventIsResumedWithItTaken(t *testing.T) {
	before, _ := invokeStarted(t)
	sendToInvoke(before, "a")
	saved := invokeSaved(t, before)

	// Resumed at `second`: `b` alone ends it, and its `done.invoke` arrives once.
	after, policy := invokeRestored(t, saved.ToJSON())
	if !invokeWorking(after) {
		t.Fatal("the restored machine stands in `working`")
	}
	sendToInvoke(after, "b")
	if got := policy.Completed(); got != 1 {
		t.Errorf("the child resumed where it stood and ended on `b`: completed is %d", got)
	}
	sendToInvoke(after, "b")
	if got := policy.Completed(); got != 1 {
		t.Errorf("it ended once: completed is %d", got)
	}
}

func TestARestoredTreeSavesTheTextItWasRestoredFrom(t *testing.T) {
	engine, _ := invokeRestored(t, sharedInvokeText(t, false))
	if got, want := invokeSaved(t, engine).ToJSON(), sharedInvokeText(t, false); got != want {
		t.Errorf("the child kept its id, its start and where it stood: saved as\n%s\nwant\n%s", got, want)
	}
}

func TestTheCountOfStartsIsCarriedThroughARestore(t *testing.T) {
	// The resumed child was not started again, and when it ends the count the
	// restore carried on is still its one start.
	before, _ := invokeStarted(t)
	sendToInvoke(before, "a")
	after, policy := invokeRestored(t, invokeSaved(t, before).ToJSON())
	sendToInvoke(after, "b")
	if got := policy.Completed(); got != 1 {
		t.Fatalf("completed is %d", got)
	}
	again := invokeSaved(t, after)
	if len(again.InvokeStarts) != 1 || again.InvokeStarts[0] != (sce.SavedInvokeStart{ID: "worker", Count: 1}) {
		t.Errorf("the starts are %+v", again.InvokeStarts)
	}
	if ids := again.Invokes.InvokeIDs(); len(ids) != 0 {
		t.Errorf("the child ended, and is listed as %v", ids)
	}
}

func TestAChildThatHasEndedIsNotResumedOrStartedAgain(t *testing.T) {
	// `done.invoke.worker` leaves the parent in `working`, so a machine saved
	// after its child ended stands in the state that invokes it with nothing
	// running. The invocation is complete: it is not listed, and a restore does
	// not run it a second time.
	before, policy := invokeStarted(t)
	sendToInvoke(before, "a")
	sendToInvoke(before, "b")
	if policy.Completed() != 1 || !invokeWorking(before) {
		t.Fatalf("completed %d, working %v", policy.Completed(), invokeWorking(before))
	}
	saved := invokeSaved(t, before)
	if ids := saved.Invokes.InvokeIDs(); len(ids) != 0 {
		t.Fatalf("the child ended, and is listed as %v", ids)
	}

	after, restoredPolicy := invokeRestored(t, saved.ToJSON())
	if got := restoredPolicy.Completed(); got != 1 {
		t.Errorf("the run is kept: completed is %d", got)
	}
	sendToInvoke(after, "a")
	sendToInvoke(after, "b")
	if got := restoredPolicy.Completed(); got != 1 {
		t.Errorf("a child started again would have ended again: completed is %d", got)
	}
}

func TestAMachineThatLeftTheInvokingStateSavesNoInvocation(t *testing.T) {
	engine, _ := invokeStarted(t)
	sendToInvoke(engine, "abort")
	if invokeWorking(engine) {
		t.Fatal("`abort` leaves `working`")
	}
	saved := invokeSaved(t, engine)
	if ids := saved.Invokes.InvokeIDs(); len(ids) != 0 {
		t.Fatalf("cancelled with its state: %v", ids)
	}

	// And nothing is started when it comes back: no child is there to take `a`
	// and `b`, and `idle` invokes nothing.
	again, _ := invokeRestored(t, saved.ToJSON())
	sendToInvoke(again, "a")
	sendToInvoke(again, "b")
	if again.IsInFinalState() {
		t.Error("the restored machine is in a final state")
	}
}

func TestAnEventSavedInTheChildsQueueIsTakenByTheResumedChild(t *testing.T) {
	// `a` reached the child's own queue and the child had not taken it when the
	// tree was saved: it is in the child's `external`, and the resumed child takes
	// it, then ends on `b`.
	shared := sharedInvokeText(t, false)
	empty := `"configuration":["first"],"current":"first","variables":{},"history":{},"pending":[],"invokes":[],` +
		`"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]`
	queued := strings.Replace(empty, `"external":[]`,
		`"external":[{"name":"a","data":"","type":"external","sendid":"","origin":"","origintype":"","invokeid":"","hostinvoketoken":null}]`, 1)
	text := strings.Replace(shared, empty, queued, 1)
	if text == shared {
		t.Fatal("the child's queue was changed")
	}
	engine, policy := invokeRestored(t, text)
	for i := 0; i < 3; i++ {
		engine.Tick()
	}
	sendToInvoke(engine, "b")
	if got := policy.Completed(); got != 1 {
		t.Errorf("the queued `a` reached the resumed child: completed is %d", got)
	}
}

func TestATreeWhoseChildIsNotOneTheDocumentCouldRunIsRefusedWhole(t *testing.T) {
	shared := sharedInvokeText(t, false)
	for _, c := range []struct{ what, from, to, expected string }{
		{"a child the document does not invoke", `"id":"worker"`, `"id":"stranger"`,
			"which the document does not invoke"},
		{"a start that is not the count its invoke holds", `"start":"1"`, `"start":"2"`,
			"the session counts 1 starts of it"},
		{"a child of a document its invoke does not start",
			`"document":"static_invoke__sce_synth_invoke__worker"`, `"document":"static_hybrid_holder"`,
			"does not start"},
		{"a child whose id is not its parent's and the id it was started under",
			`"sessionid":"fixture-root.working.1.worker"`, `"sessionid":"elsewhere"`,
			"a child of 'fixture-root' started under 'working.1.worker'"},
		{"a child of another shape", "688ad00d954ad9f35bdaa5eae06768a788173e0f8a8236e670efa68038781a6d",
			strings.Repeat("0", 64),
			"the session at invokes[0].session: the saved state is of a document of shape"},
		{"a child that stands in no state of its document",
			`"configuration":["first"],"current":"first"`, `"configuration":["nowhere"],"current":"nowhere"`,
			"the session at invokes[0].session: the document has no state 'nowhere'"},
	} {
		text := strings.Replace(shared, c.from, c.to, 1)
		if text == shared {
			t.Errorf("%s: the change did not apply", c.what)
			continue
		}
		_, _, err := tryInvokeRestore(text)
		if err == nil {
			t.Errorf("%s: restored", c.what)
			continue
		}
		if !strings.Contains(err.Error(), c.expected) {
			t.Errorf("%s: refused as %q, want it to contain %q", c.what, err.Error(), c.expected)
		}
	}
}

// ── format 1: a child named by its id is started again ─────────────────────

func TestAFormatOneStateStartsItsChildAgainFromTheBeginning(t *testing.T) {
	// The only case in which a child restarts: the text holds nothing of it.
	engine, policy := invokeRestored(t, sharedInvokeText(t, true))
	if !invokeWorking(engine) {
		t.Fatal("the restored machine stands in `working`")
	}

	// The child is at `first`: `b` is not what it waits for.
	sendToInvoke(engine, "b")
	if !invokeWorking(engine) || policy.Completed() != 0 {
		t.Errorf("`b` alone does not end a child that starts over: working %v, completed %d",
			invokeWorking(engine), policy.Completed())
	}

	sendToInvoke(engine, "a")
	sendToInvoke(engine, "b")
	if got := policy.Completed(); got != 1 {
		t.Errorf("`a` then `b` ends it, and `done.invoke.worker` arrived once: completed is %d", got)
	}
}

func TestAFormatOneStateNamingAnInvocationTheDocumentCannotHaveRunIsRefused(t *testing.T) {
	shared := sharedInvokeText(t, true)
	for _, c := range []struct{ what, text, expected string }{
		{"an id the document does not invoke",
			strings.Replace(shared, `"invokes":["worker"]`, `"invokes":["stranger"]`, 1),
			"which the document does not invoke"},
		{"an id named twice",
			strings.Replace(shared, `"invokes":["worker"]`, `"invokes":["worker","worker"]`, 1),
			"which an earlier entry already names"},
		{"an invocation whose state the configuration is not in",
			strings.Replace(strings.Replace(shared, `"configuration":["working"]`, `"configuration":["idle"]`, 1),
				`"current":"working"`, `"current":"idle"`, 1),
			"whose state the saved configuration does not stand in"},
	} {
		_, _, err := tryInvokeRestore(c.text)
		if err == nil {
			t.Errorf("%s: restored", c.what)
			continue
		}
		if !strings.Contains(err.Error(), c.expected) {
			t.Errorf("%s: refused as %q, want it to contain %q", c.what, err.Error(), c.expected)
		}
	}
}
