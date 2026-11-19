// SCE-GENERATED — DO NOT EDIT
// source-hash: a6959f2507ab21010c436ad97f3b251215f490556992ce77a48ad5ba8d0202b7
// source: schema_signal.scxml
// SCE-MAP: schema_signal.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: signal.raised
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_signal
import com.sce.generated.enum_signal_level.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "signal.raised"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 2u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 49uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_signal:2"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_signal:2\",\"x-sce-source-hash\":\"a6959f2507ab21010c436ad97f3b251215f490556992ce77a48ad5ba8d0202b7\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"level\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"urgent\":{\"type\":\"boolean\"}},\"required\":[\"level\",\"count\",\"urgent\"],\"additionalProperties\":false}"

data class SchemaSignalPayload(    val level: EnumSignalLevel,    val count: UShort,    val urgent: Boolean)
