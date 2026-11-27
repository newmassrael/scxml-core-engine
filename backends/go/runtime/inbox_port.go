// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// The inbox port: what a run tells a test of each event the engine puts on its
// external queue, so the run can be written as an inbox history and judged
// linearizable to the promise of the inbox (docs/adr/0147, D3;
// docs/formal/InboxService.tla) by `sce-codegen check-inbox-history`.
//
// The engine tells the port twice for each event it puts on its external queue
// (enqueueExternal, the one door every external enqueue of the engine goes
// through): when it begins to put the event there, ahead of the drain that
// applies what its inbox holds first (docs/adr/0084, D2 as amended at step 9),
// and once the event has entered. That drain's read of the inbox is where the
// event linearizes: what the inbox held then enters ahead of it, and a post made
// after it may follow it. The inputs the drain applies are told inside the
// event's two calls, each with its own two. Each call says whether the inbox
// held the event, an input a goroutine posted that the owner applied, or the
// engine raised it itself. A test reads one clock in both calls and has each
// enqueue of the history (tests/forge/conformance/inbox_history.schema.json).
//
// With no port set each external enqueue asks one nil check, as the transition
// observer's does (transition_observer.go). The port reads and is asked nothing,
// so an engine with one set takes the steps one without takes.
//
// The cross-language siblings are the C11 machine's
// `<PREFIX><NAME>_external_enqueue` (SCE_INBOX_PORT), the C++ engine's
// `setInboxPort` (core/InboxPort.h), and the Rust runtime's external queue a
// policy supplies (docs/adr/0076).

// InboxEnqueue is one of the two calls the inbox port is told for an event the
// engine puts on its external queue.
type InboxEnqueue[E comparable] struct {
	// Event is the event, with the metadata it enters the queue with. The port
	// reads it and changes nothing it reaches.
	Event EventWithMetadata[E]
	// Posted is true for an input the engine applied from its inbox, and false
	// for an event of its own: a <send> to itself, a delayed send come due, an
	// ending or a report a host gave it on the owner, the host's own raise.
	Posted bool
	// Entered is false when the engine begins to put the event on its queue,
	// ahead of the drain before it, and true once the event has entered.
	Entered bool
}

// SetInboxPort tells port each event this engine puts on its external queue
// from now on, twice: when the engine begins to put it there, ahead of the
// drain that applies what the inbox holds first, and once it has entered; nil
// stops it (docs/adr/0147, D3). The inputs that drain applies are told inside
// the event's two calls.
//
// It runs on the goroutine that drives the engine, inside the call that drove
// it, and must not call back into the engine; a panic in it is not recovered.
func (e *Engine[S, E]) SetInboxPort(port func(InboxEnqueue[E])) {
	e.inboxPort = port
}

// tellInboxPort tells the inbox port, if one is set, one of the two calls of
// event.
func (e *Engine[S, E]) tellInboxPort(event EventWithMetadata[E], posted, entered bool) {
	if e.inboxPort != nil {
		e.inboxPort(InboxEnqueue[E]{Event: event, Posted: posted, Entered: entered})
	}
}
