// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c7eecbd6ea7a5af5ad1b7b15b76745f93d801a41a7208304e2df177edc22db
// source: sce:std/host/kv_delete_result.scxml
// SCE-MAP: kv_delete_result.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.delete.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_delete_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.delete.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 172uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_delete_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_delete_result:1\",\"x-sce-source-hash\":\"d8c7eecbd6ea7a5af5ad1b7b15b76745f93d801a41a7208304e2df177edc22db\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"existed\":{\"type\":\"boolean\"}},\"required\":[\"error\",\"existed\"],\"additionalProperties\":false}"

data class KvDeleteResultPayload(    val error: String,    val existed: Boolean)
