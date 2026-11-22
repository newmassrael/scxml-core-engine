// SCE-GENERATED — DO NOT EDIT
// source-hash: 569251024dafb92923640e308d79f98d409527af6ca4b736e77edae808cc3a18
// source: grow_shaped_1.scxml
// SCE-MAP: grow_shaped_1.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.shaped
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.grow_shaped_1
import com.sce.generated.grow_shape_1.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "grow.shaped"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 37uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:grow_shaped_1:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:grow_shaped_1:1\",\"x-sce-source-hash\":\"569251024dafb92923640e308d79f98d409527af6ca4b736e77edae808cc3a18\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"shape\":{\"\$ref\":\"#/\$defs/grow_shape_1\"}},\"required\":[\"shape\"],\"additionalProperties\":false,\"\$defs\":{\"grow_shape_1\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"single\"},\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"n\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"none\"}},\"required\":[\"kind\"],\"additionalProperties\":false}]}}}"

data class GrowShaped1Payload(    val shape: GrowShape1)
