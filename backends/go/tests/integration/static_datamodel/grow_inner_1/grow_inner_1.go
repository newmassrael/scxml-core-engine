// SCE-GENERATED — DO NOT EDIT
// source-hash: d952dece001e225dcdd9261acbf66c206ec1d1bb2565a04751e4d18dfb845f7f
// source: grow_inner_1.scxml
// SCE-MAP: grow_inner_1.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.inner
// Do not edit — regenerate from the source SCXML file.

package grow_inner_1

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "grow.inner"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 11

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:grow_inner_1:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_inner_1:1\",\"x-sce-source-hash\":\"d952dece001e225dcdd9261acbf66c206ec1d1bb2565a04751e4d18dfb845f7f\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false}"

type GrowInner1Payload struct {
	N uint16
}
