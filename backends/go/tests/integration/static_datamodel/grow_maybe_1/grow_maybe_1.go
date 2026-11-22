// SCE-GENERATED — DO NOT EDIT
// source-hash: 75ff8a217f4bbd86f0ed3149c6c9f6f26f6413a55e448662e67c1f3f0faa3182
// source: grow_maybe_1.scxml
// SCE-MAP: grow_maybe_1.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.maybe
// Do not edit — regenerate from the source SCXML file.

package grow_maybe_1
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_grow_inner_1 "github.com/newmassrael/sce-go-tests/integration/static_datamodel/grow_inner_1"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "grow.maybe"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 21

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:grow_maybe_1:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_maybe_1:1\",\"x-sce-source-hash\":\"75ff8a217f4bbd86f0ed3149c6c9f6f26f6413a55e448662e67c1f3f0faa3182\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"inner\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_inner_1\"},{\"type\":\"null\"}]}},\"required\":[\"inner\"],\"additionalProperties\":false,\"$defs\":{\"grow_inner_1\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false}}}"

type GrowMaybe1Payload struct {
	Inner optional.Of[sce_grow_inner_1.GrowInner1Payload]
}
