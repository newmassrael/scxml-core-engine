// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// Generated text (docs/SCE_HOST_FEATURES.md, section 6), apart from the scenario
// the lanes share.

var testSchemas = []sce.AnswerSchema{{ID: "urn:sce:schema:answer:1", JSONSchema: "{}", GBNF: "root ::= \"{}\""}}

func ask(prompt, answerSchema, maxTokens, temperature string) Params {
	return Params{"prompt": prompt, "answer_schema": answerSchema, "max_tokens": maxTokens, "temperature_milli": temperature}
}

func modelHost(provider ModelProvider) *ReferenceHost {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	h.SetAnswerSchemas(testSchemas)
	h.SetModelProvider(provider)
	return h
}

func scripted(texts ...string) *ScriptedProvider {
	p := NewScriptedProvider()
	p.Constrained = true
	for _, text := range texts {
		p.Answers = append(p.Answers, ScriptedAnswer{Text: text, Finish: "stop"})
	}
	return p
}

// Each argument the operation does not accept is refused before the provider is
// asked, so the answer it would give stays.
func TestAGenerationTheOperationDoesNotAcceptIsRefused(t *testing.T) {
	h := modelHost(scripted("kept"))
	for why, params := range map[string]Params{
		"an empty prompt":               ask("", "", "8", "0"),
		"max_tokens zero":               ask("p", "", "0", "0"),
		"a temperature past 2000":       ask("p", "", "8", "2001"),
		"an id the table does not hold": ask("p", "urn:sce:schema:other:1", "8", "0"),
	} {
		if outcome := start(t, h, ModelGenerate, params); outcome.Kind != Refuse {
			t.Fatalf("%s was not refused: %+v", why, outcome)
		}
	}
	if fields := decode(t, start(t, h, ModelGenerate, ask("p", "", "8", "2000"))); fields["text"] != "kept" {
		t.Fatalf("a refusal took an answer: %v", fields)
	}
}

// A constrained provider says so only for a request that names an answer schema;
// its tokens are bytes; with no answer left it is unavailable.
func TestAScriptedProviderIsConstrainedOnlyUnderAnAnswerSchema(t *testing.T) {
	h := modelHost(scripted("one", "two"))
	if free := decode(t, start(t, h, ModelGenerate, ask("ab", "", "8", "0"))); free["constrained"] != false || free["error"] != "" {
		t.Fatalf("a free text was constrained: %v", free)
	}
	held := decode(t, start(t, h, ModelGenerate, ask("abc", "urn:sce:schema:answer:1", "8", "0")))
	if held["constrained"] != true || held["prompt_tokens"] != float64(3) || held["answer_tokens"] != float64(3) ||
		held["finish"] != "stop" || held["model"] != "scripted" {
		t.Fatalf("a constrained answer was %v", held)
	}
	if none := decode(t, start(t, h, ModelGenerate, ask("ab", "", "8", "0"))); none["error"] != "unavailable" || none["text"] != "" {
		t.Fatalf("a provider with no answer left gave %v", none)
	}
}

// An answer past the bound of `text` is never cut; a prompt and max_tokens past
// the context fail and take no answer.
func TestAnAnswerPastItsBoundOrItsContextFailsWhole(t *testing.T) {
	h := modelHost(scripted(strings.Repeat("a", AnswerMaxBytes+1), strings.Repeat("a", AnswerMaxBytes)))
	if past := decode(t, start(t, h, ModelGenerate, ask("p", "", "8", "0"))); past["error"] != "answer_too_large" {
		t.Fatalf("an answer past its bound was %v", past)
	}
	if whole := decode(t, start(t, h, ModelGenerate, ask("p", "", "8", "0"))); len(whole["text"].(string)) != AnswerMaxBytes {
		t.Fatalf("an answer at its bound was cut")
	}
	limited := scripted("kept")
	limited.Context = 10
	h = modelHost(limited)
	if past := decode(t, start(t, h, ModelGenerate, ask("abc", "", "8", "0"))); past["error"] != "context_exceeded" {
		t.Fatalf("a request past the context was %v", past)
	}
	if within := decode(t, start(t, h, ModelGenerate, ask("abc", "", "7", "0"))); within["text"] != "kept" {
		t.Fatalf("the answer waiting was taken by a failure: %v", within)
	}
}

// The exhaustive provider answers each class it is named, in order, behind the
// host's own judgement of the request.
func TestTheExhaustiveProviderAnswersEachClassInTurn(t *testing.T) {
	h := modelHost(NewExhaustiveProvider([]OutcomeClass{
		{Kind: Answered, Text: "t", Finish: "length"},
		{Kind: Failed, Code: "busy"},
		{Kind: Refused},
		{Kind: Held},
	}))
	answer := decode(t, start(t, h, ModelGenerate, ask("p", "", "8", "0")))
	if answer["text"] != "t" || answer["finish"] != "length" || answer["model"] != "exhaustive" || answer["constrained"] != false {
		t.Fatalf("an answer class gave %v", answer)
	}
	if failed := decode(t, start(t, h, ModelGenerate, ask("p", "", "8", "0"))); failed["error"] != "busy" {
		t.Fatalf("a failure class gave %v", failed)
	}
	if refused := start(t, h, ModelGenerate, ask("p", "", "8", "0")); refused.Kind != Refuse {
		t.Fatalf("a refusal class gave %+v", refused)
	}
	if held := start(t, h, ModelGenerate, ask("p", "", "8", "0")); held.Kind != Hold {
		t.Fatalf("a hold class gave %+v", held)
	}
	if past := decode(t, start(t, h, ModelGenerate, ask("p", "", "8", "0"))); past["error"] != "unavailable" {
		t.Fatalf("a call past the sequence gave %v", past)
	}
}
