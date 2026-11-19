// SCE-GENERATED — DO NOT EDIT
// source-hash: 62baaaee532ce05683ea46a77ba1777a9aef787e934ae52b10c4b70563eff745
// source: lift_answer.scxml
// SCE-MAP: lift_answer.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: lift.answer
// Do not edit — regenerate from the source SCXML file.

package lift_answer
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_enum_lift_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_lift_kind"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "lift.answer"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 2

// TitleMaxSize is the most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
const TitleMaxSize uint32 = 16

// NoteMaxSize is the most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
const NoteMaxSize uint32 = 8

// TagMaxSize is the most bytes `tag` holds (the bound its type writes), as this build resolved it.
const TagMaxSize uint32 = 4

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 279

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:lift_answer:2"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:lift_answer:2\",\"x-sce-source-hash\":\"62baaaee532ce05683ea46a77ba1777a9aef787e934ae52b10c4b70563eff745\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"kind\":{\"enum\":[\"meeting\",\"task\"]},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]},\"tag\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"priority\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"id\",\"amount\",\"title\",\"kind\",\"note\",\"tag\",\"priority\"],\"additionalProperties\":false}"

type LiftAnswerPayload struct {
	Id uint32
	Amount int64
	Title string
	Kind sce_enum_lift_kind.EnumLiftKind
	Note optional.Of[string]
	Tag []byte
	Priority uint8
}
