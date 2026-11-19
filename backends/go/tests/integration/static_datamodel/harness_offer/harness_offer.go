// SCE-GENERATED — DO NOT EDIT
// source-hash: c8de46d791fd409d79a5a83c269792b484ddd4f3d8111bc3839a0c84cbdc4557
// source: harness_offer.scxml
// SCE-MAP: harness_offer.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: harness.offer
// Do not edit — regenerate from the source SCXML file.

package harness_offer

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "harness.offer"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 17

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:harness_offer:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:harness_offer:1\",\"x-sce-source-hash\":\"c8de46d791fd409d79a5a83c269792b484ddd4f3d8111bc3839a0c84cbdc4557\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"id\"],\"additionalProperties\":false}"

type HarnessOfferPayload struct {
	Id uint32
}
