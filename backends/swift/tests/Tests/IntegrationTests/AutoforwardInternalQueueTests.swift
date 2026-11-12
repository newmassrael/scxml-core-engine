// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an internal-queue event never reaches an autoforward child. Swift AOT path.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AutoforwardInternalQueue {
    @Test func anInternalQueueEventIsNeverAutoforwarded() throws {
        let engine = Engine(policy: AutoforwardInternalQueuePolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): the watcher saw `error.execution`, so an internal-queue event was autoforwarded. The main event loop forwards only what it dequeues from the external queue, and the clause raises `error.execution` onto the internal one: check that the event was not routed onto the external queue for some unrelated reason, which would leak it past any name-blind forward."
        )
    }
}
