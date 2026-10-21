// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
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
