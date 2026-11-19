// SCE-GENERATED — DO NOT EDIT
// source-hash: 14720d95df52630d59d66d9fb5b0f1721a9da83639ec5118a6260c6a8d149629
// source: sce:std/shared/record_room.scxml
// SCE-MAP: record_room.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.room
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.record_room

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "record_list.room"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 62uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:record_room:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:record_room:1\",\"x-sce-source-hash\":\"14720d95df52630d59d66d9fb5b0f1721a9da83639ec5118a6260c6a8d149629\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"values\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"history\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"digest\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"values\",\"history\",\"digest\"],\"additionalProperties\":false}"

data class RecordRoomPayload(    val values: UInt,    val history: UInt,    val digest: UInt)
