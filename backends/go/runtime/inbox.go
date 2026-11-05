// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "sync"

// The inbox of an engine: the one part of it a goroutine that does not own the
// engine may use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").
//
// A multi-producer, single-consumer queue of what other goroutines have for an
// engine, which the engine's owner applies at its next macrostep boundary, or
// sooner, before it puts an event of its own on the external queue, each input
// by the owner-goroutine operation of the same name. A slice behind a
// mutex, as ParentEventQueue is: a hosted inbox is unbounded (docs/adr/0084, D2),
// and a buffered channel is a bounded queue that would refuse, or block, a post
// when full. A post never waits for a macrostep: the lock is held for an append
// on one side and for taking the slice on the other.
//
// Every session of a tree has an inbox, a child as well as the root, and the
// tree keeps a directory from session id to inbox (sessionDirectory), so an input
// naming a session is posted to that session's inbox and its address is resolved
// when it is posted (ARCHITECTURE.md "Engine Threading", items 3 and 4).

// Inbox is the handle a goroutine that does not own an engine posts to it
// through (Engine.Inbox). It is safe for concurrent use, as is every copy of it,
// and every copy names the same inbox.
//
// What may be posted is closed: one method per input, each taking what the
// owner-goroutine operation of the same name takes, because the owner applies a
// posted input by calling that operation. The owner applies what was posted at
// its next macrostep boundary, before the external queue is consulted, in the
// order the inbox took the posts, so a posted input and a direct call cannot
// differ. A running engine applies it sooner when it puts an event of its own
// on the external queue (a <send> to itself, a host-run invocation's
// `done.invoke`): that queue takes events in the order they arrived, so the
// posts go ahead of the owner's event.
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
// sessionID ends an invocation of the machine whose inbox this is. A session id
// ends one of that session of the tree, this machine or one below it: the post
// is made to that session's own inbox, found in the tree's directory now, so the
// session takes the ending ahead of anything its own owner puts on its external
// queue afterwards, a parent's `<send>` to it included. The machine whose inbox
// this is holds its own session from the moment it is made, so a post naming it
// before its run is kept for that run, as any post before the run is. A session
// the tree does not hold refuses the post: a child not started, or a session
// whose run has ended.
//
// The session applies the ending with Engine.CompleteHostInvokeFrom when
// ending.Done and with Engine.FailHostInvoke otherwise, at its own macrostep
// boundary, or sooner when its owner puts an event on its external queue. A child
// reaches its boundary when its parent next drives it (Engine.Tick drives every
// running child), so a host woken by a post to a child applies it on its usual
// drive. The token check those make is made when the input is applied, so an
// ending posted for a start that was cancelled, has ended, or was started again
// by then raises nothing, as a direct one would. There is no caller to hand that
// refusal to, so it is not reported; the post's answer says only that the inbox
// took it.
func (in Inbox[E]) EndHostInvoke(sessionID string, ending HostInvokeEnding) bool {
	if sessionID == "" {
		return in.state.postEnding(ending)
	}
	session, held := in.state.directoryOf().lookup(sessionID)
	if !held {
		return false
	}
	return session.postEnding(ending)
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
// inbox, or the inbox of any other session of its tree, took, so a host that
// drives the engine itself (Engine.Tick, Engine.Step) learns there is something
// to apply. A tree has one owner and one waker: the host installs it on the
// root's inbox, and a child's inbox wakes it from the moment its parent starts
// it. Nil removes it. A waker runs on whichever goroutine posted, and must not
// block it: a send on a channel of capacity one, in a select with a default, is
// the usual shape.
func (in Inbox[E]) WakeWith(waker func()) {
	in.state.directoryOf().setWaker(waker)
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

	// ending is an inboxHostInvokeEnding's, of an invocation of this session.
	ending HostInvokeEnding

	// mesh is an inboxMeshDelivery's call.
	mesh MeshCall
}

// inboxState is what an engine and every handle to its inbox share. The posting
// half (post, postEnding, directoryOf) is safe from any goroutine; the rest is
// the owner's.
type inboxState[E comparable] struct {
	mu sync.Mutex
	// held is what was posted and not yet applied, oldest first.
	held []inboxInput[E]
	// closed refuses every post while set.
	closed bool
	// directory is the directory of the tree this session belongs to, whose
	// waker a post calls: the engine's own until a parent hands it the tree's
	// (joinDirectory).
	directory *sessionDirectory
}

// post takes input unless the inbox is closed, and wakes the tree's owner when it
// took it. The waker is called after the lock is released, so a waker that posts
// again, or one slow to return, holds up no other post and no drain.
func (s *inboxState[E]) post(input inboxInput[E]) bool {
	s.mu.Lock()
	if s.closed {
		s.mu.Unlock()
		return false
	}
	s.held = append(s.held, input)
	directory := s.directory
	s.mu.Unlock()
	directory.wake()
	return true
}

// postEnding posts the ending of one of this session's own invocations
// (sessionInbox).
func (s *inboxState[E]) postEnding(ending HostInvokeEnding) bool {
	return s.post(inboxInput[E]{kind: inboxHostInvokeEnding, ending: ending})
}

// directoryOf is the directory of the tree this session belongs to now.
func (s *inboxState[E]) directoryOf() *sessionDirectory {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.directory
}

// joinDirectory makes directory, a tree's, the one this inbox wakes through and
// is found in. The owner's, when a parent hands its tree to this session.
func (s *inboxState[E]) joinDirectory(directory *sessionDirectory) {
	s.mu.Lock()
	s.directory = directory
	s.mu.Unlock()
}

// sessionInbox is the posting half of one session's inbox, whatever the
// session's event type: what the tree's directory holds for it.
type sessionInbox interface {
	// postEnding posts the ending of one of the session's own invocations and
	// answers whether its inbox took it.
	postEnding(ending HostInvokeEnding) bool
}

// sessionDirectory is the directory a tree keeps (ARCHITECTURE.md "Engine
// Threading", item 3): the inbox of each session the tree holds, by session id,
// and the tree's one waker. Safe for concurrent use: a post reads it on the
// posting goroutine, and the owner enters a session when its run starts, before
// its first macrostep (the root already when it is made), and removes it when the
// run ends, after closing its inbox (Engine.enterDirectory,
// Engine.leaveDirectory).
type sessionDirectory struct {
	mu sync.Mutex
	// sessions is the inbox of every session the tree holds, by its id.
	sessions map[string]sessionInbox
	// waker is what Inbox.WakeWith installed; nil for none.
	waker func()
}

// lookup is the inbox of the session sessionID, when the tree holds it.
func (d *sessionDirectory) lookup(sessionID string) (sessionInbox, bool) {
	d.mu.Lock()
	defer d.mu.Unlock()
	inbox, held := d.sessions[sessionID]
	return inbox, held
}

// enter makes inbox the one sessionID names.
func (d *sessionDirectory) enter(sessionID string, inbox sessionInbox) {
	d.mu.Lock()
	d.sessions[sessionID] = inbox
	d.mu.Unlock()
}

// remove takes sessionID out when it names inbox, and leaves an entry another
// session made under that id alone.
func (d *sessionDirectory) remove(sessionID string, inbox sessionInbox) {
	d.mu.Lock()
	if d.sessions[sessionID] == inbox {
		delete(d.sessions, sessionID)
	}
	d.mu.Unlock()
}

// setWaker installs waker, nil for none.
func (d *sessionDirectory) setWaker(waker func()) {
	d.mu.Lock()
	d.waker = waker
	d.mu.Unlock()
}

// wake calls the tree's waker, if one is installed, with no lock held.
func (d *sessionDirectory) wake() {
	d.mu.Lock()
	waker := d.waker
	d.mu.Unlock()
	if waker != nil {
		waker()
	}
}

// enterDirectory enters this session in its tree's directory under its session
// id, so a post naming it reaches its inbox: when the engine is made, when its
// run starts, and when a session whose run is going joins a tree. A session with
// no id is not entered, nor is an engine that has no inbox (a zero value, which
// NewEngine did not make).
func (e *Engine[S, E]) enterDirectory() {
	e.leaveDirectory()
	id := e.sessionID()
	if id == "" || e.inbox == nil {
		return
	}
	e.tree().directory.enter(id, e.inbox)
	e.enteredAs = id
}

// leaveDirectory takes this session out of its tree's directory, so a post
// naming it is refused from then on.
func (e *Engine[S, E]) leaveDirectory() {
	if e.enteredAs == "" {
		return
	}
	e.tree().directory.remove(e.enteredAs, e.inbox)
	e.enteredAs = ""
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

// drainInbox is the owner's half of the inbox (docs/adr/0084): it applies, on
// the owner, what other goroutines posted, in the order the inbox took it, each
// by the owner-goroutine operation of the same name. Called at each macrostep
// boundary, before the external queue is consulted, and by enqueueExternal,
// before the owner puts an event of its own on that queue.
//
// One drain at a time. Applying an input reaches enqueueExternal again: a posted
// event is raised by Engine.RaiseExternal's own body, and a posted ending raises
// its `done.invoke`. That call must not drain in turn, or the inputs posted
// after the one being applied would join the queue ahead of what it raises.
// Inside a drain it is a plain push, and this drain applies the rest in order.
func (e *Engine[S, E]) drainInbox() {
	if e.drainingInbox {
		return
	}
	e.drainingInbox = true
	defer func() { e.drainingInbox = false }()
	for _, input := range e.inbox.take() {
		e.applyInboxInput(input)
	}
}

// enqueueExternal is the one door onto the external queue for what the owner
// puts on it: a <send> to this session, an event the host raises on the owner,
// a host-run invocation's ending or a host processor's reply, what a child
// driven by this owner sends its parent, a delayed send come due.
// docs/adr/0084, D2: that queue takes events in the order they arrived, and
// what another goroutine posted before this call arrived first (W3C test 201:
// the BasicHTTP listener posts `event1` while the send waits for it, and the
// `timeout` sent next must come second), so a running engine applies its inbox
// before event joins the queue. One not running yet keeps what was posted for
// its first boundary, as Inbox documents.
//
// The internal queue has no such door: it is the running macrostep's own, and
// arrival order is a property of the external queue, so a <raise> or a platform
// error waits for no post (Engine.Raise).
func (e *Engine[S, E]) enqueueExternal(event EventWithMetadata[E]) {
	if e.isRunning {
		e.drainInbox()
	}
	e.externalQueue.Raise(event)
}

// closeInbox closes the inbox, applies what it still holds, and takes the session
// out of its tree's directory. A run that ended, or was stopped, takes no more
// posts; what was posted before is applied as a direct call would be now, so an
// event joins the external queue of a machine that will not dequeue it and is
// counted with the rest (UnseenExternalEvents), and an ending finds its
// invocation over. Nothing taken is dropped unsaid.
func (e *Engine[S, E]) closeInbox() {
	e.inbox.close()
	e.drainInbox()
	e.leaveDirectory()
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
		e.applyHostInvokeEnding(input.ending)
	case inboxMeshDelivery:
		e.ApplyMeshCall(input.mesh)
	}
}

// applyHostInvokeEnding applies a posted ending of one of this session's own
// invocations by CompleteHostInvokeFrom or FailHostInvoke: an ending naming
// another session of the tree was posted to that session's inbox. What either
// answers is a refusal there is no caller left to hand to (see
// Inbox.EndHostInvoke).
func (e *Engine[S, E]) applyHostInvokeEnding(ending HostInvokeEnding) {
	if ending.Done {
		e.CompleteHostInvokeFrom(ending.ProcessorType, ending.InvokeID, ending.Token, ending.Data, ending.Origin, ending.OriginType)
		return
	}
	e.FailHostInvoke(ending.ProcessorType, ending.InvokeID, ending.Token, ending.Data, ending.Origin, ending.OriginType)
}
