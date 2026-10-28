// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a
// SCE-MAP: schema_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ledger.total
// Do not edit — regenerate from the source SCXML file.

package schema_total

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "ledger.total"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type SchemaTotalPayload struct {
	Count uint32
	Sum int32
}
