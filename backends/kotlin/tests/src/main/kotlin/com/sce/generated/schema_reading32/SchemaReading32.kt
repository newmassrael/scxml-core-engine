// SCE-GENERATED — DO NOT EDIT
// source-hash: 5bb9ee005cfd081320e2ea81c3edaa782a7f2a6d97c0b01c826458a9f4b1203d
// source: schema_reading32.scxml
// SCE-MAP: schema_reading32.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading32.taken
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_reading32

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "reading32.taken"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 48uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_reading32:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_reading32:1\",\"x-sce-source-hash\":\"5bb9ee005cfd081320e2ea81c3edaa782a7f2a6d97c0b01c826458a9f4b1203d\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sensor\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"value\":{\"type\":\"number\",\"minimum\":-3.4028234663852886e+38,\"maximum\":3.4028234663852886e+38}},\"required\":[\"sensor\",\"value\"],\"additionalProperties\":false}"

data class SchemaReading32Payload(    val sensor: UByte,    val value: Float)
