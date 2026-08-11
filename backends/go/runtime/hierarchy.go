// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import "fmt"

// MaxHierarchyDepth is the maximum supported state hierarchy depth. Prevents
// infinite loops from cyclic parent relationships and bounds stack/heap allocation.
// Matches Rust MAX_HIERARCHY_DEPTH = 16.
//
// W3C SCXML has no normative depth limit; 16 covers every real-world document
// (typical: 1-5, complex: up to 10).
const MaxHierarchyDepth = 16

// FindLCA finds the Least Common Ancestor of two states (§scxml-3.12).
//
// Returns (lca, true) if the states share a common ancestor (or are equal),
// or (zero, false) if they belong to disjoint hierarchies.
//
// 1:1 port of Rust hierarchy::find_lca from backends/rust/runtime/src/helpers/hierarchy.rs.
//
// Algorithm:
//  1. If state1 == state2, return it directly.
//  2. Walk up from state1's parent, collecting ancestors into a list.
//  3. Walk up from state2, checking each ancestor against the list; first
//     match is the LCA.
//
// Panics if either state's ancestor chain exceeds MaxHierarchyDepth (indicates a cycle).
func FindLCA[S comparable, E comparable](policy StatePolicy[S, E], state1, state2 S) (S, bool) {
	if state1 == state2 {
		return state1, true
	}

	// Build state1's ancestor chain (starting from state1's parent, per W3C 3.13 test 504)
	ancestors1 := make([]S, 0, 8)
	if current, hasParent := policy.GetParent(state1); hasParent {
		depth := 0
		for {
			if depth >= MaxHierarchyDepth {
				panic(fmt.Sprintf(
					"FindLCA: cyclic parent relationship detected while walking ancestors of state1"))
			}
			ancestors1 = append(ancestors1, current)
			parent, ok := policy.GetParent(current)
			if !ok {
				break
			}
			current = parent
			depth++
		}
	}

	// Walk up from state2, looking for intersection
	current := state2
	depth := 0
	for {
		if depth >= MaxHierarchyDepth {
			panic(fmt.Sprintf(
				"FindLCA: cyclic parent relationship detected while walking ancestors of state2"))
		}
		for _, a := range ancestors1 {
			if a == current {
				return current, true
			}
		}
		parent, ok := policy.GetParent(current)
		if !ok {
			var zero S
			return zero, false
		}
		current = parent
		depth++
	}
}

// IsDescendantOfByWalk checks whether descendant is a strict descendant of
// ancestor by walking the hierarchy (§scxml-D-getProperAncestors).
//
// Returns true if walking up from descendant via GetParent eventually reaches
// ancestor. Returns false if the walk terminates at the root without finding it,
// or if descendant == ancestor (strict descendancy).
//
// 1:1 port of Rust hierarchy::is_descendant_of.
//
// Panics if the ancestor walk exceeds MaxHierarchyDepth.
func IsDescendantOfByWalk[S comparable, E comparable](policy StatePolicy[S, E], descendant, ancestor S) bool {
	current := descendant
	depth := 0
	for {
		if depth >= MaxHierarchyDepth {
			panic("IsDescendantOfByWalk: cyclic parent relationship detected")
		}
		parent, ok := policy.GetParent(current)
		if !ok {
			return false
		}
		if parent == ancestor {
			return true
		}
		current = parent
		depth++
	}
}

// BuildExitChain builds an exit chain from fromState up to (but excluding)
// stopBeforeState (§scxml-3.12).
//
// Returns states in leaf-to-ancestor order, suitable for calling
// ExecuteExitActions in sequence. The stop state is the LCA -- it is not exited
// because the transition stays within its subtree.
//
// 1:1 port of Rust hierarchy::build_exit_chain.
//
// Panics if the chain exceeds MaxHierarchyDepth.
func BuildExitChain[S comparable, E comparable](policy StatePolicy[S, E], fromState, stopBeforeState S) []S {
	chain := make([]S, 0, 8)
	current := fromState
	depth := 0

	for current != stopBeforeState {
		if depth >= MaxHierarchyDepth {
			panic("BuildExitChain: cyclic parent relationship detected")
		}
		chain = append(chain, current)
		parent, ok := policy.GetParent(current)
		if !ok {
			break // Reached root without finding stop state
		}
		current = parent
		depth++
	}

	return chain
}

// BuildEntryChainFromAncestor builds an entry chain from ancestor down to
// target (exclusive of ancestor) (§scxml-3.12).
//
// Returns states in ancestor-to-descendant order, suitable for calling
// ExecuteEntryActions in sequence. The ancestor itself is not included because
// it is already active.
//
// 1:1 port of Rust hierarchy::build_entry_chain_from_ancestor.
//
// Panics if the chain exceeds MaxHierarchyDepth.
func BuildEntryChainFromAncestor[S comparable, E comparable](policy StatePolicy[S, E], target, ancestor S) []S {
	chain := make([]S, 0, 8)
	current := target
	depth := 0

	for current != ancestor {
		if depth >= MaxHierarchyDepth {
			panic("BuildEntryChainFromAncestor: cyclic parent relationship detected")
		}
		chain = append(chain, current)
		parent, ok := policy.GetParent(current)
		if !ok {
			break // Reached root without finding ancestor
		}
		current = parent
		depth++
	}

	// Reverse to ancestor-to-descendant order
	for i, j := 0, len(chain)-1; i < j; i, j = i+1, j-1 {
		chain[i], chain[j] = chain[j], chain[i]
	}

	return chain
}

// BuildEntryChain builds the complete entry chain from root down to leafState
// (§scxml-3.3).
//
// Returns states in root-to-leaf order. Parallel region children are NOT added
// here -- that responsibility belongs to ExecuteEntryActions.
//
// 1:1 port of Rust hierarchy::build_entry_chain.
//
// Panics if the chain exceeds MaxHierarchyDepth.
func BuildEntryChain[S comparable, E comparable](policy StatePolicy[S, E], leafState S) []S {
	chain := make([]S, 0, 8)
	current := leafState
	depth := 0

	// Walk from leaf to root
	for {
		if depth >= MaxHierarchyDepth {
			panic("BuildEntryChain: cyclic parent relationship detected")
		}
		chain = append(chain, current)
		parent, ok := policy.GetParent(current)
		if !ok {
			break // Reached root
		}
		current = parent
		depth++
	}

	// Reverse to root-to-leaf order
	for i, j := 0, len(chain)-1; i < j; i, j = i+1, j-1 {
		chain[i], chain[j] = chain[j], chain[i]
	}

	return chain
}
