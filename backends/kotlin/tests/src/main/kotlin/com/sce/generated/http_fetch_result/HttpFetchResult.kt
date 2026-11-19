// SCE-GENERATED — DO NOT EDIT
// source-hash: c4c72e13c93868d9f1549ee1e441bafbfeb18ef7b10118b142f357c2c3ace0b0
// source: sce:std/host/http_fetch_result.scxml
// SCE-MAP: http_fetch_result.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.http.fetch.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.http_fetch_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.http.fetch.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most UTF-8 bytes `headers` holds (the bound its type writes), as this build resolved it. */
const val HEADERS_MAX_SIZE: UInt = 4096u

/** The most UTF-8 bytes `body` holds (the bound its type writes), as this build resolved it. */
const val BODY_MAX_SIZE: UInt = 16384u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 123074uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:http_fetch_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:http_fetch_result:1\",\"x-sce-source-hash\":\"c4c72e13c93868d9f1549ee1e441bafbfeb18ef7b10118b142f357c2c3ace0b0\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"status\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"headers\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"body\":{\"type\":\"string\",\"maxLength\":16384,\"x-sce-max-bytes\":16384}},\"required\":[\"error\",\"status\",\"headers\",\"body\"],\"additionalProperties\":false}"

data class HttpFetchResultPayload(    val error: String,    val status: UShort,    val headers: String,    val body: String)
