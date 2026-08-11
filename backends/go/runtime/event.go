// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
	// Data is the _event.data payload as a JSON string (empty when no payload).
	Data string

	// EventType is the _event.type classification (internal / external / platform).
	EventType EventType

	// SendID is the _event.sendid from originating <send> (empty if none).
	SendID string

	// Origin is the _event.origin URI (empty if no origin).
	Origin string

	// OriginType is the _event.origintype (e.g., SCXML Event I/O Processor URI).
	OriginType string

	// InvokeID is the _event.invokeid if event came from a child invoke (W3C 6.4.1).
	InvokeID string

	// TypedPayload is the NL→IR Item C1 Path A typed `_event.data` payload —
	// a generated per-event payload struct carried through the queue when an
	// EventSchema-imported event is injected via the generated
	// `Raise<Event>` seam. The generated policy's PopulateEventMetadata
	// type-asserts it into the policy's typed `pending<Event>Payload` field,
	// which the natively-lowered transition guards read (no script engine).
	// Nil for every event raised without a typed payload, against which the
	// native guards' tag check simply fails. The type-erased carrier is the
	// Go-idiomatic twin of the Rust runtime's statically-typed
	// `EventWithMetadata<E, P>.payload`; the name↔type pairing is enforced at
	// the single generated `Raise<Event>` constructor, so it can never be
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

// ExternalMetadata constructs external metadata with a send ID and origin (e.g., for <send>).
func ExternalMetadata(sendID, origin string) EventMetadata {
	return EventMetadata{
		EventType:  EventTypeExternal,
		SendID:     sendID,
		Origin:     origin,
		OriginType: SCXMLEventProcessorType,
	}
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

// NewEventWithFields constructs an EventWithMetadata with full metadata.
//
// Parameter order matches the Rust with_fields constructor.
func NewEventWithFields[E any](
	event E,
	data string,
	origin string,
	sendID string,
	eventType EventType,
	originType string,
	invokeID string,
	target string,
) EventWithMetadata[E] {
	return EventWithMetadata[E]{
		Event: event,
		Metadata: EventMetadata{
			Data:       data,
			EventType:  eventType,
			SendID:     sendID,
			Origin:     origin,
			OriginType: originType,
			InvokeID:   invokeID,
		},
		Target: target,
	}
}
