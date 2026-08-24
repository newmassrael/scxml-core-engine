// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-3.2 — what `sce.Engine.EnterAt` accepts, and what it refuses, on the
// Go engine.
//
// The door exists so a host can bring a machine back where it was, in a new
// process, without replaying the entry actions the earlier run already ran.
// Refusals are the part that has to be enumerated rather than sampled: entering
// "near" the requested configuration is the one outcome this door must never
// produce, because nothing afterwards can detect it — the machine reports
// itself running, GetCurrentState answers, and the set behind those answers is
// one the document never describes. A gate holding only the accepting case
// would pass on an engine that accepted everything.
//
// The Go sibling of `backends/rust/runtime/tests/configuration_entry.rs` and
// `tests/integration/ConfigurationEntryAotTest.cpp`, asking the same questions
// of the same rules, so a set one engine accepts is one the others accept.
//
// Two machines, because the two halves of the door are different code paths:
//
//   - `parallel_regions_take_own_transitions` keeps its own active set
//     (HasActiveStates), which is the shape a restore has to hand back whole,
//     and the case where the current state is NOT recoverable from the set.
//   - `statechart_native_action` has no parallel regions, so its configuration
//     is the parent walk from the leaf and GetActiveStates derives it rather
//     than reading the policy. EnterAt has to close the round trip there too,
//     through a different path.
//
// This package is deliberately NOT a fixture stem: it drives documents that
// already exist in the tree rather than adding a document of its own, because
// the claim is about a runtime door and not about a topology. See
// `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for what a stem commits to.
package configuration_entry

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
	parallel "github.com/newmassrael/sce-go-tests/integration/parallel_regions_take_own_transitions"
	linear "github.com/newmassrael/sce-go-tests/integration/statechart_native_action"
)

type parallelState = parallel.ParallelRegionsTakeOwnTransitionsState
type parallelEvent = parallel.ParallelRegionsTakeOwnTransitionsEvent

// atWork is a mid-run configuration of the parallel document: both regions
// live, the deeper one in `working` and the shallower in `within`. Written out
// rather than taken from a live run because every refusal below is a MUTATION
// of it — one change each, so a refusal names one rule.
func atWork() []parallelState {
	return []parallelState{
		parallel.ParallelRegionsTakeOwnTransitionsStateRun,
		parallel.ParallelRegionsTakeOwnTransitionsStateDrive,
		parallel.ParallelRegionsTakeOwnTransitionsStateRunning,
		parallel.ParallelRegionsTakeOwnTransitionsStateWorking,
		parallel.ParallelRegionsTakeOwnTransitionsStateBudget,
		parallel.ParallelRegionsTakeOwnTransitionsStateWithin,
	}
}

// newParallel builds the parallel machine WITHOUT a script engine, which is
// what every refusal case wants: a refused entry returns before the §scxml-5.3
// declaration, so needing no engine is itself part of "validation runs before
// any mutation".
func newParallel() (*parallel.ParallelRegionsTakeOwnTransitionsPolicy, *sce.Engine[parallelState, parallelEvent]) {
	policy := parallel.NewParallelRegionsTakeOwnTransitionsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[parallelState, parallelEvent](&policy)
	return &policy, engine
}

// newParallelWithEngine is the accepted-entry variant: an accepted entry
// DECLARES the datamodel, and this document's `<data>` carries initialisers, so
// it needs its script engine in place first — the same requirement Initialize
// has, for the same reason.
func newParallelWithEngine() (*parallel.ParallelRegionsTakeOwnTransitionsPolicy, *sce.Engine[parallelState, parallelEvent]) {
	policy := parallel.NewParallelRegionsTakeOwnTransitionsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[parallelState, parallelEvent](&policy)
	return &policy, engine
}

func sameSet(got, want []parallelState) bool {
	if len(got) != len(want) {
		return false
	}
	for _, w := range want {
		found := false
		for _, g := range got {
			if g == w {
				found = true
				break
			}
		}
		if !found {
			return false
		}
	}
	return true
}

// The set written above is a configuration of the document, so it is accepted
// and the machine comes back holding exactly it. This is the baseline every
// refusal below is one mutation away from — without it, a validator that
// refused everything would pass every other case in this file.
func TestAParallelConfigurationIsAccepted(t *testing.T) {
	policy, engine := newParallelWithEngine()
	configuration := atWork()

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationAccepted {
		t.Fatalf("a configuration of the document was refused: %s", verdict)
	}
	if got := policy.GetActiveStates(); !sameSet(got, configuration) {
		t.Fatalf("the machine came back holding %v, not the configuration it was handed (%v)", got, configuration)
	}
	if got := engine.GetCurrentState(); got != parallel.ParallelRegionsTakeOwnTransitionsStateWorking {
		t.Fatalf("current state is %v, not the leaf the entry claimed", got)
	}
	if !engine.IsRunning() {
		t.Fatal("an accepted entry left the machine stopped")
	}
}

// The set the engine hands back has to be a copy: a host that keeps the slice
// it passed in and writes to it must not be able to move the machine behind the
// engine's back.
func TestAnAcceptedConfigurationIsNotAliasedToTheCallersSlice(t *testing.T) {
	policy, engine := newParallelWithEngine()
	configuration := atWork()

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationAccepted {
		t.Fatalf("a configuration of the document was refused: %s", verdict)
	}

	configuration[0] = parallel.ParallelRegionsTakeOwnTransitionsStateSettled
	if got := policy.GetActiveStates(); !sameSet(got, atWork()) {
		t.Fatalf("writing to the caller's slice changed the machine's configuration to %v", got)
	}
}

// A machine with no `<parallel>` keeps no active set of its own — the engine
// rebuilds the hierarchy from the current state. So the round trip has to work
// without SetActiveStates doing anything at all, which is a different code path
// and the one most documents take.
func TestALinearConfigurationRoundTripsWithoutAPolicyActiveSet(t *testing.T) {
	var host silentActions
	policy := linear.NewStatechartNativeActionPolicy(&host)
	engine := sce.NewEngine[linear.StatechartNativeActionState, linear.StatechartNativeActionEvent](&policy)

	if verdict := engine.EnterAt(
		[]linear.StatechartNativeActionState{linear.StatechartNativeActionStateAssembling},
		linear.StatechartNativeActionStateAssembling,
	); verdict != sce.ConfigurationAccepted {
		t.Fatalf("a single-state configuration was refused: %s", verdict)
	}
	if got := engine.GetCurrentState(); got != linear.StatechartNativeActionStateAssembling {
		t.Fatalf("current state is %v after a linear entry", got)
	}
	active := engine.GetActiveStates()
	if len(active) != 1 || active[0] != linear.StatechartNativeActionStateAssembling {
		t.Fatalf("the derived configuration is %v, not the single state entered", active)
	}
	if !engine.IsRunning() {
		t.Fatal("an accepted entry left the machine stopped")
	}
	if host.idleEntries != 0 || host.assemblingExits != 0 {
		t.Fatalf("entry/exit content ran during a resume: %d entries, %d exits", host.idleEntries, host.assemblingExits)
	}
}

func TestAnEmptyConfigurationIsRefused(t *testing.T) {
	_, engine := newParallel()
	if verdict := engine.EnterAt(nil, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationEmpty {
		t.Fatalf("a machine is never in nothing, but the empty set answered %s", verdict)
	}
}

// §scxml-3.3: a compound state holds exactly one active child. `working` and
// `judging` are both children of `running`, and a run stands in one of them.
func TestTwoSiblingsOfOneRegionAreRefused(t *testing.T) {
	_, engine := newParallel()
	configuration := append(atWork(), parallel.ParallelRegionsTakeOwnTransitionsStateJudging)

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationCompoundChildCount {
		t.Fatalf("`running` was given two active children and the answer was %s; that is a "+
			"configuration the document has no reading for", verdict)
	}
}

// §scxml-3.4: a `<parallel>` holds EVERY region. Dropping one is the shape a
// host produces when it journals only the region it cares about.
func TestAParallelWithARegionMissingIsRefused(t *testing.T) {
	_, engine := newParallel()
	configuration := []parallelState{
		parallel.ParallelRegionsTakeOwnTransitionsStateRun,
		parallel.ParallelRegionsTakeOwnTransitionsStateDrive,
		parallel.ParallelRegionsTakeOwnTransitionsStateRunning,
		parallel.ParallelRegionsTakeOwnTransitionsStateWorking,
	}

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationParallelRegionMissing {
		t.Fatalf("`budget` is a region of `run` and a run is always in both at once, but the "+
			"answer was %s", verdict)
	}
}

// The set has to be ancestor-closed: a state is active only if its parent is.
func TestAConfigurationThatSkipsAnAncestorIsRefused(t *testing.T) {
	_, engine := newParallel()
	configuration := []parallelState{
		parallel.ParallelRegionsTakeOwnTransitionsStateRun,
		parallel.ParallelRegionsTakeOwnTransitionsStateDrive,
		parallel.ParallelRegionsTakeOwnTransitionsStateWorking,
		parallel.ParallelRegionsTakeOwnTransitionsStateBudget,
		parallel.ParallelRegionsTakeOwnTransitionsStateWithin,
	}

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationAncestorMissing {
		t.Fatalf("`working` is a child of `running`, which the set does not hold, but the answer "+
			"was %s", verdict)
	}
}

// Checked before the arity counts, because a duplicate would otherwise read as
// a second child and the refusal would name the wrong rule.
func TestARepeatedStateIsRefused(t *testing.T) {
	_, engine := newParallel()
	configuration := append(atWork(), parallel.ParallelRegionsTakeOwnTransitionsStateWorking)

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationDuplicate {
		t.Fatalf("a state named twice answered %s", verdict)
	}
}

// §scxml-3.2: a configuration closes on exactly one root. `settled` is a
// top-level `<final>`, so a set holding both it and `run` describes two
// machines.
func TestTwoRootsAreRefused(t *testing.T) {
	_, engine := newParallel()
	configuration := append(atWork(), parallel.ParallelRegionsTakeOwnTransitionsStateSettled)

	if verdict := engine.EnterAt(configuration, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationRootCount {
		t.Fatalf("two disjoint trees answered %s", verdict)
	}
}

func TestACurrentStateOutsideTheConfigurationIsRefused(t *testing.T) {
	_, engine := newParallel()
	if verdict := engine.EnterAt(atWork(), parallel.ParallelRegionsTakeOwnTransitionsStateJudging); verdict != sce.ConfigurationCurrentNotActive {
		t.Fatalf("the current state is the one the machine is standing in, so it is in the set by "+
			"definition, but the answer was %s", verdict)
	}
}

// §scxml-3.3 makes the current state the ATOMIC state the engine descended to.
// A compound one is the shape a host produces when it journals the ancestor
// rather than the leaf.
func TestANonAtomicCurrentStateIsRefused(t *testing.T) {
	_, engine := newParallel()
	if verdict := engine.EnterAt(atWork(), parallel.ParallelRegionsTakeOwnTransitionsStateRunning); verdict != sce.ConfigurationCurrentNotAtomic {
		t.Fatalf("a compound current state answered %s", verdict)
	}
}

// The claim that makes every refusal above safe to act on: validation runs
// BEFORE any mutation, so a host that gets a rejection still holds the machine
// it had. Without this the door could half-enter, and a host reading a
// rejection would be told nothing happened while the engine had already moved.
func TestARefusedEntryLeavesTheEngineUntouched(t *testing.T) {
	policy, engine := newParallel()
	before := engine.GetCurrentState()

	if verdict := engine.EnterAt(nil, parallel.ParallelRegionsTakeOwnTransitionsStateWorking); verdict != sce.ConfigurationEmpty {
		t.Fatalf("the empty set answered %s", verdict)
	}

	if got := engine.GetCurrentState(); got != before {
		t.Fatalf("a refused entry moved the current state from %v to %v", before, got)
	}
	if engine.IsRunning() {
		t.Fatal("a refused entry started the machine")
	}
	if got := policy.GetActiveStates(); len(got) != 0 {
		t.Fatalf("a refused entry wrote an active set: %v", got)
	}
}

// §scxml-3.3: every state this document declares reads back from its own name.
//
// A host can only record a configuration as TEXT — the generated state
// constants are a build artefact of one binary, and the process that resumes is
// a different one. The forward and reverse tables are emitted from one loop
// over the document's states so they age together; this walks the document's
// own state list rather than a list spelled here, so a document that grows a
// state grows this check with it.
func TestEveryStateReadsBackFromItsOwnName(t *testing.T) {
	policy, _ := newParallel()

	if len(parallel.ParallelRegionsTakeOwnTransitionsAllStates) < 8 {
		t.Fatalf("the document declares %d states; this walk is measuring something other than "+
			"the document it names", len(parallel.ParallelRegionsTakeOwnTransitionsAllStates))
	}

	for _, state := range parallel.ParallelRegionsTakeOwnTransitionsAllStates {
		name := policy.GetStateName(state)
		back, known := policy.GetStateFromName(name)
		if !known {
			t.Fatalf("%q is the name this policy publishes for a state of its own document, and "+
				"reading it back reported the name unknown", name)
		}
		if back != state {
			t.Fatalf("%q read back as %v, not the state it names", name, back)
		}
	}

	if _, known := policy.GetStateFromName("a-state-this-document-does-not-declare"); known {
		t.Fatal("a name the document does not carry was answered with a state rather than refused; " +
			"a name guessed at is how a restore reaches a configuration nobody recorded")
	}
}

// A configuration that crossed a process: journalled as names, read back
// through the generated reverse table, and handed to the door. This is the
// whole point of the pair — the two halves in one call chain rather than each
// proved alone.
func TestAConfigurationJournalledAsNamesIsAcceptedBack(t *testing.T) {
	writer, writerEngine := newParallelWithEngine()
	writerEngine.Initialize()
	writerEngine.RaiseExternal(parallel.ParallelRegionsTakeOwnTransitionsEventE, "", "")
	writerEngine.Step()

	journal := make([]string, 0, 8)
	for _, state := range writerEngine.GetActiveStates() {
		journal = append(journal, writer.GetStateName(state))
	}
	currentName := writer.GetStateName(writerEngine.GetCurrentState())

	reader, readerEngine := newParallelWithEngine()
	configuration := make([]parallelState, 0, len(journal))
	for _, name := range journal {
		state, known := reader.GetStateFromName(name)
		if !known {
			t.Fatalf("the journal names %q and the reader could not read it back", name)
		}
		configuration = append(configuration, state)
	}
	current, known := reader.GetStateFromName(currentName)
	if !known {
		t.Fatalf("the journal's current state %q could not be read back", currentName)
	}

	if verdict := readerEngine.EnterAt(configuration, current); verdict != sce.ConfigurationAccepted {
		t.Fatalf("a configuration a run actually reached was refused on the way back: %s", verdict)
	}
	if got := readerEngine.GetCurrentState(); got != current {
		t.Fatalf("the resumed machine stands in %v, not where the journal said (%v)", got, current)
	}
	if got := reader.GetActiveStates(); !sameSet(got, configuration) {
		t.Fatalf("the resumed configuration is %v, not the journalled one %v", got, configuration)
	}
}

// silentActions is the host for the linear machine. Its every effect is a
// `<sce:action>`, so it cannot be constructed without one — which is the point
// of that seam and merely plumbing here, except for the two counters, which are
// what says no entry or exit content ran during a resume.
type silentActions struct {
	idleEntries     int
	assemblingExits int
}

func (s *silentActions) AppendFragmentPayload(_ []byte, _ uint32) {}

func (s *silentActions) ResetSlot() {}

func (s *silentActions) OnIdleEntry() { s.idleEntries++ }

func (s *silentActions) OnAssemblingExit() { s.assemblingExits++ }
