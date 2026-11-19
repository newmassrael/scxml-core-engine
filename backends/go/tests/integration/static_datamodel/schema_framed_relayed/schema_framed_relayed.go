// SCE-GENERATED — DO NOT EDIT
// source-hash: e850f1f0717e18b411848c6f75fd321152d6d79eee7e9881271965cbaf8c7287
// source: schema_framed_relayed.scxml
// SCE-MAP: schema_framed_relayed.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: framed.relayed
// Do not edit — regenerate from the source SCXML file.

package schema_framed_relayed

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "framed.relayed"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// FrameMaxSize is the most bytes `frame` holds (the bound its type writes), as this build resolved it.
const FrameMaxSize uint32 = 8

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 73

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:schema_framed_relayed:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_framed_relayed:1\",\"x-sce-source-hash\":\"e850f1f0717e18b411848c6f75fd321152d6d79eee7e9881271965cbaf8c7287\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sensor\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"frame\":{\"type\":\"string\",\"maxLength\":8,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"sensor\",\"frame\"],\"additionalProperties\":false}"

type SchemaFramedRelayedPayload struct {
	Sensor uint8
	Frame []byte
}
