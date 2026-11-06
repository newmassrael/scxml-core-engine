// SCE-GENERATED — DO NOT EDIT
// source-hash: ad4bedcb756956ce54bdaf3d75bb21306a16abe1845fd50a6c67924ec38392a8
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
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_item_pick:1\",\"x-sce-source-hash\":\"ad4bedcb756956ce54bdaf3d75bb21306a16abe1845fd50a6c67924ec38392a8\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"id\",\"title\",\"count\"],\"additionalProperties\":false}"

type SchemaItemPickPayload struct {
	Id uint32
	Title string
	Count int32
}
