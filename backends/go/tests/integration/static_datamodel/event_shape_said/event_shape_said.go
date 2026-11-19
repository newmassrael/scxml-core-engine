// SCE-GENERATED — DO NOT EDIT
// source-hash: c0a22f7087ae1a662758481a5db83f6694decd14f2d8e252aa2a44fa0f5e527b
// source: event_shape_said.scxml
// SCE-MAP: event_shape_said.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.said
// Do not edit — regenerate from the source SCXML file.

package event_shape_said

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "shape.said"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TextMaxSize is the most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
const TextMaxSize uint32 = 96

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 587

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:event_shape_said:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shape_said:1\",\"x-sce-source-hash\":\"c0a22f7087ae1a662758481a5db83f6694decd14f2d8e252aa2a44fa0f5e527b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":96,\"x-sce-max-bytes\":96}},\"required\":[\"text\"],\"additionalProperties\":false}"

type EventShapeSaidPayload struct {
	Text string
}
