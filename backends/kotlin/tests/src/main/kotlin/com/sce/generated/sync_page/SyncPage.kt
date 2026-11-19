// SCE-GENERATED — DO NOT EDIT
// source-hash: b81adbfc5f503ac11fb4879daa5cf0f87f4e30b7b6cc3cf3a69c5835224f3658
// source: sce:std/sync/sync_page.scxml
// SCE-MAP: sync_page.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.page
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.sync_page

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "sync.page"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 14uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:sync_page:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:sync_page:1\",\"x-sce-source-hash\":\"b81adbfc5f503ac11fb4879daa5cf0f87f4e30b7b6cc3cf3a69c5835224f3658\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"more\":{\"type\":\"boolean\"}},\"required\":[\"more\"],\"additionalProperties\":false}"

data class SyncPagePayload(    val more: Boolean)
