// SCE-GENERATED — DO NOT EDIT
// source-hash: c090e931f092b0370328464cce7a355fb68b64ba23ab2308efbad976050737de
// source: schema_revised.scxml
// SCE-MAP: schema_revised.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading.revised
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_revised
import com.sce.generated.enum_view_mode.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "reading.revised"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 3u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 51uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_revised:3"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_revised:3\",\"x-sce-source-hash\":\"c090e931f092b0370328464cce7a355fb68b64ba23ab2308efbad976050737de\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"level\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"urgent\":{\"type\":\"boolean\"}},\"required\":[\"level\",\"layout\",\"urgent\"],\"additionalProperties\":false}"

data class SchemaRevisedPayload(    val level: UByte,    val layout: EnumViewMode,    val urgent: Boolean)
