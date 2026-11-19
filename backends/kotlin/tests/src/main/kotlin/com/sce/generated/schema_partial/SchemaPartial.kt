// SCE-GENERATED — DO NOT EDIT
// source-hash: a9892366fdd147be953ddf1a986120de05d135e5d4598fc7b7fb886fa0b8ae66
// source: schema_partial.scxml
// SCE-MAP: schema_partial.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: partial
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_partial

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "partial"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 31uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_partial:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_partial:1\",\"x-sce-source-hash\":\"a9892366fdd147be953ddf1a986120de05d135e5d4598fc7b7fb886fa0b8ae66\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"total\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"part\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"total\",\"part\"],\"additionalProperties\":false}"

data class SchemaPartialPayload(    val total: UInt,    val part: UByte)
