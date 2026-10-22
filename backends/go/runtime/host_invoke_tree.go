// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "fmt"

// Host-run invocations in a tree of sessions (docs/adr/0045).
//
// The invokers a host registers serve the whole tree of sessions, as the set of
// invocable types is the processor's and not one session's (§scxml-6.4.1), and
// each request names the session that made it (HostInvokeRequest.SessionID). The
// host ends an invocation of any session of the tree through the engine it built,
// naming the session and the start (Engine.CompleteHostInvokeIn), which reaches
// the session that started it wherever it stands. The Go twin of
// `sce_rust_runtime::host_processor::TreeInvokers` and the endings of
// `Engine::end_host_invoke_in`.

// TreeInvokers is the invokers a host registered for the types it serves, shared
// by every session of one tree: the engine the host built makes it, and a parent
// hands its own to each child it starts or resumes. Opaque: a host registers
// through Engine.RegisterInvoker, and the generated parent only passes it on.
type TreeInvokers struct {
	handlers map[string]HostInvokeHandler
}

// tree is the invokers this session is served by, made when it has none yet.
func (e *Engine[S, E]) tree() *TreeInvokers {
	if e.treeInvokers == nil {
		e.treeInvokers = &TreeInvokers{handlers: map[string]HostInvokeHandler{}}
	}
	return e.treeInvokers
}

// invokerFor is what runs the `<invoke type>` processorType: the invoker of a type
// SCE serves on a session's behalf (ReservedTypePrefix: a Mesh request) is this
// session's alone, since a child's is its router's and never its parent's, which
// would send under the parent's name (docs/adr/0043); every other is the tree's.
func (e *Engine[S, E]) invokerFor(processorType string) (HostInvokeHandler, bool) {
	if IsReservedType(processorType) {
		handler, ok := e.hostInvokers[processorType]
		return handler, ok
	}
	handler, ok := e.tree().handlers[processorType]
	return handler, ok
}

// sessionID is this session's own id, empty for a policy that keeps none.
func (e *Engine[S, E]) sessionID() string {
	if holder, ok := any(e.policy).(interface{ OwnSessionID() string }); ok {
		return holder.OwnSessionID()
	}
	return ""
}

// HostInvokeTree is the part of a running child session the tree's host reaches
// it by: the invokers it is served by, and the endings of the invocations of the
// sessions at and below it. Engine implements it, and a generated parent holds
// its children as one.
type HostInvokeTree interface {
	// AdoptHostInvokers makes invokers the ones this session, and every child it
	// holds now, is served by.
	AdoptHostInvokers(invokers *TreeInvokers)
	// RouteHostInvokeEnd delivers ending to the session sessionID names, when it is
	// this one or one below it, and answers false when it is neither.
	RouteHostInvokeEnd(sessionID string, ending HostInvokeEnding) (bool, error)
}

// HostInvokeChildren is a generated policy's answer for the children it holds:
// what a tree needs of it to hand the invokers down and to reach a session below
// it. A policy that starts no child session has no answer, and its engine is a
// leaf.
type HostInvokeChildren interface {
	// AdoptHostInvokersInChildren hands invokers on to every child the policy
	// holds now: a child a restore resumed holds children of its own, resumed with
	// it before its parent could hand it anything.
	AdoptHostInvokersInChildren(invokers *TreeInvokers)
	// RouteHostInvokeEndToChild delivers ending to the child at or below which
	// the session sessionID is, and answers false when no child holds it.
	RouteHostInvokeEndToChild(sessionID string, ending HostInvokeEnding) (bool, error)
}

// ShareHostInvokersWith hands this machine's invokers to child, a session it is
// starting or resuming, before the child runs (docs/adr/0045): the types a host
// registers through RegisterInvoker serve every session of the tree. An invoker
// registered afterwards serves the child too, since it is the tree's own registry
// the child holds. The Mesh request invoker is not handed on: a child's is the
// router its parent's factory gave it.
func (e *Engine[S, E]) ShareHostInvokersWith(child HostInvokeTree) {
	child.AdoptHostInvokers(e.tree())
}

// AdoptHostInvokers is served by invokers, the tree's, and hands them on to every
// child this machine holds now.
func (e *Engine[S, E]) AdoptHostInvokers(invokers *TreeInvokers) {
	e.treeInvokers = invokers
	if holder, ok := any(e.policy).(HostInvokeChildren); ok {
		holder.AdoptHostInvokersInChildren(invokers)
	}
}

// HostInvokeEnding is how a host ends a host-run invocation of some session of a
// tree: the start it ends, and whether it completed (`done.invoke`) or failed
// (`error.invoke`), with Data as the event's `_event.data`.
type HostInvokeEnding struct {
	// ProcessorType is the `type` the request named.
	ProcessorType string
	// InvokeID is the invocation's id, as the request named it.
	InvokeID string
	// Token is the one the request carried.
	Token uint64
	// Done is true for a completion and false for a failure.
	Done bool
	// Data is `_event.data` of the event the ending raises.
	Data string
}

// RouteHostInvokeEnd delivers ending to the session sessionID names, if that
// session is this machine or one in the tree below it: false when it is neither,
// so the generated parent that asked can try another child.
func (e *Engine[S, E]) RouteHostInvokeEnd(sessionID string, ending HostInvokeEnding) (bool, error) {
	if sessionID == e.sessionID() {
		if e.endHostInvoke(ending.ProcessorType, ending.InvokeID, ending.Token, !ending.Done, ending.Data, "", "") {
			return true, nil
		}
		return true, refuse("session '%s' is not running start %d of the '%s' invocation '%s': it was cancelled, has "+
			"ended, or was started again since", sessionID, ending.Token, ending.ProcessorType, ending.InvokeID)
	}
	if holder, ok := any(e.policy).(HostInvokeChildren); ok {
		return holder.RouteHostInvokeEndToChild(sessionID, ending)
	}
	return false, nil
}

// EndHostInvokeIn ends a host-run invocation of the session sessionID, this one or
// one below it at any depth, reached by its id as a send to a child session is.
// The session's queue takes `done.invoke` or `error.invoke` as CompleteHostInvoke
// and FailHostInvoke raise them; a child takes it when its parent next drives it.
//
// The refusal says which when no session of that id runs in the tree, or that
// session is not running that start of the invocation (cancelled, ended, or started
// again since). Nothing is raised then.
func (e *Engine[S, E]) EndHostInvokeIn(sessionID string, ending HostInvokeEnding) error {
	routed, err := e.RouteHostInvokeEnd(sessionID, ending)
	if !routed {
		return &StateRefusal{reason: fmt.Sprintf(
			"no session '%s' runs in this tree: it was never started, or it has ended", sessionID)}
	}
	return err
}

// CompleteHostInvokeIn is EndHostInvokeIn for a completion: `done.invoke.<invokeID>`
// with doneData as `_event.data`, in the session sessionID names. What a host that
// serves the whole tree calls with the session id and token the request carried.
func (e *Engine[S, E]) CompleteHostInvokeIn(sessionID, processorType, invokeID string, token uint64, doneData string) error {
	return e.EndHostInvokeIn(sessionID, HostInvokeEnding{
		ProcessorType: processorType, InvokeID: invokeID, Token: token, Done: true, Data: doneData,
	})
}

// FailHostInvokeIn is EndHostInvokeIn for a failure: `error.invoke.<invokeID>`
// with errorData as `_event.data`.
func (e *Engine[S, E]) FailHostInvokeIn(sessionID, processorType, invokeID string, token uint64, errorData string) error {
	return e.EndHostInvokeIn(sessionID, HostInvokeEnding{
		ProcessorType: processorType, InvokeID: invokeID, Token: token, Done: false, Data: errorData,
	})
}
