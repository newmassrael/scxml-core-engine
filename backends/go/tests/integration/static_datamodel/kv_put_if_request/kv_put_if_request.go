// SCE-GENERATED — DO NOT EDIT
// source-hash: 792468e33f930b682d12af1a48545cd870803e0bcabb117eb4a2299af5ef6f2b
// source: sce:std/host/kv_put_if_request.scxml
// SCE-MAP: kv_put_if_request.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put_if.request
// Do not edit — regenerate from the source SCXML file.

package kv_put_if_request

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.kv.put_if.request"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// KeyMaxSize is the most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it.
const KeyMaxSize uint32 = 256

// ValueMaxSize is the most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it.
const ValueMaxSize uint32 = 4096

// ExpectedVersionMaxSize is the most UTF-8 bytes `expected_version` holds (the bound its type writes), as this build resolved it.
const ExpectedVersionMaxSize uint32 = 20

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 26275

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:kv_put_if_request:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_put_if_request:1\",\"x-sce-source-hash\":\"792468e33f930b682d12af1a48545cd870803e0bcabb117eb4a2299af5ef6f2b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"expected_version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"key\",\"value\",\"expected_version\"],\"additionalProperties\":false}"

type KvPutIfRequestPayload struct {
	Key string
	Value string
	ExpectedVersion string
}
