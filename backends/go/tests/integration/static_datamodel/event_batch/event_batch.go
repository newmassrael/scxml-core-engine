// SCE-GENERATED — DO NOT EDIT
// source-hash: 6bb2728794ee589a7db108584f8b5faa85f79922b77ffb9e5983644d2090619e
// source: event_batch.scxml
// SCE-MAP: event_batch.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: batch.sent
// Do not edit — regenerate from the source SCXML file.

package event_batch
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_enum_signal_level "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_signal_level"
	sce_event_batch_item "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_batch_item"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "batch.sent"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 2

// ItemsMaxCount is the most elements `items` holds (the bound its type writes, named `batch_items`), as this build resolved it.
const ItemsMaxCount uint32 = 3

// GridMaxCount is the most elements `grid` holds (the bound its type writes), as this build resolved it.
const GridMaxCount uint32 = 2

// LevelsMaxCount is the most elements `levels` holds (the bound its type writes), as this build resolved it.
const LevelsMaxCount uint32 = 2

// MaybeMaxCount is the most elements `maybe` holds (the bound its type writes), as this build resolved it.
const MaybeMaxCount uint32 = 2

// SpareMaxCount is the most elements `spare` holds (the bound its type writes), as this build resolved it.
const SpareMaxCount uint32 = 2

// NoteMaxSize is the most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
const NoteMaxSize uint32 = 4

// LateMaxCount is the most elements `late` holds (the bound its type writes), as this build resolved it.
const LateMaxCount uint32 = 2

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 1376

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:event_batch:2"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_batch:2\",\"x-sce-source-hash\":\"6bb2728794ee589a7db108584f8b5faa85f79922b77ffb9e5983644d2090619e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"items\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/event_batch_item\"},\"maxItems\":3},\"grid\":{\"type\":\"array\",\"items\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"maxItems\":2},\"levels\":{\"type\":\"array\",\"items\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"maxItems\":2},\"maybe\":{\"type\":\"array\",\"items\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},{\"type\":\"null\"}]},\"maxItems\":2},\"spare\":{\"anyOf\":[{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},{\"type\":\"null\"}]},\"note\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"late\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2}},\"required\":[\"items\",\"grid\",\"levels\",\"maybe\",\"spare\",\"note\",\"late\"],\"additionalProperties\":false,\"$defs\":{\"event_batch_item\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"label\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"level\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"due\":{\"anyOf\":[{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},{\"type\":\"null\"}]},\"blob\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"maxItems\":4}},\"required\":[\"id\",\"amount\",\"label\",\"level\",\"due\",\"blob\",\"tags\"],\"additionalProperties\":false}}}"

type EventBatchPayload struct {
	Items []sce_event_batch_item.EventBatchItemPayload
	Grid [][]uint8
	Levels []sce_enum_signal_level.EnumSignalLevel
	Maybe []optional.Of[uint16]
	Spare optional.Of[[]uint8]
	Note string
	Late []uint8
}
