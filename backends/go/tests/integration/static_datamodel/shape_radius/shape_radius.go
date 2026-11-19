// SCE-GENERATED — DO NOT EDIT
// source-hash: a642ecb025986ca626e48c617da36f22d67f31b07e81a8b8eda784fc31698d4d
// source: shape_radius.scxml
// SCE-MAP: shape_radius.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.radius
// Do not edit — regenerate from the source SCXML file.

package shape_radius

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "shape.radius"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 16

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:shape_radius:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:shape_radius:1\",\"x-sce-source-hash\":\"a642ecb025986ca626e48c617da36f22d67f31b07e81a8b8eda784fc31698d4d\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"radius\"],\"additionalProperties\":false}"

type ShapeRadiusPayload struct {
	Radius uint16
}
