// SCE-GENERATED — DO NOT EDIT
// source-hash: 33737f0585c52cefa06871ed2f44569ce1a532f2ac26439d7ba7fdf961ee33b8
// source: grow_item_1.scxml
// SCE-MAP: grow_item_1.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.item
// Do not edit — regenerate from the source SCXML file.

package grow_item_1

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "grow.item"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 11

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:grow_item_1:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_item_1:1\",\"x-sce-source-hash\":\"33737f0585c52cefa06871ed2f44569ce1a532f2ac26439d7ba7fdf961ee33b8\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false}"

type GrowItem1Payload struct {
	N uint16
}
