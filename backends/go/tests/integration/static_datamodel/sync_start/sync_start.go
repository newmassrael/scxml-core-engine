// SCE-GENERATED — DO NOT EDIT
// source-hash: 3088e9cab925ffee9d43a9e4845aee06c06e06454f13eca167167a445cb4f899
// source: sce:std/sync/sync_start.scxml
// SCE-MAP: sync_start.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.start
// Do not edit — regenerate from the source SCXML file.

package sync_start

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "sync.start"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 17

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:sync_start:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:sync_start:1\",\"x-sce-source-hash\":\"3088e9cab925ffee9d43a9e4845aee06c06e06454f13eca167167a445cb4f899\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"byToken\":{\"type\":\"boolean\"}},\"required\":[\"byToken\"],\"additionalProperties\":false}"

type SyncStartPayload struct {
	ByToken bool
}
