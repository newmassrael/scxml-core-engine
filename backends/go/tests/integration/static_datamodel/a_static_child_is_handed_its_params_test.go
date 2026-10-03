// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Child sessions": an `<invoke type="scxml">` hands
// its child the values its `<param>`s and `namelist` name (W3C SCXML 6.4.1),
// each to the child's variable of the same name, of that variable's own type.
//
// `static_invoke_params.scxml` invokes `worker`, which ends the moment it holds
// `start = 7` (from a `<param>` reading `base`) and `enabled = true` (from the
// `namelist`); handed less, it would wait and the parent would stay in
// `working`. `base` is 4 when `working` is entered and the entry action adds 3,
// so 7 arrives only if the value is read when the invoke executes, at the end of
// the macrostep. `control`, the same child handed nothing, keeps its declared
// defaults and never ends.
//
// The Rust and Kotlin halves are `a_static_child_is_handed_its_params.rs` and
// `AStaticChildIsHandedItsParamsTest.kt`. Neither of those restores a saved
// machine here: the Go runtime has no saved state to restore.

package static_datamodel

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_params"
)

// startedParent starts the parent and lets its children run and report: a child
// reports to its parent on the engine's tick.
func startedParent(t *testing.T) (*static_invoke_params.StaticInvokeParamsPolicy, *sce.Engine[static_invoke_params.StaticInvokeParamsState, static_invoke_params.StaticInvokeParamsEvent]) {
	t.Helper()
	policy := static_invoke_params.NewStaticInvokeParamsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_invoke_params.StaticInvokeParamsState, static_invoke_params.StaticInvokeParamsEvent](&policy)
	engine.Initialize()
	settle(engine)
	return &policy, engine
}

// settle lets the children run and report to their parent.
func settle(engine *sce.Engine[static_invoke_params.StaticInvokeParamsState, static_invoke_params.StaticInvokeParamsEvent]) {
	for i := 0; i < 5; i++ {
		engine.Tick()
	}
}

func inState(engine *sce.Engine[static_invoke_params.StaticInvokeParamsState, static_invoke_params.StaticInvokeParamsEvent], want static_invoke_params.StaticInvokeParamsState) bool {
	for _, state := range engine.GetActiveStates() {
		if state == want {
			return true
		}
	}
	return false
}

func TestAChildIsHandedTheValuesItsInvokeNames(t *testing.T) {
	policy, engine := startedParent(t)
	// `worker` ended, so it held both values: the parent left `working` and
	// counted it.
	if !inState(engine, static_invoke_params.StaticInvokeParamsStatePlain) {
		t.Fatalf("the child was handed `start` and `enabled`, so it ended: the parent is still in %v", engine.GetActiveStates())
	}
	if got := policy.Completed(); got != 1 {
		t.Errorf("completed is %d, not 1", got)
	}
}

func TestAChildHandedNothingKeepsTheValuesItsDataGaveIt(t *testing.T) {
	policy, engine := startedParent(t)
	settle(engine)
	// `control` is the same child handed nothing: it still waits for 7 and true,
	// so `done.invoke.control` never counted. `watcher` was handed 7 and waits
	// for 8.
	if !inState(engine, static_invoke_params.StaticInvokeParamsStatePlain) {
		t.Fatalf("the parent is not in `plain`: %v", engine.GetActiveStates())
	}
	if got := policy.Completed(); got != 1 {
		t.Errorf("completed is %d, not 1", got)
	}
}

func TestAChildIsHandedItsValuesOnceWhenItStarts(t *testing.T) {
	policy, engine := startedParent(t)
	// `bump` makes `base` 8 while `watcher` runs with the 7 it was handed when it
	// started: a child is handed its values once, so it does not end.
	engine.RaiseExternalByName("bump", "")
	engine.Step()
	settle(engine)
	if got := policy.Completed(); got != 1 {
		t.Errorf("completed is %d, not 1: `watcher` saw the later value", got)
	}
}
