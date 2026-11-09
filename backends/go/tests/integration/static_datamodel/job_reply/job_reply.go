// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: job_reply.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.reply
// Do not edit — regenerate from the source SCXML file.

package job_reply

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "job.reply"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 36

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:job_reply:1"

// JSONSchema is the payload as a JSON Schema (2020-12): one spelling of each value, every
// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:job_reply:1\",\"x-sce-source-hash\":\"16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"job\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"code\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"job\",\"code\"],\"additionalProperties\":false}"

type JobReplyPayload struct {
	Job uint32
	Code uint32
}
