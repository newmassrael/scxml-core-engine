// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
// SCE-MAP: text_slot.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.slot
// Do not edit — regenerate from the source SCXML file.

package text_slot

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "text.slot"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TextMaxSize is the most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
const TextMaxSize uint32 = 32

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 254

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:text_slot:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:text_slot:1\",\"x-sce-source-hash\":\"848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"slot\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"version\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"text\":{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32}},\"required\":[\"slot\",\"version\",\"text\"],\"additionalProperties\":false}"

type TextSlotPayload struct {
	Slot uint32
	Version int64
	Text string
}
