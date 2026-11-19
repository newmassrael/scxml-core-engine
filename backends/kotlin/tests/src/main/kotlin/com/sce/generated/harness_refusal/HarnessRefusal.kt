// SCE-GENERATED — DO NOT EDIT
// source-hash: c35a07cbfbf9d94465fe237a3784126e85a8fd8cc754be15ffc2023f0c77f597
// source: harness_refusal.scxml
// SCE-MAP: harness_refusal.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: harness.refusal
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.harness_refusal
import com.sce.generated.enum_harness_code.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "harness.refusal"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `path` holds (the bound its type writes), as this build resolved it. */
const val PATH_MAX_SIZE: UInt = 256u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 1572uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:harness_refusal:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:harness_refusal:1\",\"x-sce-source-hash\":\"c35a07cbfbf9d94465fe237a3784126e85a8fd8cc754be15ffc2023f0c77f597\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"code\":{\"enum\":[\"notJson\",\"notObject\",\"tooDeep\",\"missing\",\"nullNotOptional\",\"wrongKind\",\"badNumber\",\"outOfRange\",\"tooLong\",\"tooMany\",\"unknownVariant\",\"notByteText\",\"nulInText\",\"tooFew\",\"notOffered\"]},\"path\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"code\",\"path\"],\"additionalProperties\":false}"

data class HarnessRefusalPayload(    val code: EnumHarnessCode,    val path: String)
