// SCE-GENERATED — DO NOT EDIT
// source-hash: 44e42be193b6ad6075c6d0b043abdcd82b148ab78d0b09d43794619a04e8d622
// SCE-MAP: job_status.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.status
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.job_status

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "job.status"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it. */
const val NOTE_MAX_SIZE: UInt = 32u

/** The most elements `codes` holds (the bound its type writes), as this build resolved it. */
const val CODES_MAX_COUNT: UInt = 4u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 296uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:job_status:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:job_status:1\",\"x-sce-source-hash\":\"44e42be193b6ad6075c6d0b043abdcd82b148ab78d0b09d43794619a04e8d622\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"elapsed_ms\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32},{\"type\":\"null\"}]},\"codes\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":4},\"tag\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"retry\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"elapsed_ms\",\"note\",\"codes\",\"tag\",\"retry\",\"count\"],\"additionalProperties\":false}"

data class JobStatusPayload(    val elapsed_ms: UInt,    val note: String?,    val codes: List<UByte>,    val tag: UByte?,    val retry: UByte?,    val count: UInt)
