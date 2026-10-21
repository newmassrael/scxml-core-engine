// SCE-GENERATED — DO NOT EDIT
// source-hash: 9313ff05336b94eeb54552b43ab70c62d4e55585e0cedc6ece4c89ceb09077d5
// SCE-MAP: record_member.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.member
// Do not edit — regenerate from the source SCXML file.

package record_member

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "record_list.member"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type RecordMemberPayload struct {
	Node uint64
	Role uint8
}
