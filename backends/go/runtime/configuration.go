// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "slices"

// Is a set of states a CONFIGURATION of the document, and is `current` its
// current state? The clauses that decide are cited on ValidateConfiguration,
// which is where they are answered.
//
// Engine.GetActiveStates publishes the configuration a machine is in and
// Engine.EnterAt is the door that takes one back. This file owns the question
// between them, and it is the only place that asks it.
//
// # Why this rejects instead of panicking
//
// Every other input to this engine is authored: the generator wrote the policy,
// and a hierarchy that does not walk is a generator defect. A restored
// configuration is the single exception — it arrives from OUTSIDE the process,
// read back by a host from wherever it persisted it, recorded against a
// document revision that may since have moved. A host holding a stale record
// has to be able to handle that answer, so this returns a reason and the engine
// hands it on.
//
// The Go twin of the Rust runtime's `helpers::configuration::validate` and of
// the C++ `SCE::Core::validateConfiguration`, asking the same questions of the
// same static hierarchy, so a configuration one engine accepts is one the
// others accept.

// ConfigurationRejection is why a configuration was refused.
// ConfigurationAccepted is the accepting answer.
//
// An enumerated reason rather than a bool: a host handing back a journal it
// wrote itself needs to know WHICH rule its record broke, and "invalid" sends
// it looking at the wrong half.
type ConfigurationRejection int

const (
	// ConfigurationAccepted means the set is a configuration of this document.
	ConfigurationAccepted ConfigurationRejection = iota
	// ConfigurationEmpty means no states at all. A machine is never in nothing.
	ConfigurationEmpty
	// ConfigurationDuplicate means a state appears twice. Checked before every
	// arity count below, which would otherwise read a duplicate as a second
	// child and blame the wrong rule.
	ConfigurationDuplicate
	// ConfigurationAncestorMissing means a state is present whose parent is
	// not, so the set is not ancestor-closed.
	ConfigurationAncestorMissing
	// ConfigurationRootCount means the set closes on the wrong number of roots.
	// Exactly one is a configuration; more than one is two disjoint trees.
	ConfigurationRootCount
	// ConfigurationCompoundChildCount means a compound state holds a number of
	// active children that is not one (§scxml-3.11).
	ConfigurationCompoundChildCount
	// ConfigurationParallelRegionMissing means a <parallel> region is absent, so
	// the set is not one in which every region is simultaneously active
	// (§scxml-3.11).
	ConfigurationParallelRegionMissing
	// ConfigurationParallelChildCount means a <parallel> holds a number of
	// children that is not its region count (§scxml-3.11).
	ConfigurationParallelChildCount
	// ConfigurationAtomicHasChildren means an atomic state has a child in the
	// set, so it is not atomic here.
	ConfigurationAtomicHasChildren
	// ConfigurationCurrentNotActive means the claimed current state is not in
	// the configuration at all.
	ConfigurationCurrentNotActive
	// ConfigurationCurrentNotAtomic means the claimed current state is compound
	// or parallel. §scxml-3.11 makes the current state the atomic one the engine
	// descended to.
	ConfigurationCurrentNotAtomic
)

// String is the human-readable reason, for the message a refusal carries.
func (r ConfigurationRejection) String() string {
	switch r {
	case ConfigurationAccepted:
		return "accepted"
	case ConfigurationEmpty:
		return "the configuration is empty; a machine is never in nothing"
	case ConfigurationDuplicate:
		return "a state appears twice"
	case ConfigurationAncestorMissing:
		return "a state is present whose parent is not, so the set is not ancestor-closed"
	case ConfigurationRootCount:
		return "a configuration closes on exactly one root (W3C SCXML 3.11)"
	case ConfigurationCompoundChildCount:
		return "a compound state holds exactly one active child (W3C SCXML 3.11)"
	case ConfigurationParallelRegionMissing:
		return "a <parallel> holds every region and one is missing (W3C SCXML 3.11)"
	case ConfigurationParallelChildCount:
		return "a <parallel> holds every region and nothing else (W3C SCXML 3.11)"
	case ConfigurationAtomicHasChildren:
		return "an atomic state has a child in the set"
	case ConfigurationCurrentNotActive:
		return "the current state is not in the configuration"
	case ConfigurationCurrentNotAtomic:
		return "the current state must be the atomic state the engine descended to"
	}
	return "unknown"
}

// ValidateConfiguration reports whether configuration is a configuration of
// policy's document, with current as its current state.
//
// Pure: reads the policy's static hierarchy and nothing else. Cost is quadratic
// in the set length, which is a handful of states, and this runs once per
// restore — the shape is chosen for being obviously right rather than for being
// fast, exactly as its Rust and C++ twins are.
//
// What cannot be wrong, and why it is not checked: a member is an S, a
// generated state type with one value per state of this document, so "no such
// state" needs no rejection variant.
func ValidateConfiguration[S comparable, E comparable](
	policy StatePolicy[S, E],
	configuration []S,
	current S,
) ConfigurationRejection {
	if len(configuration) == 0 {
		return ConfigurationEmpty
	}

	for i := range configuration {
		for j := 0; j < i; j++ {
			if configuration[j] == configuration[i] {
				return ConfigurationDuplicate
			}
		}
	}

	holds := func(s S) bool {
		for _, member := range configuration {
			if member == s {
				return true
			}
		}
		return false
	}

	roots := 0
	for _, state := range configuration {
		parent, hasParent := policy.GetParent(state)
		if !hasParent {
			roots++
		} else if !holds(parent) {
			return ConfigurationAncestorMissing
		}
	}
	if roots != 1 {
		return ConfigurationRootCount
	}

	for _, state := range configuration {
		if verdict := childArity(policy, configuration, state); verdict != ConfigurationAccepted {
			return verdict
		}
	}

	if !holds(current) {
		return ConfigurationCurrentNotActive
	}
	if policy.IsCompoundState(current) {
		return ConfigurationCurrentNotAtomic
	}
	if policy.HasParallelStates() && policy.IsParallelState(current) {
		return ConfigurationCurrentNotAtomic
	}

	return ConfigurationAccepted
}

// childArity is whether state, in the set among, holds the number of children a
// configuration gives it: one for a compound state (§scxml-3.11), every region
// and nothing else for a <parallel> (§scxml-3.4), none for an atomic one.
func childArity[S comparable, E comparable](
	policy StatePolicy[S, E],
	among []S,
	state S,
) ConfigurationRejection {
	children := 0
	for _, candidate := range among {
		parent, hasParent := policy.GetParent(candidate)
		if hasParent && parent == state {
			children++
		}
	}

	if policy.HasParallelStates() && policy.IsParallelState(state) {
		regions := policy.GetChildStates(state)
		// §scxml-3.4: every region, simultaneously.
		for _, region := range regions {
			if !slices.Contains(among, region) {
				return ConfigurationParallelRegionMissing
			}
		}
		if children != len(regions) {
			return ConfigurationParallelChildCount
		}
		return ConfigurationAccepted
	}

	// §scxml-3.11: exactly one.
	if policy.IsCompoundState(state) {
		if children != 1 {
			return ConfigurationCompoundChildCount
		}
	} else if children != 0 {
		return ConfigurationAtomicHasChildren
	}
	return ConfigurationAccepted
}

// HistoryRejectionKind is why a recorded <history> value was refused
// (§scxml-3.10).
type HistoryRejectionKind int

const (
	// HistoryAccepted means the value is one the <history> could have recorded.
	HistoryAccepted HistoryRejectionKind = iota
	// HistoryEmpty means the value holds no state. A history records at least
	// one.
	HistoryEmpty
	// HistoryDuplicate means a state appears more than once.
	HistoryDuplicate
	// HistoryNotBelow means a state is not below the history's parent.
	HistoryNotBelow
	// HistoryNotAChild means a shallow history records the children of its parent,
	// and a state is deeper.
	HistoryNotAChild
	// HistoryNotAtomic means a deep history records atomic states, and one has
	// children.
	HistoryNotAtomic
	// HistoryArity means what the value holds is not part of any configuration
	// of the document: a compound state with two active children, a region
	// missing.
	HistoryArity
)

// HistoryRejection is the verdict on a recorded <history> value: its kind, the
// state it is about and the history's parent where the kind names them, and the
// configuration rule broken for HistoryArity.
type HistoryRejection[S comparable] struct {
	Kind   HistoryRejectionKind
	State  S
	Parent S
	Arity  ConfigurationRejection
}

// ValidateHistory reports whether recorded is a value the <history> of parent
// could have recorded: its active children for a shallow one, the active atomic
// states below it for a deep one (deep) (§scxml-3.10).
//
// What a history recorded is part of a configuration, so a value read back from a
// saved state is held to that, and the restored machine can only be taken
// through the history to a configuration its document could have been in. It is
// read as a configuration of the subtree under parent: the recorded states and
// every state between each of them and parent satisfy the child arity a whole
// configuration does. A shallow history records only the children, so the arity
// is judged on parent alone.
func ValidateHistory[S comparable, E comparable](
	policy StatePolicy[S, E],
	parent S,
	deep bool,
	recorded []S,
) HistoryRejection[S] {
	if len(recorded) == 0 {
		return HistoryRejection[S]{Kind: HistoryEmpty}
	}
	for at, state := range recorded {
		if slices.Contains(recorded[:at], state) {
			return HistoryRejection[S]{Kind: HistoryDuplicate, State: state}
		}
	}

	// The recorded states, and every state between each of them and parent.
	subtree := []S{parent}
	for _, state := range recorded {
		var chain []S
		for at := state; at != parent; {
			chain = append(chain, at)
			above, hasParent := policy.GetParent(at)
			if !hasParent {
				return HistoryRejection[S]{Kind: HistoryNotBelow, State: state, Parent: parent}
			}
			at = above
		}
		// A state equal to parent is below nothing.
		if len(chain) == 0 {
			return HistoryRejection[S]{Kind: HistoryNotBelow, State: state, Parent: parent}
		}
		for _, between := range chain {
			if !slices.Contains(subtree, between) {
				subtree = append(subtree, between)
			}
		}
	}

	if deep {
		for _, state := range recorded {
			if len(policy.GetChildStates(state)) != 0 {
				return HistoryRejection[S]{Kind: HistoryNotAtomic, State: state}
			}
		}
		for _, state := range subtree {
			if verdict := childArity(policy, subtree, state); verdict != ConfigurationAccepted {
				return HistoryRejection[S]{Kind: HistoryArity, Arity: verdict}
			}
		}
		return HistoryRejection[S]{}
	}
	for _, state := range recorded {
		if above, hasParent := policy.GetParent(state); !hasParent || above != parent {
			return HistoryRejection[S]{Kind: HistoryNotAChild, State: state, Parent: parent}
		}
	}
	if verdict := childArity(policy, subtree, parent); verdict != ConfigurationAccepted {
		return HistoryRejection[S]{Kind: HistoryArity, Arity: verdict}
	}
	return HistoryRejection[S]{}
}
