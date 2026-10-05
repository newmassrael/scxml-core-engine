// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.13, "Hybrid `<invoke>`": an `<invoke srcexpr>` that
// declares `sce:candidates` starts the document its value names (W3C SCXML
// 6.4), by the document's stem, and hands it the invoke's arguments, each to the
// variable of the same name that candidate declares (W3C SCXML 6.4.3).
//
// `static_invoke_hybrid.scxml` runs four phases, each in a state of its own:
//
//	first    `file:static_hybrid_first.scxml`: handed start = 7 and
//	         enabled = true, and ends. `extra` is evaluated and left out, the
//	         candidate declaring none.
//	second   an absolute path to `static_hybrid_second.scxml`: handed start = 7
//	         and extra = 3, and ends. `enabled` is left out.
//	lossy    `./static_hybrid_first.scxml` with an `extra` no 32-bit field can
//	         hold: reported as `error.execution` and left out, and the child
//	         still starts and ends.
//	missing  a document the invoke did not declare: `error.execution`, and no
//	         child starts, so no `done.invoke` follows.
//
// A candidate handed what the OTHER one declares would never end, and the run
// would stop short of `over`. The Rust half is
// `a_static_hybrid_invoke_starts_the_candidate_its_value_names.rs`.

package static_datamodel

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_hybrid"
)

// settledHybrid starts the machine and lets its children run and report: a
// child that ends during its own Initialize has reported by the time the
// parent's start returns, so the whole run may be over before the first tick.
func settledHybrid(t *testing.T) (*static_invoke_hybrid.StaticInvokeHybridPolicy, *sce.Engine[static_invoke_hybrid.StaticInvokeHybridState, static_invoke_hybrid.StaticInvokeHybridEvent]) {
	t.Helper()
	policy := static_invoke_hybrid.NewStaticInvokeHybridPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_invoke_hybrid.StaticInvokeHybridState, static_invoke_hybrid.StaticInvokeHybridEvent](&policy)
	engine.Initialize()
	for i := 0; i < 40; i++ {
		engine.Tick()
	}
	return &policy, engine
}

func TestEachPhaseStartsTheCandidateItsValueNamesAndEnds(t *testing.T) {
	policy, engine := settledHybrid(t)
	// Each phase's `done.invoke` adds a power of ten of its own, so the sum says
	// WHICH candidates ended: `first` (1), `second` (10) and the retry of `first`
	// that carried an argument it could not hold (100).
	if got := policy.Completed(); got != 111 {
		t.Errorf("completed is %d, not 111", got)
	}
	if !engine.IsInFinalState() {
		t.Errorf("the last phase named no declared candidate, so the run is over: %v", engine.GetActiveStates())
	}
}

func TestAnArgumentIsEvaluatedWhateverTheCandidateKeeps(t *testing.T) {
	policy, _ := settledHybrid(t)
	// `lossy` hands an `extra` that overflows to a candidate that declares none
	// (one error), and `missing` names no declared candidate (the other).
	if got := policy.Errors(); got != 2 {
		t.Errorf("errors is %d, not 2", got)
	}
}

func TestAValueNamingNoDeclaredCandidateStartsNothing(t *testing.T) {
	policy, _ := settledHybrid(t)
	// `done.invoke.missing_run` would add 1000: nothing started to send it.
	if got := policy.Completed(); got >= 1000 {
		t.Errorf("completed is %d: a child started for a document the invoke did not declare", got)
	}
}
