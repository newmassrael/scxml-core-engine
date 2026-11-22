// SCE-GENERATED — DO NOT EDIT
// source-hash: e2c4afce99b5864384371f59adef9037ad47caf1b6a3a30e2828e7cf29919277
// source: grow_outer_1.scxml
// SCE-MAP: grow_outer_1.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.outer
// Do not edit — regenerate from the source SCXML file.

package grow_outer_1
import (
	sce_grow_inner_1 "github.com/newmassrael/sce-go-tests/integration/static_datamodel/grow_inner_1"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "grow.outer"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// InnersMaxCount is the most elements `inners` holds (the bound its type writes), as this build resolved it.
const InnersMaxCount uint32 = 2

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 72

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:grow_outer_1:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_outer_1:1\",\"x-sce-source-hash\":\"e2c4afce99b5864384371f59adef9037ad47caf1b6a3a30e2828e7cf29919277\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"inner\":{\"$ref\":\"#/$defs/grow_inner_1\"},\"inners\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/grow_inner_1\"},\"maxItems\":2}},\"required\":[\"id\",\"inner\",\"inners\"],\"additionalProperties\":false,\"$defs\":{\"grow_inner_1\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false}}}"

type GrowOuter1Payload struct {
	Id uint32
	Inner sce_grow_inner_1.GrowInner1Payload
	Inners []sce_grow_inner_1.GrowInner1Payload
}
