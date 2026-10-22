// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Running host invocations" (docs/adr/0045, section
// 3): the invokers a host registers on the machine it built serve every session
// of that machine's tree. Each request names the session that made it, and the
// host ends an invocation of any session through the machine it holds, by that
// session's id and the request's token.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml,
// whose in-place child `worker` asks its host for the time
// (`sce-host-clock-now`, id `clock`) as it enters `second`, tells its parent
// `stamped` when the host answers, and keeps the id its grandchild is started
// under in `helper_id` (`idlocation`). The Rust and Kotlin halves are
// `a_child_sessions_host_invocation_is_served_by_the_trees_host.rs` and
// `AChildSessionsHostInvocationIsServedByTheTreesHostTest.kt`.

package static_datamodel

import (
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_tree"
)

const (
	// treeRoot is the session id the shared format-2 fixtures give their root.
	treeRoot = "fixture-root"
	// treeWorker is the session `worker` is: the root's id and the id its first
	// start is started under (`<state>.<start>.<invoke>`).
	treeWorker = "fixture-root.working.1.worker"
	// treeClock is the standard host type the child asks.
	treeClock = "sce-host-clock-now"
	// treeNow is a completion of the clock the child's result record reads.
	treeNow = `{"error":"","utc_ms":1700000000000,"monotonic_ms":5}`
)

type treeEngine = sce.Engine[static_invoke_tree.StaticInvokeTreeState, static_invoke_tree.StaticInvokeTreeEvent]

// treeStarts is every start the host was handed, in order.
type treeStarts struct{ requests []sce.HostInvokeRequest }

// serveTreeClock registers the clock on engine, the root, recording each start.
func serveTreeClock(engine *treeEngine) *treeStarts {
	starts := &treeStarts{}
	engine.RegisterInvoker(treeClock, func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if event.Start != nil {
			starts.requests = append(starts.requests, *event.Start)
		}
		return nil
	})
	return starts
}

// treeStarted is a root built under treeRoot, served, and initialized.
func treeStarted(t *testing.T) (*treeEngine, *static_invoke_tree.StaticInvokeTreePolicy, *treeStarts) {
	t.Helper()
	policy := static_invoke_tree.NewStaticInvokeTreePolicy()
	if err := policy.SetSessionID(treeRoot); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[static_invoke_tree.StaticInvokeTreeState, static_invoke_tree.StaticInvokeTreeEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	starts := serveTreeClock(engine)
	engine.Initialize()
	return engine, &policy, starts
}

// intoSecond is `go`, which the root forwards to the child in one tick and the
// child takes, entering `second` and starting the clock, in the next.
func intoSecond(engine *treeEngine) {
	engine.RaiseExternalByName("go", "")
	engine.Tick()
	engine.Tick()
}

func onlyTreeStart(t *testing.T, starts *treeStarts) sce.HostInvokeRequest {
	t.Helper()
	if len(starts.requests) != 1 {
		t.Fatalf("one start: %+v", starts.requests)
	}
	return starts.requests[0]
}

func TestAChildsInvocationReachesTheRootsInvokerAndIsEndedThroughTheRoot(t *testing.T) {
	engine, policy, starts := treeStarted(t)
	intoSecond(engine)
	request := onlyTreeStart(t, starts)
	if request.ProcessorType != treeClock || request.InvokeID != "clock" {
		t.Errorf("the request is %q %q", request.ProcessorType, request.InvokeID)
	}
	if request.SessionID != treeWorker {
		t.Errorf("the request names the session %q, not the child session that made it", request.SessionID)
	}
	if request.Restarted {
		t.Error("a start the document makes is never a restart")
	}

	if err := engine.CompleteHostInvokeIn(treeWorker, treeClock, "clock", request.Token, treeNow); err != nil {
		t.Fatalf("the child is running that start: %v", err)
	}
	engine.Tick()
	if got := policy.Stamps(); got != 1 {
		t.Errorf("the child took done.invoke.clock and told its parent: stamps is %d", got)
	}
}

func TestAnEndingForASessionTheTreeDoesNotHoldOrAStartItDoesNotRunIsRefused(t *testing.T) {
	engine, policy, starts := treeStarted(t)
	intoSecond(engine)
	request := onlyTreeStart(t, starts)
	for _, nowhere := range []string{"fixture-root.nowhere", "fixture-root.working.1.worker.nowhere", "elsewhere"} {
		err := engine.CompleteHostInvokeIn(nowhere, treeClock, "clock", request.Token, treeNow)
		if err == nil || !strings.Contains(err.Error(), "no session") {
			t.Errorf("%s: %v", nowhere, err)
		}
	}
	// The root runs no clock of its own.
	if err := engine.CompleteHostInvokeIn(treeRoot, treeClock, "clock", request.Token, treeNow); err == nil ||
		!strings.Contains(err.Error(), "is not running") {
		t.Errorf("the root runs no such start: %v", err)
	}
	// A start ended once is ended: the second ending is stale.
	if err := engine.CompleteHostInvokeIn(treeWorker, treeClock, "clock", request.Token, treeNow); err != nil {
		t.Fatalf("the first ending: %v", err)
	}
	if err := engine.FailHostInvokeIn(treeWorker, treeClock, "clock", request.Token, "late"); err == nil ||
		!strings.Contains(err.Error(), "is not running") {
		t.Errorf("the start has ended: %v", err)
	}
	engine.Tick()
	if got := policy.Stamps(); got != 1 {
		t.Errorf("the refused endings raised nothing: stamps is %d", got)
	}
}

func TestAResumedChildsInvocationStartsAgainAtItsFirstDrive(t *testing.T) {
	engine, _, starts := treeStarted(t)
	intoSecond(engine)
	first := onlyTreeStart(t, starts)
	saved, err := static_invoke_tree.StaticInvokeTreeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatalf("saves with the clock running: %v", err)
	}
	again, err := sce.SavedStateFromJSON(saved.ToJSON())
	if err != nil {
		t.Fatal(err)
	}

	next := static_invoke_tree.NewStaticInvokeTreePolicy()
	restored, err := static_invoke_tree.StaticInvokeTreeRestoreAt(&next, again, sce.NewManualClock(0), savedAtMs)
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	// The host registers its invokers on the machine the restore handed it, after
	// the restore built the child: nothing has started yet.
	served := serveTreeClock(restored)
	if len(served.requests) != 0 {
		t.Fatal("a restore calls no host")
	}

	// The parent's first drive of the child is the child's first macrostep.
	restored.Tick()
	request := onlyTreeStart(t, served)
	if !request.Restarted {
		t.Error("started again from the saved request")
	}
	if request.SessionID != treeWorker || request.InvokeID != "clock" {
		t.Errorf("the restarted start names %q %q", request.SessionID, request.InvokeID)
	}
	if request.Token == first.Token {
		t.Error("a new token")
	}

	if err := restored.CompleteHostInvokeIn(treeWorker, treeClock, "clock", request.Token, treeNow); err != nil {
		t.Fatalf("the restarted start: %v", err)
	}
	restored.Tick()
	if got := next.Stamps(); got != 1 {
		t.Errorf("stamps is %d", got)
	}
}

func TestAnInvokeIdlocationKeepsTheIdTheGrandchildIsStartedUnder(t *testing.T) {
	// docs/adr/0045: the grandchild is started under `second.1.<its invoke id>`,
	// the `invokeid` a saved tree holds for it, and `helper_id` holds the same.
	engine, _, _ := treeStarted(t)
	intoSecond(engine)
	saved, err := static_invoke_tree.StaticInvokeTreeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	worker := saved.Invokes.Children[0].Session
	if len(worker.Invokes.Children) != 1 {
		t.Fatalf("the hybrid candidate runs: %d grandchildren", len(worker.Invokes.Children))
	}
	startedUnder := worker.Invokes.Children[0].InvokeID
	if !strings.HasPrefix(startedUnder, "second.1.") {
		t.Errorf("the first start of the invoke of `second`: %s", startedUnder)
	}
	held, err := worker.Variable("helper_id")
	if err != nil {
		t.Fatal(err)
	}
	if held != startedUnder {
		t.Errorf("`helper_id` holds %v, and the grandchild was started under %q", held, startedUnder)
	}
}
