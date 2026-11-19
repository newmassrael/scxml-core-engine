// SCE-GENERATED — DO NOT EDIT
// source-hash: 959cbb52145e825ba3ae7d873268e4eb17d0d63fe01a8c94fd1077b456638947
// source: lift_answer_list.scxml
// SCE-MAP: lift_answer_list.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: lift.list
// Do not edit — regenerate from the source SCXML file.

package lift_answer_list
import (
	sce_lift_item "github.com/newmassrael/sce-go-tests/integration/static_datamodel/lift_item"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "lift.list"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 2

// TitleMaxSize is the most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
const TitleMaxSize uint32 = 16

// ItemsMaxCount is the most elements `items` holds (the bound its type writes), as this build resolved it.
const ItemsMaxCount uint32 = 3

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 487

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:lift_answer_list:2"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:lift_answer_list:2\",\"x-sce-source-hash\":\"959cbb52145e825ba3ae7d873268e4eb17d0d63fe01a8c94fd1077b456638947\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"items\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/lift_item\"},\"minItems\":1,\"maxItems\":3},\"priority\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"title\",\"items\",\"priority\"],\"additionalProperties\":false,\"$defs\":{\"lift_item\":{\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"at\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"title\",\"at\"],\"additionalProperties\":false}}}"

type LiftAnswerListPayload struct {
	Title string
	Items []sce_lift_item.LiftItemPayload
	Priority uint8
}
