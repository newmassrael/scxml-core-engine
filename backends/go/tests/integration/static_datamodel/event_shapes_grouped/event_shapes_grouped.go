// SCE-GENERATED — DO NOT EDIT
// source-hash: 1d74a5169a7952ddf84dbc0e50317c8497b6dde1ec823124ae8920a1c7507a70
// source: event_shapes_grouped.scxml
// SCE-MAP: event_shapes_grouped.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shapes.grouped
// Do not edit — regenerate from the source SCXML file.

package event_shapes_grouped
import (
	sce_shape_group "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_group"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "shapes.grouped"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// GroupsMaxCount is the most elements `groups` holds (the bound its type writes), as this build resolved it.
const GroupsMaxCount uint32 = 2

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 713

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:event_shapes_grouped:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shapes_grouped:1\",\"x-sce-source-hash\":\"1d74a5169a7952ddf84dbc0e50317c8497b6dde1ec823124ae8920a1c7507a70\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"groups\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/shape_group\"},\"maxItems\":2},\"at\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"inner\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"groups\",\"at\",\"inner\"],\"additionalProperties\":false,\"$defs\":{\"shape_group\":{\"type\":\"object\",\"properties\":{\"shapes\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/union_shape\"},\"maxItems\":3}},\"required\":[\"shapes\"],\"additionalProperties\":false},\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"

type EventShapesGroupedPayload struct {
	Groups []sce_shape_group.ShapeGroupPayload
	At uint8
	Inner uint8
}
