// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// W3C SCXML Appendix D — what a microstep selects, exits and enters, asked of
// microstep.go over a document written out by hand.
//
// Each case states the answer the appendix computes, worked from the
// pseudo-code, and asks the transcription for it. The cases, the document and
// the answers are those of backends/rust/runtime/tests/microstep_algorithms.rs
// and backends/python/tests/microstep/test_microstep_algorithms.py, and through
// them of the C++ engines' tests/states/EntrySetAlgorithmsTest.cpp and
// tests/states/CompletionAlgorithmsTest.cpp, so the four transcriptions answer
// one set of questions. Three answers are the reason the entry procedures exist
// and are pinned by name:
//
//   - a target SET enters every target, and a <parallel> region no target
//     descends into still gets its default;
//   - a compound state entered only as an ANCESTOR of a deeper target is not in
//     statesForDefaultEntry, so its initial transition content does not run;
//   - a history that recorded several regions restores all of them.
//
//	<scxml initial="s">
//	  <state id="s" initial="p">
//	    <parallel id="p">
//	      <state id="r1" initial="a1"> a1 a2 </state>
//	      <state id="r2" initial="b1"> b1 b2
//	        <history id="hb" type="shallow"> -> b2 </history> </state>
//	      <state id="r3"> c1 c2 </state>                    (no initial: c1)
//	    </parallel>
//	    <state id="q" initial="q1"> q1 <final id="qf"/> </state>
//	    <history id="hs" type="deep"> -> "a2 b2" </history>
//	    <state id="m" initial="m1a m2b">
//	      <parallel id="mp">
//	        <state id="m1"> m1a m1b </state>
//	        <state id="m2"> m2a m2b </state>
//	      </parallel>
//	    </state>
//	  </state>
//	  <state id="t"/>
//	</scxml>

import (
	"fmt"
	"reflect"
	"sort"
	"testing"
)

// st is declared in document order, so a value IS the document position.
type st int

const (
	stS st = iota
	stP
	stR1
	stA1
	stA2
	stR2
	stB1
	stB2
	stR3
	stC1
	stC2
	stQ
	stQ1
	stQf
	stM
	stMp
	stM1
	stM1a
	stM1b
	stM2
	stM2a
	stM2b
	stT
)

var stNames = [...]string{
	"S", "P", "R1", "A1", "A2", "R2", "B1", "B2", "R3", "C1", "C2",
	"Q", "Q1", "Qf", "M", "Mp", "M1", "M1a", "M1b", "M2", "M2a", "M2b", "T",
}

func (s st) String() string { return stNames[s] }

type hist int

const (
	hb hist = iota
	hs
)

func (h hist) String() string { return [...]string{"Hb", "Hs"}[h] }

type target = EntryTarget[st, hist]

func to(state st) target { return StateTarget[st, hist](state) }

func toHistory(history hist) target { return HistoryTarget[st, hist](history) }

// ev names the events the transition table below answers; evNull is the
// eventless selection's "no event".
type ev int

const (
	evE ev = iota
	evF
	evG
	evH
	evK
	evR
	evNull
)

type tr struct {
	event    ev
	targets  []target
	internal bool
}

// The transitions, per source, in document order. Every one has content:
//
//	a1: E -> a2, F -> a2, H (targetless), K -> a2, R -> a1 (a self-transition)
//	b1: E -> b2, G -> b2, K -> b2, R -> b2
//	p:  F -> t,  G -> t,  H -> t,  K (targetless)
var chartTransitions = map[st][]tr{
	stA1: {
		{event: evE, targets: []target{to(stA2)}},
		{event: evF, targets: []target{to(stA2)}},
		{event: evH},
		{event: evK, targets: []target{to(stA2)}},
		{event: evR, targets: []target{to(stA1)}},
	},
	stB1: {
		{event: evE, targets: []target{to(stB2)}},
		{event: evG, targets: []target{to(stB2)}},
		{event: evK, targets: []target{to(stB2)}},
		{event: evR, targets: []target{to(stB2)}},
	},
	stP: {
		{event: evF, targets: []target{to(stT)}},
		{event: evG, targets: []target{to(stT)}},
		{event: evH, targets: []target{to(stT)}},
		{event: evK},
	},
}

var chartParent = map[st]st{
	stP: stS, stQ: stS, stM: stS,
	stR1: stP, stR2: stP, stR3: stP,
	stA1: stR1, stA2: stR1,
	stB1: stR2, stB2: stR2,
	stC1: stR3, stC2: stR3,
	stQ1: stQ, stQf: stQ,
	stMp: stM,
	stM1: stMp, stM2: stMp,
	stM1a: stM1, stM1b: stM1,
	stM2a: stM2, stM2b: stM2,
}

var chartChildren = map[st][]st{
	stS:  {stP, stQ, stM},
	stP:  {stR1, stR2, stR3},
	stR1: {stA1, stA2},
	stR2: {stB1, stB2},
	stR3: {stC1, stC2},
	stQ:  {stQ1, stQf},
	stM:  {stMp},
	stMp: {stM1, stM2},
	stM1: {stM1a, stM1b},
	stM2: {stM2a, stM2b},
}

var chartInitial = map[st][]target{
	stS:  {to(stP)},
	stR1: {to(stA1)},
	stR2: {to(stB1)},
	stR3: {to(stC1)},
	stQ:  {to(stQ1)},
	stM:  {to(stM1a), to(stM2b)},
	stM1: {to(stM1a)},
	stM2: {to(stM2a)},
}

var initialConfiguration = []st{stS, stP, stR1, stA1, stR2, stB1, stR3, stC1}

// chart is the document above, a configuration, and a record of what the
// microstep asked the machine to do.
type chart struct {
	recorded      map[hist][]st
	configuration []st
	log           []string
	exitsSaw      [][]st
}

func newChart() *chart { return &chart{recorded: map[hist][]st{}} }

func atInitialConfiguration() *chart {
	c := newChart()
	c.configuration = append([]st(nil), initialConfiguration...)
	return c
}

// Document

func (c *chart) ParentOf(state st) (st, bool) {
	parent, ok := chartParent[state]
	return parent, ok
}

func (c *chart) IsCompound(state st) bool {
	switch state {
	case stS, stR1, stR2, stR3, stQ, stM, stM1, stM2:
		return true
	}
	return false
}

func (c *chart) IsParallel(state st) bool { return state == stP || state == stMp }

func (c *chart) IsFinal(state st) bool { return state == stQf }

func (c *chart) ChildStates(state st) []st { return chartChildren[state] }

func (c *chart) InitialTargets(state st) []target { return chartInitial[state] }

func (c *chart) HistoryParent(history hist) st {
	if history == hb {
		return stR2
	}
	return stS
}

func (c *chart) HistoryValue(history hist) ([]st, bool) {
	recorded, ok := c.recorded[history]
	return recorded, ok
}

func (c *chart) HistoryDefaultTargets(history hist) []target {
	if history == hb {
		return []target{to(stB2)}
	}
	return []target{to(stA2), to(stB2)}
}

func (c *chart) DocumentOrder(state st) int { return int(state) }

// Run

func (c *chart) Configuration() []st { return append([]st(nil), c.configuration...) }

func (c *chart) FirstEnabledTransition(state st, event ev) (EnabledTransition[st, hist], bool) {
	for index, t := range chartTransitions[state] {
		if t.event == event {
			return EnabledTransition[st, hist]{
				Source:          state,
				Targets:         t.targets,
				TransitionIndex: index,
				HasActions:      true,
				IsInternal:      t.internal,
			}, true
		}
	}
	return EnabledTransition[st, hist]{}, false
}

func (c *chart) ExitState(state st, configurationBeforeExit []st) {
	c.exitsSaw = append(c.exitsSaw, append([]st(nil), configurationBeforeExit...))
	kept := c.configuration[:0]
	for _, s := range c.configuration {
		if s != state {
			kept = append(kept, s)
		}
	}
	c.configuration = kept
	c.log = append(c.log, fmt.Sprintf("exit %v", state))
}

func (c *chart) ExecuteTransitionContent(transition EnabledTransition[st, hist]) {
	c.log = append(c.log, fmt.Sprintf("content %v#%d", transition.Source, transition.TransitionIndex))
}

func (c *chart) EnterState(state st, isDefaultEntry bool) {
	c.configuration = append(c.configuration, state)
	how := ""
	if isDefaultEntry {
		how = " (default)"
	}
	c.log = append(c.log, fmt.Sprintf("enter %v%s", state, how))
}

func (c *chart) ExecuteHistoryDefaultContent(history hist) {
	c.log = append(c.log, fmt.Sprintf("history default %v", history))
}

func entering(source st, targets ...target) EntryTransition[st, hist] {
	return EntryTransition[st, hist]{Source: source, HasSource: true, Targets: targets}
}

func enteringInternal(source st, targets ...target) EntryTransition[st, hist] {
	t := entering(source, targets...)
	t.IsInternal = true
	return t
}

func fromDocument(targets ...target) EntryTransition[st, hist] {
	return EntryTransition[st, hist]{Targets: targets}
}

func enabled(source st, index int) EnabledTransition[st, hist] {
	t := chartTransitions[source][index]
	return EnabledTransition[st, hist]{
		Source:          source,
		Targets:         t.targets,
		TransitionIndex: index,
		HasActions:      true,
		IsInternal:      t.internal,
	}
}

type selected struct {
	source st
	index  int
}

func sources(transitions []EnabledTransition[st, hist]) []selected {
	picked := []selected{}
	for _, t := range transitions {
		picked = append(picked, selected{t.Source, t.TransitionIndex})
	}
	return picked
}

func sortedStates(states []st) []st {
	out := append([]st(nil), states...)
	sort.Slice(out, func(i, j int) bool { return out[i] < out[j] })
	return out
}

func expectStates(t *testing.T, what string, got, want []st) {
	t.Helper()
	if len(got) == 0 && len(want) == 0 {
		return
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("%s: got %v, want %v", what, got, want)
	}
}

func expectLog(t *testing.T, got, want []string) {
	t.Helper()
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("log:\n got  %q\n want %q", got, want)
	}
}

func expectSelected(t *testing.T, got []EnabledTransition[st, hist], want ...selected) {
	t.Helper()
	if len(want) == 0 {
		want = []selected{}
	}
	if picked := sources(got); !reflect.DeepEqual(picked, want) {
		t.Fatalf("selected %v, want %v", picked, want)
	}
}

// ════════════════════════════════════════════════════════════════════════
// computeEntrySet
// ════════════════════════════════════════════════════════════════════════

func TestTheInitialConfigurationEntersEveryRegionByDefault(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{fromDocument(to(stS))})
	expectStates(t, "states to enter", entry.StatesToEnter, initialConfiguration)
	// A <parallel> is not compound, so it has no initial transition to run.
	expectStates(t, "default entry", sortedStates(entry.StatesForDefaultEntry), []st{stS, stR1, stR2, stR3})
	if len(entry.DefaultHistoryContent) != 0 {
		t.Fatalf("no history was taken, but content is owed: %v", entry.DefaultHistoryContent)
	}
}

func TestATargetSetEntersEveryTargetAndDefaultsTheRegionNoTargetReaches(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stT, to(stA2), to(stB2))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stS, stP, stR1, stA2, stR2, stB2, stR3, stC1})
	// s, r1 and r2 are entered as ANCESTORS of a named target, so their initial
	// transitions do not run; only r3, reached by nobody, defaults.
	expectStates(t, "default entry", entry.StatesForDefaultEntry, []st{stR3})
	if entry.IsDefaultEntry(stS) || !entry.IsDefaultEntry(stR3) {
		t.Fatalf("IsDefaultEntry disagrees with StatesForDefaultEntry %v", entry.StatesForDefaultEntry)
	}
}

func TestAnUnrecordedShallowHistoryTakesItsDefaultAndOwesItsContentToItsParent(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stA1, toHistory(hb))})
	// The domain is s: the history stands for b2, and the least compound
	// ancestor of a1 and b2 walks past the <parallel>.
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stP, stR1, stA1, stR2, stB2, stR3, stC1})
	expectStates(t, "default entry", sortedStates(entry.StatesForDefaultEntry), []st{stR1, stR3})
	if history, owed := entry.DefaultHistoryContentOf(stR2); !owed || history != hb {
		t.Fatalf("r2 must owe hb's default content, got (%v, %v)", history, owed)
	}
	if _, owed := entry.DefaultHistoryContentOf(stP); owed {
		t.Fatalf("p owes no history content")
	}
}

func TestARecordedShallowHistoryRestoresWhatItRecordedAndOwesNoContent(t *testing.T) {
	c := newChart()
	c.recorded[hb] = []st{stB1}
	entry := ComputeEntrySet[st, hist](c, []EntryTransition[st, hist]{entering(stA1, toHistory(hb))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stP, stR1, stA1, stR2, stB1, stR3, stC1})
	if len(entry.DefaultHistoryContent) != 0 {
		t.Fatalf("a recorded history owes no default content: %v", entry.DefaultHistoryContent)
	}
}

func TestADeepHistoryThatRecordedEveryRegionRestoresAllOfThem(t *testing.T) {
	c := newChart()
	c.recorded[hs] = []st{stA2, stB1, stC2}
	entry := ComputeEntrySet[st, hist](c, []EntryTransition[st, hist]{entering(stT, toHistory(hs))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stS, stP, stR1, stA2, stR2, stB1, stR3, stC2})
	// Nothing is entered by default: every compound state on the way is an
	// ancestor of a recorded state.
	expectStates(t, "default entry", entry.StatesForDefaultEntry, nil)
	if len(entry.DefaultHistoryContent) != 0 {
		t.Fatalf("a recorded history owes no default content: %v", entry.DefaultHistoryContent)
	}
}

func TestAnUnrecordedDeepHistoryWithAMultiStateDefaultEntersTheWholeSet(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stT, toHistory(hs))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stS, stP, stR1, stA2, stR2, stB2, stR3, stC1})
	expectStates(t, "default entry", entry.StatesForDefaultEntry, []st{stR3})
	if history, owed := entry.DefaultHistoryContentOf(stS); !owed || history != hs {
		t.Fatalf("s must owe hs's default content, got (%v, %v)", history, owed)
	}
}

func TestAnInternalTransitionToADescendantLeavesItsSourceOutOfTheSet(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{enteringInternal(stS, to(stQ1))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stQ, stQ1})
	expectStates(t, "default entry", entry.StatesForDefaultEntry, nil)
}

func TestAnExternalTransitionToADescendantReentersItsSource(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stS, to(stQ1))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stS, stQ, stQ1})
	expectStates(t, "default entry", entry.StatesForDefaultEntry, nil)
}

func TestAnExternalTransitionOnARegionRootReentersEverySiblingRegion(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stR1, to(stA2))})
	// A <parallel> is never a domain, so the domain is s, and the sibling
	// regions are exited and entered again at their defaults.
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stP, stR1, stA2, stR2, stB1, stR3, stC1})
	expectStates(t, "default entry", sortedStates(entry.StatesForDefaultEntry), []st{stR2, stR3})
}

func TestAnInternalTransitionOnARegionRootStaysInsideItsRegion(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{enteringInternal(stR1, to(stA2))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stA2})
}

func TestTransitionsOfOneMicrostepEnterInDocumentOrder(t *testing.T) {
	// Selected second-region first, as a set is free to be; entry order is
	// document order all the same.
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{
		entering(stB1, to(stB2)),
		entering(stA1, to(stA2)),
	})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stA2, stB2})
}

func TestADeepMultiTargetInitialDefaultsOnlyTheStateThatNamesIt(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stT, to(stM))})
	expectStates(t, "states to enter", entry.StatesToEnter, []st{stS, stM, stMp, stM1, stM1a, stM2, stM2b})
	// m is the target and defaults; m1 and m2 are ancestors of its initial
	// targets, and s an ancestor of the target itself.
	expectStates(t, "default entry", entry.StatesForDefaultEntry, []st{stM})
}

func TestATargetlessTransitionEntersNothing(t *testing.T) {
	entry := ComputeEntrySet[st, hist](newChart(), []EntryTransition[st, hist]{entering(stA1)})
	expectStates(t, "states to enter", entry.StatesToEnter, nil)
	expectStates(t, "default entry", entry.StatesForDefaultEntry, nil)
}

func TestTheDomainIsAskedOfTheStatesAHistoryStandsFor(t *testing.T) {
	c := newChart()
	// Unrecorded, hs stands for "a2 b2", whose least compound ancestor with t
	// is the document itself.
	unrecorded := EffectiveTargetStates[st, hist](c, []target{toHistory(hs)})
	expectStates(t, "unrecorded", unrecorded, []st{stA2, stB2})
	if domain, ok := TransitionDomain[st, hist](c, entering(stT), unrecorded); ok {
		t.Fatalf("the domain of t -> hs is the <scxml> element, got %v", domain)
	}

	// Recorded inside q, an internal transition on q keeps q as its domain.
	c.recorded[hs] = []st{stQ1}
	recorded := EffectiveTargetStates[st, hist](c, []target{toHistory(hs)})
	expectStates(t, "recorded", recorded, []st{stQ1})
	if domain, ok := TransitionDomain[st, hist](c, enteringInternal(stQ), recorded); !ok || domain != stQ {
		t.Fatalf("the domain of an internal q -> hs is q, got (%v, %v)", domain, ok)
	}
}

// ════════════════════════════════════════════════════════════════════════
// isDescendant
// ════════════════════════════════════════════════════════════════════════

func TestAChildAndAGrandchildAreDescendants(t *testing.T) {
	c := newChart()
	for _, pair := range [][2]st{{stA1, stR1}, {stA1, stP}, {stM2b, stS}} {
		if !IsDescendant[st, hist](c, pair[0], pair[1]) {
			t.Fatalf("%v lies below %v", pair[0], pair[1])
		}
	}
}

func TestAStateIsNotItsOwnDescendant(t *testing.T) {
	// Appendix D's isDescendant is strict.
	if IsDescendant[st, hist](newChart(), stR1, stR1) {
		t.Fatalf("r1 is not its own descendant")
	}
}

func TestStatesOnUnrelatedBranchesAreNotDescendants(t *testing.T) {
	c := newChart()
	for _, pair := range [][2]st{{stA1, stR2}, {stQ1, stP}, {stT, stS}} {
		if IsDescendant[st, hist](c, pair[0], pair[1]) {
			t.Fatalf("%v does not lie below %v", pair[0], pair[1])
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// Domains and exit sets
// ════════════════════════════════════════════════════════════════════════

func TestASelfTransitionExitsOnlyItsSource(t *testing.T) {
	c := atInitialConfiguration()
	// §scxml-D-findLCCA chooses among the PROPER ancestors, so the domain of
	// a1 -> a1 is r1, and the exit set is a1 alone. A source that were its own
	// domain would exit nothing, and the self-transition would not leave and
	// re-enter its state at all.
	expectStates(t, "exit set", ComputeExitSet[st, hist](c, enabled(stA1, 4), c.configuration), []st{stA1})
}

func TestAnExternalTransitionLeavingAParallelExitsEveryRegion(t *testing.T) {
	c := atInitialConfiguration()
	// p -> t: the domain is the <scxml> element, so the whole configuration
	// goes — the sibling regions of p included, which no walk up from the
	// source alone could name.
	exitSet := ComputeExitSet[st, hist](c, enabled(stP, 0), c.configuration)
	expectStates(t, "exit set", sortedStates(exitSet), sortedStates(c.configuration))
}

func TestAnInternalTransitionFromARegionRootExitsOnlyInsideTheRegion(t *testing.T) {
	c := atInitialConfiguration()
	internal := EnabledTransition[st, hist]{Source: stR1, Targets: []target{to(stA2)}, IsInternal: true}
	external := internal
	external.IsInternal = false
	expectStates(t, "internal exit set", ComputeExitSet[st, hist](c, internal, c.configuration), []st{stA1})
	// Written external, the same transition's domain is s — a <parallel> is
	// never a domain — so every region goes with it.
	expectStates(t, "external exit set",
		sortedStates(ComputeExitSet[st, hist](c, external, c.configuration)),
		[]st{stP, stR1, stA1, stR2, stB1, stR3, stC1})
}

func TestATargetlessTransitionExitsNothing(t *testing.T) {
	c := atInitialConfiguration()
	expectStates(t, "exit set", ComputeExitSet[st, hist](c, enabled(stA1, 2), c.configuration), nil)
}

func TestTheStatesToExitComeOutInReverseDocumentOrder(t *testing.T) {
	c := atInitialConfiguration()
	expectStates(t, "states to exit",
		ComputeStatesToExit[st, hist](c, []EnabledTransition[st, hist]{enabled(stP, 0)}, c.configuration),
		[]st{stC1, stR3, stB1, stR2, stA1, stR1, stP, stS})
}

// ════════════════════════════════════════════════════════════════════════
// selectTransitions / removeConflictingTransitions
// ════════════════════════════════════════════════════════════════════════

func TestEveryRegionTakesItsOwnTransition(t *testing.T) {
	// §scxml-3.4: two regions' transitions have domains in disjoint subtrees,
	// so their exit sets are disjoint and both survive.
	c := atInitialConfiguration()
	expectSelected(t, SelectTransitions[st, hist, ev](c, evE), selected{stA1, 0}, selected{stB1, 0})
}

func TestATransitionSelectedFirstPreemptsALaterOneThatIsNotItsDescendant(t *testing.T) {
	// a1 selects a1 -> a2; b1 walks up to p -> t, which exits a1 too, and p
	// does not descend from a1: it is preempted.
	c := atInitialConfiguration()
	expectSelected(t, SelectTransitions[st, hist, ev](c, evF), selected{stA1, 1})
}

func TestADescendantSourcePreemptsAnAncestorSelectedBeforeIt(t *testing.T) {
	// a1 walks up to p -> t first; b1 then selects b1 -> b2, which conflicts
	// with it and descends from p, so it wins. c1 reaches p -> t again — one
	// transition, already in the set.
	c := atInitialConfiguration()
	expectSelected(t, SelectTransitions[st, hist, ev](c, evG), selected{stB1, 1})
}

func TestATargetlessTransitionIsNeverPreempted(t *testing.T) {
	// W3C test 403c: an empty exit set conflicts with nothing, so a1's
	// targetless transition survives the p -> t that exits a1 itself.
	c := atInitialConfiguration()
	expectSelected(t, SelectTransitions[st, hist, ev](c, evH), selected{stA1, 2}, selected{stP, 2})
}

func TestATransitionReachedFromTwoRegionsIsSelectedOnce(t *testing.T) {
	// In the initial configuration only c1 walks up to p's targetless K — a1
	// and b1 answer K themselves. Move the first two regions to a2 and b2,
	// which answer nothing, and all three atomic states reach it: it is still
	// one element of the set.
	c := atInitialConfiguration()
	expectSelected(t, SelectTransitions[st, hist, ev](c, evK), selected{stA1, 3}, selected{stB1, 2}, selected{stP, 3})
	c.configuration = []st{stS, stP, stR1, stA2, stR2, stB2, stR3, stC1}
	expectSelected(t, SelectTransitions[st, hist, ev](c, evK), selected{stP, 3})
}

func TestAnEventlessSelectionWithNothingEnabledSelectsNothing(t *testing.T) {
	expectSelected(t, SelectTransitions[st, hist, ev](atInitialConfiguration(), evNull))
}

// ════════════════════════════════════════════════════════════════════════
// The microstep
// ════════════════════════════════════════════════════════════════════════

func TestAMicrostepExitsThenRunsContentInSelectionOrderThenEnters(t *testing.T) {
	c := atInitialConfiguration()
	Microstep[st, hist, ev](c, SelectTransitions[st, hist, ev](c, evK))
	// §scxml-D-executeTransitionContent runs content in SELECTION order: p's K
	// was reached last, from c1, though p comes first in document order.
	expectLog(t, c.log, []string{
		"exit B1",
		"exit A1",
		"content A1#3",
		"content B1#2",
		"content P#3",
		"enter A2",
		"enter B2",
	})
}

func TestEveryExitIsHandedTheConfigurationBeforeTheFirstExit(t *testing.T) {
	c := atInitialConfiguration()
	before := append([]st(nil), c.configuration...)
	// K exits two states, b1 then a1: the second exit must still be handed the
	// configuration from before the first — a microstep with one exit could
	// not tell that from the configuration as it stands at each exit.
	Microstep[st, hist, ev](c, SelectTransitions[st, hist, ev](c, evK))
	if want := [][]st{before, before}; !reflect.DeepEqual(c.exitsSaw, want) {
		t.Fatalf("exits saw %v, want %v", c.exitsSaw, want)
	}
}

func TestASelfTransitionLeavesAndReentersItsState(t *testing.T) {
	c := atInitialConfiguration()
	taken := SelectTransitions[st, hist, ev](c, evR)
	expectSelected(t, taken, selected{stA1, 4}, selected{stB1, 3})
	Microstep[st, hist, ev](c, taken)
	expectLog(t, c.log, []string{
		"exit B1",
		"exit A1",
		"content A1#4",
		"content B1#3",
		"enter A1",
		"enter B2",
	})
}

func TestAHistoryDefaultContentRunsAfterItsParentIsEntered(t *testing.T) {
	c := atInitialConfiguration()
	toHb := EnabledTransition[st, hist]{Source: stA1, Targets: []target{toHistory(hb)}}
	Microstep[st, hist, ev](c, []EnabledTransition[st, hist]{toHb})
	// The domain is s, so all of p leaves and comes back; r2 is an ancestor of
	// the default target b2, not entered by default, and the history's default
	// content runs once r2 has been entered. The transition itself has no
	// content, so the machine is not asked to run any.
	expectLog(t, c.log, []string{
		"exit C1",
		"exit R3",
		"exit B1",
		"exit R2",
		"exit A1",
		"exit R1",
		"exit P",
		"enter P",
		"enter R1 (default)",
		"enter A1",
		"enter R2",
		"history default Hb",
		"enter B2",
		"enter R3 (default)",
		"enter C1",
	})
}

// ════════════════════════════════════════════════════════════════════════
// isInFinalState
// ════════════════════════════════════════════════════════════════════════
//
//	run (parallel)
//	  form > filling, formDone (final)
//	  checks (parallel)
//	    left  > leftPending,  leftDone (final)
//	    right > rightPending, rightDone (final)
//
// checks is a <parallel> among the regions of a <parallel>: it has no <final>
// child of its own and is final only because both of its regions are.

type cs int

const (
	csRun cs = iota
	csForm
	csFilling
	csFormDone
	csChecks
	csLeft
	csLeftPending
	csLeftDone
	csRight
	csRightPending
	csRightDone
)

var completionParent = map[cs]cs{
	csForm: csRun, csChecks: csRun,
	csFilling: csForm, csFormDone: csForm,
	csLeft: csChecks, csRight: csChecks,
	csLeftPending: csLeft, csLeftDone: csLeft,
	csRightPending: csRight, csRightDone: csRight,
}

var completionChildren = map[cs][]cs{
	csRun:    {csForm, csChecks},
	csForm:   {csFilling, csFormDone},
	csChecks: {csLeft, csRight},
	csLeft:   {csLeftPending, csLeftDone},
	csRight:  {csRightPending, csRightDone},
}

// completion is the document above; it declares no <history>.
type completion struct{}

func (completion) ParentOf(state cs) (cs, bool) {
	parent, ok := completionParent[state]
	return parent, ok
}

func (completion) IsCompound(state cs) bool {
	return state == csForm || state == csLeft || state == csRight
}

func (completion) IsParallel(state cs) bool { return state == csRun || state == csChecks }

func (completion) IsFinal(state cs) bool {
	return state == csFormDone || state == csLeftDone || state == csRightDone
}

func (completion) ChildStates(state cs) []cs { return completionChildren[state] }

func (completion) InitialTargets(cs) []EntryTarget[cs, HistoryID] { return nil }

func (completion) HistoryParent(HistoryID) cs {
	panic("the completion document declares no <history>")
}

func (completion) HistoryValue(HistoryID) ([]cs, bool) { return nil, false }

func (completion) HistoryDefaultTargets(HistoryID) []EntryTarget[cs, HistoryID] { return nil }

func (completion) DocumentOrder(state cs) int { return int(state) }

func TestACompoundStateIsFinalWhenAFinalChildIsActive(t *testing.T) {
	doc := completion{}
	if !IsInFinalState[cs, HistoryID](doc, csForm,
		[]cs{csRun, csForm, csFormDone, csChecks, csLeft, csLeftPending, csRight, csRightPending}) {
		t.Fatalf("form is in a final state while formDone is active")
	}
	if IsInFinalState[cs, HistoryID](doc, csForm,
		[]cs{csRun, csForm, csFilling, csChecks, csLeft, csLeftPending, csRight, csRightPending}) {
		t.Fatalf("form is not in a final state while filling is active")
	}
}

func TestANestedParallelIsFinalOnlyWhenEveryRegionIs(t *testing.T) {
	doc := completion{}
	if !IsInFinalState[cs, HistoryID](doc, csChecks, []cs{csChecks, csLeft, csLeftDone, csRight, csRightDone}) {
		t.Fatalf("checks is final once both of its regions are")
	}
	if IsInFinalState[cs, HistoryID](doc, csChecks, []cs{csChecks, csLeft, csLeftDone, csRight, csRightPending}) {
		t.Fatalf("one region still pending leaves the <parallel> short of final")
	}
}

func TestAParallelRegionOfAParallelCounts(t *testing.T) {
	doc := completion{}
	if !IsInFinalState[cs, HistoryID](doc, csRun,
		[]cs{csRun, csForm, csFormDone, csChecks, csLeft, csLeftDone, csRight, csRightDone}) {
		t.Fatalf("checks has no <final> child of its own; it is final because both of its regions are")
	}
	if IsInFinalState[cs, HistoryID](doc, csRun,
		[]cs{csRun, csForm, csFormDone, csChecks, csLeft, csLeftDone, csRight, csRightPending}) {
		t.Fatalf("run is not final while one region of checks is pending")
	}
}

func TestNeitherAnAtomicStateNorAFinalElementIsInAFinalState(t *testing.T) {
	doc := completion{}
	configuration := []cs{csRun, csForm, csFormDone, csChecks, csLeft, csLeftPending, csRight, csRightPending}
	if IsInFinalState[cs, HistoryID](doc, csLeftPending, configuration) {
		t.Fatalf("an atomic state has no <final> child to be active")
	}
	if IsInFinalState[cs, HistoryID](doc, csFormDone, configuration) {
		t.Fatalf("being a <final> element is not being IN a final state — the predicate asks about children")
	}
}
