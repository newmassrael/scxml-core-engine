// SCE-GENERATED — DO NOT EDIT
// source-hash: a9892366fdd147be953ddf1a986120de05d135e5d4598fc7b7fb886fa0b8ae66
// source: schema_partial.scxml
// SCE-MAP: schema_partial.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: partial
// Do not edit — regenerate from the source SCXML file.

package schema_partial

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "partial"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 31

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_partial:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_partial:1\",\"x-sce-source-hash\":\"a9892366fdd147be953ddf1a986120de05d135e5d4598fc7b7fb886fa0b8ae66\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"total\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"part\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"total\",\"part\"],\"additionalProperties\":false}"

type SchemaPartialPayload struct {
	Total uint32
	Part uint8
}
