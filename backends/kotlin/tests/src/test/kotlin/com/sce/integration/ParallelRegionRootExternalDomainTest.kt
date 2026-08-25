// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: a `<parallel>` is not a transition domain — Kotlin AOT.
//
// §scxml-D-getTransitionDomain sends an external transition to `findLCCA`,
// which filters the source's proper ancestors with
// `isCompoundStateOrScxmlElement`. A `<parallel>` is neither a compound
// `<state>` nor the `<scxml>` element, so an external transition written on a
// REGION ROOT resolves to the document root: every region exits and re-enters,
// and a sibling region's transition on the same event is preempted because the
// two exit sets intersect and the sibling's source is not a descendant.
//
// This channel is the LAST of the six to get this witness, and the only one
// that needed no engine repair to pass it. That is the whole reason it was
// missing: Kotlin was already the sole engine applying the filter, which is how
// the divergence was found at all — measured 2026-08-25 on
// `examples/ai_loop/ai_loop.scxml`, this engine ended `session.lost` in
// `[alive, restarting]` where C++, Rust and Go ended in
// `[rebuilding, restarting]`. The five siblings were then repaired against this
// document and each got a driver; this one kept the rule and got none. An
// engine that is right today with nothing asserting it is an engine a later
// edit can regress silently, and being the reference is not a reason to be the
// unmeasured channel — it is the reason to be measured.
//
// Fixture: tests/integration/parallel_region_root_external_domain.scxml
// (canonical, shared verbatim with the C++ AOT / C++ Interpreter / Rust / Go /
// Python / C11 drivers). It sits beside its first driver rather than under
// `integration_resources/` because a stem there is a seven-channel contract and
// this document has six drivers — there is no mesh channel for it.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_region_root_external_domain_kotlin.sh

package com.sce.integration

import com.sce.integration.parallel_region_root_external_domain.ParallelRegionRootExternalDomainEvent
import com.sce.integration.parallel_region_root_external_domain.ParallelRegionRootExternalDomainState
import com.sce.integration.parallel_region_root_external_domain.ParallelRegionRootExternalDomainStateMachine
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML Appendix D — a region root's external transition has the document
/// root as its domain, and its internal twin has the region.
@DisplayName("ParallelRegionRootExternalDomain — W3C SCXML Appendix D findLCCA")
class ParallelRegionRootExternalDomainTest {

    // The document declares `datamodel="null"`, so this machine takes no script
    // engine — unlike every other integration driver in this package.
    private fun started(): ParallelRegionRootExternalDomainStateMachine {
        val sm = ParallelRegionRootExternalDomainStateMachine()
        sm.initialize()
        return sm
    }

    private fun ParallelRegionRootExternalDomainStateMachine.step(
        event: ParallelRegionRootExternalDomainEvent,
    ) {
        send(event)
        tick()
    }

    private fun ParallelRegionRootExternalDomainStateMachine.holds(
        state: ParallelRegionRootExternalDomainState,
    ): Boolean = activeConfiguration.contains(state)

    // The whole configuration in the document's own words. Membership alone is
    // a poor failure message here: the defect's shape is one `<parallel>` region
    // holding a state it should have left, and naming only the state asked
    // about hides which region moved.
    private fun ParallelRegionRootExternalDomainStateMachine.where(): List<String> =
        activeConfiguration.map { nameOfState(it) }.sorted()

    /// The clause itself. Both halves are asserted separately because they fail
    /// for different reasons: the first says the domain was too narrow, the
    /// second says the preemption a document-root domain implies did not happen.
    @Test
    fun anExternalRegionRootTransitionExitsEveryRegion() {
        val sm = started()

        assertTrue(
            sm.holds(ParallelRegionRootExternalDomainState.Working) &&
                sm.holds(ParallelRegionRootExternalDomainState.Alive),
            "precondition: both regions start at their defaults; active: ${sm.where()}",
        )

        sm.step(ParallelRegionRootExternalDomainEvent.Restart)

        assertTrue(
            sm.holds(ParallelRegionRootExternalDomainState.Restarting),
            "the transition's own target must be entered; active: ${sm.where()}",
        )
        // The domain is the document root, so `watch` exited with everything
        // else and came back at its default. Reading `rebuilding` here means the
        // domain was resolved to `run` (or to `drive`) and `watch` was left
        // alone to answer the event itself.
        assertTrue(
            sm.holds(ParallelRegionRootExternalDomainState.Alive) &&
                !sm.holds(ParallelRegionRootExternalDomainState.Rebuilding),
            "an external transition on a region root has the DOCUMENT ROOT as its domain " +
                "(Appendix D findLCCA filters <parallel> out of the candidates), so every region " +
                "exits and re-enters and `watch` is back at its default while its own transition " +
                "on `restart` is preempted by document order; active: ${sm.where()}",
        )
    }

    /// The contrast, and the reason `examples/ai_loop/ai_loop.scxml` spells
    /// `type="internal"`. A test pinning only the external case would pass just
    /// as well on an engine that sent EVERY region-root transition to the
    /// document root.
    @Test
    fun anInternalRegionRootTransitionLeavesTheOtherRegion() {
        val sm = started()

        sm.step(ParallelRegionRootExternalDomainEvent.Hold)

        assertTrue(
            sm.holds(ParallelRegionRootExternalDomainState.Paused),
            "the transition's own target must be entered; active: ${sm.where()}",
        )
        // Domain is `drive`: the source is compound and the target is its
        // descendant, so `watch` never exits and answers the event itself.
        assertTrue(
            sm.holds(ParallelRegionRootExternalDomainState.Rebuilding) &&
                !sm.holds(ParallelRegionRootExternalDomainState.Alive),
            "an internal region-root transition has the region as its domain, so the sibling " +
                "region keeps its own answer to the same event; active: ${sm.where()}",
        )
    }
}
