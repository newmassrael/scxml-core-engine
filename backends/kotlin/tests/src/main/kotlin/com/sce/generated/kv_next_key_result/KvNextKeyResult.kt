// SCE-GENERATED — DO NOT EDIT
// source-hash: 68905caaea48e3f3d7b9a55cc8652aeb24ac1f483a666c26e1409a29153d9ed0
// source: sce:std/host/kv_next_key_result.scxml
// SCE-MAP: kv_next_key_result.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.next_key.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_next_key_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.next_key.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it. */
const val KEY_MAX_SIZE: UInt = 256u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 1715uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_next_key_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_next_key_result:1\",\"x-sce-source-hash\":\"68905caaea48e3f3d7b9a55cc8652aeb24ac1f483a666c26e1409a29153d9ed0\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"found\":{\"type\":\"boolean\"},\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"error\",\"found\",\"key\"],\"additionalProperties\":false}"

data class KvNextKeyResultPayload(    val error: String,    val found: Boolean,    val key: String)
