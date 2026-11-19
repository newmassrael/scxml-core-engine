// SCE-GENERATED — DO NOT EDIT
// source-hash: 3088e9cab925ffee9d43a9e4845aee06c06e06454f13eca167167a445cb4f899
// source: sce:std/sync/sync_start.scxml
// SCE-MAP: sync_start.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.start
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.sync_start

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "sync.start"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 17uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:sync_start:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:sync_start:1\",\"x-sce-source-hash\":\"3088e9cab925ffee9d43a9e4845aee06c06e06454f13eca167167a445cb4f899\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"byToken\":{\"type\":\"boolean\"}},\"required\":[\"byToken\"],\"additionalProperties\":false}"

data class SyncStartPayload(    val byToken: Boolean)
