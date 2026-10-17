// SCE-GENERATED — DO NOT EDIT
// source-hash: 38a2b8ab9943c3f76e7281d40f9dae3ea2effb54aa8b7ab5baa1ac0eaba97f5b
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
