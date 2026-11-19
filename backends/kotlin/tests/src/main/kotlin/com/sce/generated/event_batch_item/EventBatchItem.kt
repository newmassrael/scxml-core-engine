// SCE-GENERATED — DO NOT EDIT
// source-hash: 5b3cca3ed594e12d650da768e79aecbb481c76becb4340136f5ab34c5ec2d655
// source: event_batch_item.scxml
// SCE-MAP: event_batch_item.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: batch.item
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.event_batch_item
import com.sce.generated.enum_signal_level.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "batch.item"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `label` holds (the bound its type writes), as this build resolved it. */
const val LABEL_MAX_SIZE: UInt = 8u

/** The most bytes `blob` holds (the bound its type writes), as this build resolved it. */
const val BLOB_MAX_SIZE: UInt = 4u

/** The most elements `tags` holds (the bound its type writes), as this build resolved it. */
const val TAGS_MAX_COUNT: UInt = 4u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 403uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:event_batch_item:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:event_batch_item:1\",\"x-sce-source-hash\":\"5b3cca3ed594e12d650da768e79aecbb481c76becb4340136f5ab34c5ec2d655\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},\"label\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"level\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"due\":{\"anyOf\":[{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},{\"type\":\"null\"}]},\"blob\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*\$\"},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"maxItems\":4}},\"required\":[\"id\",\"amount\",\"label\",\"level\",\"due\",\"blob\",\"tags\"],\"additionalProperties\":false}"

data class EventBatchItemPayload(    val id: UInt,    val amount: Long,    val label: String,    val level: EnumSignalLevel,    val due: Long?,    val blob: ByteArray,    val tags: List<String>)
