// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "sync"

// The inbox of an engine: the one part of it a goroutine that does not own the
// engine may use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").
//
// A multi-producer, single-consumer queue of what other goroutines have for an
// engine, which the engine's owner applies at its next macrostep boundary, each
// input by the owner-goroutine operation of the same name. A slice behind a
// mutex, as ParentEventQueue is: a hosted inbox is unbounded (docs/adr/0084, D2),
// and a buffered channel is a bounded queue that would refuse, or block, a post
// when full. A post never waits for a macrostep: the lock is held for an append
// on one side and for taking the slice on the other.

// Inbox is the handle a goroutine that does not own an engine posts to it
// through (Engine.Inbox). It is safe for concurrent use, as is every copy of it,
// and every copy names the same inbox.
//
// What may be posted is closed: one method per input, each taking what the
// owner-goroutine operation of the same name takes, because the owner applies a
// posted input by calling that operation. The owner applies what was posted at
// its next macrostep boundary, before the external queue is consulted, in the
// order the inbox took the posts, so a posted input and a direct call cannot
// differ.
//
// Each post answers whether the inbox took the input. The inbox takes every post
// from the moment the engine is made until its run ends in a top-level <final> or
// is stopped (Engine.Stop), and refuses every post after either until a run is
// entered again (Engine.Initialize, Engine.EnterAt). A post taken before the
// first run is applied at that run's first boundary.
type Inbox[E comparable] struct {
	state *inboxState[E]
}

// RaiseExternal posts what Engine.RaiseExternal takes: an external event with
// eventData as `_event.data` and origin as `_event.origin`.
func (in Inbox[E]) RaiseExternal(event E, eventData, origin string) bool {
	return in.state.post(inboxInput[E]{kind: inboxExternalEvent, event: event, data: eventData, origin: origin})
}

// RaiseExternalByName posts what Engine.RaiseExternalByName takes: an external
// event named by text. The name is resolved against the document's names when
// the input is applied, and the event is dropped then if no event of the document
// could take it, as a direct call drops it.
func (in Inbox[E]) RaiseExternalByName(eventName, eventData string) bool {
	return in.state.post(inboxInput[E]{kind: inboxExternalEventByName, name: eventName, data: eventData})
}

// EndHostInvoke posts the end of a host-run invocation's start. An empty
// sessionID ends an invocation of the machine whose inbox this is, applied with
// Engine.CompleteHostInvokeFrom when ending.Done and with Engine.FailHostInvoke
// otherwise; a session id ends one of that session of the tree, applied with
// Engine.EndHostInvokeIn.
//
// The token check those make is made when the input is applied, so an ending
// posted for a start that was cancelled, has ended, or was started again by then
// raises nothing, as a direct one would. There is no caller to hand that refusal
// to, so it is not reported; the post's answer says only that the inbox took it.
func (in Inbox[E]) EndHostInvoke(sessionID string, ending HostInvokeEnding) bool {
	return in.state.post(inboxInput[E]{kind: inboxHostInvokeEnding, sessionID: sessionID, ending: ending})
}

// ApplyMeshCall posts what Engine.ApplyMeshCall takes: one thing the Mesh
// endpoint serving the machine whose inbox this is has for it, posted from the
// goroutine the endpoint receives on. A child session's router keeps the calls
// it holds until its parent takes them (Engine.ApplyMeshCalls), on the goroutine
// that drives the tree, so it has no need of this.
func (in Inbox[E]) ApplyMeshCall(call MeshCall) bool {
	return in.state.post(inboxInput[E]{kind: inboxMeshDelivery, mesh: call})
}

// WakeWith installs waker, called on the posting goroutine after each post the
// inbox took, so a host that drives the engine itself (Engine.Tick, Engine.Step)
// learns there is something to apply. Nil removes it. A waker runs on whichever
// goroutine posted, and must not block it: a send on a channel of capacity one,
// in a select with a default, is the usual shape.
func (in Inbox[E]) WakeWith(waker func()) {
	in.state.mu.Lock()
	in.state.waker = waker
	in.state.mu.Unlock()
}

// inboxInputKind says which input an inboxInput is.
type inboxInputKind int

const (
	// inboxExternalEvent is Inbox.RaiseExternal's input.
	inboxExternalEvent inboxInputKind = iota
	// inboxExternalEventByName is Inbox.RaiseExternalByName's input.
	inboxExternalEventByName
	// inboxHostInvokeEnding is Inbox.EndHostInvoke's input.
	inboxHostInvokeEnding
	// inboxMeshDelivery is Inbox.ApplyMeshCall's input.
	inboxMeshDelivery
)

// inboxInput is one input of an inbox: the kind, and the arguments of the
// owner-goroutine operation that applies it.
type inboxInput[E comparable] struct {
	kind inboxInputKind

	// event is an inboxExternalEvent's event; name an inboxExternalEventByName's
	// name. data is either's `_event.data`, origin the first's `_event.origin`.
	event  E
	name   string
	data   string
	origin string

	// sessionID and ending are an inboxHostInvokeEnding's.
	sessionID string
	ending    HostInvokeEnding

	// mesh is an inboxMeshDelivery's call.
	mesh MeshCall
}

// inboxState is what an engine and every handle to its inbox share. The posting
// half (post, and Inbox.WakeWith) is safe from any goroutine; the rest is the
// owner's.
type inboxState[E comparable] struct {
	mu sync.Mutex
	// held is what was posted and not yet applied, oldest first.
	held []inboxInput[E]
	// closed refuses every post while set.
	closed bool
	// waker is what Inbox.WakeWith installed; nil for none.
	waker func()
}

// post takes input unless the inbox is closed, and wakes the owner when it took
// it. The waker is called after the lock is released, so a waker that posts
// again, or one slow to return, holds up no other post and no drain.
func (s *inboxState[E]) post(input inboxInput[E]) bool {
	s.mu.Lock()
	if s.closed {
		s.mu.Unlock()
		return false
	}
	s.held = append(s.held, input)
	waker := s.waker
	s.mu.Unlock()
	if waker != nil {
		waker()
	}
	return true
}

// take answers what the inbox holds, oldest first, and empties it. What other
// goroutines post after it returns waits for the next boundary, so posters that
// never stop cannot hold the owner at this one.
func (s *inboxState[E]) take() []inboxInput[E] {
	s.mu.Lock()
	held := s.held
	s.held = nil
	s.mu.Unlock()
	return held
}

// open takes posts from now on.
func (s *inboxState[E]) open() {
	s.mu.Lock()
	s.closed = false
	s.mu.Unlock()
}

// close refuses every post from now on. What the inbox held when it closed is
// still there for take, and nothing more can join it.
func (s *inboxState[E]) close() {
	s.mu.Lock()
	s.closed = true
	s.mu.Unlock()
}

// Inbox is this engine's inbox: the handle a goroutine that does not own the
// engine posts external events, host-run invocations' endings and Mesh
// deliveries through. Safe to call from any goroutine. See Inbox for what is
// posted and when the owner applies it, and ARCHITECTURE.md "Engine Threading"
// for the rule it serves.
func (e *Engine[S, E]) Inbox() Inbox[E] {
	return Inbox[E]{state: e.inbox}
}

// drainInbox is the macrostep boundary's half of the inbox (docs/adr/0084): it
// applies, on the owner, what other goroutines posted, in the order the inbox
// took it, each by the owner-goroutine operation of the same name.
func (e *Engine[S, E]) drainInbox() {
	for _, input := range e.inbox.take() {
		e.applyInboxInput(input)
	}
}

// closeInbox closes the inbox and applies what it still holds. A run that ended,
// or was stopped, takes no more posts; what was posted before is applied as a
// direct call would be now, so an event joins the external queue of a machine
// that will not dequeue it and is counted with the rest (UnseenExternalEvents),
// and an ending finds its invocation over. Nothing taken is dropped unsaid.
func (e *Engine[S, E]) closeInbox() {
	e.inbox.close()
	e.drainInbox()
}

// applyInboxInput applies one posted input by the operation a direct call on the
// owner makes.
func (e *Engine[S, E]) applyInboxInput(input inboxInput[E]) {
	switch input.kind {
	case inboxExternalEvent:
		e.RaiseExternal(input.event, input.data, input.origin)
	case inboxExternalEventByName:
		e.RaiseExternalByName(input.name, input.data)
	case inboxHostInvokeEnding:
		e.applyHostInvokeEnding(input.sessionID, input.ending)
	case inboxMeshDelivery:
		e.ApplyMeshCall(input.mesh)
	}
}

// applyHostInvokeEnding applies a posted ending: on this machine by
// CompleteHostInvokeFrom or FailHostInvoke when sessionID is empty, and on the
// session it names by EndHostInvokeIn otherwise. What either answers is a
// refusal there is no caller left to hand to (see Inbox.EndHostInvoke).
func (e *Engine[S, E]) applyHostInvokeEnding(sessionID string, ending HostInvokeEnding) {
	if sessionID != "" {
		_ = e.EndHostInvokeIn(sessionID, ending)
		return
	}
	if ending.Done {
		e.CompleteHostInvokeFrom(ending.ProcessorType, ending.InvokeID, ending.Token, ending.Data, ending.Origin, ending.OriginType)
		return
	}
	e.FailHostInvoke(ending.ProcessorType, ending.InvokeID, ending.Token, ending.Data, ending.Origin, ending.OriginType)
}
