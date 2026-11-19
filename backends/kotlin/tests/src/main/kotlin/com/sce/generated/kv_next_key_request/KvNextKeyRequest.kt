// SCE-GENERATED — DO NOT EDIT
// source-hash: 381fc3ff26f6dafd9233aeb3119d4fa3c2748595bf17f58ff51bc0b2a1358fff
// source: sce:std/host/kv_next_key_request.scxml
// SCE-MAP: kv_next_key_request.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.next_key.request
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.kv_next_key_request

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.kv.next_key.request"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `after` holds (the bound its type writes), as this build resolved it. */
const val AFTER_MAX_SIZE: UInt = 256u

/** The most UTF-8 bytes `prefix` holds (the bound its type writes), as this build resolved it. */
const val PREFIX_MAX_SIZE: UInt = 256u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 3096uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:kv_next_key_request:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:kv_next_key_request:1\",\"x-sce-source-hash\":\"381fc3ff26f6dafd9233aeb3119d4fa3c2748595bf17f58ff51bc0b2a1358fff\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"after\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"prefix\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"after\",\"prefix\"],\"additionalProperties\":false}"

data class KvNextKeyRequestPayload(    val after: String,    val prefix: String)
