// SCE-GENERATED — DO NOT EDIT
// source-hash: cc48aba50cc8e6a27cb6667317f3a37270ccb5a9151b7ad0c867333d2544020e
// source: harness_start.scxml
// SCE-MAP: harness_start.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: harness.start
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.harness_start

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "harness.start"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `request` holds (the bound its type writes), as this build resolved it. */
const val REQUEST_MAX_SIZE: UInt = 512u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 3086uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:harness_start:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:harness_start:1\",\"x-sce-source-hash\":\"cc48aba50cc8e6a27cb6667317f3a37270ccb5a9151b7ad0c867333d2544020e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"request\":{\"type\":\"string\",\"maxLength\":512,\"x-sce-max-bytes\":512}},\"required\":[\"request\"],\"additionalProperties\":false}"

data class HarnessStartPayload(    val request: String)
