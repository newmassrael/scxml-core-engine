// SCE-GENERATED — DO NOT EDIT
// source-hash: 0a5499883e3dede76e0da3aef9c50d1970b09b6f668a62e144200d84c0fbe2ef
// source: schema_day.scxml
// SCE-MAP: schema_day.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: day.picked
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_day

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "day.picked"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 43uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_day:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_day:1\",\"x-sce-source-hash\":\"0a5499883e3dede76e0da3aef9c50d1970b09b6f668a62e144200d84c0fbe2ef\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"year\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"month\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"dayOfMonth\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"year\",\"month\",\"dayOfMonth\"],\"additionalProperties\":false}"

data class SchemaDayPayload(    val year: UShort,    val month: UByte,    val dayOfMonth: UByte)
