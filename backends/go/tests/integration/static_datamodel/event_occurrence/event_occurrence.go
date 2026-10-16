// SCE-GENERATED — DO NOT EDIT
// source-hash: 5b579589f1ff33cd05a081b40dc01d4d0d0b2ef410a9718dd0ae11e84c00a952
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
