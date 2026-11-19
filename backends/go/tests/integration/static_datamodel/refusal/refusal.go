// SCE-GENERATED — DO NOT EDIT
// source-hash: 3862752ac9ccab5eefa2c5b6f8d90f0288b61c11fb1e80be71091ff4f67fcf6d
// source: sce:std/lift/refusal.scxml
// SCE-MAP: refusal.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: lift.refusal
// Do not edit — regenerate from the source SCXML file.

package refusal
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_refusal_code "github.com/newmassrael/sce-go-tests/integration/static_datamodel/refusal_code"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "lift.refusal"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// PathMaxSize is the most UTF-8 bytes `path` holds (the bound its type writes, named `lift_path`), as this build resolved it.
const PathMaxSize uint32 = 256

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 1591

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:refusal:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:refusal:1\",\"x-sce-source-hash\":\"3862752ac9ccab5eefa2c5b6f8d90f0288b61c11fb1e80be71091ff4f67fcf6d\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"code\":{\"enum\":[\"notJson\",\"notObject\",\"tooDeep\",\"missing\",\"nullNotOptional\",\"wrongKind\",\"badNumber\",\"outOfRange\",\"tooLong\",\"tooMany\",\"unknownVariant\",\"notByteText\",\"nulInText\",\"tooFew\"]},\"path\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"bound\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},{\"type\":\"null\"}]}},\"required\":[\"code\",\"path\",\"bound\"],\"additionalProperties\":false}"

type RefusalPayload struct {
	Code sce_refusal_code.RefusalCode
	Path string
	Bound optional.Of[uint32]
}
