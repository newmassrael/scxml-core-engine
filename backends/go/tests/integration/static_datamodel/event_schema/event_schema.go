// SCE-GENERATED — DO NOT EDIT
// source-hash: 2884e8ac02ea86db2d9e5284c5b2f9b95a656fcff43225b5228d1c1d9875a288
// source: event-schema.scxml
// SCE-MAP: event-schema.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.completed
// Do not edit — regenerate from the source SCXML file.

package event_schema

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "job.completed"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 25

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:event-schema:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event-schema:1\",\"x-sce-source-hash\":\"2884e8ac02ea86db2d9e5284c5b2f9b95a656fcff43225b5228d1c1d9875a288\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"elapsed_ms\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"elapsed_ms\"],\"additionalProperties\":false}"

type EventSchemaPayload struct {
	ElapsedMs uint32
}
