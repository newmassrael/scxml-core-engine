// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package harness

import (
	"fmt"
	"sort"
	"testing"
)

// TransitionFunction is a generated transition function
// (docs/SCE_TRANSITION_FUNCTION.md), read through its accessors: the package
// is the generator's, so the drive is handed what it needs of it by name.
type TransitionFunction[S comparable, E comparable] struct {
	States        []S
	Events        []E
	Initial       S
	Transition    func(S, E) (S, bool)
	IsFinal       func(S) bool
	Configuration func(S) []string
	StateName     func(S) string
	EventName     func(E) string
}

// EngineProbe is what the drive asks of the engine it holds to a function:
// a fresh machine at its initial configuration, an event delivered by name and
// run to completion, the ids of the active states, the final the run ended in,
// and how many events the engine has discarded.
type EngineProbe struct {
	Start     func()
	Send      func(name string)
	Active    func() []string
	Terminal  func() (string, bool)
	Discarded func() uint32
}

// pathStep is a state ShortestPath has reached, with the events that reach it.
type pathStep[S comparable, E comparable] struct {
	state S
	path  []E
}

// ShortestPath is the shortest sequence of events that takes the function from
// its initial state to target, read from the function alone, and false when no
// sequence does.
func ShortestPath[S comparable, E comparable](function TransitionFunction[S, E], target S) ([]E, bool) {
	frontier := []pathStep[S, E]{{function.Initial, nil}}
	seen := map[S]bool{function.Initial: true}
	for next := 0; next < len(frontier); next++ {
		current := frontier[next]
		if current.state == target {
			return current.path, true
		}
		for _, event := range function.Events {
			if to, ok := function.Transition(current.state, event); ok && !seen[to] {
				seen[to] = true
				longer := append(append([]E{}, current.path...), event)
				frontier = append(frontier, pathStep[S, E]{to, longer})
			}
		}
	}
	return nil, false
}

func sameIDs(a, b []string) bool {
	x := append([]string{}, a...)
	y := append([]string{}, b...)
	sort.Strings(x)
	sort.Strings(y)
	if len(x) != len(y) {
		return false
	}
	for i := range x {
		if x[i] != y[i] {
			return false
		}
	}
	return true
}

// ExpectEveryPairAgrees drives the engine to every state of the function by
// the shortest path the function gives, sends it every event there, and
// reports a failure wherever the configuration it reaches, or whether it took
// the event or discarded it, is not what the function says. The configuration
// is compared as a set of ids: a state's ancestors are fixed by the document,
// so the set says what the ordered list says. It returns how many pairs it
// checked, which a caller holds to the number of states times the number of
// events.
func ExpectEveryPairAgrees[S comparable, E comparable](t *testing.T, function TransitionFunction[S, E], engine EngineProbe) int {
	t.Helper()
	engine.Start()
	if !sameIDs(engine.Active(), function.Configuration(function.Initial)) {
		t.Errorf("the machine does not start where the function does: %v, want %v",
			engine.Active(), function.Configuration(function.Initial))
	}
	pairs := 0
	for _, state := range function.States {
		name := function.StateName(state)
		path, reached := ShortestPath(function, state)
		if !reached {
			t.Errorf("%s is not reached from the initial state", name)
			continue
		}
		for _, event := range function.Events {
			pair := fmt.Sprintf("(%s, %s)", name, function.EventName(event))
			engine.Start()
			for _, taken := range path {
				engine.Send(function.EventName(taken))
			}
			pairs++
			if function.IsFinal(state) {
				if ended, ok := engine.Terminal(); !ok || ended != name {
					t.Errorf("%s: the path does not end the machine there", pair)
				}
				continue
			}
			if !sameIDs(engine.Active(), function.Configuration(state)) {
				t.Errorf("%s: the path leaves the machine elsewhere: %v", pair, engine.Active())
			}
			discarded := engine.Discarded()
			engine.Send(function.EventName(event))
			to, taken := function.Transition(state, event)
			if !taken {
				if engine.Discarded() != discarded+1 {
					t.Errorf("%s: the function refuses it and the machine took it", pair)
				}
				if !sameIDs(engine.Active(), function.Configuration(state)) {
					t.Errorf("%s: the machine moved on an event it discarded: %v", pair, engine.Active())
				}
				continue
			}
			if engine.Discarded() != discarded {
				t.Errorf("%s: the function takes it and the machine discarded it", pair)
			}
			if function.IsFinal(to) {
				if ended, ok := engine.Terminal(); !ok || ended != function.StateName(to) {
					t.Errorf("%s: the machine did not end where the function says", pair)
				}
			} else if !sameIDs(engine.Active(), function.Configuration(to)) {
				t.Errorf("%s: the machine is not where the function says: %v, want %v",
					pair, engine.Active(), function.Configuration(to))
			}
		}
	}
	return pairs
}
