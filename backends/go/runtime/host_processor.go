// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// Host-supplied Event I/O Processors — the payload types a host registers a
// handler for.
//
// §scxml-6.2.5 makes a `<send>` `type` an extensible identifier, so the set of
// Event I/O Processors is open by design. SCE implements two of them; anything
// else was refused with `error.execution` and no platform could widen the set —
// a consumer could name a processor and be refused, but not name one and be
// served. A host declares the types it serves at build time (so codegen emits a
// dispatch instead of a refusal) and registers a handler for each at run time.
//
// The Go port of `sce_rust_runtime::host_processor` and
// `sce/include/core/HostProcessor.h`, field for field. Keeping the shape
// identical is what lets one host be described once and ported, and it is the
// same reason HttpSendRequest here matches its siblings.
//
// Shaped after HttpSendRequest, which is the same idea fixed to one type: a
// request struct carrying what `<send>` said, and a reply the engine turns back
// into an event. The difference is the key — this one is looked up by the
// `type` string.

// HostSendRequest is what a `<send>` addressed to a host-served processor said.
//
// Every field is what the document wrote, not an interpretation of it: a
// handler that wants to reject a malformed request needs to see the same thing
// the author typed.
type HostSendRequest struct {
	// ProcessorType is the `type` this send named. Present even though the
	// handler was looked up by it, because one handler may be registered for
	// several types and would otherwise have to be told which it is by a
	// closure capture per registration.
	ProcessorType string
	// EventName is `<send event="...">`, or the value `eventexpr` evaluated to.
	EventName string
	// Target is `<send target="...">`, empty when the document named none. The
	// specification leaves a target's meaning to the processor that serves it,
	// so SCE passes it through uninterpreted.
	Target string
	// Content is inline `<content>`, empty when the document carried none.
	Content string
	// Params holds `<param>` values, keyed by name. A repeated name keeps every
	// value in document order rather than the last one winning — §scxml-6.2
	// permits repetition and dropping it would lose data the author wrote.
	Params map[string][]string
	// SendID is the send's id, auto-generated when the document declared none.
	// A handler correlating a reply, or honouring a `<cancel>`, needs it.
	SendID string
}

// HostSendResponse is one event a host-served act produced.
//
// The engine raises each on the external queue, which is where a reply from
// outside the machine belongs (§scxml-C-1).
type HostSendResponse struct {
	// EventName is the event to raise. A name the generated machine does not
	// declare is dropped, matching what the engine does with any such event.
	EventName string
	// EventData is the payload for `_event.data`, empty for a bare reply.
	EventData string
}

// HostSendHandler is what a host registers for one declared processor type.
//
// It answers with the events the act produced, IN ORDER. A list rather than a
// single reply, for one reason: an act can produce two observations that the
// document must see in a particular order, and every other way of expressing
// that costs portability or hides state.
//
// `examples/ai_loop/` is the case. Its `priming` state leaves on `prompt.sent`
// — "the session has been told what it is here for" — and only then is the
// machine somewhere a turn result means anything; reporting the turn first
// leaves the run sitting in `priming` forever. So prompting a fresh session
// produces exactly two events with exactly one correct order.
//
// A nil or empty answer is "performed, nothing to report", which is the common
// case for a fire-and-forget act and for real work that will answer later
// through the host's own loop. It is NOT an error, and the engine does not
// treat it as one.
//
// A handler that panics is a host defect and is not recovered here — the engine
// cannot invent a W3C-meaningful outcome for it, and swallowing it would
// produce exactly the silence this whole surface exists to remove.
type HostSendHandler func(HostSendRequest) []HostSendResponse

// RegisterEventProcessor registers what performs every `<send type="<t>">` this
// machine executes (§scxml-6.2.5).
//
// The build's half of the contract is the `--host-processor` declaration that
// made codegen emit a dispatch here instead of a refusal. A type declared to
// the build and never registered raises `error.execution` at the send, because
// from the document's side an act nobody performed is the same either way.
//
// Registering twice for one type replaces the handler: two handlers for one
// type would make dispatch depend on registration order, and a host
// re-registering during a run means to change what serves the act.
func (e *Engine[S, E]) RegisterEventProcessor(processorType string, handler HostSendHandler) {
	if e.hostProcessors == nil {
		e.hostProcessors = make(map[string]HostSendHandler)
	}
	e.hostProcessors[processorType] = handler
}

// HasEventProcessor reports whether a handler is registered for processorType.
//
// Two things can go wrong with a host-served send — no handler, or a handler
// that answered nothing — and only the first is an error. The generated site
// reads this to tell them apart, which is why it is exported.
func (e *Engine[S, E]) HasEventProcessor(processorType string) bool {
	_, ok := e.hostProcessors[processorType]
	return ok
}

// PerformHostSend dispatches a host-served `<send>` and raises, in order, every
// event the handler says the act produced (§scxml-6.2).
//
// With no handler registered the send raises `error.execution` at the generated
// site, the same outcome an undeclared type produces. That is the point: the
// document asked for an act, and from its side "no processor implements this
// type" and "the processor was never wired up" are one fact. Reporting them
// differently would make a wiring mistake look like a document error, or worse,
// look like success.
//
// The two answers are therefore `false` ("no handler") and `true` plus a list
// ("the handler ran and produced these"). An empty list with `true` is a
// success.
//
// §scxml-C-1: a reply from outside the machine arrives on the EXTERNAL queue,
// like any event the host raises — resolved by name, so a reply naming an event
// this machine does not declare is dropped exactly as any such event is rather
// than derailing the run. Raised in list order, because a handler reporting two
// observations is reporting a sequence: see HostSendHandler for the case that
// decided the shape.
func (e *Engine[S, E]) PerformHostSend(request HostSendRequest) ([]HostSendResponse, bool) {
	handler, ok := e.hostProcessors[request.ProcessorType]
	if !ok {
		return nil, false
	}
	replies := handler(request)
	for _, reply := range replies {
		if reply.EventName == "" {
			continue
		}
		if evt, known := e.policy.GetEventFromName(reply.EventName); known {
			meta := NewEventWithMetadata(evt)
			meta.Metadata = ExternalMetadata("", "")
			meta.Metadata.Data = reply.EventData
			e.externalQueue.Raise(meta)
		}
	}
	return replies, true
}
