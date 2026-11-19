// SCE-GENERATED — DO NOT EDIT
// source-hash: 6dd7b079972debe1c7a0a50fbe29c9c040fe9594ef33e937a793ed3b8213f25d
// source: event_shape_boarded.scxml
// SCE-MAP: event_shape_boarded.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.boarded
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.event_shape_boarded

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "shape.boarded"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 256u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 1547uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:event_shape_boarded:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:event_shape_boarded:1\",\"x-sce-source-hash\":\"6dd7b079972debe1c7a0a50fbe29c9c040fe9594ef33e937a793ed3b8213f25d\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"text\"],\"additionalProperties\":false}"

data class EventShapeBoardedPayload(    val text: String)
