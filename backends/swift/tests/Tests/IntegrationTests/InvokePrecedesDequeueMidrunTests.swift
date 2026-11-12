// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: the invoke-before-dequeue order holds mid-run, not only at start-up.
// Swift AOT path.
//
// `mainEventLoop` is one loop. Every iteration completes a macrostep, starts the invokes for the
// states that macrostep entered, and only then dequeues, so a state entered by an external event's
// transition arms an invoke that must start before the next event comes off the queue. An engine
// that drains the external queue to exhaustion inside one step satisfies the start-up ordering and
// still loses this one: what the entered state's `<onentry>` queued is consumed while the invoke is
// still pending, and an autoforward child misses it.
//
// Fixture: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokePrecedesDequeueMidrun {
    @Test func pendingInvokesStartBeforeTheDequeueMidRun() throws {
        let engine = try initializedOnVirtualTime(InvokePrecedesDequeueMidrunPolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)); the watching child answered `probe` from `waiting`, so it never saw `kick`. The parent drained its external queue before starting the invoke, and the event `<onentry>` had queued for itself was consumed while no child existed. W3C Appendix D `mainEventLoop` runs `invoke(inv)` for every state entered on the last iteration before it reaches `externalQueue.dequeue()`, so an autoforward child is live for the whole external queue."
        )
    }
}
