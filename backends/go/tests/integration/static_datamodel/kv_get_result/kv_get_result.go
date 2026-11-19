// SCE-GENERATED — DO NOT EDIT
// source-hash: 45260521b7a1b8de46f9c0511b40e9fa6c3287c2c89986939b7f90cc4a9dd513
// source: sce:std/host/kv_get_result.scxml
// SCE-MAP: kv_get_result.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.get.result
// Do not edit — regenerate from the source SCXML file.

package kv_get_result

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.kv.get.result"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// ErrorMaxSize is the most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
const ErrorMaxSize uint32 = 24

// ValueMaxSize is the most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it.
const ValueMaxSize uint32 = 4096

// VersionMaxSize is the most UTF-8 bytes `version` holds (the bound its type writes), as this build resolved it.
const VersionMaxSize uint32 = 20

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 24890

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:kv_get_result:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_get_result:1\",\"x-sce-source-hash\":\"45260521b7a1b8de46f9c0511b40e9fa6c3287c2c89986939b7f90cc4a9dd513\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"found\":{\"type\":\"boolean\"},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"error\",\"found\",\"value\",\"version\"],\"additionalProperties\":false}"

type KvGetResultPayload struct {
	Error string
	Found bool
	Value string
	Version string
}
