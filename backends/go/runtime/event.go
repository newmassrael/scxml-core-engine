// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

// EventType classification (§scxml-5.10.1).
//
// Ports the Rust EventType enum from backends/rust/runtime/src/event.rs.
type EventType int

const (
	// EventTypeInternal represents events raised internally via <raise> or
	// a targetless <send> (§scxml-5.10.1).
	EventTypeInternal EventType = iota

	// EventTypeExternal represents events from an external source (W3C SCXML
	// 5.10.1 -- the default classification).
	EventTypeExternal

	// EventTypePlatform represents platform-generated events (e.g.,
	// error.execution, done.state.*).
	EventTypePlatform
)

// String returns the C++ string-equivalent representation for interop and logging.
func (t EventType) String() string {
	switch t {
	case EventTypeInternal:
		return "internal"
	case EventTypeExternal:
		return "external"
	case EventTypePlatform:
		return "platform"
	default:
		return "unknown"
	}
}

// EventMetadata carries §scxml-5.10 metadata fields attached to events.
//
// Ports Rust EventMetadata from backends/rust/runtime/src/event.rs. Every event
// flowing through the engine carries an EventMetadata, which is copied into
// _event.* fields in the script engine.
type EventMetadata struct {
	// Name is the _event.name the event ARRIVED under (§scxml-5.10), when that is
	// not the name of the enum value that carries it. The event is carried as the
	// value of a name the document writes, and §scxml-3.12.1 lets `request.new`
	// take a transition on `request`, so the value alone loses the longer name.
	// Set by a door that resolved a name through a prefix
	// (Engine.ResolveEventByName); empty means the value's own name IS the name,
	// which is every event the document itself raises or sends.
	Name string

	// Data is the _event.data payload as a JSON string (empty when no payload).
	Data string

	// EventType is the _event.type classification (internal / external / platform).
	EventType EventType

	// SendID is the _event.sendid from originating <send> (empty if none).
	SendID string

	// Origin is `_event.origin` and `_event.origintype` — the reply address the
	// processor that delivered the event wrote when it built it, as one value
	// (docs/adr/0090); nil for an event no processor stamped, and then the
	// document reads both fields as the empty string.
	Origin *EventOrigin

	// Sender is the id of the session that sent the event, written by the
	// processor that delivered it from that session: the SCXML Event I/O
	// Processor for a local session, and the Mesh transport for a remote child
	// or parent session. Empty for every other sender (a host, BasicHTTP, a
	// Mesh peer that is no session of this tree). It is independent of Origin,
	// whose address is the reply address that processor wrote. Not an `_event`
	// field: `<finalize>` and a saved state find the sending session by it, and
	// nothing reads the origin's address to find a session (docs/adr/0090,
	// section 1).
	Sender string

	// InvokeID is the _event.invokeid if event came from a child invoke (W3C 6.4.1).
	InvokeID string

	// HostInvokeToken is the token of the host-run invocation this event
	// completes, set only by Engine.CompleteHostInvoke after it confirmed the
	// invocation was still running. A host invoke's `done.invoke.<id>` that
	// arrives without one did not come through that check, so the engine
	// refuses it (§scxml-6.4). Not script-visible: `_event` has no field for it.
	HostInvokeToken *uint64

	// TypedPayload is the NL→IR Item C1 Path A typed `_event.data` payload —
	// a generated per-event payload struct carried through the queue when an
	// EventSchema-imported event is injected via the generated
	// `<Machine>Raise<Event>` seam. The generated policy's PopulateEventMetadata
	// type-asserts it into the policy's typed `pending<Event>Payload` field,
	// which the natively-lowered transition guards read (no script engine).
	// Nil for every event raised without a typed payload, against which the
	// native guards' tag check simply fails. The type-erased carrier is the
	// Go-idiomatic twin of the Rust runtime's statically-typed
	// `EventWithMetadata<E, P>.payload`; the name↔type pairing is enforced at
	// the single generated `<Machine>Raise<Event>` constructor, so it can never be
	// constructed inconsistently.
	TypedPayload any
}

// PlatformMetadata constructs platform metadata (e.g., for error.execution, done.state.*).
func PlatformMetadata() EventMetadata {
	return EventMetadata{EventType: EventTypePlatform}
}

// InternalMetadata constructs internal metadata (e.g., for <raise>).
func InternalMetadata() EventMetadata {
	return EventMetadata{EventType: EventTypeInternal}
}

// ExternalMetadata constructs external metadata with a send ID and no origin:
// an event no Event I/O Processor stamped — one a host raises naming no party,
// a reply a host processor gives naming none, an HTTP response (docs/adr/0090,
// sections 2 and 3). `_event.origin` and `_event.origintype` read empty.
func ExternalMetadata(sendID string) EventMetadata {
	return EventMetadata{
		EventType: EventTypeExternal,
		SendID:    sendID,
	}
}

// SentBySession constructs the external metadata the SCXML Event I/O
// Processor writes for an event the session sender sent: the pair of its own
// type and the location sender publishes, and sender beside it (docs/adr/0090,
// section 2). A machine that holds no session id sends an event with no
// origin.
func SentBySession(sendID, sender string) EventMetadata {
	meta := ExternalMetadata(sendID)
	meta.Origin = ScxmlOriginOf(sender)
	meta.Sender = sender
	return meta
}

// OriginAddress is `_event.origin`: the address of the event's origin, or the
// empty string when it has none.
func (m EventMetadata) OriginAddress() string {
	return m.Origin.Address()
}

// OriginProcessor is `_event.origintype`: the processor of the event's origin,
// or the empty string when it has none.
func (m EventMetadata) OriginProcessor() string {
	return m.Origin.Processor()
}

// EventOrigin is where a reply to an event goes: the Event I/O Processor that
// delivered it and the address that processor accepts as a `<send>` target
// (docs/adr/0090).
//
// Ports C++ `SCE::Core::EventOrigin`. The document reads the two as
// `_event.origintype` and `_event.origin`; they are one value because the
// address means something only relative to its processor. The processor that
// delivers an event writes the pair once, where it builds the event, and
// nothing converts it later. Its fields are unexported, so a pair is made whole
// by NewEventOrigin or not at all: an event with no origin holds nil.
type EventOrigin struct {
	processor string
	address   string
}

// NewEventOrigin is the pair of processor, the canonical type URI of an Event
// I/O Processor, and address, a URI that processor accepts as a target.
func NewEventOrigin(processor, address string) *EventOrigin {
	return &EventOrigin{processor: processor, address: address}
}

// Processor is the canonical type URI of the Event I/O Processor that delivered
// the event (`_event.origintype`); empty for a nil origin.
func (o *EventOrigin) Processor() string {
	if o == nil {
		return ""
	}
	return o.processor
}

// Address is a URI the processor accepts as a `<send>` target
// (`_event.origin`); empty for a nil origin.
func (o *EventOrigin) Address() string {
	if o == nil {
		return ""
	}
	return o.address
}

// Equal reports whether o and other are the same pair, or both none.
func (o *EventOrigin) Equal(other *EventOrigin) bool {
	if o == nil || other == nil {
		return o == other
	}
	return *o == *other
}

// EventWithMetadata wraps a typed event value with its full §scxml-5.10 metadata.
//
// Ports the Rust EventWithMetadata<E> from backends/rust/runtime/src/event.rs. The
// type parameter E is the generated Policy's event enum type.
type EventWithMetadata[E any] struct {
	// Event is the typed event value (e.g., TestXXXEvent.Foo).
	Event E

	// Metadata contains event data, type, sendid, origin, origintype, invokeid.
	Metadata EventMetadata

	// Target is the §scxml-C-2 HTTP POST target URL (empty if not an HTTP send).
	Target string
}

// NewEventWithMetadata constructs an EventWithMetadata from a bare event with
// default (external) metadata.
func NewEventWithMetadata[E any](event E) EventWithMetadata[E] {
	return EventWithMetadata[E]{
		Event: event,
	}
}

// NewPlatformEvent constructs a platform event (for error.execution, done.state, etc.)
// with EventType set to Platform (§scxml-5.10.1).
func NewPlatformEvent[E any](event E) EventWithMetadata[E] {
	return EventWithMetadata[E]{
		Event:    event,
		Metadata: PlatformMetadata(),
	}
}

// NewPlatformError constructs an error.* event the processor itself raised,
// carrying a description of what failed.
//
// The single way generated code raises a platform error, so what a reader is
// owed gets filled in one place rather than at each site that happens to
// remember. NewPlatformEvent above sets EventType correctly and leaves Data
// empty, which handed a document that answers error.execution an undefined
// _event.data: it could see THAT something failed and never WHAT. The clause
// closes with "platforms MAY include additional information about the nature of
// the error in the 'data' field", and the C++ Interpreter shipped from this
// same repository has passed the failing construct all along — measured
// 2026-08-19, five of six generated backends passed nothing.
//
// message describes the failing construct and is a literal formed at generation
// time, not the engine's own error text. The dynamic detail stays on the log
// line beside the call, which is where it can afford to be built.
func NewPlatformError[E any](event E, message string) EventWithMetadata[E] {
	// §scxml-3.12.2: the processor signals its own failures as error.*, and
	// "platforms MAY include additional information about the nature of the
	// error in the 'data' field". Cited in the body rather than the doc comment
	// so the ledger binds it to this symbol.
	meta := PlatformMetadata()
	meta.Data = message
	return EventWithMetadata[E]{
		Event:    event,
		Metadata: meta,
	}
}

// NewEventWithFields constructs an EventWithMetadata with full metadata.
//
// Parameter order matches the Rust with_fields constructor.
func NewEventWithFields[E any](
	event E,
	data string,
	origin *EventOrigin,
	sendID string,
	eventType EventType,
	invokeID string,
	target string,
) EventWithMetadata[E] {
	return EventWithMetadata[E]{
		Event: event,
		Metadata: EventMetadata{
			Data:      data,
			EventType: eventType,
			SendID:    sendID,
			Origin:    origin,
			InvokeID:  invokeID,
		},
		Target: target,
	}
}
