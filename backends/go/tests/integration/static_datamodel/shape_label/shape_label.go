// SCE-GENERATED — DO NOT EDIT
// source-hash: 54edb71a4c4e5de4983131c0d7575af9aef5bffe95b29a03f9d789a8dc8bb5ba
// source: shape_label.scxml
// SCE-MAP: shape_label.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.label
// Do not edit — regenerate from the source SCXML file.

package shape_label
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "shape.label"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TextMaxSize is the most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
const TextMaxSize uint32 = 8

// NoteMaxSize is the most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
const NoteMaxSize uint32 = 4

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 93

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:shape_label:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:shape_label:1\",\"x-sce-source-hash\":\"54edb71a4c4e5de4983131c0d7575af9aef5bffe95b29a03f9d789a8dc8bb5ba\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"text\",\"note\"],\"additionalProperties\":false}"

type ShapeLabelPayload struct {
	Text string
	Note optional.Of[string]
}
