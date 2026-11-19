// SCE-GENERATED — DO NOT EDIT
// source-hash: c699932b2babadfb70f140f172501ec77ff7cfd53faf407b29eb0bf0946a8b2d
// source: lift_item.scxml
// SCE-MAP: lift_item.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: lift.item
// Do not edit — regenerate from the source SCXML file.

package lift_item

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "lift.item"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TitleMaxSize is the most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
const TitleMaxSize uint32 = 16

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 117

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:lift_item:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:lift_item:1\",\"x-sce-source-hash\":\"c699932b2babadfb70f140f172501ec77ff7cfd53faf407b29eb0bf0946a8b2d\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"at\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"title\",\"at\"],\"additionalProperties\":false}"

type LiftItemPayload struct {
	Title string
	At uint8
}
