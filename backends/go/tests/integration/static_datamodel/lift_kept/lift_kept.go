// SCE-GENERATED — DO NOT EDIT
// source-hash: 65dc738f80ddcf851ae8848bee40a3244882a93c4459336fd50ee57f02af2bf3
// source: lift_kept.scxml
// SCE-MAP: lift_kept.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.kept
// Do not edit — regenerate from the source SCXML file.

package lift_kept

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "text.kept"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TextMaxSize is the most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
const TextMaxSize uint32 = 512

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 3083

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:lift_kept:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:lift_kept:1\",\"x-sce-source-hash\":\"65dc738f80ddcf851ae8848bee40a3244882a93c4459336fd50ee57f02af2bf3\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":512,\"x-sce-max-bytes\":512}},\"required\":[\"text\"],\"additionalProperties\":false}"

type LiftKeptPayload struct {
	Text string
}
