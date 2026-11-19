// SCE-GENERATED — DO NOT EDIT
// source-hash: 264bd4f4032d73a075f28ad1aa2365aa46368292fa96a06a28809bd85fd1cc26
// source: sce:std/host/model_generate_request.scxml
// SCE-MAP: model_generate_request.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.model.generate.request
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.model_generate_request

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.model.generate.request"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `prompt` holds (the bound its type writes, named `model_prompt`), as this build resolved it. */
const val PROMPT_MAX_SIZE: UInt = 8192u

/** The most UTF-8 bytes `answer_schema` holds (the bound its type writes), as this build resolved it. */
const val ANSWER_SCHEMA_MAX_SIZE: UInt = 128u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 50002uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:model_generate_request:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:model_generate_request:1\",\"x-sce-source-hash\":\"264bd4f4032d73a075f28ad1aa2365aa46368292fa96a06a28809bd85fd1cc26\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"prompt\":{\"type\":\"string\",\"maxLength\":8192,\"x-sce-max-bytes\":8192},\"answer_schema\":{\"type\":\"string\",\"maxLength\":128,\"x-sce-max-bytes\":128},\"max_tokens\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"temperature_milli\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"prompt\",\"answer_schema\",\"max_tokens\",\"temperature_milli\"],\"additionalProperties\":false}"

data class ModelGenerateRequestPayload(    val prompt: String,    val answer_schema: String,    val max_tokens: UInt,    val temperature_milli: UShort)
