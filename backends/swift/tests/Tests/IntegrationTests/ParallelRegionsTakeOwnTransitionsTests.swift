// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a self-transition must not preempt a sibling region. Swift AOT path.
//
// Fixture: integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ParallelRegionsTakeOwnTransitions {
    @Test func everyRegionTakesItsOwnTransition() throws {
        // The fixture's `<assign>`s make this an ECMAScript-datamodel machine.
        let engine = Engine(policy: ParallelRegionsTakeOwnTransitionsPolicy())
        try engine.initialize()

        engine.processEvent(.e)
        engine.processEvent(.check)

        // `settled` is a top-level `<final>` guarded on `n == 1 && m == 1`, so reaching it says
        // both regions took their transition on `e` AND ran its content. A deeper region that was
        // preempted never enters `judging`, so `check` finds no enabled transition and the machine
        // stays inside the parallel.
        #expect(
            engine.getCurrentState() == .settled,
            "`check` did not carry the machine to `settled`, which the document guards on both regions' assignments having run"
        )
    }
}
