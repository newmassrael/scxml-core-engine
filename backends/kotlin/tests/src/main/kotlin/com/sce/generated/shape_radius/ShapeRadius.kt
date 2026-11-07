// SCE-GENERATED — DO NOT EDIT
// source-hash: 185d995fbef5e4650ae7c20d165e4a208237e26dd7d946178edb7b10ba93cc9e
// SCE-MAP: shape_radius.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.radius
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.shape_radius

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "shape.radius"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 16uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:shape_radius:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:shape_radius:1\",\"x-sce-source-hash\":\"185d995fbef5e4650ae7c20d165e4a208237e26dd7d946178edb7b10ba93cc9e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"radius\"],\"additionalProperties\":false}"

data class ShapeRadiusPayload(    val radius: UShort)
