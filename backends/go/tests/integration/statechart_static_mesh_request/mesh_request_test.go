// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19 under `datamodel="sce-static"` (docs/adr/0005,
// decisions 5 and 7): an `<invoke type="sce:mesh-rpc">` whose peer and `<param>`s
// are typed expressions over the machine's own fields reaches the host's Mesh
// router through the runtime's mesh-rpc door — Go AOT path. The Rust twin is
// `a_static_mesh_request_reaches_the_router.rs`, and the script-model one is
// `a_mesh_request_reaches_the_router`.
//
// The committed machine in this directory is generated from
// `sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml`
// (regen: `scripts/regen_host_processor.sh`) and is built with NO script engine:
// its variables are fields, and a peer or a `<param>` that needed an engine to be
// read would not have one to ask.
//
// The run `bump`, `go` changes `load` and `label` before the request reads them,
// so a copy taken at start-up (3, "idle") is told from what the fields hold now
// (4, "busy"): the request carries 8 and "busy".

package statechart_static_mesh_request

import (
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

type Engine = sce.Engine[StatechartStaticMeshRequestState, StatechartStaticMeshRequestEvent]

// started is a machine with no script engine, standing at `idle`, with `router`
// registered through the mesh-rpc door when it is not nil.
func started(router sce.HostInvokeHandler) (*Engine, *StatechartStaticMeshRequestPolicy) {
	policy := NewStatechartStaticMeshRequestPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartStaticMeshRequestState, StatechartStaticMeshRequestEvent](&policy)
	if router != nil {
		engine.RegisterMeshRPCInvoker(router)
	}
	engine.Initialize()
	engine.Step()
	return engine, &policy
}

func drive(engine *Engine, events ...string) {
	for _, event := range events {
		engine.RaiseExternalByName(event, "")
		engine.Step()
	}
}

// recording is a router that keeps every request it is started with and
// answers none, so the test decides how each one ends.
func recording(requests *[]sce.HostInvokeRequest) sce.HostInvokeHandler {
	return func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if event.Start != nil {
			*requests = append(*requests, *event.Start)
		}
		return nil
	}
}

// theRequest is the one request the router was started with, checked against
// what the document wrote and the fields held when it started.
func theRequest(t *testing.T, requests []sce.HostInvokeRequest) sce.HostInvokeRequest {
	t.Helper()
	if len(requests) != 1 {
		t.Fatalf("exactly one request reaches the router: %+v", requests)
	}
	request := requests[0]
	if request.ProcessorType != sce.MeshRPCInvokeType || request.InvokeID != "ask" || request.Src != "#motor" {
		t.Errorf("the router received %+v", request)
	}
	want := map[string][]string{
		"_mesh_event":       {"service.request.force"},
		"_mesh_deadline_ms": {"250"},
		"force":             {"8"},
		"speed":             {"busy"},
	}
	if !reflect.DeepEqual(request.Params, want) {
		t.Errorf("params = %v, want %v", request.Params, want)
	}
	// The author's pairs alone, typed: `force` was computed, `speed` is a
	// string, and the envelope fields are not payload.
	if want := `{"force":8,"speed":"busy"}`; request.EventData != want {
		t.Errorf("EventData = %s, want %s", request.EventData, want)
	}
	return request
}

func expect(t *testing.T, name string, got, want uint32) {
	t.Helper()
	if got != want {
		t.Errorf("%s = %d, want %d", name, got, want)
	}
}

func ended(t *testing.T, engine *Engine) {
	t.Helper()
	if state, ok := engine.TerminalState(); !ok || state != StatechartStaticMeshRequestStateDone {
		t.Errorf("the run must end in `done`")
	}
}

// The router answers: done.invoke.ask ends the run.
func TestAStaticMeshRequestIsAnsweredThroughTheRouter(t *testing.T) {
	var requests []sce.HostInvokeRequest
	engine, policy := started(recording(&requests))
	drive(engine, "bump", "go")
	request := theRequest(t, requests)
	if !engine.CompleteHostInvoke(sce.MeshRPCInvokeType, "ask", request.Token, `"ok"`) {
		t.Fatalf("the running request's completion was refused")
	}
	engine.Step()
	expect(t, "answered", policy.Answered(), 1)
	expect(t, "failed", policy.Failed(), 0)
	expect(t, "refused", policy.Refused(), 0)
	ended(t, engine)
}

// The router fails the request: error.invoke.ask, carrying the router's data.
func TestAStaticMeshRequestTheRouterFailsIsErrorInvoke(t *testing.T) {
	var requests []sce.HostInvokeRequest
	engine, policy := started(recording(&requests))
	drive(engine, "bump", "go")
	request := theRequest(t, requests)
	if !engine.FailHostInvoke(sce.MeshRPCInvokeType, "ask", request.Token, `"unreachable"`, "mesh://motor", "") {
		t.Fatalf("the running request's failure was refused")
	}
	engine.Step()
	expect(t, "answered", policy.Answered(), 0)
	expect(t, "failed", policy.Failed(), 1)
	expect(t, "refused", policy.Refused(), 0)
	ended(t, engine)
}

// The control: nothing has written a field, so the request carries what
// `<data expr>` gave them and not the values `bump` would have set.
func TestTheRequestCarriesTheFieldsAsTheyStandAndNotACopyFromStartUp(t *testing.T) {
	var requests []sce.HostInvokeRequest
	engine, _ := started(recording(&requests))
	drive(engine, "go")
	if len(requests) != 1 {
		t.Fatalf("one request: %+v", requests)
	}
	if want := `{"force":6,"speed":"idle"}`; requests[0].EventData != want {
		t.Errorf("EventData = %s, want %s", requests[0].EventData, want)
	}
}

// The peer is the string the machine holds when the invocation starts: `ghost`
// changes it, so the router is handed that name as the request's `src`, and the
// host's router is the one that looks it up among its bindings.
func TestThePeerIsTheStringTheMachineHoldsWhenTheRequestStarts(t *testing.T) {
	var requests []sce.HostInvokeRequest
	engine, _ := started(recording(&requests))
	drive(engine, "ghost", "go")
	if len(requests) != 1 {
		t.Fatalf("one request: %+v", requests)
	}
	if requests[0].Src != "#ghost" {
		t.Errorf("Src = %q, want %q", requests[0].Src, "#ghost")
	}
}

// With no router registered the invoke names a type nobody runs:
// error.execution (W3C SCXML 6.4.1).
func TestAStaticMeshRequestWithNoRouterRegisteredIsErrorExecution(t *testing.T) {
	engine, policy := started(nil)
	drive(engine, "bump", "go")
	expect(t, "answered", policy.Answered(), 0)
	expect(t, "failed", policy.Failed(), 0)
	expect(t, "refused", policy.Refused(), 1)
	ended(t, engine)
}
