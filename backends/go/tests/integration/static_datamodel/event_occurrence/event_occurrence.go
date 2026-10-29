// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: event_occurrence.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.occurrence
// Do not edit — regenerate from the source SCXML file.

package event_occurrence

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "calendar.occurrence"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type EventOccurrencePayload struct {
	Id uint64
	Start int64
	End int64
	Nth uint32
	Conflict uint8
}
