// SCE-GENERATED — DO NOT EDIT
// source-hash: b782743d856cc23e8d2d12295823ec4b1405a989e8ea6dcaf2e7d8258591bbca
// source: presence_expire.scxml
// SCE-MAP: presence_expire.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: presence.expire
// Do not edit — regenerate from the source SCXML file.

package presence_expire

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "presence.expire"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 18

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:presence_expire:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:presence_expire:1\",\"x-sce-source-hash\":\"b782743d856cc23e8d2d12295823ec4b1405a989e8ea6dcaf2e7d8258591bbca\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"gen\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"gen\"],\"additionalProperties\":false}"

type PresenceExpirePayload struct {
	Gen uint32
}
