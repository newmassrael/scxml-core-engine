// SCE-GENERATED — DO NOT EDIT
// source-hash: 8ce4d52c276926253322651e08b928073e7af237faa5416273ef4be3566b7402
// source: sce:std/shared/record_verdict.scxml
// SCE-MAP: record_verdict.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.verdict
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.record_verdict
import com.sce.generated.record_outcome.*
import com.sce.generated.record_refusal.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "record_list.verdict"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 107uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:record_verdict:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:record_verdict:1\",\"x-sce-source-hash\":\"8ce4d52c276926253322651e08b928073e7af237faa5416273ef4be3566b7402\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"wallTime\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},\"counter\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"outcome\":{\"enum\":[\"none\",\"accepted\",\"held\",\"waiting\",\"refused\"]},\"reason\":{\"anyOf\":[{\"enum\":[\"notWriter\",\"noRoomForValue\",\"noRoomForHistory\",\"noRoomForDigest\",\"changeTooLarge\",\"registerNamedTwice\",\"clockExhausted\",\"peerRefused\"]},{\"type\":\"null\"}]}},\"required\":[\"wallTime\",\"counter\",\"outcome\",\"reason\"],\"additionalProperties\":false}"

data class RecordVerdictPayload(    val wallTime: Long,    val counter: UInt,    val outcome: RecordOutcome,    val reason: RecordRefusal?)
