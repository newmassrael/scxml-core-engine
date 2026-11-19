// SCE-GENERATED — DO NOT EDIT
// source-hash: fcf5a56a410047d914b307c27348a0a3feaaeaf95ad56e63739be85c51031a6c
// source: sce:std/host/notify_permission_result.scxml
// SCE-MAP: notify_permission_result.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.notify.permission.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.notify_permission_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.notify.permission.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most UTF-8 bytes `state` holds (the bound its type writes), as this build resolved it. */
const val STATE_MAX_SIZE: UInt = 16u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 263uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:notify_permission_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:notify_permission_result:1\",\"x-sce-source-hash\":\"fcf5a56a410047d914b307c27348a0a3feaaeaf95ad56e63739be85c51031a6c\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"state\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16}},\"required\":[\"error\",\"state\"],\"additionalProperties\":false}"

data class NotifyPermissionResultPayload(    val error: String,    val state: String)
