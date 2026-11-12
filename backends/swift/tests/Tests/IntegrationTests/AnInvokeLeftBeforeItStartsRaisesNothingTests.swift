// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never attempted, so the 6.4.1
// error.execution for a type no processor runs is never raised. Swift AOT path.
//
// Fixture: integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnInvokeLeftBeforeItStartsRaisesNothing {
    @Test func theLeftStatesInvokeIsNeverAttempted() throws {
        let policy = AnInvokeLeftBeforeItStartsRaisesNothingPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        #expect(
            engine.getCurrentState() == .s1,
            "`s0` leaves on an eventless transition within the first macrostep")

        engine.processEvent(.finish)

        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`")
        #expect(
            policy.readScriptValue("errors") == .integer(0),
            "`s0` left before its macrostep ended, yet its invoke was attempted and raised error.execution; W3C SCXML 6.4 cancels an invoke whose state has left"
        )
    }
}
