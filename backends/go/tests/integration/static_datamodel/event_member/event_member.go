// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
// SCE-MAP: event_member.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.member
// Do not edit — regenerate from the source SCXML file.

package event_member

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "calendar.member"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type EventMemberPayload struct {
	Node uint64
	Role uint8
}
