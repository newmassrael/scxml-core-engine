// SCE-GENERATED — DO NOT EDIT
// source-hash: b23e463ea4883c36ecd2c9bbd8bc0bec3a32b976397343fc78b295b13d7426c3
// source: schema_due_sent.scxml
// SCE-MAP: schema_due_sent.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: due.sent
// Do not edit — regenerate from the source SCXML file.

package schema_due_sent
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "due.sent"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TitleMaxSize is the most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
const TitleMaxSize uint32 = 16

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 160

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_due_sent:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_due_sent:1\",\"x-sce-source-hash\":\"b23e463ea4883c36ecd2c9bbd8bc0bec3a32b976397343fc78b295b13d7426c3\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"due\":{\"anyOf\":[{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},{\"type\":\"null\"}]},\"title\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},{\"type\":\"null\"}]},\"status\":{\"anyOf\":[{\"enum\":[\"open\",\"in_progress\",\"done\"]},{\"type\":\"null\"}]}},\"required\":[\"due\",\"title\",\"status\"],\"additionalProperties\":false}"

type SchemaDueSentPayload struct {
	Due optional.Of[int64]
	Title optional.Of[string]
	Status optional.Of[sce_enum_item_status.EnumItemStatus]
}
