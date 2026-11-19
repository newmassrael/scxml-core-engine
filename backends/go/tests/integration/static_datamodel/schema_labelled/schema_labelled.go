// SCE-GENERATED — DO NOT EDIT
// source-hash: 8424ebfed2ae08ff623eb235e1be555740538f3bb2ff0793cc96d049316e296e
// source: schema_labelled.scxml
// SCE-MAP: schema_labelled.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: labelled.taken
// Do not edit — regenerate from the source SCXML file.

package schema_labelled

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "labelled.taken"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// LabelMaxSize is the most UTF-8 bytes `label` holds (the bound its type writes), as this build resolved it.
const LabelMaxSize uint32 = 8

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 73

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_labelled:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_labelled:1\",\"x-sce-source-hash\":\"8424ebfed2ae08ff623eb235e1be555740538f3bb2ff0793cc96d049316e296e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sensor\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"label\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8}},\"required\":[\"sensor\",\"label\"],\"additionalProperties\":false}"

type SchemaLabelledPayload struct {
	Sensor uint8
	Label string
}
