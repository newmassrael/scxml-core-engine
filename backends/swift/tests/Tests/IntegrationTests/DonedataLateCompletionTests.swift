// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1: donedata rides a late completion. Swift AOT path.
//
// Fixture: integration_resources/donedata_late_completion/donedata_late_completion.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct DonedataLateCompletion {
    @Test func donedataRidesACompletionAfterTheInvokeStarted() throws {
        // The parent's `done.invoke` guard and the child's `<donedata>` param both evaluate
        // expressions.
        let engine = Engine(policy: DonedataLateCompletionPolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): the parent's `done.invoke.inv_late` guard did not see `_event.data.result === 42`, so the child's `<donedata>` was dropped on a completion that happened after the invoke was started. The clause raises `done.invoke.<id>` wherever the child reaches its final state and puts that state's donedata on the event; neither is scoped to children that finalise during start-up."
        )
    }
}
