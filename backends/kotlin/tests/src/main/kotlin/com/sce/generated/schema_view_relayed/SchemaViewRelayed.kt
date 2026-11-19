// SCE-GENERATED — DO NOT EDIT
// source-hash: 43e9bed050cab8d77f3c6d95875bd06221234afea4f026dd19683e2207c4394e
// source: schema_view_relayed.scxml
// SCE-MAP: schema_view_relayed.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: view.relayed
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_view_relayed
import com.sce.generated.enum_view_mode.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "view.relayed"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 35uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_view_relayed:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_view_relayed:1\",\"x-sce-source-hash\":\"43e9bed050cab8d77f3c6d95875bd06221234afea4f026dd19683e2207c4394e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"zoom\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"layout\",\"zoom\"],\"additionalProperties\":false}"

data class SchemaViewRelayedPayload(    val layout: EnumViewMode,    val zoom: UByte)
