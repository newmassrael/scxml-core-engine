// SCE-GENERATED — DO NOT EDIT
// source-hash: 62baaaee532ce05683ea46a77ba1777a9aef787e934ae52b10c4b70563eff745
// source: lift_answer.scxml
// SCE-MAP: lift_answer.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: lift.answer
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lift_answer
import com.sce.generated.enum_lift_kind.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "lift.answer"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 2u

/** The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it. */
const val TITLE_MAX_SIZE: UInt = 16u

/** The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it. */
const val NOTE_MAX_SIZE: UInt = 8u

/** The most bytes `tag` holds (the bound its type writes), as this build resolved it. */
const val TAG_MAX_SIZE: UInt = 4u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 279uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:lift_answer:2"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:lift_answer:2\",\"x-sce-source-hash\":\"62baaaee532ce05683ea46a77ba1777a9aef787e934ae52b10c4b70563eff745\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"kind\":{\"enum\":[\"meeting\",\"task\"]},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]},\"tag\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*\$\"},\"priority\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"id\",\"amount\",\"title\",\"kind\",\"note\",\"tag\",\"priority\"],\"additionalProperties\":false}"

data class LiftAnswerPayload(    val id: UInt,    val amount: Long,    val title: String,    val kind: EnumLiftKind,    val note: String?,    val tag: ByteArray,    val priority: UByte)
