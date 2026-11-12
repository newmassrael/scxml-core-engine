// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Go AOT.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Rust / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh go-integration

package a_computed_event_name_is_matched_like_any_other

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAComputedNameIsMatchedByTokenAndToldWhole(t *testing.T) {
	policy := NewAComputedEventNameIsMatchedLikeAnyOtherPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AComputedEventNameIsMatchedLikeAnyOtherState, AComputedEventNameIsMatchedLikeAnyOtherEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("a_computed_event_name_is_matched_like_any_other stopped before its final state: " +
			"a computed name was dropped, or matched but told shorter than the name it was sent " +
			"under (`request.new`, `other.thing`, `request.again`, `last.one`)")
	}

	got, ended := engine.TerminalState()
	if !ended || got != AComputedEventNameIsMatchedLikeAnyOtherStatePass {
		t.Fatalf("the machine reached %v, want Pass", got)
	}
}

// runObserved makes the machine, sets a transition observer (docs/adr/0089)
// that appends each row it is told to seen, clears it again when clear, and
// runs the machine to its end, which must be pass.
func runObserved(t *testing.T, seen *[]sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState], clear bool) {
	t.Helper()
	policy := NewAComputedEventNameIsMatchedLikeAnyOtherPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AComputedEventNameIsMatchedLikeAnyOtherState, AComputedEventNameIsMatchedLikeAnyOtherEvent](&policy)
	engine.SetTransitionObserver(func(taken sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]) {
		*seen = append(*seen, taken)
	})
	if clear {
		engine.SetTransitionObserver(nil)
	}
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	if !engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond) {
		t.Fatalf("the observed run stopped before its final state")
	}
	if got, ended := engine.TerminalState(); !ended || got != AComputedEventNameIsMatchedLikeAnyOtherStatePass {
		t.Fatalf("the observed run reached %v, want Pass", got)
	}
}

// The document declares its transition table, and a run that reaches pass
// takes each of its six transitions once, in document order: the observer is
// told exactly those rows, each by the (From, Index) its row of the table
// carries.
func TestTheObserverIsToldEachRowOfTheTableTheRunTakes(t *testing.T) {
	var seen []sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]
	runObserved(t, &seen, false)

	want := []sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]{
		{Source: AComputedEventNameIsMatchedLikeAnyOtherStateS1, Index: 0},
		{Source: AComputedEventNameIsMatchedLikeAnyOtherStateS2, Index: 0},
		{Source: AComputedEventNameIsMatchedLikeAnyOtherStateS3, Index: 0},
		{Source: AComputedEventNameIsMatchedLikeAnyOtherStateS4, Index: 0},
		{Source: AComputedEventNameIsMatchedLikeAnyOtherStateS5, Index: 0},
		{Source: AComputedEventNameIsMatchedLikeAnyOtherStateS6, Index: 0},
	}
	if len(seen) != len(want) {
		t.Fatalf("the observer was told %v, want %v", seen, want)
	}
	for i := range want {
		if seen[i] != want[i] {
			t.Fatalf("the observer was told %v, want %v", seen, want)
		}
	}

	rows := make(map[sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]]bool)
	for _, row := range AComputedEventNameIsMatchedLikeAnyOtherTransitions {
		rows[sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]{Source: row.From, Index: row.Index}] = true
	}
	taken := make(map[sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]]bool)
	for _, pair := range seen {
		if !rows[pair] {
			t.Fatalf("the observer was told %v, which is no row of the table", pair)
		}
		taken[pair] = true
	}
	for row := range rows {
		if !taken[row] {
			t.Fatalf("the run never took row %v of the table", row)
		}
	}
}

// An observer set and then cleared is told nothing, and the run still reaches
// pass, as one that never had an observer does.
func TestAClearedObserverIsToldNothingAndTheRunIsTheSame(t *testing.T) {
	var seen []sce.TransitionTaken[AComputedEventNameIsMatchedLikeAnyOtherState]
	runObserved(t, &seen, true)
	if len(seen) != 0 {
		t.Fatalf("a cleared observer was told %v", seen)
	}
}
