// SCE-GENERATED — DO NOT EDIT
// source-hash: 65dc738f80ddcf851ae8848bee40a3244882a93c4459336fd50ee57f02af2bf3
// source: lift_kept.scxml
// SCE-MAP: lift_kept.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.kept
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lift_kept

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "text.kept"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 512u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 3083uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:lift_kept:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:lift_kept:1\",\"x-sce-source-hash\":\"65dc738f80ddcf851ae8848bee40a3244882a93c4459336fd50ee57f02af2bf3\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":512,\"x-sce-max-bytes\":512}},\"required\":[\"text\"],\"additionalProperties\":false}"

data class LiftKeptPayload(    val text: String)
