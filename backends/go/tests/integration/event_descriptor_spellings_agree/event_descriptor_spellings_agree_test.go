// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1: `wild`, `wild.` and `wild.*` are one descriptor — Go AOT path.
//
// The clause calls the three spellings "functionally equivalent since they are
// token prefixes of exactly the same set of event names", and a descriptor
// ending in ".*" matches "zero or more tokens", so a bare ".*" is an empty
// token prefix and matches every event.
//
// No W3C fixture delivers a bare `foo` to a `foo.*` handler, and the four that
// write a bare ".*" write it as a catch-all to fail — which an engine that
// matches nothing on ".*" passes by never taking the transition it must not
// take. This fixture puts every case in positive polarity instead.
//
// Fixture: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_descriptor_spellings_agree_go.sh

package event_descriptor_spellings_agree

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestEverySpellingOfOneDescriptorCatchesTheSameEvents(t *testing.T) {
	policy := NewEventDescriptorSpellingsAgreePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[EventDescriptorSpellingsAgreeState, EventDescriptorSpellingsAgreeEvent](&policy)
	engine.Initialize()

	if completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond); !completed {
		t.Fatalf("the machine never reached a final state (parked in %v); every case "+
			"in this fixture has a literal fallback, so parking means no transition "+
			"matched an event that two of them describe", engine.GetCurrentState())
	}

	// Each failure final names the case, so this one check says which spelling
	// disagreed with W3C SCXML 3.12.1:
	//   FailSuffixed   "wild.*" did not catch bare "wild"
	//   FailDotted     "dot." did not catch bare "dot"
	//   FailBounded    "wild.*" caught "wilder", across a token boundary
	//   FailUniversal  a bare ".*" did not catch "any.token.sequence"
	if got, ended := engine.TerminalState(); !ended || got != EventDescriptorSpellingsAgreeStatePass {
		t.Fatalf("the machine rested in %v: \"wild.*\" must catch bare \"wild\" and "+
			"\"dot.\" must catch bare \"dot\" (the clause calls the spellings "+
			"functionally equivalent), a bare \".*\" must catch every event, and "+
			"\"wild.*\" must NOT catch \"wilder\" because the prefix is a whole token", got)
	}
}
