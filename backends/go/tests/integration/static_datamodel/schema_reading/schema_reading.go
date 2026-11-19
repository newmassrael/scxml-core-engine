// SCE-GENERATED — DO NOT EDIT
// source-hash: c76c75e47e041e6aeafb35f0635b82df25066e889367466527256df659c81304
// source: schema_reading.scxml
// SCE-MAP: schema_reading.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading.taken
// Do not edit — regenerate from the source SCXML file.

package schema_reading

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "reading.taken"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 48

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_reading:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_reading:1\",\"x-sce-source-hash\":\"c76c75e47e041e6aeafb35f0635b82df25066e889367466527256df659c81304\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sensor\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"value\":{\"type\":\"number\"}},\"required\":[\"sensor\",\"value\"],\"additionalProperties\":false}"

type SchemaReadingPayload struct {
	Sensor uint8
	Value float64
}
