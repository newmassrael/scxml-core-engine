// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
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

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 38

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_entry:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_entry:1\",\"x-sce-source-hash\":\"d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"id\",\"amount\"],\"additionalProperties\":false}"

type SchemaEntryPayload struct {
	Id uint32
	Amount int32
}
