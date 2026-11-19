// SCE-GENERATED — DO NOT EDIT
// source-hash: 761fc7256099c224b2d4bfd13b2b1de7210c548c4913423b982331e6a653ca35
// source: schema_view.scxml
// SCE-MAP: schema_view.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: view.shown
// Do not edit — regenerate from the source SCXML file.

package schema_view
import (
	sce_enum_view_mode "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_view_mode"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "view.shown"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 35

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_view:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_view:1\",\"x-sce-source-hash\":\"761fc7256099c224b2d4bfd13b2b1de7210c548c4913423b982331e6a653ca35\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"zoom\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"layout\",\"zoom\"],\"additionalProperties\":false}"

type SchemaViewPayload struct {
	Layout sce_enum_view_mode.EnumViewMode
	Zoom uint8
}
