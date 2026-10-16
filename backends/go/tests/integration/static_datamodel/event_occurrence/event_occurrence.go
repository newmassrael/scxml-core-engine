// SCE-GENERATED — DO NOT EDIT
// source-hash: f00a1af9509aeb855cb8946645995169e7a8753555e3a8541ac4f9ef548d274c
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
