// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Generated text (docs/SCE_HOST_FEATURES.md, section 6; docs/adr/0087, section 4): what a
// provider of `sce-host-model-generate` is, and the two the reference host has. No provider
// here reaches a model. `ScriptedProvider` gives the answers a scenario wrote;
// `ExhaustiveProvider` gives the outcome class a test names at each call (docs/adr/0087,
// section 9.4).

import SceRuntime

/// The operation that generates a text.
public let modelGenerate = "sce-host-model-generate"

/// The bound of a result's `text`, in bytes (`model_answer` of the result record).
public let answerMaxBytes = 4096

/// The highest sampling temperature a request may ask for, in thousandths.
public let temperatureMaxMilli: UInt32 = 2000

/// A generation's request, as a provider reads it; `answerSchema` is nil for a free text.
public struct GenerateRequest: Sendable {
    public var prompt: String
    public var answerSchema: AnswerSchema?
    public var maxTokens: UInt32
    public var temperatureMilli: UInt32

    public init(prompt: String, answerSchema: AnswerSchema?, maxTokens: UInt32, temperatureMilli: UInt32) {
        self.prompt = prompt
        self.answerSchema = answerSchema
        self.maxTokens = maxTokens
        self.temperatureMilli = temperatureMilli
    }
}

/// What a provider made of one generation.
public enum Generation: Equatable, Sendable {
    /// An answer: the completion's fields but `error`.
    case answer(
        text: String, finish: String, constrained: Bool, model: String, promptTokens: UInt32, answerTokens: UInt32
    )
    /// A failure, by its code of the operation's closed set.
    case failure(String)
    /// The host declines to start it (`invalid_argument`).
    case refusal
    /// No answer yet, or ever: a deadline or a cancel ends the call.
    case hold
}

/// Where a host's generations come from: a model on the device, a remote one, or a script.
public protocol ModelProvider: AnyObject {
    func generate(_ request: GenerateRequest) -> Generation
}

/// A text's tokens as the reference providers count them: its UTF-8 bytes.
func tokens(_ text: String) -> UInt32 {
    UInt32(clamping: text.utf8.count)
}

/// One answer a scripted provider gives; `finish` is `stop` or `length`.
public struct ScriptedAnswer: Equatable, Sendable {
    public var text: String
    public var finish: String

    public init(text: String, finish: String = "stop") {
        self.text = text
        self.finish = finish
    }
}

/// A provider that gives the answers it was told, one a generation, in order. It is
/// `constrained` only when the request names an answer schema; `context` is the most tokens
/// a prompt and `max_tokens` may take together, 0 for none; `hold` holds every generation
/// unanswered.
public final class ScriptedProvider: ModelProvider {
    public var answers: [ScriptedAnswer] = []
    public var constrained = false
    public var name = "scripted"
    public var context: UInt64 = 0
    public var hold = false

    public init() {}

    public func generate(_ request: GenerateRequest) -> Generation {
        if hold {
            sceLog("scripted provider: holds the generation")
            return .hold
        }
        let promptTokens = tokens(request.prompt)
        if context != 0, UInt64(promptTokens) + UInt64(request.maxTokens) > context {
            sceLog(
                "scripted provider: \(promptTokens) prompt tokens and \(request.maxTokens) more pass a context of \(context)"
            )
            return .failure("context_exceeded")
        }
        guard !answers.isEmpty else {
            sceLog("scripted provider: no answer left to give")
            return .failure("unavailable")
        }
        let answer = answers.removeFirst()
        return .answer(
            text: answer.text,
            finish: answer.finish,
            constrained: constrained && request.answerSchema != nil,
            model: name,
            promptTokens: promptTokens,
            answerTokens: tokens(answer.text)
        )
    }
}

/// One outcome class of a generation (docs/adr/0087, section 9.4), as the table every lane
/// reads names it (tests/host_features/model_outcome_classes.json).
public enum OutcomeClass: Equatable, Sendable {
    /// A completion with this text and `finish`.
    case answer(text: String, finish: String)
    /// A failure with this code.
    case failure(String)
    /// The host declines to start it.
    case refusal
    /// No answer: the test ends the call by a deadline or a cancel.
    case hold
}

/// A provider that stands for a model that may give any outcome at any call: each
/// generation takes the next class of the sequence the test named.
public final class ExhaustiveProvider: ModelProvider {
    private var sequence: [OutcomeClass]

    public init(_ sequence: [OutcomeClass]) {
        self.sequence = sequence
    }

    public func generate(_ request: GenerateRequest) -> Generation {
        guard !sequence.isEmpty else {
            sceLog("exhaustive provider: asked past the sequence the test named")
            return .failure("unavailable")
        }
        switch sequence.removeFirst() {
        case let .answer(text, finish):
            return .answer(
                text: text,
                finish: finish,
                constrained: false,
                model: "exhaustive",
                promptTokens: tokens(request.prompt),
                answerTokens: tokens(text)
            )
        case let .failure(code):
            return .failure(code)
        case .refusal:
            return .refusal
        case .hold:
            return .hold
        }
    }
}
