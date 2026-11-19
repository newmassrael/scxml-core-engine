// SCE-GENERATED — DO NOT EDIT
// source-hash: fec66ef5d8da62f47522f29d9bfc3a4d7d48dfac5d3a4c909a75843bf5c87b5c
// source: event_counted.scxml
// SCE-MAP: event_counted.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: counted.sent
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.event_counted

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "counted.sent"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most elements `kept` holds (the bound its type writes), as this build resolved it. */
const val KEPT_MAX_COUNT: UInt = 2u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 29uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:event_counted:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:event_counted:1\",\"x-sce-source-hash\":\"fec66ef5d8da62f47522f29d9bfc3a4d7d48dfac5d3a4c909a75843bf5c87b5c\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"kept\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"minItems\":1,\"maxItems\":2},\"hops\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kept\",\"hops\"],\"additionalProperties\":false}"

data class EventCountedPayload(    val kept: List<UByte>,    val hops: UByte)
