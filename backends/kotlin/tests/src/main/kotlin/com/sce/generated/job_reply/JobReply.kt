// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: job_reply.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.reply
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.job_reply

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "job.reply"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 36uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:job_reply:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:job_reply:1\",\"x-sce-source-hash\":\"021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"job\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"code\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"job\",\"code\"],\"additionalProperties\":false}"

data class JobReplyPayload(    val job: UInt,    val code: UInt)
