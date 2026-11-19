// SCE-GENERATED — DO NOT EDIT
// source-hash: 73a21c828eb9e901913c93946a92eaa7bdb5ccca22c8d0bfc8fbd9f572185643
// source: sce:std/host/notify_cancel_request.scxml
// SCE-MAP: notify_cancel_request.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.notify.cancel.request
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.notify_cancel_request

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.notify.cancel.request"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `id` holds (the bound its type writes), as this build resolved it. */
const val ID_MAX_SIZE: UInt = 64u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 393uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:notify_cancel_request:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:notify_cancel_request:1\",\"x-sce-source-hash\":\"73a21c828eb9e901913c93946a92eaa7bdb5ccca22c8d0bfc8fbd9f572185643\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64}},\"required\":[\"id\"],\"additionalProperties\":false}"

data class NotifyCancelRequestPayload(    val id: String)
