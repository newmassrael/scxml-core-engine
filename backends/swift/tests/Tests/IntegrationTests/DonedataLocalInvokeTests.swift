// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1: donedata on the `done.invoke` of a local invoke. Swift AOT path.
//
// Fixture: integration_resources/donedata_local_invoke/donedata_local_invoke.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct DonedataLocalInvoke {
    @Test func parentObservesDonedataOnDoneInvoke() throws {
        let engine = Engine(policy: DonedataLocalInvokePolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): the parent reached `fail`, so `_event.data.result === 42` (the param branch) or `_event.data === 'hello_content'` (the content branch) failed. An empty `_event.data` means the completion callback of the invoke dropped the child's donedata."
        )
    }
}
