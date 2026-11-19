// SCE-GENERATED — DO NOT EDIT
// source-hash: daa30cdaecc96858ccbf51e627e0dbe16e82f6a1328313c53c6d8e77ddfd9e17
// source: sce:std/host/clock_zone_result.scxml
// SCE-MAP: clock_zone_result.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.clock.zone.result
// Do not edit — regenerate from the source SCXML file.

package clock_zone_result

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "host.clock.zone.result"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// ErrorMaxSize is the most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
const ErrorMaxSize uint32 = 24

// ZoneIdMaxSize is the most UTF-8 bytes `zone_id` holds (the bound its type writes), as this build resolved it.
const ZoneIdMaxSize uint32 = 64

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 580

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:clock_zone_result:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:clock_zone_result:1\",\"x-sce-source-hash\":\"daa30cdaecc96858ccbf51e627e0dbe16e82f6a1328313c53c6d8e77ddfd9e17\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"zone_id\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"utc_offset_s\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"error\",\"zone_id\",\"utc_offset_s\"],\"additionalProperties\":false}"

type ClockZoneResultPayload struct {
	Error string
	ZoneId string
	UtcOffsetS int32
}
