// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e805a5830a04b72b7fab7ee9d2d94516798993677d83eaf9c70d63bb32d7e55
// source: sce:std/host/notify_cancel_result.scxml
// SCE-MAP: notify_cancel_result.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.notify.cancel.result
// Do not edit — regenerate from the source SCXML file.

package notify_cancel_result

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.notify.cancel.result"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// ErrorMaxSize is the most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
const ErrorMaxSize uint32 = 24

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 172

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:notify_cancel_result:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:notify_cancel_result:1\",\"x-sce-source-hash\":\"9e805a5830a04b72b7fab7ee9d2d94516798993677d83eaf9c70d63bb32d7e55\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"existed\":{\"type\":\"boolean\"}},\"required\":[\"error\",\"existed\"],\"additionalProperties\":false}"

type NotifyCancelResultPayload struct {
	Error string
	Existed bool
}
