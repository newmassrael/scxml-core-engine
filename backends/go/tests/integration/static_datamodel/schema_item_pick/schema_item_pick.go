// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

package schema_item_pick

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "item.picked"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TitleMaxSize is the most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
const TitleMaxSize uint32 = 16

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 144

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_item_pick:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$id\":\"urn:sce:schema:schema_item_pick:1\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"count\":{\"maximum\":2147483647,\"minimum\":-2147483648,\"type\":\"integer\"},\"id\":{\"maximum\":4294967295,\"minimum\":0,\"type\":\"integer\"},\"title\":{\"maxLength\":16,\"type\":\"string\",\"x-sce-max-bytes\":16}},\"required\":[\"count\",\"id\",\"title\"],\"type\":\"object\",\"x-sce-source-hash\":\"c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c\"}"

type SchemaItemPickPayload struct {
	Id uint32
	Title string
	Count int32
}
