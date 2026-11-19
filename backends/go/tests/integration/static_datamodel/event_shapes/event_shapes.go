// SCE-GENERATED — DO NOT EDIT
// source-hash: a2a41f9c114721c6259e3c466cc2c319cbe1a1fd35c33c6c230e49b79b20b88f
// source: event_shapes.scxml
// SCE-MAP: event_shapes.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shapes.sent
// Do not edit — regenerate from the source SCXML file.

package event_shapes
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_union_shape "github.com/newmassrael/sce-go-tests/integration/static_datamodel/union_shape"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "shapes.sent"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// ManyMaxCount is the most elements `many` holds (the bound its type writes), as this build resolved it.
const ManyMaxCount uint32 = 3

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 569

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:event_shapes:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shapes:1\",\"x-sce-source-hash\":\"a2a41f9c114721c6259e3c466cc2c319cbe1a1fd35c33c6c230e49b79b20b88f\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"one\":{\"$ref\":\"#/$defs/union_shape\"},\"many\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/union_shape\"},\"minItems\":1,\"maxItems\":3},\"maybe\":{\"anyOf\":[{\"$ref\":\"#/$defs/union_shape\"},{\"type\":\"null\"}]}},\"required\":[\"one\",\"many\",\"maybe\"],\"additionalProperties\":false,\"$defs\":{\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"

type EventShapesPayload struct {
	One sce_union_shape.UnionShape
	Many []sce_union_shape.UnionShape
	Maybe optional.Of[sce_union_shape.UnionShape]
}
