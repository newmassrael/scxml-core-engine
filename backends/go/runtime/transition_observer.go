// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// TransitionTaken is one transition the engine took, as its row of the emitted
// transition table names it (docs/adr/0089, docs/SCE_TRANSITION_TABLE.md).
//
// Source is the machine's own state type, the type the table's From holds, and
// Index is the table's Index type, so a pair compares with a row
// (row.From, row.Index) without a conversion.
type TransitionTaken[S comparable] struct {
	// Source is the state the transition is written in: the From of its row.
	Source S
	// Index is its position among Source's own transitions in document order:
	// the Index of its row.
	Index uint32
}

// SetTransitionObserver tells observer each transition this engine takes from
// now on, by the row of its emitted transition table; nil stops it
// (docs/adr/0089).
//
// The observer is told after the selection and before the microstep's first
// exit, each transition of the optimal enabled set once and in selection
// order, so a run's set of pairs is the set of rows it took: the table's
// coverage, measured on the engine that ran. It reads and is asked nothing, so
// a machine with one set takes the same transitions as one without. It runs on
// the goroutine that drives the engine, inside the macrostep, and must not
// call back into the engine; a panic in it is not recovered. Set it before
// Initialize to be told the transitions the first macrostep takes.
func (e *Engine[S, E]) SetTransitionObserver(observer func(TransitionTaken[S])) {
	e.transitionObserver = observer
}
