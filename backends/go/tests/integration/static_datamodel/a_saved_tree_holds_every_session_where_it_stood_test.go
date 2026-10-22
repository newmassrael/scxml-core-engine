// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045,
// section 8) — the Go half of the Rust suite's
// `a_saved_tree_holds_every_session_where_it_stood.rs`: the tree fixture saved
// byte for byte, restored from that text, and refused for a grandchild of
// another shape and a child of a later revision.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml.
// The shared text, sce-build/tests/fixtures/static_datamodel/saved/
// static_invoke_tree_working.json, is the tree `working` leaves:
//
//   - `worker` was started twice (`rest`, `back`), and has taken `go`: it counted
//     a step, holds a delayed `report` to its parent, asks its host for the time (a
//     host-run invocation that has not answered), and runs a grandchild under the
//     hybrid candidate `static_hybrid_watcher`, whose `pick` was changed to the
//     other candidate after it started (`swap`); it keeps the id the grandchild
//     was started under in `helper_id` (`idlocation`).
//   - The parent holds a delayed `nudge` to the child (`arm`) and the child's
//     `answer` in its queue; the child holds the parent's `swap` in its own.

package static_datamodel

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_tree"
)

func sharedTreeText(t *testing.T) string {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures",
		"static_datamodel", "saved", "static_invoke_tree_working.json"))
	if err != nil {
		t.Fatalf("the shared saved tree: %v", err)
	}
	return strings.TrimSpace(string(raw))
}

// treeWorking is the run the shared text is the save of. The host serves the clock
// and never answers, so the invocation is running when the tree is saved.
func treeWorking(t *testing.T) *treeEngine {
	t.Helper()
	engine, _, starts := treeStarted(t)
	// Started twice: left and entered again.
	for _, event := range []string{"rest", "back"} {
		engine.RaiseExternalByName(event, "")
		engine.Tick()
	}
	// `go`, forwarded in one tick and taken by the child in the next.
	engine.RaiseExternalByName("go", "")
	engine.Tick()
	engine.Tick()
	if len(starts.requests) != 1 {
		t.Fatalf("the child asked its host once: %d starts", len(starts.requests))
	}
	// `swap`, forwarded and taken: `pick` names the other candidate now.
	engine.RaiseExternalByName("swap", "")
	engine.Tick()
	engine.Tick()
	// A delayed send to the child.
	engine.RaiseExternalByName("arm", "")
	engine.Tick()
	// `ask`, then `swap`, taken two to a call: the child answers `question` at once,
	// behind the host's `swap`, and the call ends after the parent takes `swap`,
	// which it forwards to the child, so the child's answer waits in the parent's
	// queue and the parent's `swap` in the child's.
	if err := engine.SetMaxExternalEventsPerCall(2); err != nil {
		t.Fatal(err)
	}
	engine.RaiseExternalByName("ask", "")
	engine.RaiseExternalByName("swap", "")
	engine.Tick()
	return engine
}

func savedTree(t *testing.T, engine *treeEngine) *sce.SavedState {
	t.Helper()
	saved, err := static_invoke_tree.StaticInvokeTreeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatalf("the tree saves: %v", err)
	}
	return saved
}

func restoredTree(text string) (*treeEngine, error) {
	saved, err := sce.SavedStateFromJSON(text)
	if err != nil {
		return nil, err
	}
	policy := static_invoke_tree.NewStaticInvokeTreePolicy()
	return static_invoke_tree.StaticInvokeTreeRestoreAt(&policy, saved, sce.NewManualClock(0), savedAtMs)
}

func eventNames(events []sce.SavedEvent) []string {
	names := []string{}
	for _, event := range events {
		names = append(names, event.Name)
	}
	return names
}

func actsOf(sends []sce.SavedSend, kind sce.SavedActKind) []string {
	events := []string{}
	for _, send := range sends {
		if send.Act.Kind == kind {
			events = append(events, send.Act.Event)
		}
	}
	return events
}

func sameTexts(a, b []string) bool {
	return strings.Join(a, "\x00") == strings.Join(b, "\x00") && len(a) == len(b)
}

func TestTheTreeHoldsEverySessionWhereItStood(t *testing.T) {
	root := savedTree(t, treeWorking(t))
	if root.SessionID != treeRoot || root.Current != "working" {
		t.Errorf("the root is %q standing in %q", root.SessionID, root.Current)
	}
	if len(root.InvokeStarts) != 1 || root.InvokeStarts[0] != (sce.SavedInvokeStart{ID: "worker", Count: 2}) {
		t.Errorf("the worker was started twice: %+v", root.InvokeStarts)
	}
	if got := eventNames(root.External); !sameTexts(got, []string{"answer"}) {
		t.Errorf("the child's answer waits in the parent's queue: %v", got)
	}
	if got := actsOf(root.Pending, sce.SavedActChild); !sameTexts(got, []string{"nudge"}) {
		t.Errorf("the parent holds a delayed send to the child: %v", got)
	}

	if len(root.Invokes.Children) != 1 {
		t.Fatalf("one running child: %d", len(root.Invokes.Children))
	}
	worker := root.Invokes.Children[0]
	if worker.ID != "worker" || worker.Start != 2 || worker.InvokeID != "working.2.worker" {
		t.Errorf("the worker is %+v", worker)
	}
	child := worker.Session
	if child.SessionID != "fixture-root.working.2.worker" {
		t.Errorf("the child session is %q", child.SessionID)
	}
	if child.Current != "second" {
		t.Errorf("the child took `go`: it stands in %q", child.Current)
	}
	if steps, _ := child.Variable("steps"); steps != sce.JSONNumber("1") {
		t.Errorf("steps is %v", steps)
	}
	if pick, _ := child.Variable("pick"); pick != "static_hybrid_holder.scxml" {
		t.Errorf("`pick` was changed after the grandchild started: %v", pick)
	}
	if got := eventNames(child.External); !sameTexts(got, []string{"swap"}) {
		t.Errorf("the parent's `swap` waits in the child's queue: %v", got)
	}
	if got := actsOf(child.Pending, sce.SavedActParent); !sameTexts(got, []string{"report"}) {
		t.Errorf("the child holds a delayed send to its parent: %v", got)
	}
	if len(child.HostInvokes) != 1 || child.HostInvokes[0].ProcessorType != treeClock || child.HostInvokes[0].InvokeID != "clock" {
		t.Errorf("the child's host-run invocation runs: %+v", child.HostInvokes)
	}

	if len(child.Invokes.Children) != 1 {
		t.Fatalf("one grandchild: %d", len(child.Invokes.Children))
	}
	helper := child.Invokes.Children[0]
	if helper.Document != "static_hybrid_watcher" {
		t.Errorf("the candidate that was started, whatever `pick` names now: %q", helper.Document)
	}
	if helper.Start != 1 || !strings.HasPrefix(helper.InvokeID, "second.1.") {
		t.Errorf("the grandchild is start %d under %q", helper.Start, helper.InvokeID)
	}
	if held, _ := child.Variable("helper_id"); held != helper.InvokeID {
		t.Errorf("the idlocation holds the id the grandchild was started under: %v", held)
	}
}

func TestTheTreeSavesTheSharedTextByteForByte(t *testing.T) {
	if got, want := savedTree(t, treeWorking(t)).ToJSON(), sharedTreeText(t); got != want {
		t.Errorf("the tree's text is not the shared one:\n%s\nwant\n%s", got, want)
	}
}

func TestTheSharedTextRestoresAndSavesAgainAsItWas(t *testing.T) {
	engine, err := restoredTree(sharedTreeText(t))
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	if got, want := savedTree(t, engine).ToJSON(), sharedTreeText(t); got != want {
		t.Errorf("a restored tree stands where the saved one stood:\n%s\nwant\n%s", got, want)
	}
}

func TestAGrandchildOfAnotherShapeRefusesTheTreeByItsPath(t *testing.T) {
	shared := sharedTreeText(t)
	shape := "a85ea43d8de628b004074e59c0ca90c198a4f7736073bd953750bca35c174c7a"
	text := strings.Replace(shared, shape, strings.Repeat("0", 64), 1)
	if text == shared {
		t.Fatal("the change applied")
	}
	_, err := restoredTree(text)
	if err == nil {
		t.Fatal("restored")
	}
	if !strings.Contains(err.Error(), "invokes[0].session.invokes[0].session") || !strings.Contains(err.Error(), "shape") {
		t.Errorf("the refusal names the grandchild's path: %v", err)
	}
}

func TestAChildOfALaterRevisionRefusesTheTreeByItsPath(t *testing.T) {
	// The child holds a record of `schema_revised.scxml`, saved at its third
	// revision; a fourth this build does not know is refused, naming the child.
	shared := sharedTreeText(t)
	if !strings.Contains(shared, `"revision":3`) {
		t.Fatal("the child names the revision of its record")
	}
	_, err := restoredTree(strings.Replace(shared, `"revision":3`, `"revision":4`, 1))
	if err == nil {
		t.Fatal("restored")
	}
	if !strings.Contains(err.Error(), "invokes[0].session") || !strings.Contains(err.Error(), "later than") {
		t.Errorf("the refusal names the child's path: %v", err)
	}
}

func TestASaveAfterAnyHostCallFindsNothingInTransit(t *testing.T) {
	// Half of the Rust suite's `nothing_is_in_transit_when_a_tree_is_saved.rs`: after
	// each call of a run that drives the child every way there is (a send made at
	// once, a delayed one armed, a forwarded event, a tick, a start again), the tree
	// saves, the text it saves restores, and the restored tree saves that text again.
	engine, _, _ := treeStarted(t)
	check := func(after string) {
		t.Helper()
		saved, err := static_invoke_tree.StaticInvokeTreeSaveAt(engine, savedAtMs)
		if err != nil {
			t.Fatalf("the tree saves at every boundary, but after %s: %v", after, err)
		}
		text := saved.ToJSON()
		copied, err := restoredTree(text)
		if err != nil {
			t.Fatalf("the text saved after %s restores: %v", after, err)
		}
		again, err := static_invoke_tree.StaticInvokeTreeSaveAt(copied, savedAtMs)
		if err != nil {
			t.Fatalf("the restored tree saves after %s: %v", after, err)
		}
		if again.ToJSON() != text {
			t.Fatalf("the tree restored after %s stands where it was saved:\n%s\nwant\n%s", after, again.ToJSON(), text)
		}
	}
	for _, event := range []string{"ask", "go", "ask", "swap", "ask", "arm", "ask", "rest", "back", "ask", "go", "ask"} {
		// The two calls a host makes: the event it raises, then the tick.
		engine.RaiseExternalByName(event, "")
		check("raising `" + event + "`")
		engine.Tick()
		check("ticking after `" + event + "`")
	}
}
