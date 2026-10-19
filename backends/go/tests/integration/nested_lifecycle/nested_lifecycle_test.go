// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — Go AOT, the nested lifecycle.
//
// The document declares `sce:transition-function="emit"` and reaches every rule
// the function is computed by: transitions held by an ancestor, a compound
// state entered by its `initial` attribute, by an `<initial>` element and by its
// first child, an internal and an external transition, a prefix and a wildcard
// descriptor, a targetless transition and a top-level final. This drives the
// machine to every state by the function's shortest path, sends every event
// there, and holds what the engine does to what the function says, pair by
// pair.
//
// Fixture: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// (canonical, shared with the C++ / Rust / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_nested_lifecycle_go.sh

package nested_lifecycle

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
	tf "github.com/newmassrael/sce-go-tests/integration/nested_lifecycle/nested_lifecycle_transition_function"
)

func TestTheMachineAndItsTransitionFunctionAgreeOnEveryPair(t *testing.T) {
	var engine *sce.Engine[NestedLifecycleState, NestedLifecycleEvent]
	probe := scegotest.EngineProbe{
		Start: func() {
			policy := NewNestedLifecyclePolicy()
			engine = sce.NewEngine[NestedLifecycleState, NestedLifecycleEvent](&policy)
			engine.Initialize()
		},
		Send: func(name string) {
			engine.RaiseExternalByName(name, "")
			engine.Step()
		},
		Active: func() []string {
			names := []string{}
			for _, state := range engine.GetActiveStates() {
				names = append(names, state.String())
			}
			return names
		},
		Terminal: func() (string, bool) {
			state, ended := engine.TerminalState()
			return state.String(), ended
		},
		Discarded: func() uint32 {
			return engine.DiscardedExternalEvents()
		},
	}
	function := scegotest.TransitionFunction[tf.State, tf.Event]{
		States:        tf.AllStates,
		Events:        tf.AllEvents,
		Initial:       tf.Initial,
		Transition:    tf.Transition,
		IsFinal:       tf.IsFinal,
		Configuration: tf.Configuration,
		StateName:     tf.State.String,
		EventName:     tf.Event.String,
	}
	pairs := scegotest.ExpectEveryPairAgrees(t, function, probe)
	if want := len(tf.AllStates) * len(tf.AllEvents); pairs != want {
		t.Fatalf("the drive checked %d pairs, want %d", pairs, want)
	}
}
