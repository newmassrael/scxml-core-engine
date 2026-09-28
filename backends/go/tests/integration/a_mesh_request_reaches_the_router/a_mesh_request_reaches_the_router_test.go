// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE_MESH.md §9.5, §mesh-19: an `<invoke type="sce:mesh-rpc">` reaches the
// host's Mesh router through the runtime's mesh-rpc door — Go AOT path.
//
// Fixture: integration_resources/a_mesh_request_reaches_the_router/
// a_mesh_request_reaches_the_router.scxml (canonical, shared with the other
// channels).
//
// Regeneration: scripts/regen_a_mesh_request_reaches_the_router_go.sh

package a_mesh_request_reaches_the_router

import (
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

type machine = sce.Engine[AMeshRequestReachesTheRouterState, AMeshRequestReachesTheRouterEvent]

// started runs the machine into `asking`, with `router` registered through
// the mesh-rpc door when it is not nil.
func started(t *testing.T, router sce.HostInvokeHandler) (*machine, *AMeshRequestReachesTheRouterPolicy) {
	t.Helper()
	policy := NewAMeshRequestReachesTheRouterPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AMeshRequestReachesTheRouterState, AMeshRequestReachesTheRouterEvent](&policy)
	if router != nil {
		engine.RegisterMeshRPCInvoker(router)
	}
	engine.Initialize()
	engine.Step()
	return engine, &policy
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
// what the document wrote.
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
		"force":             {"3"},
		"speed":             {"3"},
	}
	if !reflect.DeepEqual(request.Params, want) {
		t.Errorf("params = %v, want %v", request.Params, want)
	}
	// The author's pairs alone, typed: `force` was computed, `speed` was
	// written as a string, and the envelope fields are not payload.
	if want := `{"force":3,"speed":"3"}`; request.EventData != want {
		t.Errorf("EventData = %s, want %s", request.EventData, want)
	}
	return request
}

func expect(t *testing.T, name string, read func() (int64, bool), want int64) {
	t.Helper()
	if got, ok := read(); !ok || got != want {
		t.Errorf("%s = %d (readable %v), want %d", name, got, ok, want)
	}
}

func ended(t *testing.T, engine *machine) {
	t.Helper()
	if state, ok := engine.TerminalState(); !ok || state != AMeshRequestReachesTheRouterStateDone {
		t.Errorf("the run must end in `done`")
	}
}

// The router answers: done.invoke.ask ends the run.
func TestAMeshRequestIsAnsweredThroughTheRouter(t *testing.T) {
	var requests []sce.HostInvokeRequest
	engine, policy := started(t, recording(&requests))
	request := theRequest(t, requests)
	if !engine.CompleteHostInvoke(sce.MeshRPCInvokeType, "ask", request.Token, `"ok"`) {
		t.Fatalf("the running request's completion was refused")
	}
	engine.Step()
	expect(t, "answered", policy.Answered, 1)
	expect(t, "failed", policy.Failed, 0)
	expect(t, "refused", policy.Refused, 0)
	ended(t, engine)
}

// The router fails the request: error.invoke.ask, carrying the router's data.
func TestAMeshRequestTheRouterFailsIsErrorInvoke(t *testing.T) {
	var requests []sce.HostInvokeRequest
	engine, policy := started(t, recording(&requests))
	request := theRequest(t, requests)
	if !engine.FailHostInvoke(sce.MeshRPCInvokeType, "ask", request.Token, `"unreachable"`, "mesh://motor", "") {
		t.Fatalf("the running request's failure was refused")
	}
	engine.Step()
	expect(t, "answered", policy.Answered, 0)
	expect(t, "failed", policy.Failed, 1)
	expect(t, "refused", policy.Refused, 0)
	ended(t, engine)
}

// With no router registered the invoke names a type nobody runs:
// error.execution (W3C SCXML 6.4.1).
func TestAMeshRequestWithNoRouterRegisteredIsErrorExecution(t *testing.T) {
	engine, policy := started(t, nil)
	expect(t, "answered", policy.Answered, 0)
	expect(t, "failed", policy.Failed, 0)
	expect(t, "refused", policy.Refused, 1)
	ended(t, engine)
}
