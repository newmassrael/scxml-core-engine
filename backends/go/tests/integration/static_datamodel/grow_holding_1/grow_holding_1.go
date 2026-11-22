// SCE-GENERATED — DO NOT EDIT
// source-hash: cbae23ec3e271a5d3d031cf17aaa399ae7d046b79eff29ef521b985e000db27e
// source: grow_holding_1.scxml
// SCE-MAP: grow_holding_1.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.holding
// Do not edit — regenerate from the source SCXML file.

package grow_holding_1
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_grow_outer_1 "github.com/newmassrael/sce-go-tests/integration/static_datamodel/grow_outer_1"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "grow.holding"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 82

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:grow_holding_1:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_holding_1:1\",\"x-sce-source-hash\":\"cbae23ec3e271a5d3d031cf17aaa399ae7d046b79eff29ef521b985e000db27e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"outer\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_outer_1\"},{\"type\":\"null\"}]}},\"required\":[\"outer\"],\"additionalProperties\":false,\"$defs\":{\"grow_inner_1\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false},\"grow_outer_1\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"inner\":{\"$ref\":\"#/$defs/grow_inner_1\"},\"inners\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/grow_inner_1\"},\"maxItems\":2}},\"required\":[\"id\",\"inner\",\"inners\"],\"additionalProperties\":false}}}"

type GrowHolding1Payload struct {
	Outer optional.Of[sce_grow_outer_1.GrowOuter1Payload]
}
