// SCE-GENERATED — DO NOT EDIT
// source-hash: 842c7bdea0644aef8efe41e1b76ca837dbf2e5877133fa8fa148199f07b491d8
// source: schema_item_move.scxml
// SCE-MAP: schema_item_move.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.moved
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_item_move
import com.sce.generated.enum_item_status.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "item.moved"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 40uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_item_move:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_item_move:1\",\"x-sce-source-hash\":\"842c7bdea0644aef8efe41e1b76ca837dbf2e5877133fa8fa148199f07b491d8\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"status\":{\"enum\":[\"open\",\"in_progress\",\"done\"]}},\"required\":[\"id\",\"status\"],\"additionalProperties\":false}"

data class SchemaItemMovePayload(    val id: UInt,    val status: EnumItemStatus)
