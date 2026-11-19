// SCE-GENERATED — DO NOT EDIT
// source-hash: cda25fd9852678e0727d69d229372dda1d349f5db443a3d2b3036d84089ac674
// source: sce:std/host/kv_get_request.scxml
// SCE-MAP: kv_get_request.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.get.request
// Do not edit — regenerate from the source SCXML file.

package kv_get_request

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.kv.get.request"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// KeyMaxSize is the most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it.
const KeyMaxSize uint32 = 256

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 1546

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:kv_get_request:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_get_request:1\",\"x-sce-source-hash\":\"cda25fd9852678e0727d69d229372dda1d349f5db443a3d2b3036d84089ac674\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"key\"],\"additionalProperties\":false}"

type KvGetRequestPayload struct {
	Key string
}
