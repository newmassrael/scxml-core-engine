// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
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
