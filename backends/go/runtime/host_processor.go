// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"math"
	"slices"
	"strconv"
	"strings"
)

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

// performDeferredHostSend performs a host-served send whose delay has elapsed,
// and reports it if nobody did (§scxml-6.2 + §scxml-6.2.4).
//
// The immediate path raises `error.execution` at the send site, which knows the
// document's event enum by name. A deferred one cannot: that site returned when
// the send was armed, so the engine owes the report — and it can make it,
// because GetEventFromName is the same lookup PerformHostSend already uses to
// turn a handler's replies into events. A document that declares no
// `error.execution` transition resolves nothing and nothing is raised, which is
// what the generated site's own template guard does.
//
// Without this, a wiring mistake on a delayed send is perfect silence: the act
// never happens, nothing says so, and the document goes on waiting for a reply
// that has nobody left to come from.
func (e *Engine[S, E]) performDeferredHostSend(request HostSendRequest) {
	if _, served := e.PerformHostSend(request); served {
		return
	}
	if e.HasEventProcessor(request.ProcessorType) {
		return
	}
	evt, known := e.policy.GetEventFromName("error.execution")
	if !known {
		return
	}
	// The same sentence the immediate site emits. One wording for one fact: a
	// consumer matching on the message must not have to know whether the send
	// it wrote carried a delay.
	meta := NewPlatformError(evt,
		"<send type='"+request.ProcessorType+"'> names a processor the host declared but never registered")
	// §scxml-6.2.4 + §scxml-5.10 (test 332): the error event MUST carry the sendid, and
	// a deferred send always has one — the scheduler needed it to be
	// cancellable.
	meta.Metadata.SendID = request.SendID
	e.Raise(meta)
}

// ── §scxml-6.4.1: `<invoke>` the HOST runs ────────────────────────────
//
// The clause leaves the invokable set to the platform in the same words
// §scxml-6.2.5 uses for `<send>`, so a host may implement its own `type` here
// too — but an invoke is not a send. It has a LIFETIME: it starts when the
// state is entered, it is cancelled if the state exits, and the document may
// be waiting on `done.invoke.<id>`. That is why the handler receives an event
// rather than a bare request, and why this is a second registry rather than a
// second use of the first one: a host that can deliver an event is not thereby
// able to run a process it must also be able to stop.
//
// The Go port of `sce_rust_runtime::host_processor`'s invoker half, field for
// field, for the reason the send half is a port: one host described once and
// ported is the whole point of keeping the shapes identical.

// HostInvokeRequest is an `<invoke>` the host runs, at the point the state was
// entered.
type HostInvokeRequest struct {
	// ProcessorType is the `type` this `<invoke>` named.
	ProcessorType string
	// InvokeID is the invoke's id (§scxml-6.4.1), auto-derived when the author
	// declared none. This is the name the DOCUMENT waits on: a completion is
	// `done.invoke.<InvokeID>`, so a host that finishes asynchronously must
	// keep it.
	InvokeID string
	// Src is `<invoke src="...">`, empty when the document named none. SCE does
	// not interpret it — what a src means is the invoked processor's business.
	Src string
	// Params are `<param>` values keyed by name; a repeated name keeps every
	// value in document order.
	Params map[string][]string
	// Content is inline `<content>`, empty when the document carried none.
	Content string
	// Token says which start of this invoke this is. The engine assigns it,
	// and a host that finishes later hands it back to CompleteHostInvoke.
	//
	// The id alone cannot say it: a state that exits and is entered again
	// starts the same `<invoke>` a second time under the same id, and a result
	// the first run produces after it was cancelled would otherwise read as
	// the second run's (§scxml-6.4 — once the state has exited, what the
	// cancelled process sends is ignored). Distinct for every start of every
	// invocation within one engine.
	Token uint64
}

// HostInvokeCancel is an `<invoke>` the host was running, at the point its
// state exited.
type HostInvokeCancel struct {
	// ProcessorType is the `type` the `<invoke>` named.
	ProcessorType string
	// InvokeID is the invocation being cancelled — the same id its Start
	// carried.
	InvokeID string
	// Token is the one its Start carried, so a host running more than one
	// start of the same id stops the right one.
	Token uint64
}

// HostInvokeEvent is one turn of a host-run invoke's lifecycle. Exactly one of
// Start and Cancel is non-nil.
//
// Both arms go to ONE registered handler rather than to two separately
// registered callbacks, because a host that can start an invocation and cannot
// stop it is not a working invoker — and two registrations make that state
// reachable. One handler means the pair is registered together or not at all.
type HostInvokeEvent struct {
	// Start is §scxml-6.4: the state was entered and the macrostep has
	// settled. Begin the invoked process.
	Start *HostInvokeRequest
	// Cancel is §scxml-6.4: the state exited. Stop it.
	//
	// Delivered only for an invocation that is still running: one that never
	// started (its state exited before the macrostep ended) has nothing to
	// tear down, and one that already completed has nothing left to stop —
	// `done.invoke` said the process is over.
	Cancel *HostInvokeCancel
}

// HostInvokeResponse is a host invoker's answer to a Start.
//
// Read only for Start; an answer to a Cancel is ignored, because there is
// nothing left for it to mean.
type HostInvokeResponse struct {
	// DoneData is the payload for an immediate `done.invoke.<InvokeID>`, for an
	// invocation that completed before returning. Nil is the ordinary case: the
	// work outlives the call and the host reports it through CompleteHostInvoke
	// with the request's Token when it finishes. SCE does not synthesise a
	// completion the host did not report —
	// an invoked process that never terminates never fires `done.invoke`, which
	// is what §scxml-6.4 says.
	DoneData *string
}

// HostInvokeHandler is a registered invoke-lifecycle handler.
type HostInvokeHandler func(HostInvokeEvent) *HostInvokeResponse

// RegisterInvoker registers what runs every `<invoke type="<t>">` this machine
// executes (§scxml-6.4.1).
//
// Separate from RegisterEventProcessor because they are separate contracts.
// Registering twice for one type replaces the handler, for the reason the send
// half does.
func (e *Engine[S, E]) RegisterInvoker(processorType string, handler HostInvokeHandler) {
	if e.hostInvokers == nil {
		e.hostInvokers = make(map[string]HostInvokeHandler)
	}
	e.hostInvokers[processorType] = handler
}

// HasInvoker reports whether an invoker is registered for processorType.
func (e *Engine[S, E]) HasInvoker(processorType string) bool {
	_, ok := e.hostInvokers[processorType]
	return ok
}

// PerformHostInvoke starts a host-run invocation and reports whether anything
// ran (§scxml-6.4.1).
//
// `false` means no invoker was registered, which the generated site turns into
// `error.execution` — an invoke nobody ran is the same fact whether the type
// was undeclared or the handler was never wired up.
//
// A started invocation is RECORDED here rather than in the generated machine,
// because "is this one still running?" is the question both the cancel path
// and a completion have to answer, and answering it in each backend's template
// would be the same bookkeeping written once per language. The token is
// assigned and recorded BEFORE the handler runs, so a handler that completes
// synchronously completes an invocation the engine already knows is running.
//
// A `_sce_deadline_ms` param is the engine's, not the host's: it is taken out
// of the request and armed with the start. A value that is not a whole number
// of milliseconds raises `error.execution` and starts nothing — an argument of
// the element that cannot be evaluated (§scxml-6.4.1) — and the call answers
// true because the report has been made; false is kept for "no invoker ran
// it".
func (e *Engine[S, E]) PerformHostInvoke(request HostInvokeRequest) bool {
	handler, ok := e.hostInvokers[request.ProcessorType]
	if !ok {
		return false
	}
	deadlineMs, hasDeadline, err := takeHostInvokeDeadline(&request)
	if err != nil {
		if evt, known := e.policy.GetEventFromName("error.execution"); known {
			e.Raise(NewPlatformError(evt, fmt.Sprintf(
				"<invoke type='%s' id='%s'>: %v", request.ProcessorType, request.InvokeID, err)))
		}
		return true
	}
	token := e.nextHostInvokeToken
	e.nextHostInvokeToken++
	request.Token = token
	if e.startedHostInvokes == nil {
		e.startedHostInvokes = make(map[hostInvokeKey]uint64)
	}
	e.startedHostInvokes[hostInvokeKey{request.ProcessorType, request.InvokeID}] = token
	if hasDeadline {
		e.scheduler.ScheduleHostInvokeDeadlineAt(HostInvokeDeadline{
			ProcessorType: request.ProcessorType,
			InvokeID:      request.InvokeID,
			Token:         token,
		}, saturatingAddMs(e.schedNowMs(), deadlineMs))
	}
	response := handler(HostInvokeEvent{Start: &request})
	if response != nil && response.DoneData != nil {
		// §scxml-6.4: a completion the host reported NOW takes the same door as
		// one it reports later, so the two cannot disagree about whether the
		// invocation is over. The engine never synthesises a completion the host
		// did not report.
		e.CompleteHostInvoke(request.ProcessorType, request.InvokeID, token, *response.DoneData)
	}
	return true
}

// CompleteHostInvoke reports that a host-run invocation finished, raising its
// `done.invoke.<invokeID>` with doneData as `_event.data` (§scxml-6.4).
//
// token is the one the invocation's Start request carried. The completion is
// accepted only while that start is still running, and accepting it ends the
// invocation, so it is accepted at most once and the state's exit no longer
// cancels it. It returns false — raising nothing — for a completion of an
// invocation that was cancelled, already completed, or restarted since (a
// stale token): §scxml-6.4 has the processor ignore what a cancelled process
// sends, and a restarted `<invoke>` is a different process under the same id.
//
// This is the only way a host-run invocation's `done.invoke` reaches the
// document. One raised through the ordinary external-event API skips the check
// above, so the engine refuses it when it is dequeued.
func (e *Engine[S, E]) CompleteHostInvoke(processorType, invokeID string, token uint64, doneData string) bool {
	key := hostInvokeKey{processorType, invokeID}
	if running, ok := e.startedHostInvokes[key]; !ok || running != token {
		return false
	}
	delete(e.startedHostInvokes, key)
	e.scheduler.DropHostInvokeDeadline(token)
	// The id is the DOCUMENT's, because `done.invoke.<id>` is the name the
	// author wrote a transition for. §scxml-3.12.1: when the document names no
	// specific completion, the generic `done.invoke` its descriptor matches —
	// as an SCXML child's completion does.
	evt, known := e.policy.GetEventFromName(CreateDoneInvokeEventName(invokeID))
	if !known {
		evt, known = e.policy.GetEventFromName(DoneInvokeEvent)
	}
	if known {
		meta := NewEventWithMetadata(evt)
		meta.Metadata = ExternalMetadata("", "")
		meta.Metadata.Data = doneData
		// §scxml-5.10.1: the completion is an event of the invocation, so
		// `_event.invokeid` is the invocation's id — the one `done.invoke.<id>`
		// names and the host was handed.
		meta.Metadata.InvokeID = invokeID
		meta.Metadata.HostInvokeToken = &token
		e.externalQueue.Raise(meta)
	}
	return true
}

// RefusedHostInvokeCompletions reports how many host-run invocations'
// `done.invoke` events the engine refused because they did not arrive through
// CompleteHostInvoke.
//
// A refusal is correct — such an event may be a cancelled run's late reply —
// and saying it happened is not optional: a host that raised its completion
// the old way and saw the document never move would otherwise have nothing to
// find.
func (e *Engine[S, E]) RefusedHostInvokeCompletions() uint64 {
	return e.refusedHostInvokeCompletions
}

// CancelHostInvoke stops a host-run invocation, if it is still running
// (§scxml-6.4).
//
// Unconditional at the call site: the engine knows whether this one is still
// running and stays silent when it never started or already completed, so the
// emitted exit chain does not need its own bookkeeping.
func (e *Engine[S, E]) CancelHostInvoke(processorType, invokeID string) bool {
	key := hostInvokeKey{processorType, invokeID}
	token, running := e.startedHostInvokes[key]
	if !running {
		return false
	}
	delete(e.startedHostInvokes, key)
	e.scheduler.DropHostInvokeDeadline(token)
	return e.deliverHostInvokeCancel(processorType, invokeID, token)
}

func (e *Engine[S, E]) deliverHostInvokeCancel(processorType, invokeID string, token uint64) bool {
	handler, ok := e.hostInvokers[processorType]
	if !ok {
		return false
	}
	handler(HostInvokeEvent{Cancel: &HostInvokeCancel{
		ProcessorType: processorType,
		InvokeID:      invokeID,
		Token:         token,
	}})
	return true
}

// expireHostInvoke ends the start the deadline names, if it is still running:
// the host is told to stop, and the document receives `error.invoke.<id>` — or
// the generic `error.invoke` when it names no specific one — with
// `_event.invokeid` set and `_event.data` the string "deadline". Through the
// same door a completion takes, so exactly one of the two happens.
func (e *Engine[S, E]) expireHostInvoke(deadline HostInvokeDeadline) {
	key := hostInvokeKey{deadline.ProcessorType, deadline.InvokeID}
	if running, ok := e.startedHostInvokes[key]; !ok || running != deadline.Token {
		return
	}
	delete(e.startedHostInvokes, key)
	if !e.deliverHostInvokeCancel(deadline.ProcessorType, deadline.InvokeID, deadline.Token) {
		return
	}
	evt, known := e.policy.GetEventFromName(ErrorInvokePrefix + deadline.InvokeID)
	if !known {
		evt, known = e.policy.GetEventFromName(ErrorInvokeEvent)
	}
	if known {
		meta := NewEventWithMetadata(evt)
		meta.Metadata = ExternalMetadata("", "")
		meta.Metadata.Data = `"deadline"`
		meta.Metadata.InvokeID = deadline.InvokeID
		e.externalQueue.Raise(meta)
	}
}

// HostInvokeDeadlineParam is the reserved `<param>` a host-run `<invoke>`
// names its deadline with, in milliseconds. The engine reads it and does not
// hand it to the host. A neutral name rather than `sce:mesh-rpc`'s
// `_mesh_deadline_ms`, so the two invoke types converge on one spelling.
const HostInvokeDeadlineParam = "_sce_deadline_ms"

// ParseHostInvokeDeadlineMs reads the text of a HostInvokeDeadlineParam value
// as milliseconds.
//
// One or more ASCII digits, optionally followed by `.` and one or more `0` — a
// `<param expr>` reaches the request as the text of its value, and a script
// engine may render a whole number `5000.0` — within a signed 64-bit count.
// Anything else is refused: no sign, no whitespace, no exponent, no digit
// separator.
//
// Spelled out rather than left to strconv because every runtime implements it
// and the languages' parsers disagree (ParseFloat reads hex floats, Python
// reads `1_000`); the table they are all held to is
// sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json.
func ParseHostInvokeDeadlineMs(written string) (int64, bool) {
	whole, fraction, dotted := strings.Cut(written, ".")
	if whole == "" || strings.IndexFunc(whole, func(r rune) bool { return r < '0' || r > '9' }) >= 0 {
		return 0, false
	}
	if dotted && (fraction == "" || strings.Trim(fraction, "0") != "") {
		return 0, false
	}
	// All ASCII digits, so the only way this fails is a value past MaxInt64.
	ms, err := strconv.ParseInt(whole, 10, 64)
	if err != nil {
		return 0, false
	}
	return ms, true
}

// saturatingAddMs is now plus a deadline, held at MaxInt64: a deadline may be
// as large as MaxInt64 itself, and a wrapped sum would put it in the past and
// end the invocation at once.
func saturatingAddMs(nowMs, deadlineMs int64) int64 {
	if deadlineMs > math.MaxInt64-nowMs {
		return math.MaxInt64
	}
	return nowMs + deadlineMs
}

// takeHostInvokeDeadline takes `_sce_deadline_ms` out of request.
//
// It returns (ms, true, nil) for one value ParseHostInvokeDeadlineMs reads,
// (0, false, nil) when the document gave none, and an error naming what was
// written otherwise, including the name given twice, for the caller to report.
func takeHostInvokeDeadline(request *HostInvokeRequest) (int64, bool, error) {
	values, present := request.Params[HostInvokeDeadlineParam]
	if !present {
		return 0, false, nil
	}
	delete(request.Params, HostInvokeDeadlineParam)
	if len(values) == 1 {
		if ms, ok := ParseHostInvokeDeadlineMs(values[0]); ok {
			return ms, true, nil
		}
	}
	return 0, false, fmt.Errorf("%s=%q is not a whole number of milliseconds",
		HostInvokeDeadlineParam, strings.Join(values, ","))
}

// HostInvokePolicy is implemented by a generated policy whose document hands
// `<invoke>`s to a host invoker (§scxml-6.4.1). Optional, because a document
// with no host-run invoke has nothing to list.
type HostInvokePolicy interface {
	// HostInvokeIDs returns the ids of those invokes. Their `done.invoke.<id>`
	// is accepted only through CompleteHostInvoke.
	HostInvokeIDs() []string
}

// isRefusedHostInvokeCompletion reports whether meta is a host-run
// invocation's `done.invoke` that did not come through CompleteHostInvoke —
// one the engine refuses at dequeue (§scxml-6.4), because it may be a
// cancelled run's late reply.
func (e *Engine[S, E]) isRefusedHostInvokeCompletion(meta EventWithMetadata[E]) bool {
	if meta.Metadata.HostInvokeToken != nil {
		return false
	}
	hostPolicy, ok := any(e.policy).(HostInvokePolicy)
	if !ok {
		return false
	}
	name := e.policy.GetEventName(meta.Event)
	if id, isSpecific := strings.CutPrefix(name, DoneInvokePrefix); isSpecific {
		return slices.Contains(hostPolicy.HostInvokeIDs(), id)
	}
	// The generic `done.invoke` a document that names no specific completion
	// receives: its invokeid says whose completion it is.
	return name == DoneInvokeEvent && slices.Contains(hostPolicy.HostInvokeIDs(), meta.Metadata.InvokeID)
}

// hostInvokeKey identifies one host-run invocation that was started and has
// neither completed nor been cancelled. One entry per key is enough: an
// `<invoke>` belongs to one state and a state is in the configuration at most
// once, so the same id is never running twice. A second start under the same
// id replaces the entry, and its new token is what makes the first run's late
// reply stale.
type hostInvokeKey struct {
	processorType string
	invokeID      string
}
