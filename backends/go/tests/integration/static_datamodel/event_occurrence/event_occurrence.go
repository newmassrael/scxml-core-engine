// SCE-GENERATED — DO NOT EDIT
// source-hash: 6caf91b7141c88d0a508e4aad3e1cd20df805c8c56903e8b0db3f1f4b706723b
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
