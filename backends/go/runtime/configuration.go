// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

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
		children := 0
		for _, candidate := range configuration {
			parent, hasParent := policy.GetParent(candidate)
			if hasParent && parent == state {
				children++
			}
		}

		if policy.HasParallelStates() && policy.IsParallelState(state) {
			regions := policy.GetParallelRegions(state)
			// §scxml-3.4: every region, simultaneously.
			for _, region := range regions {
				if !holds(region) {
					return ConfigurationParallelRegionMissing
				}
			}
			if children != len(regions) {
				return ConfigurationParallelChildCount
			}
			continue
		}

		// §scxml-3.11: exactly one.
		if policy.IsCompoundState(state) {
			if children != 1 {
				return ConfigurationCompoundChildCount
			}
		} else if children != 0 {
			return ConfigurationAtomicHasChildren
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
