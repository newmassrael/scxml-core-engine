// SCE-GENERATED — DO NOT EDIT
// source-hash: cc48aba50cc8e6a27cb6667317f3a37270ccb5a9151b7ad0c867333d2544020e
// source: harness_start.scxml
// SCE-MAP: harness_start.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: harness.start
// Do not edit — regenerate from the source SCXML file.

package harness_start

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "harness.start"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// RequestMaxSize is the most UTF-8 bytes `request` holds (the bound its type writes), as this build resolved it.
const RequestMaxSize uint32 = 512

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 3086

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:harness_start:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:harness_start:1\",\"x-sce-source-hash\":\"cc48aba50cc8e6a27cb6667317f3a37270ccb5a9151b7ad0c867333d2544020e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"request\":{\"type\":\"string\",\"maxLength\":512,\"x-sce-max-bytes\":512}},\"required\":[\"request\"],\"additionalProperties\":false}"

type HarnessStartPayload struct {
	Request string
}
