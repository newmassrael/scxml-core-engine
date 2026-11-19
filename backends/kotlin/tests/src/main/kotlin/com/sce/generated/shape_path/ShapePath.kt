// SCE-GENERATED — DO NOT EDIT
// source-hash: 50d1a8e706f08b895c7eedfc02c028c2162d599f1bdbd9a78b287febb7c74c4a
// source: shape_path.scxml
// SCE-MAP: shape_path.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.path
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.shape_path

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "shape.path"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most elements `points` holds (the bound its type writes), as this build resolved it. */
const val POINTS_MAX_COUNT: UInt = 3u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 24uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:shape_path:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:shape_path:1\",\"x-sce-source-hash\":\"50d1a8e706f08b895c7eedfc02c028c2162d599f1bdbd9a78b287febb7c74c4a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"points\"],\"additionalProperties\":false}"

data class ShapePathPayload(    val points: List<UByte>)
