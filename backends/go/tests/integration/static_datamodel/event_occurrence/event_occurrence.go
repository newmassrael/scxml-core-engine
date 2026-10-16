// SCE-GENERATED — DO NOT EDIT
// source-hash: b9ab4ddbd49e8ce2a3b4c80917bc3eecdf0d5285c15974d4e0a00636f93fd573
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
