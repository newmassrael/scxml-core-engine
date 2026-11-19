// SCE-GENERATED — DO NOT EDIT
// source-hash: e00a564262ee4e7e7cb57d1d71658c3b0317c877d1fb1f99afa10415c0cec7d9
// source: sce:std/host/kv_put_request.scxml
// SCE-MAP: kv_put_request.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put.request
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_put_request

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.put.request"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it. */
const val KEY_MAX_SIZE: UInt = 256u

/** The most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it. */
const val VALUE_MAX_SIZE: UInt = 4096u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 26133uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_put_request:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_put_request:1\",\"x-sce-source-hash\":\"e00a564262ee4e7e7cb57d1d71658c3b0317c877d1fb1f99afa10415c0cec7d9\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096}},\"required\":[\"key\",\"value\"],\"additionalProperties\":false}"

data class KvPutRequestPayload(    val key: String,    val value: String)
