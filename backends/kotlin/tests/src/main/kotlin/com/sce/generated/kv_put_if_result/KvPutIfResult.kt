// SCE-GENERATED — DO NOT EDIT
// source-hash: 01c29ab3058ee393d25a0aed0511ef7120730d5121386ee20787b72fa27766ff
// source: sce:std/host/kv_put_if_result.scxml
// SCE-MAP: kv_put_if_result.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put_if.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_put_if_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.put_if.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most UTF-8 bytes `version` holds (the bound its type writes), as this build resolved it. */
const val VERSION_MAX_SIZE: UInt = 20u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 305uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_put_if_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_put_if_result:1\",\"x-sce-source-hash\":\"01c29ab3058ee393d25a0aed0511ef7120730d5121386ee20787b72fa27766ff\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"written\":{\"type\":\"boolean\"},\"version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"error\",\"written\",\"version\"],\"additionalProperties\":false}"

data class KvPutIfResultPayload(    val error: String,    val written: Boolean,    val version: String)
