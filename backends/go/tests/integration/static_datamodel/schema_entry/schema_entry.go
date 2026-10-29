// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: schema_entry.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ledger.entry
// Do not edit — regenerate from the source SCXML file.

package schema_entry

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "ledger.entry"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type SchemaEntryPayload struct {
	Id uint32
	Amount int32
}
