// SCE-GENERATED — DO NOT EDIT
// source-hash: e201dbad52ebff31de7aee132f3d26002929ddef7fe03d44700baefd0ef100cf
// source: schema_view_grown.scxml
// SCE-MAP: schema_view_grown.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: view.grown
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_view_grown
import com.sce.generated.enum_view_mode.*
import com.sce.generated.schema_view_mark.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "view.grown"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 2u

/** The most elements `marks` holds (the bound its type writes), as this build resolved it. */
const val MARKS_MAX_COUNT: UInt = 4u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 234uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:schema_view_grown:2"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:schema_view_grown:2\",\"x-sce-source-hash\":\"e201dbad52ebff31de7aee132f3d26002929ddef7fe03d44700baefd0ef100cf\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"marks\":{\"type\":\"array\",\"items\":{\"\$ref\":\"#/\$defs/schema_view_mark\"},\"maxItems\":4}},\"required\":[\"layout\",\"marks\"],\"additionalProperties\":false,\"\$defs\":{\"schema_view_mark\":{\"type\":\"object\",\"properties\":{\"day\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"shift\":{\"anyOf\":[{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},{\"type\":\"null\"}]}},\"required\":[\"day\",\"shift\"],\"additionalProperties\":false}}}"

data class SchemaViewGrownPayload(    val layout: EnumViewMode,    val marks: List<SchemaViewMarkPayload>)
