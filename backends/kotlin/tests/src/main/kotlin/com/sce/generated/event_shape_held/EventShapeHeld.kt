// SCE-GENERATED — DO NOT EDIT
// source-hash: c5c5d4fd439280a01fde979f3185d9ae7bc8e8d9185453e1fd4ac5dfd2d38783
// source: event_shape_held.scxml
// SCE-MAP: event_shape_held.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.held
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.event_shape_held

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "shape.held"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 128u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 779uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:event_shape_held:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:event_shape_held:1\",\"x-sce-source-hash\":\"c5c5d4fd439280a01fde979f3185d9ae7bc8e8d9185453e1fd4ac5dfd2d38783\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":128,\"x-sce-max-bytes\":128}},\"required\":[\"text\"],\"additionalProperties\":false}"

data class EventShapeHeldPayload(    val text: String)
