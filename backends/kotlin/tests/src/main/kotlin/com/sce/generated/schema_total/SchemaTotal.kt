// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
// SCE-MAP: schema_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ledger.total
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_total

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "ledger.total"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 38uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_total:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_total:1\",\"x-sce-source-hash\":\"d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"sum\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"count\",\"sum\"],\"additionalProperties\":false}"

data class SchemaTotalPayload(    val count: UInt,    val sum: Int)
