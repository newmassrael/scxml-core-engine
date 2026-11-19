// SCE-GENERATED — DO NOT EDIT
// source-hash: bfa0e09589276663490e5f6c9dcd4cdcc0c060ce6dacbd739fa4a7f565615775
// source: schema_echo.scxml
// SCE-MAP: schema_echo.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: echo
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_echo

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "echo"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `tag` holds (the bound its type writes), as this build resolved it. */
const val TAG_MAX_SIZE: UInt = 32u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 232uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_echo:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_echo:1\",\"x-sce-source-hash\":\"bfa0e09589276663490e5f6c9dcd4cdcc0c060ce6dacbd739fa4a7f565615775\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"total\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"ok\":{\"type\":\"boolean\"},\"tag\":{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32}},\"required\":[\"total\",\"ok\",\"tag\"],\"additionalProperties\":false}"

data class SchemaEchoPayload(    val total: UInt,    val ok: Boolean,    val tag: String)
