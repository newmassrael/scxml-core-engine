// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"sort"
)

// W3C SCXML Appendix D's microstep — which transitions an event selects, which
// of them survive preemption, which states they exit and in which order, the
// order their content runs in, which states they enter — written once for
// every machine this runtime runs.
//
// The procedures are transcribed rather than paraphrased, one function per
// procedure and under the appendix's names, so a reader can hold this file
// against the specification line by line — and against the other engines'
// transcriptions of it: the C++ engines' sce/include/core/MicrostepAlgorithms.h,
// the Rust runtime's backends/rust/runtime/src/helpers/microstep.rs and the
// Python runtime's backends/python/runtime/sce_runtime/microstep.py, which this
// file follows function for function.
//
// What differs between machines is injected. A Document describes the
// document — its structure, and what each <history> recorded. A Run is the
// running machine: its configuration, which of a state's transitions an event
// enables, and what exiting a state, running a transition's content and
// entering a state DO. Nothing here knows how a machine stores either, so a
// table written by hand answers them as well as a generated policy does.
//
// A document whose <history> defaults name one another in a cycle has no entry
// set — dereferencing never reaches a state — and a hierarchy whose parent
// links cycle has no domain; both are generator defects. The procedures assume
// a legal document, and the parent walks below panic rather than spin when
// handed one that is not.

// MaxHierarchyDepth is the maximum supported state hierarchy depth. It bounds
// every walk up the parent links here, so a cyclic parent relationship — a
// generator defect — is reported rather than walked forever. Matches Rust
// MAX_HIERARCHY_DEPTH = 16.
//
// W3C SCXML has no normative depth limit; 16 covers every real-world document
// (typical: 1-5, complex: up to 10).
const MaxHierarchyDepth = 16

// HistoryID names one <history> pseudo-state of a generated document.
//
// A history is not a state — it is never in a configuration — so a generated
// machine numbers its histories apart from its states, and a target list names
// one through HistoryTarget. The number means nothing outside the policy that
// issued it.
type HistoryID int

// EntryTarget is one token of a target list, as the document wrote it.
//
// A transition's target, a state's initial transition and a <history>'s default
// transition all name either states or <history> pseudo-states. The two kinds
// are kept apart rather than folded into one identifier: the entry procedures
// dereference a history (to what it recorded, or to its default) and add a
// state. Build one with StateTarget or HistoryTarget.
type EntryTarget[S comparable, H comparable] struct {
	state     S
	history   H
	isHistory bool
}

// StateTarget is the target token that names a <state>, <parallel> or <final>.
func StateTarget[S comparable, H comparable](state S) EntryTarget[S, H] {
	return EntryTarget[S, H]{state: state}
}

// HistoryTarget is the target token that names a <history> pseudo-state.
func HistoryTarget[S comparable, H comparable](history H) EntryTarget[S, H] {
	return EntryTarget[S, H]{history: history, isHistory: true}
}

// State reports the state this token names, and false when it names a history.
func (t EntryTarget[S, H]) State() (S, bool) {
	return t.state, !t.isHistory
}

// History reports the history this token names, and false when it names a
// state.
func (t EntryTarget[S, H]) History() (H, bool) {
	return t.history, t.isHistory
}

// String spells the token for diagnostics.
func (t EntryTarget[S, H]) String() string {
	if t.isHistory {
		return fmt.Sprintf("history(%v)", t.history)
	}
	return fmt.Sprintf("state(%v)", t.state)
}

// EnabledTransition is a transition selection enabled: one member of Appendix
// D's enabledTransitions.
//
// What the microstep reads of it — its source, its target list as written,
// whether it is internal — plus the index its source state knows it by, so the
// machine that owns its executable content can run it. A transition with no
// targets exits and enters nothing and only runs its content.
//
// Targets is the document's own table: a generated machine hands out slices of
// its package-level tables, because a transition is a fact about the document,
// not about the run. Nothing here writes to it.
type EnabledTransition[S comparable, H comparable] struct {
	// Source is the state whose transition this is.
	Source S
	// Targets is the target list as written — empty for a targetless
	// transition.
	Targets []EntryTarget[S, H]
	// TransitionIndex is the position of this transition among its source's
	// own transitions.
	TransitionIndex int
	// HasActions reports whether the transition has executable content to run.
	HasActions bool
	// IsInternal reports whether the transition was written type="internal".
	IsInternal bool
}

// IsTargetless reports whether the transition has no targets.
// §scxml-D-computeExitSet guards the whole computation with `if t.target`: a
// transition without targets exits and enters nothing.
func (t EnabledTransition[S, H]) IsTargetless() bool {
	return len(t.Targets) == 0
}

// EntryTransition is the same transition as the entry procedures read it.
func (t EnabledTransition[S, H]) EntryTransition() EntryTransition[S, H] {
	return EntryTransition[S, H]{
		Source:     t.Source,
		HasSource:  true,
		Targets:    t.Targets,
		IsInternal: t.IsInternal,
	}
}

// EntryTransition is the part of a transition the entry procedures read.
//
// HasSource is false for the document's own initial transition, whose source is
// the <scxml> element — which has no state identifier here, and whose domain is
// the whole document. The zero value's HasSource is therefore the <scxml>
// element's transition, which is what a caller building the initial one wants.
type EntryTransition[S comparable, H comparable] struct {
	// Source is the transition's source state; meaningful only when HasSource.
	Source S
	// HasSource is false when the source is the <scxml> element.
	HasSource bool
	// Targets is the target list as written.
	Targets []EntryTarget[S, H]
	// IsInternal reports whether the transition was written type="internal".
	IsInternal bool
}

// EntrySet is what a microstep enters: the appendix's three out-parameters.
//
// StatesToEnter is already sorted into entry order, so a machine enters it
// front to back. StatesForDefaultEntry answers whether a state's initial
// transition content runs; DefaultHistoryContent whether, and whose, <history>
// default transition content runs after that state's own entry — keyed by the
// history's parent, as the appendix keys it.
type EntrySet[S comparable, H comparable] struct {
	// StatesToEnter holds the states to enter, in entry order.
	StatesToEnter []S
	// StatesForDefaultEntry holds the compound states whose initial state is
	// entered by default.
	StatesForDefaultEntry []S
	// DefaultHistoryContent maps a history's parent to the history whose
	// default content runs when that parent is entered.
	DefaultHistoryContent map[S]H
}

// Contains reports whether state is entered by this microstep.
func (e *EntrySet[S, H]) Contains(state S) bool {
	return containsState(e.StatesToEnter, state)
}

// IsDefaultEntry reports whether state's initial state is entered by default,
// so its initial transition's executable content runs.
func (e *EntrySet[S, H]) IsDefaultEntry(state S) bool {
	return containsState(e.StatesForDefaultEntry, state)
}

// DefaultHistoryContentOf reports the <history> whose default transition
// content runs after state is entered, if a history of state was taken with
// nothing recorded.
func (e *EntrySet[S, H]) DefaultHistoryContentOf(state S) (H, bool) {
	history, ok := e.DefaultHistoryContent[state]
	return history, ok
}

// Document is the document, as the entry and exit procedures read it.
//
// Structure only, plus the one piece of run-time state the entry procedures
// need: what each <history> recorded.
type Document[S comparable, H comparable] interface {
	// ParentOf reports the state's parent, and false when its parent is the
	// <scxml> element.
	ParentOf(state S) (S, bool)

	// IsCompound reports whether the state is a <state> with child states. A
	// <parallel> answers false: this is the appendix's isCompoundState, and
	// with the <scxml> element it is the set findLCCA chooses a domain from.
	IsCompound(state S) bool

	// IsParallel reports whether the state is a <parallel>.
	IsParallel(state S) bool

	// IsFinal reports whether the state is a <final> element — not whether it
	// is IN a final state; that is IsInFinalState.
	IsFinal(state S) bool

	// ChildStates is §scxml-D-getChildStates: the state's <state>, <parallel>
	// and <final> children, in document order.
	ChildStates(state S) []S

	// InitialTargets is a compound state's initial transition target, as
	// written — the first child state when the document names none.
	InitialTargets(state S) []EntryTarget[S, H]

	// HistoryParent is the state a <history> is declared in.
	HistoryParent(history H) S

	// HistoryValue reports what the history recorded when its parent was last
	// exited, and false before that ever happened.
	HistoryValue(history H) ([]S, bool)

	// HistoryDefaultTargets is a <history>'s default transition target, as
	// written.
	HistoryDefaultTargets(history H) []EntryTarget[S, H]

	// DocumentOrder is the state's position in document order, which is also
	// entry order.
	DocumentOrder(state S) int
}

// Run is the running machine, as the microstep drives it.
type Run[S comparable, H comparable, E any] interface {
	Document[S, H]

	// Configuration reports the active states, in any order. The procedures
	// hold the answer across the exits and entries they drive, so it must not
	// alias storage those mutate.
	Configuration() []S

	// FirstEnabledTransition reports this state's first transition, in
	// document order, that the event enables and whose guard holds; for the
	// machine's "no event", its first eventless transition whose guard holds.
	// The only place a guard is evaluated.
	FirstEnabledTransition(state S, event E) (EnabledTransition[S, H], bool)

	// ExitState records this state's histories from the configuration as it
	// stood before the microstep's first exit, runs its onexit, cancels its
	// invocations and removes it from the configuration.
	ExitState(state S, configurationBeforeExit []S)

	// ExecuteTransitionContent runs one transition's executable content.
	ExecuteTransitionContent(transition EnabledTransition[S, H])

	// EnterState adds the state to the configuration and schedules its
	// invocations, runs its onentry, then its initial transition's content when
	// isDefaultEntry; for a <final>, what the appendix does on entering one.
	EnterState(state S, isDefaultEntry bool)

	// ExecuteHistoryDefaultContent runs a <history>'s default transition
	// content.
	ExecuteHistoryDefaultContent(history H)
}

// ════════════════════════════════════════════════════════════════════════
// Selection
// ════════════════════════════════════════════════════════════════════════

// SelectTransitions is Appendix D's selectTransitions, or its
// selectEventlessTransitions when event is the machine's "no event".
//
// Returns the optimal enabled transition set, in selection order.
func SelectTransitions[S comparable, H comparable, E any](run Run[S, H, E], event E) []EnabledTransition[S, H] {
	configuration := run.Configuration()

	atomicStates := make([]S, 0, len(configuration))
	for _, state := range configuration {
		if !run.IsCompound(state) && !run.IsParallel(state) {
			atomicStates = append(atomicStates, state)
		}
	}
	sort.SliceStable(atomicStates, func(i, j int) bool {
		return run.DocumentOrder(atomicStates[i]) < run.DocumentOrder(atomicStates[j])
	})

	var enabled []EnabledTransition[S, H]
	for _, atomic := range atomicStates {
		// §scxml-D-selectTransitions: the atomic state first, then its proper
		// ancestors, and the first enabled transition in document order ends
		// the walk for this atomic state. The set is ORDERED and a set: two
		// atomic states under one ancestor both reach its transition, and it is
		// one transition, taken once. With the machine's "no event" this is
		// §scxml-D-selectEventlessTransitions, the same walk over transitions
		// that have no event.
		current, ok := atomic, true
		for depth := 0; ok; depth++ {
			if depth == MaxHierarchyDepth {
				panic(fmt.Sprintf("SelectTransitions: cyclic parent relationship detected walking from %v", atomic))
			}
			if found, enables := run.FirstEnabledTransition(current, event); enables {
				seen := false
				for _, t := range enabled {
					if t.Source == found.Source && t.TransitionIndex == found.TransitionIndex {
						seen = true
						break
					}
				}
				if !seen {
					enabled = append(enabled, found)
				}
				break
			}
			current, ok = run.ParentOf(current)
		}
	}
	return RemoveConflictingTransitions[S, H](run, enabled, configuration)
}

// RemoveConflictingTransitions is Appendix D's removeConflictingTransitions.
//
// Two transitions conflict when their exit sets intersect, and the one whose
// source is a descendant of the other's wins; otherwise the one selected first
// does.
func RemoveConflictingTransitions[S comparable, H comparable](
	doc Document[S, H],
	enabled []EnabledTransition[S, H],
	configuration []S,
) []EnabledTransition[S, H] {
	var filtered []EnabledTransition[S, H]
	for _, t1 := range enabled {
		t1Exit := ComputeExitSet[S, H](doc, t1, configuration)
		conflicts := func(t2 EnabledTransition[S, H]) bool {
			return intersects(t1Exit, ComputeExitSet[S, H](doc, t2, configuration))
		}

		// §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
		// transition it conflicts with whose source it does not descend from.
		// A transition that exits nothing — a targetless one — conflicts with
		// nothing and can never be preempted.
		preempted := false
		for _, t2 := range filtered {
			if conflicts(t2) && !IsDescendant[S, H](doc, t1.Source, t2.Source) {
				preempted = true
				break
			}
		}
		if preempted {
			continue
		}
		// Not preempted means every kept transition t1 conflicts with has a
		// source t1 descends from: the appendix removes exactly those.
		var kept []EnabledTransition[S, H]
		for _, t2 := range filtered {
			if !conflicts(t2) {
				kept = append(kept, t2)
			}
		}
		filtered = append(kept, t1)
	}
	return filtered
}

// ════════════════════════════════════════════════════════════════════════
// Domains and exit sets
// ════════════════════════════════════════════════════════════════════════

// EffectiveTargetStates is Appendix D's getEffectiveTargetStates, over a target
// list.
//
// Returns the states the list stands for, histories dereferenced — to what each
// recorded or, before its parent was ever exited, to its default — each state
// once and in the order first named. The domain, and so the exit set, is a
// question about these.
func EffectiveTargetStates[S comparable, H comparable](doc Document[S, H], targets []EntryTarget[S, H]) []S {
	var effective []S
	addEffectiveTargetStates(doc, targets, &effective)
	return effective
}

func addEffectiveTargetStates[S comparable, H comparable](doc Document[S, H], targets []EntryTarget[S, H], effective *[]S) {
	for _, target := range targets {
		if state, isState := target.State(); isState {
			addOnce(effective, state)
			continue
		}
		// §scxml-D-getEffectiveTargetStates: a history stands for the
		// configuration it recorded, and before its parent was ever exited for
		// the targets of its default transition — which may name histories
		// themselves, hence the recursion.
		history, _ := target.History()
		if recorded, ok := doc.HistoryValue(history); ok && len(recorded) > 0 {
			for _, state := range recorded {
				addOnce(effective, state)
			}
		} else {
			addEffectiveTargetStates(doc, doc.HistoryDefaultTargets(history), effective)
		}
	}
}

// TransitionDomain is Appendix D's getTransitionDomain, for a transition that
// has targets.
//
// effectiveTargets are the transition's EFFECTIVE targets: a history that
// recorded a state deep inside its parent is a target that deep. Reports the
// domain, and false when it is the <scxml> element — always so for the
// document's initial transition.
func TransitionDomain[S comparable, H comparable](
	doc Document[S, H],
	transition EntryTransition[S, H],
	effectiveTargets []S,
) (S, bool) {
	if !transition.HasSource {
		var scxml S
		return scxml, false
	}
	source := transition.Source

	// §scxml-D-getTransitionDomain: an internal transition whose targets all
	// lie below a compound source has the SOURCE as its domain, so the source
	// stays active and only its active descendants are exited. That is not the
	// same as exiting nothing: a transition rooted at one of those descendants
	// exits it too, and the appendix expects the two to be found in conflict.
	if transition.IsInternal && doc.IsCompound(source) && allDescend(doc, effectiveTargets, source) {
		return source, true
	}
	return findLCCA(doc, source, effectiveTargets)
}

// findLCCA is Appendix D's findLCCA over [head] + tail: the first proper
// ancestor of head that is a compound state — or the <scxml> element, reported
// as false — and contains every state of tail.
//
// Asked of the source and EVERY target at once: combining pairwise answers can
// only widen the domain.
func findLCCA[S comparable, H comparable](doc Document[S, H], head S, tail []S) (S, bool) {
	current := head
	for depth := 0; depth < MaxHierarchyDepth; depth++ {
		ancestor, ok := doc.ParentOf(current)
		if !ok {
			return ancestor, false
		}
		// §scxml-D-findLCCA filters the ancestors with
		// isCompoundStateOrScxmlElement: a <parallel> is never a domain.
		if doc.IsCompound(ancestor) && allDescend(doc, tail, ancestor) {
			return ancestor, true
		}
		current = ancestor
	}
	panic(fmt.Sprintf("findLCCA: cyclic parent relationship detected walking from %v", head))
}

// ComputeExitSet is Appendix D's computeExitSet, for one transition: the active
// proper descendants of its domain, in the configuration's own order.
func ComputeExitSet[S comparable, H comparable](
	doc Document[S, H],
	transition EnabledTransition[S, H],
	configuration []S,
) []S {
	// §scxml-D-computeExitSet: the appendix guards the whole computation with
	// `if t.target`, so a transition without one exits nothing at all and can
	// therefore never be preempted.
	if transition.IsTargetless() {
		return nil
	}

	targets := EffectiveTargetStates(doc, transition.Targets)
	domain, hasDomain := TransitionDomain(doc, transition.EntryTransition(), targets)
	var exitSet []S
	for _, state := range configuration {
		// The domain itself is not exited; everything active below it is. When
		// the domain is the <scxml> element every active state is a descendant
		// of it — the sibling regions of an enclosing <parallel> included.
		if !hasDomain || IsDescendant(doc, state, domain) {
			exitSet = append(exitSet, state)
		}
	}
	return exitSet
}

// ComputeStatesToExit is Appendix D's computeExitSet over the microstep's
// transitions, in exitOrder — the states exitStates exits.
func ComputeStatesToExit[S comparable, H comparable](
	doc Document[S, H],
	transitions []EnabledTransition[S, H],
	configuration []S,
) []S {
	var statesToExit []S
	// §scxml-D-computeExitSet takes the microstep's whole transition list and
	// unions the exit sets; §scxml-D-exitStates exits that union in exitOrder —
	// descendants before their ancestors and reverse document order among the
	// rest, which together are exactly reverse document order.
	for _, transition := range transitions {
		for _, state := range ComputeExitSet(doc, transition, configuration) {
			addOnce(&statesToExit, state)
		}
	}
	sort.SliceStable(statesToExit, func(i, j int) bool {
		return doc.DocumentOrder(statesToExit[i]) > doc.DocumentOrder(statesToExit[j])
	})
	return statesToExit
}

// ════════════════════════════════════════════════════════════════════════
// The microstep
// ════════════════════════════════════════════════════════════════════════

// Microstep is Appendix D's microstep: exit, run the transitions' content,
// enter.
//
// Returns what was entered — the entry set, StatesToEnter in entry order.
func Microstep[S comparable, H comparable, E any](run Run[S, H, E], transitions []EnabledTransition[S, H]) EntrySet[S, H] {
	// §scxml-D-microstepProcedure: every exit, then every transition's content,
	// then every entry — for the whole set at once, which is what lets the
	// regions of a <parallel> each take their own transition in one step.
	ExitStates(run, transitions)
	ExecuteTransitionContent(run, transitions)
	entering := make([]EntryTransition[S, H], 0, len(transitions))
	for _, transition := range transitions {
		entering = append(entering, transition.EntryTransition())
	}
	return EnterStates(run, entering)
}

// ExitStates is Appendix D's exitStates.
func ExitStates[S comparable, H comparable, E any](run Run[S, H, E], transitions []EnabledTransition[S, H]) {
	// §scxml-D-exitStates: the union of the transitions' exit sets, exited in
	// exitOrder. Every history is recorded from the configuration as it stood
	// BEFORE the first exit, which is the snapshot each exit is handed.
	configuration := run.Configuration()
	for _, state := range ComputeStatesToExit[S, H](run, transitions, configuration) {
		run.ExitState(state, configuration)
	}
}

// ExecuteTransitionContent is Appendix D's executeTransitionContent.
func ExecuteTransitionContent[S comparable, H comparable, E any](run Run[S, H, E], transitions []EnabledTransition[S, H]) {
	// §scxml-D-executeTransitionContent: in the order the transitions were
	// selected, which is not their sources' document order once an ancestor's
	// transition is reached from a later region.
	for _, transition := range transitions {
		if transition.HasActions {
			run.ExecuteTransitionContent(transition)
		}
	}
}

// EnterStates is Appendix D's enterStates.
//
// Also the whole of the appendix's entry into the initial configuration
// (§scxml-D-interpret): hand it the document's initial transition, whose source
// is the <scxml> element (HasSource false).
func EnterStates[S comparable, H comparable, E any](run Run[S, H, E], transitions []EntryTransition[S, H]) EntrySet[S, H] {
	entry := ComputeEntrySet[S, H](run, transitions)
	for _, state := range entry.StatesToEnter {
		// §scxml-D-enterStates: onentry, then the initial transition's content
		// if and only if this state's initial state is being entered by
		// default, then a history's default content owed to it.
		run.EnterState(state, entry.IsDefaultEntry(state))
		if history, owed := entry.DefaultHistoryContentOf(state); owed {
			run.ExecuteHistoryDefaultContent(history)
		}
	}
	return entry
}

// ════════════════════════════════════════════════════════════════════════
// The entry set
// ════════════════════════════════════════════════════════════════════════

// ComputeEntrySet is Appendix D's computeEntrySet.
//
// transitions are the microstep's, in the order they were selected; a
// transition without targets enters nothing. Returns the entry set,
// StatesToEnter sorted into entry order.
func ComputeEntrySet[S comparable, H comparable](doc Document[S, H], transitions []EntryTransition[S, H]) EntrySet[S, H] {
	var entry EntrySet[S, H]
	for _, transition := range transitions {
		// §scxml-D-computeEntrySet: first every target with its default
		// descendants, then the ancestors that are entered inside the domain —
		// ancestors outside it were never exited. The order matters for a
		// target SET: the regions of a <parallel> that another target already
		// descends into must be seen as taken before the ancestor walk fills
		// the rest with defaults.
		for _, target := range transition.Targets {
			AddDescendantStatesToEnter(doc, target, &entry)
		}
		effective := EffectiveTargetStates(doc, transition.Targets)
		if len(effective) == 0 {
			continue
		}
		domain, hasDomain := TransitionDomain(doc, transition, effective)
		for _, state := range effective {
			AddAncestorStatesToEnter(doc, StateTarget[S, H](state), domain, hasDomain, &entry)
		}
	}

	// §scxml-D-enterStates: states are entered in entryOrder — ancestors before
	// descendants, document order between the rest, which is exactly document
	// order, because that is a pre-order walk.
	sort.SliceStable(entry.StatesToEnter, func(i, j int) bool {
		return doc.DocumentOrder(entry.StatesToEnter[i]) < doc.DocumentOrder(entry.StatesToEnter[j])
	})
	return entry
}

// AddDescendantStatesToEnter is Appendix D's addDescendantStatesToEnter.
//
// Adds target and every descendant entering it enters: a history's recorded or
// default configuration, a compound state's initial state(s), every region of a
// <parallel> not already on the set.
func AddDescendantStatesToEnter[S comparable, H comparable](doc Document[S, H], target EntryTarget[S, H], entry *EntrySet[S, H]) {
	if history, isHistory := target.History(); isHistory {
		parent := doc.HistoryParent(history)
		// §scxml-3.10: a transition to a history behaves as a transition to the
		// configuration it stored, or — before its parent was ever visited — to
		// its default stored configuration.
		if recorded, ok := doc.HistoryValue(history); ok && len(recorded) > 0 {
			// §scxml-D-addDescendantStatesToEnter: a history that has recorded a
			// configuration enters it, and the ancestors between it and the
			// history's parent — a deep history records atomic states, which
			// need not be children.
			for _, state := range recorded {
				AddDescendantStatesToEnter(doc, StateTarget[S, H](state), entry)
			}
			for _, state := range recorded {
				AddAncestorStatesToEnter(doc, StateTarget[S, H](state), parent, true, entry)
			}
			return
		}
		// §scxml-D-addDescendantStatesToEnter: nothing recorded yet, so the
		// default transition is taken, and its content is owed once the parent
		// has been entered — keyed by the parent, a later history of the same
		// parent replacing it.
		if entry.DefaultHistoryContent == nil {
			entry.DefaultHistoryContent = make(map[S]H)
		}
		entry.DefaultHistoryContent[parent] = history
		defaults := doc.HistoryDefaultTargets(history)
		for _, def := range defaults {
			AddDescendantStatesToEnter(doc, def, entry)
		}
		for _, def := range defaults {
			AddAncestorStatesToEnter(doc, def, parent, true, entry)
		}
		return
	}

	state, _ := target.State()
	addOnce(&entry.StatesToEnter, state)
	if doc.IsCompound(state) {
		// §scxml-D-addDescendantStatesToEnter: a compound state entered as a
		// target is entered by DEFAULT — the one condition under which its
		// initial transition's content runs. Its initial transition names the
		// child or children it enters (§scxml-3.3), possibly several and
		// possibly deep (§scxml-3.6).
		addOnce(&entry.StatesForDefaultEntry, state)
		initial := doc.InitialTargets(state)
		for _, child := range initial {
			AddDescendantStatesToEnter(doc, child, entry)
		}
		for _, child := range initial {
			AddAncestorStatesToEnter(doc, child, state, true, entry)
		}
	} else if doc.IsParallel(state) {
		addRegionDefaults(doc, state, entry)
	}
}

// AddAncestorStatesToEnter is Appendix D's addAncestorStatesToEnter.
//
// Adds the proper ancestors of target up to, not including, ancestor
// (hasAncestor false: up to the <scxml> element), filling in the regions of
// every <parallel> among them.
func AddAncestorStatesToEnter[S comparable, H comparable](
	doc Document[S, H],
	target EntryTarget[S, H],
	ancestor S,
	hasAncestor bool,
	entry *EntrySet[S, H],
) {
	for _, anc := range properAncestors(doc, target, ancestor, hasAncestor) {
		// §scxml-D-addAncestorStatesToEnter: an ancestor is entered WITHOUT its
		// default initial state — the set already holds the descendant it leads
		// to. A <parallel> still gives its other regions their defaults,
		// because all of them are entered.
		addOnce(&entry.StatesToEnter, anc)
		if doc.IsParallel(anc) {
			addRegionDefaults(doc, anc, entry)
		}
	}
}

// properAncestors is Appendix D's getProperAncestors: the ancestors of target
// in ancestry order up to, not including, ancestor — and nothing at all when
// ancestor is not above it.
//
// A <history> sits in its parent, so the parent is its first proper ancestor.
// hasAncestor false is the <scxml> element, which is above every state and is
// never entered, so it is not in the list.
func properAncestors[S comparable, H comparable](doc Document[S, H], target EntryTarget[S, H], ancestor S, hasAncestor bool) []S {
	var chain []S
	var current S
	var ok bool
	if history, isHistory := target.History(); isHistory {
		current, ok = doc.HistoryParent(history), true
	} else {
		state, _ := target.State()
		current, ok = doc.ParentOf(state)
	}
	for depth := 0; depth < MaxHierarchyDepth; depth++ {
		if !ok {
			// §scxml-D-getProperAncestors: ancestor is the state's parent, the
			// state itself, or one of its descendants — none of which the walk
			// can meet — so the answer is the empty set.
			if hasAncestor {
				return nil
			}
			return chain
		}
		if hasAncestor && current == ancestor {
			return chain
		}
		chain = append(chain, current)
		current, ok = doc.ParentOf(current)
	}
	panic(fmt.Sprintf("properAncestors: cyclic parent relationship detected walking from %v", target))
}

func addRegionDefaults[S comparable, H comparable](doc Document[S, H], parallel S, entry *EntrySet[S, H]) {
	// §scxml-3.4: every child of an active <parallel> is active, so a region
	// nothing on the set descends into is entered by default.
	for _, child := range doc.ChildStates(parallel) {
		taken := false
		for _, state := range entry.StatesToEnter {
			if IsDescendant(doc, state, child) {
				taken = true
				break
			}
		}
		if !taken {
			AddDescendantStatesToEnter(doc, StateTarget[S, H](child), entry)
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// History
// ════════════════════════════════════════════════════════════════════════

// RecordedHistory is Appendix D's exitStates, its history half: what a
// <history> of parent records as parent is exited — for a deep history the
// active atomic states below parent, for a shallow one its active children.
//
// Read off configurationBeforeExit, the configuration as it stood before the
// microstep's first exit: the appendix records every history before it exits
// any state, so every history of one microstep reads the same configuration.
// Returned in that configuration's order.
func RecordedHistory[S comparable, H comparable](doc Document[S, H], parent S, deep bool, configurationBeforeExit []S) []S {
	var recorded []S
	for _, state := range configurationBeforeExit {
		var records bool
		if deep {
			// §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`.
			records = !doc.IsCompound(state) && !doc.IsParallel(state) && IsDescendant(doc, state, parent)
		} else {
			// §scxml-D-exitStates: `s0.parent == s`.
			stateParent, ok := doc.ParentOf(state)
			records = ok && stateParent == parent
		}
		if records {
			recorded = append(recorded, state)
		}
	}
	return recorded
}

// ════════════════════════════════════════════════════════════════════════
// Predicates
// ════════════════════════════════════════════════════════════════════════

// IsInFinalState is Appendix D's isInFinalState: a compound state is in a final
// state when one of its <final> children is active; a <parallel> when EVERY one
// of its child states is — asked recursively, so a region that is itself a
// <parallel> counts only once all of ITS regions do. Nothing else ever is.
func IsInFinalState[S comparable, H comparable](doc Document[S, H], state S, configuration []S) bool {
	if doc.IsCompound(state) {
		for _, child := range doc.ChildStates(state) {
			if doc.IsFinal(child) && containsState(configuration, child) {
				return true
			}
		}
		return false
	}
	if doc.IsParallel(state) {
		for _, child := range doc.ChildStates(state) {
			if !IsInFinalState(doc, child, configuration) {
				return false
			}
		}
		return true
	}
	return false
}

// IsDescendant is Appendix D's isDescendant: whether state lies strictly below
// ancestor.
func IsDescendant[S comparable, H comparable](doc Document[S, H], state, ancestor S) bool {
	current := state
	for depth := 0; depth < MaxHierarchyDepth; depth++ {
		parent, ok := doc.ParentOf(current)
		if !ok {
			return false
		}
		if parent == ancestor {
			return true
		}
		current = parent
	}
	panic(fmt.Sprintf("IsDescendant: cyclic parent relationship detected walking from %v", state))
}

func allDescend[S comparable, H comparable](doc Document[S, H], states []S, ancestor S) bool {
	for _, state := range states {
		if !IsDescendant(doc, state, ancestor) {
			return false
		}
	}
	return true
}

func intersects[S comparable](a, b []S) bool {
	for _, state := range a {
		if containsState(b, state) {
			return true
		}
	}
	return false
}

func containsState[S comparable](states []S, state S) bool {
	for _, s := range states {
		if s == state {
			return true
		}
	}
	return false
}

func addOnce[S comparable](set *[]S, state S) {
	if !containsState(*set, state) {
		*set = append(*set, state)
	}
}
