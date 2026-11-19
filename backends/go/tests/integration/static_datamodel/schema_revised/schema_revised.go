// SCE-GENERATED — DO NOT EDIT
// source-hash: c090e931f092b0370328464cce7a355fb68b64ba23ab2308efbad976050737de
// source: schema_revised.scxml
// SCE-MAP: schema_revised.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading.revised
// Do not edit — regenerate from the source SCXML file.

package schema_revised
import (
	sce_enum_view_mode "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_view_mode"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "reading.revised"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 3

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 51

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_revised:3"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_revised:3\",\"x-sce-source-hash\":\"c090e931f092b0370328464cce7a355fb68b64ba23ab2308efbad976050737de\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"level\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"urgent\":{\"type\":\"boolean\"}},\"required\":[\"level\",\"layout\",\"urgent\"],\"additionalProperties\":false}"

type SchemaRevisedPayload struct {
	Level uint8
	Layout sce_enum_view_mode.EnumViewMode
	Urgent bool
}
