// SCE-GENERATED — DO NOT EDIT
// source-hash: 52dec3eb3cccdf24690d474eaf44f8ac67bafec1b1ed5060c73db2a6256056db
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
