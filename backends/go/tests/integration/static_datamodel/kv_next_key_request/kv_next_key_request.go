// SCE-GENERATED — DO NOT EDIT
// source-hash: 381fc3ff26f6dafd9233aeb3119d4fa3c2748595bf17f58ff51bc0b2a1358fff
// source: sce:std/host/kv_next_key_request.scxml
// SCE-MAP: kv_next_key_request.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.next_key.request
// Do not edit — regenerate from the source SCXML file.

package kv_next_key_request

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.kv.next_key.request"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// AfterMaxSize is the most UTF-8 bytes `after` holds (the bound its type writes), as this build resolved it.
const AfterMaxSize uint32 = 256

// PrefixMaxSize is the most UTF-8 bytes `prefix` holds (the bound its type writes), as this build resolved it.
const PrefixMaxSize uint32 = 256

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 3096

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:kv_next_key_request:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_next_key_request:1\",\"x-sce-source-hash\":\"381fc3ff26f6dafd9233aeb3119d4fa3c2748595bf17f58ff51bc0b2a1358fff\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"after\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"prefix\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"after\",\"prefix\"],\"additionalProperties\":false}"

type KvNextKeyRequestPayload struct {
	After string
	Prefix string
}
