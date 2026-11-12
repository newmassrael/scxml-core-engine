// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: the parent forwards where it dequeues, not where it queues. Swift AOT path.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AutoforwardDequeuePoint {
    @Test func anExternalEventIsForwardedAtTheDequeueNotTheEnqueue() throws {
        let engine = Engine(policy: AutoforwardDequeuePointPolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): the probe child saw `second` before `mark`, so both events were handed over while the parent was still executing the transition that queued them. The main event loop forwards one statement after `externalQueue.dequeue()`, and the clause puts it \"at the point at which it removes it from the external event queue\": forwarding at the enqueue lets the child run ahead of the parent by a whole event."
        )
    }
}
