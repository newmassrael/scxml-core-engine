// SCE-GENERATED — DO NOT EDIT
// source-hash: 533ef0760e619c9413bd88ab32b43d3a9be7377c761db11817ce503ffebcd536
// source: sce:std/host/model_generate_result.scxml
// SCE-MAP: model_generate_result.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.model.generate.result
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.model_generate_result

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "host.model.generate.result"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it. */
const val ERROR_MAX_SIZE: UInt = 24u

/** The most UTF-8 bytes `text` holds (the bound its type writes, named `model_answer`), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 4096u

/** The most UTF-8 bytes `model` holds (the bound its type writes), as this build resolved it. */
const val MODEL_MAX_SIZE: UInt = 128u

/** The most UTF-8 bytes `finish` holds (the bound its type writes), as this build resolved it. */
const val FINISH_MAX_SIZE: UInt = 16u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 25703uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:model_generate_result:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:model_generate_result:1\",\"x-sce-source-hash\":\"533ef0760e619c9413bd88ab32b43d3a9be7377c761db11817ce503ffebcd536\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"text\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"constrained\":{\"type\":\"boolean\"},\"model\":{\"type\":\"string\",\"maxLength\":128,\"x-sce-max-bytes\":128},\"prompt_tokens\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"answer_tokens\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"finish\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16}},\"required\":[\"error\",\"text\",\"constrained\",\"model\",\"prompt_tokens\",\"answer_tokens\",\"finish\"],\"additionalProperties\":false}"

data class ModelGenerateResultPayload(    val error: String,    val text: String,    val constrained: Boolean,    val model: String,    val prompt_tokens: UInt,    val answer_tokens: UInt,    val finish: String)
