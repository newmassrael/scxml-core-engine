// SCE-GENERATED — DO NOT EDIT
// source-hash: 3f4753152b9b5118cb7561342df9cdc2bd1e2efdb92c16bedc13cf0d23aefe34
// source: event_shapes_rows.scxml
// SCE-MAP: event_shapes_rows.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shapes.rowed
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.event_shapes_rows
import com.sce.generated.union_shape.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "shapes.rowed"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most elements `rows` holds (the bound its type writes), as this build resolved it. */
const val ROWS_MAX_COUNT: UInt = 2u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 689uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:event_shapes_rows:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:event_shapes_rows:1\",\"x-sce-source-hash\":\"3f4753152b9b5118cb7561342df9cdc2bd1e2efdb92c16bedc13cf0d23aefe34\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"rows\":{\"type\":\"array\",\"items\":{\"type\":\"array\",\"items\":{\"\$ref\":\"#/\$defs/union_shape\"},\"maxItems\":3},\"maxItems\":2},\"at\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"inner\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"rows\",\"at\",\"inner\"],\"additionalProperties\":false,\"\$defs\":{\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"

data class EventShapesRowsPayload(    val rows: List<List<UnionShape>>,    val at: UByte,    val inner: UByte)
