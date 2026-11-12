// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a self-transitioned region must still be able to answer. Swift AOT path.
//
// Fixture: integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ParallelSelfTransitionKeepsItsLeaf {
    @Test func theSelfTransitionedRegionAnswersTheNextEvent() throws {
        // The fixture's `<assign>`s make this an ECMAScript-datamodel machine.
        let engine = Engine(policy: ParallelSelfTransitionKeepsItsLeafPolicy())
        try engine.initialize()

        // Two `e`s, then the verdict. The second one is the point: `judging` has no `e`
        // transition, so nothing but the self-transitioning region can answer it, and it can only
        // answer from a leaf.
        engine.processEvent(.e)
        engine.processEvent(.e)
        engine.processEvent(.check)

        // `settled` is a top-level `<final>` guarded on `n == 1 && m == 2`, so reaching it says the
        // region still held `within` when the second `e` arrived AND ran its transition content
        // both times.
        #expect(
            engine.getCurrentState() == .settled,
            "`check` did not carry the machine to `settled`, which the document guards on `n == 1 && m == 2`; `m` reaches 2 only if the self-transitioning region still had a leaf to transition from when the second `e` arrived"
        )
    }
}
