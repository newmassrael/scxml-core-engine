// SCE-GENERATED — DO NOT EDIT
// source-hash: cda25fd9852678e0727d69d229372dda1d349f5db443a3d2b3036d84089ac674
// source: sce:std/host/kv_get_request.scxml
// SCE-MAP: kv_get_request.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.get.request
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_get_request

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.get.request"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it. */
const val KEY_MAX_SIZE: UInt = 256u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 1546uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_get_request:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_get_request:1\",\"x-sce-source-hash\":\"cda25fd9852678e0727d69d229372dda1d349f5db443a3d2b3036d84089ac674\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"key\"],\"additionalProperties\":false}"

data class KvGetRequestPayload(    val key: String)
