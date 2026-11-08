// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_item_pick

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "item.picked"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it. */
const val TITLE_MAX_SIZE: UInt = 16u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 144uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_item_pick:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_item_pick:1\",\"x-sce-source-hash\":\"9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"id\",\"title\",\"count\"],\"additionalProperties\":false}"

data class SchemaItemPickPayload(    val id: UInt,    val title: String,    val count: Int)
