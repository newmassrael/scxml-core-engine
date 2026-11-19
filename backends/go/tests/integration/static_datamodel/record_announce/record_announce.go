// SCE-GENERATED — DO NOT EDIT
// source-hash: 307690ff649e3407d37ebe0c4fe9921df944c34ecb90ad8ff73bc6b8454d4876
// source: sce:std/shared/record_announce.scxml
// SCE-MAP: record_announce.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.announce
// Do not edit — regenerate from the source SCXML file.

package record_announce

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "record_list.announce"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 31

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:record_announce:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:record_announce:1\",\"x-sce-source-hash\":\"307690ff649e3407d37ebe0c4fe9921df944c34ecb90ad8ff73bc6b8454d4876\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"field\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"id\",\"field\"],\"additionalProperties\":false}"

type RecordAnnouncePayload struct {
	Id uint32
	Field uint16
}
