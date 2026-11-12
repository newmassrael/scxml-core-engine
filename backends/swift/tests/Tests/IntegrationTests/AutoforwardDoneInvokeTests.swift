// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: a sibling's `done.invoke` reaches the autoforward child. Swift AOT path.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AutoforwardDoneInvoke {
    @Test func doneInvokeFromASiblingReachesTheAutoforwardChild() throws {
        let engine = Engine(policy: AutoforwardDoneInvokePolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): the watcher saw only `probe`, so `done.invoke.inv_short` was withheld from a live `autoforward` child. The main event loop forwards every event dequeued from the external queue and excludes only the cancel event, and the clause places `done.invoke.<id>` on that queue: no name-based platform-event filter belongs on the forwarding path."
        )
    }
}
