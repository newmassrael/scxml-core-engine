// SCE-GENERATED — DO NOT EDIT
// source-hash: e201dbad52ebff31de7aee132f3d26002929ddef7fe03d44700baefd0ef100cf
// source: schema_view_grown.scxml
// SCE-MAP: schema_view_grown.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: view.grown
// Do not edit — regenerate from the source SCXML file.

package schema_view_grown
import (
	sce_enum_view_mode "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_view_mode"
	sce_schema_view_mark "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_view_mark"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "view.grown"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 2

// MarksMaxCount is the most elements `marks` holds (the bound its type writes), as this build resolved it.
const MarksMaxCount uint32 = 4

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 234

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_view_grown:2"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_view_grown:2\",\"x-sce-source-hash\":\"e201dbad52ebff31de7aee132f3d26002929ddef7fe03d44700baefd0ef100cf\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"marks\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/schema_view_mark\"},\"maxItems\":4}},\"required\":[\"layout\",\"marks\"],\"additionalProperties\":false,\"$defs\":{\"schema_view_mark\":{\"type\":\"object\",\"properties\":{\"day\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"shift\":{\"anyOf\":[{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},{\"type\":\"null\"}]}},\"required\":[\"day\",\"shift\"],\"additionalProperties\":false}}}"

type SchemaViewGrownPayload struct {
	Layout sce_enum_view_mode.EnumViewMode
	Marks []sce_schema_view_mark.SchemaViewMarkPayload
}
