// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: every region's `<final>` completes the parallel. Swift AOT path.
//
// Fixture: integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ParallelCompletionRaisesDoneState {
    @Test func everyRegionFinalCompletesTheParallel() throws {
        // `datamodel="null"`: this machine needs no script engine.
        let engine = Engine(policy: ParallelCompletionRaisesDoneStatePolicy())
        try engine.initialize()

        engine.processEvent(.go)

        // The whole configuration is read, which the Kotlin channel cannot do: a region that lost
        // its `<final>` is missing from it, and one that never left its leaf is too.
        #expect(
            Set(engine.getActiveStates()).isSuperset(of: [.a2, .b2]),
            "a region did not reach its `<final>` on `go`: \(engine.getActiveStates())"
        )
    }
}
