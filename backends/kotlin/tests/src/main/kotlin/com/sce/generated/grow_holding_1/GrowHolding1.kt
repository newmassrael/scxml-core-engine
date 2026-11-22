// SCE-GENERATED — DO NOT EDIT
// source-hash: cbae23ec3e271a5d3d031cf17aaa399ae7d046b79eff29ef521b985e000db27e
// source: grow_holding_1.scxml
// SCE-MAP: grow_holding_1.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.holding
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.grow_holding_1
import com.sce.generated.grow_outer_1.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "grow.holding"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 82uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:grow_holding_1:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:grow_holding_1:1\",\"x-sce-source-hash\":\"cbae23ec3e271a5d3d031cf17aaa399ae7d046b79eff29ef521b985e000db27e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"outer\":{\"anyOf\":[{\"\$ref\":\"#/\$defs/grow_outer_1\"},{\"type\":\"null\"}]}},\"required\":[\"outer\"],\"additionalProperties\":false,\"\$defs\":{\"grow_inner_1\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false},\"grow_outer_1\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"inner\":{\"\$ref\":\"#/\$defs/grow_inner_1\"},\"inners\":{\"type\":\"array\",\"items\":{\"\$ref\":\"#/\$defs/grow_inner_1\"},\"maxItems\":2}},\"required\":[\"id\",\"inner\",\"inners\"],\"additionalProperties\":false}}}"

data class GrowHolding1Payload(    val outer: GrowOuter1Payload?)
