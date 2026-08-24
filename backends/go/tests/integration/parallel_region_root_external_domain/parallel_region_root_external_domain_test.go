// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: a <parallel> is not a transition domain — Go AOT.
//
// getTransitionDomain sends an external transition to findLCCA, which filters
// the proper ancestors with isCompoundStateOrScxmlElement. A <parallel> is
// neither, so an external transition written on a REGION ROOT has the document
// root as its domain: every region exits and re-enters, and a sibling region's
// transition on the same event is preempted because the two exit sets
// intersect and the sibling's source is not a descendant of this one's.
//
// The engine answered the enclosing <parallel> here instead, because both
// domain sites in the generated conflict resolver asked for a plain
// lowest-common-ancestor — the first common ancestor, whatever its kind. That
// is the findLCA the appendix distinguishes from findLCCA, and the difference
// is invisible until a <parallel> sits between the source and the first
// compound <state> above it, which is exactly a region root.
//
// Measured 2026-08-25 on examples/ai_loop/ai_loop.scxml: the Kotlin engine,
// the only one implementing the filter, ended `session.lost` in
// [alive, restarting] where C++, Rust and Go ended in [rebuilding, restarting].
// That document was then repaired to say type="internal", which is what its
// three region-root transitions meant — and that repair is why this fixture
// exists rather than the ai_loop suite: with the document fixed, no committed
// document reaches the external form.
//
// Sibling of the C++ drivers ParallelRegionRootExternalDomainTest.cpp
// (Interpreter) and ParallelRegionRootExternalDomainAotTest.cpp (AOT) and of
// backends/rust/tests/tests/parallel_region_root_external_domain.rs, all
// asking the same two clauses of the same document.
//
// Fixture: tests/integration/parallel_region_root_external_domain.scxml
// (not under integration_resources/: a stem there is a seven-channel contract
// enforced by integration_stem_registration.rs, and the Python and C11 engines
// have not been repaired yet.)
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_region_root_external_domain_go.sh

package parallel_region_root_external_domain

import (
	"sort"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// The whole configuration, sorted, rather than a handful of membership
// questions.
//
// The way this defect presents is an ILLEGAL configuration — two children of
// the same compound region active at once — and every individual "is this
// state active" check answers true to that.
func configuration(engine *sce.Engine[ParallelRegionRootExternalDomainState, ParallelRegionRootExternalDomainEvent]) string {
	names := []string{}
	for _, s := range engine.GetActiveStates() {
		names = append(names, s.String())
	}
	sort.Strings(names)
	return "[" + strings.Join(names, " ") + "]"
}

func started() *sce.Engine[ParallelRegionRootExternalDomainState, ParallelRegionRootExternalDomainEvent] {
	policy := NewParallelRegionRootExternalDomainPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[ParallelRegionRootExternalDomainState, ParallelRegionRootExternalDomainEvent](&policy)
	engine.Initialize()
	return engine
}

// The clause itself.
func TestAnExternalRegionRootTransitionExitsEveryRegion(t *testing.T) {
	engine := started()

	if got, want := configuration(engine), "[alive drive run watch working]"; got != want {
		t.Fatalf("precondition: the fixture is supposed to start with both regions at their "+
			"defaults; it came up as %s, want %s, so nothing below is testing what it claims",
			got, want)
	}

	engine.RaiseExternal(ParallelRegionRootExternalDomainEventRestart, "", "")
	engine.Step()

	if got, want := configuration(engine), "[alive drive restarting run watch]"; got != want {
		t.Errorf("active %s, want %s.\n"+
			"An external transition on a region root has the DOCUMENT ROOT as its domain "+
			"(Appendix D findLCCA filters <parallel> out of the candidate ancestors), so every "+
			"region exits and re-enters, `watch` is back at its default, and `watch`'s own "+
			"transition on the same event is preempted as conflicting.", got, want)
	}
}

// The contrast, and the reason the ai_loop document is spelled the way it is.
// A test that only pinned the external case would pass just as well on an
// engine that sent EVERY region-root transition to the document root.
func TestAnInternalRegionRootTransitionLeavesTheOtherRegion(t *testing.T) {
	engine := started()

	engine.RaiseExternal(ParallelRegionRootExternalDomainEventHold, "", "")
	engine.Step()

	if got, want := configuration(engine), "[drive paused rebuilding run watch]"; got != want {
		t.Errorf("active %s, want %s.\n"+
			"An internal region-root transition has the region as its domain (source compound, "+
			"target its descendant), so the sibling region never exits and answers the event "+
			"itself.", got, want)
	}
}
