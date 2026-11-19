// SCE-GENERATED — DO NOT EDIT
// source-hash: 6b2dc3f9a753b8e7e5a0c82dde7cb43b4fa12aef1ecd0c939f1dffc4dc1c802e
// source: sce:std/shared/record_refused.scxml
// SCE-MAP: record_refused.scxml:38 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.refused
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.record_refused
import com.sce.generated.record_command.*
import com.sce.generated.record_refusal.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "record_list.refused"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 2u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 128uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:record_refused:2"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:record_refused:2\",\"x-sce-source-hash\":\"6b2dc3f9a753b8e7e5a0c82dde7cb43b4fa12aef1ecd0c939f1dffc4dc1c802e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"field\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"kind\":{\"enum\":[\"edit\",\"clear\",\"retire\"]},\"reason\":{\"enum\":[\"notWriter\",\"noRoomForValue\",\"noRoomForHistory\",\"noRoomForDigest\",\"changeTooLarge\",\"registerNamedTwice\",\"clockExhausted\",\"peerRefused\"]},\"command\":{\"type\":\"string\",\"pattern\":\"^(0|[1-9][0-9]{0,18}|1[0-7][0-9]{18}|18[0-3][0-9]{17}|184[0-3][0-9]{16}|1844[0-5][0-9]{15}|18446[0-6][0-9]{14}|184467[0-3][0-9]{13}|1844674[0-3][0-9]{12}|184467440[0-6][0-9]{10}|1844674407[0-2][0-9]{9}|18446744073[0-6][0-9]{8}|1844674407370[0-8][0-9]{6}|18446744073709[0-4][0-9]{5}|184467440737095[0-4][0-9]{4}|18446744073709550[0-9]{3}|18446744073709551[0-5][0-9]{2}|1844674407370955160[0-9]|1844674407370955161[0-5])\$\"},\"withdrawn\":{\"type\":\"boolean\"}},\"required\":[\"id\",\"field\",\"kind\",\"reason\",\"command\",\"withdrawn\"],\"additionalProperties\":false}"

data class RecordRefusedPayload(    val id: UInt,    val field: UShort,    val kind: RecordCommand,    val reason: RecordRefusal,    val command: ULong,    val withdrawn: Boolean)
