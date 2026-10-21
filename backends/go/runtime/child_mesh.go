// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// The Mesh router a child session is given by its parent (SCE_MESH.md, Child
// Sessions in Mesh; docs/adr/0043). The Go twin of the Rust runtime's
// `ChildMeshRouter`, `MeshCall` and `mesh_event_metadata`.

// MeshOriginScheme is the scheme of a Mesh-delivered event's `_event.origin`,
// followed by the sending machine's name (SCE_MESH.md §mesh-10.7).
const MeshOriginScheme = "mesh://"

// MeshEventMetadata is the `_event` of an event that arrived over Mesh from
// source, as SCE_MESH.md §mesh-10.7 tabulates it: external, origin
// `mesh://<source>`, the SCXML processor as origin type, and the send id the
// envelope carried. The one place it is built, so a child cannot tell a peer's
// event from the one its parent would have been given.
func MeshEventMetadata(data, source, sendID string) EventMetadata {
	return EventMetadata{
		Data:       data,
		EventType:  EventTypeExternal,
		SendID:     sendID,
		Origin:     MeshOriginScheme + source,
		OriginType: SCXMLEventProcessorType,
	}
}

// MeshCallKind says which of a MeshCall's two shapes it is.
type MeshCallKind int

const (
	// MeshCallRaise raises Name on the external queue with Metadata.
	MeshCallRaise MeshCallKind = iota
	// MeshCallEndInvoke ends the start Token of the Mesh request InvokeID —
	// with `done.invoke`, or with `error.invoke` when Failed — carrying Data
	// as `_event.data`.
	MeshCallEndInvoke
)

// MeshCall is one thing a Mesh endpoint has for the engine it serves, in the
// order the endpoint came to know it: what Engine.ApplyMeshCall applies.
type MeshCall struct {
	Kind MeshCallKind

	// Name and Metadata are a MeshCallRaise's event.
	Name     string
	Metadata EventMetadata

	// InvokeID, Token, Failed and Data are a MeshCallEndInvoke's request and
	// how it ended. Source is the machine that answered, empty for a deadline
	// the requester reached itself.
	InvokeID string
	Token    uint64
	Failed   bool
	Data     string
	Source   string
}

// ChildMeshRouter is the Mesh router a child session is given by its parent:
// the child's endpoint attached to that one session.
//
// What a host installs on a machine it starts, the parent installs on the
// child it starts, before the child initializes, since an <onentry> of the
// child's initial state may already send: the child's `<send type="sce:mesh">`
// goes to Send and its `<invoke type="sce:mesh-rpc">` to Invoke, as
// RegisterMeshRouter and RegisterMeshRPCInvoker route a machine's. What arrives
// for the child waits here until the parent drives the child, which takes it
// with TakeCalls before the child's macrostep. When the session ends the
// parent says so with Detach, after the child has stopped.
type ChildMeshRouter interface {
	// Send serves a `<send type="sce:mesh">` of the child.
	Send(request HostSendRequest) []HostSendResponse
	// Invoke serves an `<invoke type="sce:mesh-rpc">` of the child starting
	// or being cancelled.
	Invoke(event HostInvokeEvent) *HostInvokeResponse
	// TakeCalls answers what arrived for the child since it was last asked,
	// oldest first.
	TakeCalls() []MeshCall
	// Detach says the session ended (§mesh-21.5): it reached a top-level <final>, or its
	// parent left the invoking state and cancelled it. Called once.
	Detach()
	// ChildRouter answers the router of a child this child starts, asked as
	// the parent's factory is asked; nil leaves that child with no router.
	ChildRouter(invokeID, document string) ChildMeshRouter
}

// ChildMeshRouters is the factory of the routers of a machine's children,
// registered with RegisterChildMeshRouters: asked, each time an invocation
// starts a child, with the invoke's id and the stem of the child's document
// (empty for a child written inline). Nil leaves the child with no router.
type ChildMeshRouters func(invokeID, document string) ChildMeshRouter

// meshChild is what a parent installs a child's router on: every Engine,
// whatever its state and event types.
type meshChild interface {
	installChildMeshRouter(router ChildMeshRouter)
}

// RegisterChildMeshRouters registers the factory of the routers of this
// machine's children (SCE_MESH.md §mesh-21.3, Child Sessions in Mesh;
// docs/adr/0043).
//
// A child session started by `<invoke type="scxml">` is given none of this
// machine's registrations: its peer sends would leave under this machine's
// name. The generated parent asks factory instead, each time an invocation
// starts a child (on entry, on a restore, and for the candidate a hybrid
// `<invoke>` chose), and installs the answer on the child before the child
// initializes (StartChildMesh). A factory that answers nil, or none
// registered, leaves the child with no router: its peer send raises
// `error.execution` in the child, and nothing falls back to this machine's
// router.
func (e *Engine[S, E]) RegisterChildMeshRouters(factory ChildMeshRouters) {
	e.childMeshRouters = factory
}

// StartChildMesh asks this machine's factory for the router of child, which
// the invoke invokeID is starting from the document whose stem is document,
// and installs the answer on child. Leaves child as it is when no factory is
// registered or it answers nil. Called by the generated parent before child
// initializes.
func (e *Engine[S, E]) StartChildMesh(child meshChild, invokeID, document string) {
	if e.childMeshRouters == nil {
		return
	}
	router := e.childMeshRouters(invokeID, document)
	if router == nil {
		return
	}
	child.installChildMeshRouter(router)
}

// installChildMeshRouter installs router as this machine's Mesh router, its
// request invoker and the factory of its children's routers — the three a
// host installs on a machine it starts, served by the one router the parent
// was given for this session.
func (e *Engine[S, E]) installChildMeshRouter(router ChildMeshRouter) {
	e.RegisterMeshRouter(router.Send)
	e.RegisterMeshRPCInvoker(router.Invoke)
	e.childMeshRouters = router.ChildRouter
	e.installedMesh = router
}

// ApplyMeshCalls applies what arrived for this machine on the router its
// parent gave it (§mesh-21.4), in the order the router released it
// (ApplyMeshCall). The
// generated parent calls it each time it drives this child, before the
// child's macrostep. Does nothing for a machine given no router.
func (e *Engine[S, E]) ApplyMeshCalls() {
	if e.installedMesh == nil {
		return
	}
	for _, call := range e.installedMesh.TakeCalls() {
		e.ApplyMeshCall(call)
	}
}

// ApplyMeshCall tells this engine one thing its Mesh endpoint has for it:
// raise an event with the `_event` the endpoint built, or end a Mesh request.
// An event name the machine does not declare is dropped, as the engine drops
// any such event; an end the engine no longer waits for is refused by the
// token check. An answer carries `_event.origin` `mesh://<source>` and origin
// type `sce:mesh-rpc` (SCE_MESH.md §mesh-9.5); a deadline the requester
// reached itself has no peer to name, and carries neither.
func (e *Engine[S, E]) ApplyMeshCall(call MeshCall) {
	switch call.Kind {
	case MeshCallRaise:
		e.RaiseExternalByNameWithMeta(call.Name, call.Metadata)
	case MeshCallEndInvoke:
		origin, originType := "", ""
		if call.Source != "" {
			origin, originType = MeshOriginScheme+call.Source, MeshRPCInvokeType
		}
		if call.Failed {
			e.FailHostInvoke(MeshRPCInvokeType, call.InvokeID, call.Token, call.Data, origin, originType)
		} else {
			e.CompleteHostInvokeFrom(MeshRPCInvokeType, call.InvokeID, call.Token, call.Data, origin, originType)
		}
	}
}

// DetachMesh tells the router this machine's parent gave it that the session
// ended, once: after the child reached a top-level <final>, or after it was
// stopped because the invoking state exited, so a send the child made while
// stopping was handed over first. Does nothing for a machine given no router.
func (e *Engine[S, E]) DetachMesh() {
	router := e.installedMesh
	if router == nil {
		return
	}
	e.installedMesh = nil
	router.Detach()
}
