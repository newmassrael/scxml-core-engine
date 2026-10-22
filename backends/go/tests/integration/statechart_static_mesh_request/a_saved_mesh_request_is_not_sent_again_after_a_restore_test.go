// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring": a Mesh request
// (`<invoke type="sce:mesh-rpc">`) has one request in flight, and the peer it
// reached may already have acted on it. A saved state holds the call as it was
// sent; a restore does NOT send it again, because the peer would act on it twice,
// and the router that carried it is gone with the process that saved it. The
// document is told on the first macrostep that the call was interrupted
// (`error.invoke.<id>` carrying `"interrupted"`) and sends a new request if it
// means to ask again.
//
// `statechart_static_mesh_request.scxml` asks from `asking`; `done.invoke.ask` and
// `error.invoke.ask` each leave it, counting in `answered` and `failed`. The Rust
// and Kotlin halves are `a_saved_mesh_request_is_not_sent_again_after_a_restore.rs`
// and `ASavedMeshRequestIsNotSentAgainAfterARestoreTest.kt`.
//
// Driven entirely on a manual clock: no case sleeps. The shared instance
// `saved/statechart_static_mesh_request_asking.json` is the format-2 text every
// backend writes (docs/adr/0045); `saved/format1/` holds the format-1 text of the
// same run.

package statechart_static_mesh_request

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// savedAtMs is the wall-clock moment the shared instance was saved at.
const savedAtMs int64 = 1_700_000_000_000

func sharedAsking(t *testing.T) string {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures",
		"host_processor", "saved", "statechart_static_mesh_request_asking.json"))
	if err != nil {
		t.Fatalf("the shared saved state: %v", err)
	}
	return strings.TrimSpace(string(raw))
}

type router struct{ log []sce.HostInvokeEvent }

func (r *router) register(engine *Engine) {
	engine.RegisterMeshRPCInvoker(func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		r.log = append(r.log, event)
		return nil
	})
}

// asking is a machine whose request to `#motor` is in flight, the router having
// been handed it, given the session id the shared format-2 fixtures name their
// root by.
func asking(t *testing.T) (*Engine, *router) {
	t.Helper()
	policy := NewStatechartStaticMeshRequestPolicy()
	if err := policy.SetSessionID("fixture-root"); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[StatechartStaticMeshRequestState, StatechartStaticMeshRequestEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	log := &router{}
	log.register(engine)
	engine.Initialize()
	engine.RaiseExternalByName("go", "")
	engine.Tick()
	return engine, log
}

// restored is text restored elapsedMs after it was saved, the router registered on
// the engine the restore returned, the way a Go host does it.
func restored(t *testing.T, text string, elapsedMs int64) (*Engine, *StatechartStaticMeshRequestPolicy, *router) {
	t.Helper()
	saved, err := sce.SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	policy := NewStatechartStaticMeshRequestPolicy()
	engine, err := StatechartStaticMeshRequestRestoreAt(&policy, saved, sce.NewManualClock(0), savedAtMs+elapsedMs)
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	log := &router{}
	log.register(engine)
	return engine, &policy, log
}

func TestARequestInFlightIsSavedAsTheCallItWas(t *testing.T) {
	engine, log := asking(t)
	if len(log.log) != 1 {
		t.Fatalf("the router was handed the request: %d", len(log.log))
	}
	saved, err := StatechartStaticMeshRequestSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if len(saved.HostInvokes) != 1 {
		t.Fatalf("one call in flight: %+v", saved.HostInvokes)
	}
	entry := saved.HostInvokes[0]
	if entry.ProcessorType != sce.MeshRPCInvokeType || entry.InvokeID != "ask" || entry.Src != "#motor" {
		t.Errorf("the call is %+v", entry)
	}
	// The router owns the deadline of a Mesh request, so the engine holds none.
	if entry.Due != nil {
		t.Errorf("due is %v", *entry.Due)
	}
	if len(saved.Pending) != 0 {
		t.Errorf("%+v", saved.Pending)
	}
	// The text every backend writes for the same machine, byte for byte.
	if got, want := saved.ToJSON(), sharedAsking(t); got != want {
		t.Errorf("saved as\n%s\nthe shared text is\n%s", got, want)
	}
}

func TestARestoreAsksNothingOfTheRouterUntilTheMachineIsDriven(t *testing.T) {
	engine, _, log := restored(t, sharedAsking(t), 1000)
	if len(log.log) != 0 {
		t.Error("the restore itself calls nobody")
	}
	if !reflect.DeepEqual(engine.GetActiveStates(), []StatechartStaticMeshRequestState{StatechartStaticMeshRequestStateAsking}) {
		t.Errorf("standing in %v", engine.GetActiveStates())
	}
}

func TestARestoredRequestIsNotSentAgainAndTheDocumentIsToldItWasInterrupted(t *testing.T) {
	engine, policy, log := restored(t, sharedAsking(t), 1000)
	engine.Tick()
	if len(log.log) != 0 {
		t.Errorf("the peer may have acted on the first request already, so no second one leaves: %+v", log.log)
	}
	if policy.Failed() != 1 || policy.Answered() != 0 || policy.Refused() != 0 {
		t.Errorf("error.invoke.ask arrived: failed %d, answered %d, refused %d",
			policy.Failed(), policy.Answered(), policy.Refused())
	}
	if state, ended := engine.TerminalState(); !ended || state != StatechartStaticMeshRequestStateDone {
		t.Errorf("the run ended in %v (%v)", state, ended)
	}
}

func TestACallIsInterruptedWheneverTheMachineComesBack(t *testing.T) {
	// There is no moment after which the call is still worth sending again.
	for _, elapsedMs := range []int64{0, 250, 60_000} {
		engine, policy, log := restored(t, sharedAsking(t), elapsedMs)
		engine.Tick()
		if len(log.log) != 0 {
			t.Errorf("%d ms: %+v", elapsedMs, log.log)
		}
		if policy.Failed() != 1 {
			t.Errorf("%d ms after the save: failed is %d", elapsedMs, policy.Failed())
		}
	}
}

func TestAMachineOwingTheInterruptionSaysItNeedsATickNow(t *testing.T) {
	// The interruption is told in the first macrostep, so a host driving the machine
	// by its wake-up query must not be told there is nothing to wait for.
	engine, _, _ := restored(t, sharedAsking(t), 1000)
	if wait, ok := engine.TimeUntilNextScheduled(); !ok || wait != 0 {
		t.Errorf("%v %v", wait, ok)
	}
}

func TestARestoredMachineSavedAgainBeforeItWasDrivenKeepsTheCall(t *testing.T) {
	engine, _, _ := restored(t, sharedAsking(t), 1000)
	again, err := StatechartStaticMeshRequestSaveAt(engine, savedAtMs+1000)
	if err != nil {
		t.Fatal(err)
	}
	if len(again.HostInvokes) != 1 || again.HostInvokes[0].ProcessorType != sce.MeshRPCInvokeType ||
		again.HostInvokes[0].InvokeID != "ask" {
		t.Errorf("the call is still held: %+v", again.HostInvokes)
	}
}
