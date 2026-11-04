// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

import com.sce.runtime.AnswerSchema

// Generated text (docs/SCE_HOST_FEATURES.md, section 6; docs/adr/0087, section
// 4): the semantics of `sce-host-model-generate` over a provider the embedding
// supplies.
//
// The host judges the request (an empty prompt, `max_tokens` zero, a
// temperature past 2000 thousandths, an answer schema the machine's table does
// not hold), then asks its ModelProvider for a Generation, and never cuts the
// answer: one past the bound of `text` fails `answer_too_large`. No provider
// here reaches a model. ScriptedProvider gives the answers a scenario wrote;
// ExhaustiveProvider gives the outcome class a test names at each call
// (docs/adr/0087, section 9.4). The Kotlin runtime has no log facility, so the
// branches here say nothing; each outcome is visible in the record it answers.

/** The operation that generates a text. */
const val MODEL_GENERATE: String = "sce-host-model-generate"

/** The bound of a result's `text`, in bytes (`model_answer` of the result record). */
const val ANSWER_MAX_BYTES: Int = 4096

/** The highest sampling temperature a request may ask for, in thousandths. */
const val TEMPERATURE_MAX_MILLI: Long = 2000

/** A generation's request, as a provider reads it; [answerSchema] is null for a free text. */
data class GenerateRequest(
    val prompt: String,
    val answerSchema: AnswerSchema?,
    val maxTokens: Long,
    val temperatureMilli: Long,
)

/** What a provider made of one generation. */
sealed class Generation {
    /** An answer: the completion's fields but `error`. */
    data class Answer(
        val text: String,
        val finish: String,
        val constrained: Boolean,
        val model: String,
        val promptTokens: Long,
        val answerTokens: Long,
    ) : Generation()

    /** A failure, by its code of the operation's closed set. */
    data class Failure(val code: String) : Generation()

    /** The host declines to start it (`invalid_argument`). */
    object Refusal : Generation()

    /** No answer yet, or ever: a deadline or a cancel ends the call. */
    object Hold : Generation()
}

/** Where a host's generations come from: a model on the device, a remote one, or a script. */
interface ModelProvider {
    fun generate(request: GenerateRequest): Generation
}

/** A text's tokens as the reference providers count them: its UTF-8 bytes. */
internal fun tokens(text: String): Long = text.encodeToByteArray().size.toLong()

/** One answer a scripted provider gives; [finish] is `stop` or `length`. */
data class ScriptedAnswer(val text: String, val finish: String = "stop")

/**
 * A provider that gives the answers it was told, one a generation, in order. It is
 * [constrained] only when the request names an answer schema; [context] is the most
 * tokens a prompt and `max_tokens` may take together, 0 for none; [hold] holds every
 * generation unanswered.
 */
class ScriptedProvider : ModelProvider {
    val answers: ArrayDeque<ScriptedAnswer> = ArrayDeque()
    var constrained: Boolean = false
    var name: String = "scripted"
    var context: Long = 0
    var hold: Boolean = false

    override fun generate(request: GenerateRequest): Generation {
        if (hold) return Generation.Hold
        val promptTokens = tokens(request.prompt)
        if (context != 0L && promptTokens + request.maxTokens > context) {
            return Generation.Failure("context_exceeded")
        }
        val answer = answers.removeFirstOrNull() ?: return Generation.Failure("unavailable")
        return Generation.Answer(
            text = answer.text,
            finish = answer.finish,
            constrained = constrained && request.answerSchema != null,
            model = name,
            promptTokens = promptTokens,
            answerTokens = tokens(answer.text),
        )
    }
}

/**
 * One outcome class of a generation (docs/adr/0087, section 9.4), as the table every
 * lane reads names it (tests/host_features/model_outcome_classes.json).
 */
sealed class OutcomeClass {
    /** A completion with this text and `finish`. */
    data class Answer(val text: String, val finish: String) : OutcomeClass()

    /** A failure with this code. */
    data class Failure(val code: String) : OutcomeClass()

    /** The host declines to start it. */
    object Refusal : OutcomeClass()

    /** No answer: the test ends the call by a deadline or a cancel. */
    object Hold : OutcomeClass()
}

/**
 * A provider that stands for a model that may give any outcome at any call: each
 * generation takes the next class of the sequence the test named.
 */
class ExhaustiveProvider(sequence: List<OutcomeClass>) : ModelProvider {
    private val sequence = ArrayDeque(sequence)

    override fun generate(request: GenerateRequest): Generation =
        when (val next = sequence.removeFirstOrNull()) {
            null -> Generation.Failure("unavailable")
            is OutcomeClass.Answer -> Generation.Answer(
                text = next.text,
                finish = next.finish,
                constrained = false,
                model = "exhaustive",
                promptTokens = tokens(request.prompt),
                answerTokens = tokens(next.text),
            )
            is OutcomeClass.Failure -> Generation.Failure(next.code)
            OutcomeClass.Refusal -> Generation.Refusal
            OutcomeClass.Hold -> Generation.Hold
        }
}

/**
 * The request the host accepts, or null for one it refuses: an empty prompt,
 * `max_tokens` zero, a temperature past [TEMPERATURE_MAX_MILLI], or an answer schema
 * [schemas] does not hold.
 */
internal fun judgeGeneration(schemas: List<AnswerSchema>, params: Map<String, String>): GenerateRequest? {
    val prompt = params["prompt"] ?: ""
    val maxTokens = params["max_tokens"]?.toLongOrNull() ?: return null
    val temperature = params["temperature_milli"]?.toLongOrNull() ?: return null
    if (prompt.isEmpty() || maxTokens <= 0 || temperature !in 0..TEMPERATURE_MAX_MILLI) return null
    val named = params["answer_schema"] ?: ""
    val schema = if (named.isEmpty()) null else schemas.firstOrNull { it.id == named } ?: return null
    return GenerateRequest(prompt, schema, maxTokens, temperature)
}

/** Generate a text for invocation [id], whose request the host accepted. */
internal fun ReferenceHost.modelGenerate(id: String, params: Map<String, String>): Outcome {
    val request = checkNotNull(judgeGeneration(answerSchemas, params)) {
        "the request was judged before the generation"
    }
    return when (val generation = provider.generate(request)) {
        is Generation.Answer ->
            if (tokens(generation.text) > ANSWER_MAX_BYTES) {
                failure(MODEL_GENERATE, "answer_too_large")
            } else {
                record(
                    "text" to generation.text,
                    "constrained" to generation.constrained,
                    "model" to generation.model,
                    "prompt_tokens" to generation.promptTokens,
                    "answer_tokens" to generation.answerTokens,
                    "finish" to generation.finish,
                )
            }
        is Generation.Failure -> failure(MODEL_GENERATE, generation.code)
        Generation.Refusal -> Outcome.Refuse(INVALID_ARGUMENT)
        Generation.Hold -> {
            held.add(id)
            Outcome.Hold
        }
    }
}
