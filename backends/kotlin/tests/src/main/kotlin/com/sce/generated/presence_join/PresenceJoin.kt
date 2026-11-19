// SCE-GENERATED — DO NOT EDIT
// source-hash: dbfaa40ace1384624ceca3ae6c2b52e1b8325e5b64c14806e6f142d419bd6387
// source: presence_join.scxml
// SCE-MAP: presence_join.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: presence.join
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.presence_join

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "presence.join"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 67uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:presence_join:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:presence_join:1\",\"x-sce-source-hash\":\"dbfaa40ace1384624ceca3ae6c2b52e1b8325e5b64c14806e6f142d419bd6387\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"node\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"hub\":{\"type\":\"boolean\"},\"ttl\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"slack\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"node\",\"hub\",\"ttl\",\"slack\"],\"additionalProperties\":false}"

data class PresenceJoinPayload(    val node: UInt,    val hub: Boolean,    val ttl: UInt,    val slack: UInt)
