// SCE-GENERATED — DO NOT EDIT
// source-hash: 0fafd5dc7cb78db436777a56fff2cdefea18c3275c9e740c945e368e87cb13ce
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
