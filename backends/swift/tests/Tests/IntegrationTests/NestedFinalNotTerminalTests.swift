// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: a nested `<final>` completes its compound state, not the session. Swift AOT path.
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct NestedFinalNotTerminal {
    @Test func aNestedFinalDoesNotEndTheSession() throws {
        let engine = Engine(policy: NestedFinalNotTerminalPolicy())
        try engine.initialize()

        #expect(
            engine.getCurrentState() == .phaseDone,
            "the fixture is supposed to come to rest in the nested <final>; it did not, so nothing below is testing what it claims"
        )
        #expect(
            !engine.isInFinalState,
            "the engine reported completion while resting in `phaseDone`, a <final> nested inside `phase`: the session ends only when the final's parent is the <scxml> element, and a nested one finishes its compound state and queues done.state.phase, leaving the machine live"
        )

        engine.processEvent(.resume)

        #expect(
            engine.getCurrentState() == .pass,
            "`resume` did not carry the machine out of the nested final to the top-level one"
        )
        #expect(engine.isInFinalState, "the top-level final ends the session")
    }
}
