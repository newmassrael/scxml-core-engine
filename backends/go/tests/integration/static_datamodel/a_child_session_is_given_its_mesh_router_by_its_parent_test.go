// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043): a child session that
// talks to a Mesh peer is given a router of its own by the machine that starts
// it, asked of the factory the host registered, and never the machine's own.
//
// `static_child_mesh.scxml` starts `list`, which greets `#server` as it is
// entered and ends when the greeting is refused; the machine counts that end in
// `refusals`. Its hybrid `peer` starts `static_mesh_relay`, whose child `inner`
// answers `inner.ping`. The Rust half is
// `a_child_session_is_given_its_mesh_router_by_its_parent.rs`.

package static_datamodel

import (
	"fmt"
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_child_mesh"
)

type childMeshEngine = sce.Engine[static_child_mesh.StaticChildMeshState, static_child_mesh.StaticChildMeshEvent]

// startedChildMesh starts the machine with `register` applied before it does,
// and answers the names of what the machine's own router was handed.
func startedChildMesh(register func(*childMeshEngine)) (*static_child_mesh.StaticChildMeshPolicy, *childMeshEngine, *[]string) {
	policy := static_child_mesh.NewStaticChildMeshPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_child_mesh.StaticChildMeshState, static_child_mesh.StaticChildMeshEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	parentSends := &[]string{}
	engine.RegisterMeshRouter(func(request sce.HostSendRequest) []sce.HostSendResponse {
		*parentSends = append(*parentSends, request.EventName)
		return nil
	})
	register(engine)
	engine.Initialize()
	settleChildMesh(engine)
	return &policy, engine, parentSends
}

// settleChildMesh runs the rounds a child needs to take an event and its end to
// reach the parent.
func settleChildMesh(engine *childMeshEngine) {
	for i := 0; i < 5; i++ {
		engine.Tick()
	}
}

// meshRecorder is a child's router that writes down what it is told and holds
// what a test hands it for the child.
type meshRecorder struct {
	name  string
	log   *[]string
	inbox *[]sce.MeshCall
}

func (r *meshRecorder) Send(request sce.HostSendRequest) []sce.HostSendResponse {
	*r.log = append(*r.log, fmt.Sprintf("%s sent %s", r.name, request.EventName))
	return nil
}

func (r *meshRecorder) Invoke(sce.HostInvokeEvent) *sce.HostInvokeResponse { return nil }

func (r *meshRecorder) TakeCalls() []sce.MeshCall {
	calls := *r.inbox
	*r.inbox = nil
	return calls
}

func (r *meshRecorder) Detach() {
	*r.log = append(*r.log, r.name+" detached")
}

func (r *meshRecorder) ChildRouter(invokeID, document string) sce.ChildMeshRouter {
	*r.log = append(*r.log, fmt.Sprintf("%s asked for %s of '%s'", r.name, invokeID, document))
	return &meshRecorder{name: r.name + "/" + invokeID, log: r.log, inbox: r.inbox}
}

func TestAChildGivenNoRouterHasItsPeerSendRefusedAndTheMachinesRouterIsNotAsked(t *testing.T) {
	policy, _, parentSends := startedChildMesh(func(*childMeshEngine) {})
	if got := policy.Refusals(); got != 1 {
		t.Errorf("refusals is %d, not 1: the child's greeting found no router and ended the child", got)
	}
	if len(*parentSends) != 0 {
		t.Errorf("the child's send fell back to the machine's router: %v", *parentSends)
	}
}

func TestAFactoryThatAnswersNoneLeavesTheChildWithNoRouter(t *testing.T) {
	policy, _, parentSends := startedChildMesh(func(engine *childMeshEngine) {
		engine.RegisterChildMeshRouters(func(string, string) sce.ChildMeshRouter { return nil })
	})
	if got := policy.Refusals(); got != 1 {
		t.Errorf("refusals is %d, not 1", got)
	}
	if len(*parentSends) != 0 {
		t.Errorf("the child's send fell back to the machine's router: %v", *parentSends)
	}
}

func TestEachStartAsksTheFactoryAndEachEndDetachesWhatItAnswered(t *testing.T) {
	log := &[]string{}
	inbox := &[]sce.MeshCall{}
	expect := func(step string, want ...string) {
		t.Helper()
		if !reflect.DeepEqual(*log, want) {
			t.Errorf("%s: the routers were told %v, not %v", step, *log, want)
		}
		*log = nil
	}
	policy, engine, parentSends := startedChildMesh(func(engine *childMeshEngine) {
		engine.RegisterChildMeshRouters(func(invokeID, document string) sce.ChildMeshRouter {
			*log = append(*log, fmt.Sprintf("asked for %s of '%s'", invokeID, document))
			return &meshRecorder{name: invokeID, log: log, inbox: inbox}
		})
	})
	// Asked before the child was entered, since the greeting went through it.
	expect("started", "asked for list of ''", "list sent list.hello")
	if got := policy.Refusals(); got != 0 {
		t.Errorf("refusals is %d, not 0", got)
	}

	// A peer's event reaches the child when the machine next drives it.
	*inbox = append(*inbox, sce.MeshCall{Kind: sce.MeshCallRaise, Name: "list.ping", Metadata: sce.MeshEventMetadata("", "server", "")})
	settleChildMesh(engine)
	expect("list.ping", "list sent list.pong")

	// The hybrid candidate is asked for by its stem, and its child of its router.
	engine.RaiseExternalByName("open", "")
	settleChildMesh(engine)
	expect("open", "asked for peer of 'static_mesh_relay'", "peer asked for inner of ''")

	// Leaving the invoking states ends every session, each detached once, a
	// grandchild before its parent.
	engine.RaiseExternalByName("again", "")
	settleChildMesh(engine)
	expect("again", "peer/inner detached", "peer detached", "list detached")

	// Entered anew, the invoke asks again.
	engine.RaiseExternalByName("back", "")
	settleChildMesh(engine)
	expect("back", "asked for list of ''", "list sent list.hello")
	if len(*parentSends) != 0 {
		t.Errorf("a child's send fell back to the machine's router: %v", *parentSends)
	}
}
