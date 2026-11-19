// SCE-GENERATED — DO NOT EDIT
// source-hash: 45260521b7a1b8de46f9c0511b40e9fa6c3287c2c89986939b7f90cc4a9dd513
// source: sce:std/host/kv_get_result.scxml
// SCE-MAP: kv_get_result.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.get.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_get_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.get.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it. */
const val VALUE_MAX_SIZE: UInt = 4096u

/** The most UTF-8 bytes `version` holds (the bound its type writes), as this build resolved it. */
const val VERSION_MAX_SIZE: UInt = 20u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 24890uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_get_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_get_result:1\",\"x-sce-source-hash\":\"45260521b7a1b8de46f9c0511b40e9fa6c3287c2c89986939b7f90cc4a9dd513\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"found\":{\"type\":\"boolean\"},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"error\",\"found\",\"value\",\"version\"],\"additionalProperties\":false}"

data class KvGetResultPayload(    val error: String,    val found: Boolean,    val value: String,    val version: String)
