// SCE-GENERATED — DO NOT EDIT
// source-hash: cea8473f82f3c5ac18d332647ad4acb1c308e9959fa1ac73c1e4621976b014a4
// source: sce:std/host/http_fetch_request.scxml
// SCE-MAP: http_fetch_request.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.http.fetch.request
// Do not edit — regenerate from the source SCXML file.

package http_fetch_request

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.http.fetch.request"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// MethodMaxSize is the most UTF-8 bytes `method` holds (the bound its type writes), as this build resolved it.
const MethodMaxSize uint32 = 8

// UrlMaxSize is the most UTF-8 bytes `url` holds (the bound its type writes), as this build resolved it.
const UrlMaxSize uint32 = 2048

// HeadersMaxSize is the most UTF-8 bytes `headers` holds (the bound its type writes), as this build resolved it.
const HeadersMaxSize uint32 = 2048

// BodyMaxSize is the most UTF-8 bytes `body` holds (the bound its type writes), as this build resolved it.
const BodyMaxSize uint32 = 16384

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 122973

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:http_fetch_request:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:http_fetch_request:1\",\"x-sce-source-hash\":\"cea8473f82f3c5ac18d332647ad4acb1c308e9959fa1ac73c1e4621976b014a4\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"method\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"url\":{\"type\":\"string\",\"maxLength\":2048,\"x-sce-max-bytes\":2048},\"headers\":{\"type\":\"string\",\"maxLength\":2048,\"x-sce-max-bytes\":2048},\"body\":{\"type\":\"string\",\"maxLength\":16384,\"x-sce-max-bytes\":16384}},\"required\":[\"method\",\"url\",\"headers\",\"body\"],\"additionalProperties\":false}"

type HttpFetchRequestPayload struct {
	Method string
	Url string
	Headers string
	Body string
}
