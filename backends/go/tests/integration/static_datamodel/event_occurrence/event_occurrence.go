// SCE-GENERATED — DO NOT EDIT
// source-hash: 66cd3fde8d86fbb89bd1a697c3bc992ba7a38c904305e582272ccf11b3c1bc2a
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
