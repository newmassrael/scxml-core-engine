// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

// Generated text (docs/SCE_HOST_FEATURES.md, section 6; docs/adr/0087, section
// 4): the semantics of `sce-host-model-generate` over a provider the embedding
// supplies.
//
// The host judges the request (an empty prompt, max_tokens zero, a temperature
// past 2000 thousandths, an answer schema the machine's table does not hold),
// then asks its ModelProvider for a Generation, and never cuts the answer: one
// past the bound of `text` fails answer_too_large. No provider here reaches a
// model. ScriptedProvider gives the answers a scenario wrote; ExhaustiveProvider
// gives the outcome class a test names at each call (docs/adr/0087, section 9.4).

import (
	"fmt"
	"log"
	"strconv"

	sce "github.com/newmassrael/sce-go-runtime"
)

// ModelGenerate is the operation that generates a text.
const ModelGenerate = "sce-host-model-generate"

// AnswerMaxBytes is the bound of a result's `text`, in bytes (`model_answer` of
// sce:std/host/model_generate_result.scxml).
const AnswerMaxBytes = 4096

// TemperatureMaxMilli is the highest sampling temperature a request may ask for,
// in thousandths.
const TemperatureMaxMilli = 2000

// GenerateRequest is a generation's request, as a provider reads it.
type GenerateRequest struct {
	Prompt string
	// AnswerSchema is the machine's answer schema the request names, nil for a
	// free text.
	AnswerSchema     *sce.AnswerSchema
	MaxTokens        uint32
	TemperatureMilli uint32
}

// GenerationKind says what a provider made of one generation.
type GenerationKind int

const (
	// Answered is an answer: the completion's fields but error.
	Answered GenerationKind = iota
	// Failed is a failure, by its code of the operation's closed set.
	Failed
	// Refused is a host that declines to start it (invalid_argument).
	Refused
	// Held is no answer yet, or ever: a deadline or a cancel ends the call.
	Held
)

// Generation is what a provider made of one generation.
type Generation struct {
	Kind         GenerationKind
	Text         string
	Finish       string
	Constrained  bool
	Model        string
	PromptTokens uint32
	AnswerTokens uint32
	// Code is a failure's code.
	Code string
}

// ModelProvider is where a host's generations come from: a model on the device,
// a remote one, or a script.
type ModelProvider interface {
	Generate(request GenerateRequest) Generation
}

// ScriptedAnswer is one answer a scripted provider gives.
type ScriptedAnswer struct {
	Text string
	// Finish is stop or length.
	Finish string
}

// ScriptedProvider gives the answers it was told, one a generation, in order. It
// counts a token as one byte of UTF-8.
type ScriptedProvider struct {
	Answers []ScriptedAnswer
	// Constrained says whether it decodes under a grammar; it does only when the
	// request names an answer schema.
	Constrained bool
	// Name is what it says answered.
	Name string
	// Context is the most tokens a prompt and max_tokens may take together; 0
	// for none.
	Context uint64
	// Hold says whether it holds every generation unanswered.
	Hold bool
}

// NewScriptedProvider is a scripted provider with no answer, named `scripted`.
func NewScriptedProvider() *ScriptedProvider { return &ScriptedProvider{Name: "scripted"} }

// Generate answers the next scripted answer.
func (p *ScriptedProvider) Generate(request GenerateRequest) Generation {
	if p.Hold {
		log.Printf("[sce] scripted provider: holds the generation")
		return Generation{Kind: Held}
	}
	promptTokens := tokens(request.Prompt)
	if p.Context != 0 && uint64(promptTokens)+uint64(request.MaxTokens) > p.Context {
		log.Printf("[sce] scripted provider: %d prompt tokens and %d more pass a context of %d",
			promptTokens, request.MaxTokens, p.Context)
		return Generation{Kind: Failed, Code: "context_exceeded"}
	}
	if len(p.Answers) == 0 {
		log.Printf("[sce] scripted provider: no answer left to give")
		return Generation{Kind: Failed, Code: "unavailable"}
	}
	answer := p.Answers[0]
	p.Answers = p.Answers[1:]
	log.Printf("[sce] scripted provider: answers %d bytes, finish %s", len(answer.Text), answer.Finish)
	return Generation{
		Kind:         Answered,
		Text:         answer.Text,
		Finish:       answer.Finish,
		Constrained:  p.Constrained && request.AnswerSchema != nil,
		Model:        p.Name,
		PromptTokens: promptTokens,
		AnswerTokens: tokens(answer.Text),
	}
}

// OutcomeClass is one outcome class of a generation (docs/adr/0087, section
// 9.4), as the table every lane reads names it
// (tests/host_features/model_outcome_classes.json).
type OutcomeClass struct {
	// Kind is Answered (a completion with Text and Finish), Failed (Code),
	// Refused, or Held (the test ends the call by a deadline or a cancel).
	Kind   GenerationKind
	Text   string
	Finish string
	Code   string
}

// ExhaustiveProvider stands for a model that may give any outcome at any call:
// each generation takes the next class of the sequence the test named.
type ExhaustiveProvider struct {
	sequence []OutcomeClass
}

// NewExhaustiveProvider answers sequence, one class a generation, in order.
func NewExhaustiveProvider(sequence []OutcomeClass) *ExhaustiveProvider {
	return &ExhaustiveProvider{sequence: append([]OutcomeClass{}, sequence...)}
}

// Generate answers the next class.
func (p *ExhaustiveProvider) Generate(request GenerateRequest) Generation {
	if len(p.sequence) == 0 {
		log.Printf("[sce] exhaustive provider: asked past the sequence the test named")
		return Generation{Kind: Failed, Code: "unavailable"}
	}
	class := p.sequence[0]
	p.sequence = p.sequence[1:]
	log.Printf("[sce] exhaustive provider: answers class %d", class.Kind)
	switch class.Kind {
	case Answered:
		return Generation{
			Kind:         Answered,
			Text:         class.Text,
			Finish:       class.Finish,
			Model:        "exhaustive",
			PromptTokens: tokens(request.Prompt),
			AnswerTokens: tokens(class.Text),
		}
	case Failed:
		return Generation{Kind: Failed, Code: class.Code}
	default:
		return Generation{Kind: class.Kind}
	}
}

// tokens is a text's tokens as the reference providers count them: its bytes.
func tokens(text string) uint32 { return uint32(len(text)) }

// SetModelProvider generates texts with provider from here on.
func (h *ReferenceHost) SetModelProvider(provider ModelProvider) { h.provider = provider }

// SetAnswerSchemas resolves a generation's answer_schema through schemas, the
// table of the machine this host serves (its generated AnswerSchemas). A request
// that names an id the table does not hold is refused.
func (h *ReferenceHost) SetAnswerSchemas(schemas []sce.AnswerSchema) {
	h.answerSchemas = append([]sce.AnswerSchema{}, schemas...)
}

// judgeGeneration is the request the host accepts, or why it refuses it.
func (h *ReferenceHost) judgeGeneration(params Params) (GenerateRequest, error) {
	if params["prompt"] == "" {
		return GenerateRequest{}, fmt.Errorf("the prompt is empty")
	}
	maxTokens, err := strconv.ParseUint(params["max_tokens"], 10, 32)
	if err != nil || maxTokens == 0 {
		return GenerateRequest{}, fmt.Errorf("max_tokens is not a count above zero")
	}
	temperature, err := strconv.ParseUint(params["temperature_milli"], 10, 32)
	if err != nil || temperature > TemperatureMaxMilli {
		return GenerateRequest{}, fmt.Errorf("the temperature is past 2000 thousandths")
	}
	request := GenerateRequest{
		Prompt:           params["prompt"],
		MaxTokens:        uint32(maxTokens),
		TemperatureMilli: uint32(temperature),
	}
	if id := params["answer_schema"]; id != "" {
		for i := range h.answerSchemas {
			if h.answerSchemas[i].ID == id {
				request.AnswerSchema = &h.answerSchemas[i]
			}
		}
		if request.AnswerSchema == nil {
			return GenerateRequest{}, fmt.Errorf("the answer schema is not one of the machine's")
		}
	}
	return request, nil
}

// refusesGeneration says whether the host declines to start a generation with
// params, and logs why.
func (h *ReferenceHost) refusesGeneration(id string, params Params) bool {
	if _, err := h.judgeGeneration(params); err != nil {
		log.Printf("[sce] %s %s: refused, %v", ModelGenerate, id, err)
		return true
	}
	return false
}

// modelGenerate generates a text for invocation id, whose request the host
// accepted.
func (h *ReferenceHost) modelGenerate(id string, params Params) Outcome {
	request, err := h.judgeGeneration(params)
	if err != nil {
		panic("the request was judged before the generation")
	}
	generation := h.provider.Generate(request)
	switch generation.Kind {
	case Answered:
		if len(generation.Text) > AnswerMaxBytes {
			log.Printf("[sce] %s %s: an answer of %d bytes is past %d",
				ModelGenerate, id, len(generation.Text), AnswerMaxBytes)
			return failure(ModelGenerate, "answer_too_large")
		}
		log.Printf("[sce] %s %s: answered by %s, finish %s", ModelGenerate, id, generation.Model, generation.Finish)
		return record(map[string]any{
			"text":          generation.Text,
			"constrained":   generation.Constrained,
			"model":         generation.Model,
			"prompt_tokens": generation.PromptTokens,
			"answer_tokens": generation.AnswerTokens,
			"finish":        generation.Finish,
		})
	case Failed:
		log.Printf("[sce] %s %s: fails %s", ModelGenerate, id, generation.Code)
		return failure(ModelGenerate, generation.Code)
	case Refused:
		log.Printf("[sce] %s %s: refused by the provider", ModelGenerate, id)
		return Outcome{Kind: Refuse, Data: invalidArgument}
	default:
		log.Printf("[sce] %s %s: held", ModelGenerate, id)
		h.held = append(h.held, id)
		return Outcome{Kind: Hold}
	}
}
