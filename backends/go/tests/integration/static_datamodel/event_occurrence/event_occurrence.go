// SCE-GENERATED — DO NOT EDIT
// source-hash: fc32dee6d2d79c8cab243a42e6e452a54e10c5eb94b8b71d728b846fba971019
// SCE-MAP: event_occurrence.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.occurrence
// Do not edit — regenerate from the source SCXML file.

package event_occurrence

type EventOccurrencePayload struct {
	Id uint64
	Start int64
	End int64
	Nth uint32
	Conflict uint8
}
