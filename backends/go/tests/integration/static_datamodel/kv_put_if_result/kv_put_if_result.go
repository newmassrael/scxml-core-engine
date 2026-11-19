// SCE-GENERATED — DO NOT EDIT
// source-hash: 01c29ab3058ee393d25a0aed0511ef7120730d5121386ee20787b72fa27766ff
// source: sce:std/host/kv_put_if_result.scxml
// SCE-MAP: kv_put_if_result.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put_if.result
// Do not edit — regenerate from the source SCXML file.

package kv_put_if_result

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.kv.put_if.result"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// ErrorMaxSize is the most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
const ErrorMaxSize uint32 = 24

// VersionMaxSize is the most UTF-8 bytes `version` holds (the bound its type writes), as this build resolved it.
const VersionMaxSize uint32 = 20

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 305

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:kv_put_if_result:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_put_if_result:1\",\"x-sce-source-hash\":\"01c29ab3058ee393d25a0aed0511ef7120730d5121386ee20787b72fa27766ff\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"written\":{\"type\":\"boolean\"},\"version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"error\",\"written\",\"version\"],\"additionalProperties\":false}"

type KvPutIfResultPayload struct {
	Error string
	Written bool
	Version string
}
