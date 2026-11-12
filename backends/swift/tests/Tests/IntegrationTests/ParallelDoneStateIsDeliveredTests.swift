// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: the parallel's completion event reaches a listener. Swift AOT path.
//
// Fixture: integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ParallelDoneStateIsDelivered {
    @Test func completionCarriesTheMachineToATopLevelFinal() throws {
        // `datamodel="null"`: this machine needs no script engine.
        let engine = Engine(policy: ParallelDoneStateIsDeliveredPolicy())
        try engine.initialize()

        engine.processEvent(.go)

        #expect(
            engine.getCurrentState() == .settled,
            "every region reaching its `<final>` completes the parallel, so done.state.run had to be raised AND selected: `settled` is reachable by nothing else"
        )
        #expect(engine.isInFinalState, "`settled` is a top-level final, which ends the session")
    }
}
