// SCE-GENERATED — DO NOT EDIT
// source-hash: e9d14a0e8166097179b777ae40cc80c19551029f15d97332ade0688c2b4c0364
// source: shape_board.scxml
// SCE-MAP: shape_board.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.board
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.shape_board
import com.sce.generated.union_shape.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "shape.board"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it. */
const val TITLE_MAX_SIZE: UInt = 8u

/** The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it. */
const val NOTE_MAX_SIZE: UInt = 8u

/** The most elements `shapes` holds (the bound its type writes), as this build resolved it. */
const val SHAPES_MAX_COUNT: UInt = 3u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 586uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:shape_board:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:shape_board:1\",\"x-sce-source-hash\":\"e9d14a0e8166097179b777ae40cc80c19551029f15d97332ade0688c2b4c0364\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]},\"shapes\":{\"type\":\"array\",\"items\":{\"\$ref\":\"#/\$defs/union_shape\"},\"maxItems\":3},\"pinned\":{\"anyOf\":[{\"\$ref\":\"#/\$defs/union_shape\"},{\"type\":\"null\"}]}},\"required\":[\"title\",\"count\",\"note\",\"shapes\",\"pinned\"],\"additionalProperties\":false,\"\$defs\":{\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"

data class ShapeBoardPayload(    val title: String,    val count: UByte,    val note: String?,    val shapes: List<UnionShape>,    val pinned: UnionShape?)
