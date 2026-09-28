// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE_MESH.md §mesh-19: a `targetexpr` that evaluates to a Mesh peer reaches
// the host's Mesh router — Go AOT path.
//
// Fixture: integration_resources/a_peer_named_at_run_time_reaches_the_router/
// a_peer_named_at_run_time_reaches_the_router.scxml (canonical, shared with the
// other channels).
//
// Regeneration: scripts/regen_a_peer_named_at_run_time_reaches_the_router_go.sh

package a_peer_named_at_run_time_reaches_the_router

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

type machine = sce.Engine[APeerNamedAtRunTimeReachesTheRouterState, APeerNamedAtRunTimeReachesTheRouterEvent]

func started(t *testing.T, router sce.HostSendHandler) (*machine, *APeerNamedAtRunTimeReachesTheRouterPolicy) {
	t.Helper()
	policy := NewAPeerNamedAtRunTimeReachesTheRouterPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[APeerNamedAtRunTimeReachesTheRouterState, APeerNamedAtRunTimeReachesTheRouterEvent](&policy)
	if router != nil {
		engine.RegisterMeshRouter(router)
	}
	engine.Initialize()
	engine.Step()
	return engine, &policy
}

func expect(t *testing.T, name string, read func() (int64, bool), want int64) {
	t.Helper()
	if got, ok := read(); !ok || got != want {
		t.Errorf("%s = %d (readable %v), want %d", name, got, ok, want)
	}
}

// With a router registered, the peer send is the router's — processor
// `sce:mesh`, the evaluated target, the event — and the non-peer target is
// still this session's.
func TestAPeerNamedAtRunTimeIsSentToTheRouter(t *testing.T) {
	var routed []sce.HostSendRequest
	engine, policy := started(t, func(request sce.HostSendRequest) []sce.HostSendResponse {
		routed = append(routed, request)
		return nil
	})
	if len(routed) != 1 {
		t.Fatalf("exactly the peer send reaches the router: %+v", routed)
	}
	if routed[0].ProcessorType != sce.MeshProcessorType || routed[0].Target != "#hmi" || routed[0].EventName != "ping" {
		t.Errorf("the router received %+v", routed[0])
	}
	expect(t, "refused", policy.Refused, 0)
	expect(t, "looped", policy.Looped, 1)
	if ended, ok := engine.TerminalState(); !ok || ended != APeerNamedAtRunTimeReachesTheRouterStateDone {
		t.Errorf("`loopback` must carry the run to `done`")
	}
}

// With no router registered, the peer send is a send to a host processor
// nobody serves: one error.execution, and the non-peer target is unaffected.
func TestAPeerWithNoRouterRegisteredIsErrorExecution(t *testing.T) {
	engine, policy := started(t, nil)
	expect(t, "refused", policy.Refused, 1)
	expect(t, "looped", policy.Looped, 1)
	if ended, ok := engine.TerminalState(); !ok || ended != APeerNamedAtRunTimeReachesTheRouterStateDone {
		t.Errorf("`loopback` must carry the run to `done`")
	}
}
