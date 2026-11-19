// SCE-GENERATED — DO NOT EDIT
// source-hash: c76c75e47e041e6aeafb35f0635b82df25066e889367466527256df659c81304
// source: schema_reading.scxml
// SCE-MAP: schema_reading.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading.taken
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_reading

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "reading.taken"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 48uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_reading:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_reading:1\",\"x-sce-source-hash\":\"c76c75e47e041e6aeafb35f0635b82df25066e889367466527256df659c81304\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sensor\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"value\":{\"type\":\"number\"}},\"required\":[\"sensor\",\"value\"],\"additionalProperties\":false}"

data class SchemaReadingPayload(    val sensor: UByte,    val value: Double)
