// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: pending invokes start before the external dequeue. Swift AOT path.
//
// `mainEventLoop` completes the macrostep on eventless and internal transitions alone, then runs
// `invoke(inv)` for every state entered on the last iteration, and only then reaches
// `externalQueue.dequeue()`. An engine that folds the external drain into its macrostep completion
// loop consumes whatever `<onentry>` queued for the parent itself while the invoked children do not
// yet exist, so an autoforward child misses every event the parent queued on the way in.
//
// Fixture: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokePrecedesExternalDequeue {
    @Test func pendingInvokesStartBeforeTheExternalDequeue() throws {
        let engine = try initializedOnVirtualTime(InvokePrecedesExternalDequeuePolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)); the watching child answered `probe` from `waiting`, so it never saw `kick`. The parent drained its external queue before starting the invoke, and the event `<onentry>` had queued for itself was consumed while no child existed. W3C Appendix D `mainEventLoop` runs `invoke(inv)` for every state entered on the last iteration before it reaches `externalQueue.dequeue()`, so an autoforward child is live for the whole external queue."
        )
    }
}
