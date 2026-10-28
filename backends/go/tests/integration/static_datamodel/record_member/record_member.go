// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
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
