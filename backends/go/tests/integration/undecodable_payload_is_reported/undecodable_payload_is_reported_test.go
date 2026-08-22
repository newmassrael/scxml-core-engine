// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
// space-normalized string, and the host that built it can find out — Go AOT.
//
// The clause gives a payload three readings and names the third "otherwise".
// That word is where a belief leaves the system quietly. A host serializes
// `{"done":true}`, something truncates it to `{"done":`, and the clause is
// satisfied: the content becomes a string. The document then evaluates
// `_event.data.done`, finds nothing, and takes the transition it would have
// taken had the host sent a payload with no `done` field at all. Nothing is
// raised — the fallback is CORRECT behaviour, not an error — so before this
// fixture nothing anywhere said it had happened.
//
// Go's decoder is go-lua, whose standard library differs from Lua 5.4's, so
// "the same JSON refuses to parse" is a claim that has to be measured on this
// channel rather than inherited from a sibling.
//
// Fixture: integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_undecodable_payload_is_reported_go.sh

package undecodable_payload_is_reported

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

const (
	// Content that announces an object and stops. The shape a truncated write,
	// a half-flushed buffer or a serializer that died mid-record produces.
	truncatedObject = `{"done":`
	// The same failure announced with `[`, under the other event name, so a
	// channel that reports "the last event" rather than "the last event that
	// lost a payload" cannot pass by accident.
	truncatedArray = "[1,2"
	// W3C test 562 sends exactly this shape and requires it to arrive as a
	// string. Counting it would make the statistic fire on working documents.
	prose = "just a sentence"
	// What the host meant to send.
	intactObject = `{"done":true}`
)

func started(t *testing.T) (*sce.Engine[UndecodablePayloadIsReportedState, UndecodablePayloadIsReportedEvent], *UndecodablePayloadIsReportedPolicy) {
	t.Helper()
	policy := NewUndecodablePayloadIsReportedPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The fixture guards on `_event.data.done` and counts deliveries with
	// <assign>, so this is an ECMAScript-datamodel machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[UndecodablePayloadIsReportedState, UndecodablePayloadIsReportedEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

func deliver(engine *sce.Engine[UndecodablePayloadIsReportedState, UndecodablePayloadIsReportedEvent],
	event UndecodablePayloadIsReportedEvent, payload string) {
	engine.RaiseExternal(event, payload, "")
	engine.Step()
}

// The axis: content that asked for the structured reading and did not get it is
// counted.
func TestAPayloadThatAnnouncedStructureAndDidNotParseIsCounted(t *testing.T) {
	engine, policy := started(t)
	if got := engine.UndecodablePayloads(); got != 0 {
		t.Fatalf("nothing has been delivered before the first event, got %d", got)
	}

	deliver(engine, UndecodablePayloadIsReportedEventAnswer, truncatedObject)

	if answers, ok := policy.Answers(); !ok || answers != 1 {
		t.Fatalf("the `answer` transition did not run (answers=%d ok=%v), so nothing "+
			"below is measuring a delivery that reached the document", answers, ok)
	}
	if got := engine.UndecodablePayloads(); got != 1 {
		t.Fatalf("the host sent `%s`, which announces an object and does not parse as "+
			"one. W3C SCXML B.2.8.1 correctly delivers it as a string; the host that "+
			"built it has no other way to learn its payload stopped being structure. "+
			"Count = %d", truncatedObject, got)
	}
	if got := engine.GetCurrentState(); got != UndecodablePayloadIsReportedStateWaiting {
		t.Fatalf("the reading a payload got must not change which transition fired, "+
			"now in %v", got)
	}
}

// The other half. A count that also counts success cannot be used to detect
// failure, and the reading the clause calls "otherwise" is the NORMAL outcome
// for a document whose author wrote prose.
func TestProseAndAPayloadThatParsedAreNotCounted(t *testing.T) {
	engine, policy := started(t)

	deliver(engine, UndecodablePayloadIsReportedEventNote, prose)
	if notes, ok := policy.Notes(); !ok || notes != 1 {
		t.Fatalf("the `note` transition did not run (notes=%d ok=%v)", notes, ok)
	}
	if got := engine.UndecodablePayloads(); got != 0 {
		t.Fatalf("`%s` is the third reading working as W3C SCXML B.2.8.1 specifies and "+
			"as W3C test 562 requires. A diagnostic that fires when nothing is wrong is "+
			"one nobody reads. Count = %d", prose, got)
	}

	deliver(engine, UndecodablePayloadIsReportedEventAnswer, intactObject)
	if got := engine.GetCurrentState(); got != UndecodablePayloadIsReportedStateAccepted {
		t.Fatalf("the guard `_event.data.done` did not hold for `%s`, so the structured "+
			"reading did not happen and the zero below would be proving nothing "+
			"(now in %v)", intactObject, got)
	}
	if got := engine.UndecodablePayloads(); got != 0 {
		t.Fatalf("a payload that parsed was counted as one that did not, count = %d", got)
	}
}

// Why the query has to exist at all: the two deliveries the fixture's comment
// names are identical through every accessor a host had.
func TestTheLossIsNotDerivableFromAnyOtherAccessor(t *testing.T) {
	broken, brokenPolicy := started(t)
	deliver(broken, UndecodablePayloadIsReportedEventAnswer, truncatedObject)

	intact, intactPolicy := started(t)
	// Valid JSON, and `done` is genuinely absent — the innocent explanation an
	// operator has to rule out.
	deliver(intact, UndecodablePayloadIsReportedEventAnswer, `{"ready":true}`)

	if a, b := broken.GetCurrentState(), intact.GetCurrentState(); a != b {
		t.Fatalf("this fixture exists because a lost payload and an absent field are "+
			"indistinguishable through the accessors a host had; the states differ "+
			"(%v vs %v), so the fixture stopped measuring what it claims", a, b)
	}
	if a, b := broken.IsRunning(), intact.IsRunning(); a != b {
		t.Fatalf("the two runs differ in IsRunning (%v vs %v)", a, b)
	}
	brokenAnswers, _ := brokenPolicy.Answers()
	intactAnswers, _ := intactPolicy.Answers()
	if brokenAnswers != intactAnswers {
		t.Fatalf("the two runs differ in the document's own count (%d vs %d)",
			brokenAnswers, intactAnswers)
	}

	if a, b := broken.UndecodablePayloads(), intact.UndecodablePayloads(); a != 1 || b != 0 {
		t.Fatalf("the two runs agree on everything else, so this count is the only thing "+
			"that separates a broken sender from a working one; got %d and %d", a, b)
	}
}

// A count says a payload was lost; a host debugging a stalled supervisor needs
// to know which delivery lost it.
func TestTheEngineNamesTheDeliveryThatLostItsPayload(t *testing.T) {
	engine, _ := started(t)
	if _, ok := engine.LastUndecodablePayload(); ok {
		t.Fatalf("nothing has been delivered yet, so there is no last lost payload")
	}

	deliver(engine, UndecodablePayloadIsReportedEventAnswer, truncatedObject)
	last, ok := engine.LastUndecodablePayload()
	if !ok || last != UndecodablePayloadIsReportedEventAnswer {
		t.Fatalf("the engine counted a lost payload but cannot say which delivery lost "+
			"it (last=%v ok=%v)", last, ok)
	}

	// A second loss, under the other event name: the accessor has to track the
	// last event THAT LOST A PAYLOAD, not the last event.
	deliver(engine, UndecodablePayloadIsReportedEventNote, truncatedArray)
	if got := engine.UndecodablePayloads(); got != 2 {
		t.Fatalf("the count is a count, not a flag; got %d", got)
	}
	if last, ok := engine.LastUndecodablePayload(); !ok || last != UndecodablePayloadIsReportedEventNote {
		t.Fatalf("the second loss arrived under `note` and the engine still names "+
			"%v (ok=%v)", last, ok)
	}

	// And a delivery that succeeds must leave both alone — otherwise the last
	// name would drift to whatever arrived most recently.
	deliver(engine, UndecodablePayloadIsReportedEventAnswer, intactObject)
	if got := engine.GetCurrentState(); got != UndecodablePayloadIsReportedStateAccepted {
		t.Fatalf("the intact payload did not take the guarded transition, so the two "+
			"checks below are not measuring a successful delivery (now in %v)", got)
	}
	if got := engine.UndecodablePayloads(); got != 2 {
		t.Fatalf("a delivery that parsed moved a count that belongs to ones that did "+
			"not; got %d", got)
	}
	if last, ok := engine.LastUndecodablePayload(); !ok || last != UndecodablePayloadIsReportedEventNote {
		t.Fatalf("a delivery that parsed moved a name that belongs to one that did not "+
			"(last=%v ok=%v)", last, ok)
	}
}
