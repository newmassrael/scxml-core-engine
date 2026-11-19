// SCE-GENERATED — DO NOT EDIT
// source-hash: 792468e33f930b682d12af1a48545cd870803e0bcabb117eb4a2299af5ef6f2b
// source: sce:std/host/kv_put_if_request.scxml
// SCE-MAP: kv_put_if_request.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put_if.request
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_put_if_request

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.put_if.request"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it. */
const val KEY_MAX_SIZE: UInt = 256u

/** The most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it. */
const val VALUE_MAX_SIZE: UInt = 4096u

/** The most UTF-8 bytes `expected_version` holds (the bound its type writes), as this build resolved it. */
const val EXPECTED_VERSION_MAX_SIZE: UInt = 20u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 26275uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_put_if_request:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_put_if_request:1\",\"x-sce-source-hash\":\"792468e33f930b682d12af1a48545cd870803e0bcabb117eb4a2299af5ef6f2b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"expected_version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"key\",\"value\",\"expected_version\"],\"additionalProperties\":false}"

data class KvPutIfRequestPayload(    val key: String,    val value: String,    val expected_version: String)
