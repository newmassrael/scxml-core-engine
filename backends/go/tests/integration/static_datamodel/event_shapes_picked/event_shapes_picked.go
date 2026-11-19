// SCE-GENERATED — DO NOT EDIT
// source-hash: 3658844914dfa19b8e6f0d55f8f8a94ebbd67b3a096eafd46942a611c9c3bf53
// source: event_shapes_picked.scxml
// SCE-MAP: event_shapes_picked.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shapes.picked
// Do not edit — regenerate from the source SCXML file.

package event_shapes_picked
import (
	sce_union_shape "github.com/newmassrael/sce-go-tests/integration/static_datamodel/union_shape"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "shapes.picked"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// ManyMaxCount is the most elements `many` holds (the bound its type writes), as this build resolved it.
const ManyMaxCount uint32 = 3

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 346

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:event_shapes_picked:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shapes_picked:1\",\"x-sce-source-hash\":\"3658844914dfa19b8e6f0d55f8f8a94ebbd67b3a096eafd46942a611c9c3bf53\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"many\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/union_shape\"},\"maxItems\":3},\"at\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"many\",\"at\"],\"additionalProperties\":false,\"$defs\":{\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"

type EventShapesPickedPayload struct {
	Many []sce_union_shape.UnionShape
	At uint8
}
