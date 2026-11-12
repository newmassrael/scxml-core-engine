// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an event the host hands over reaches the autoforward child. Swift AOT path.
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct HostEventReachesTheChild {
    @Test func anEventTheHostHandsOverReachesTheAutoforwardChild() throws {
        let engine = Engine(policy: HostEventReachesTheChildPolicy())
        try engine.initialize()

        // The child opens the exchange, so let its `ready` move the parent into `armed`: the one
        // state that can be handed an event from outside.
        let armed = tickUntil(engine) { engine.getCurrentState() == .armed }
        #expect(
            armed,
            "the probe child never sent `ready`, so the fixture never reached the state where a host event can be handed over: this is a broken handshake, not a forwarding verdict"
        )

        // The axis: the host's own entry point.
        engine.processEvent(.hostPing)
        _ = tickUntil(engine) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "the probe child answered `sawMarkerOnly`, so the event the host handed to `processEvent` was never forwarded to it and the child only ever saw the `marker` the parent's own transition body sent. The main event loop runs the autoforward `send` against the external event before it selects transitions for it, whichever door the event arrived through: an engine that runs that step only in its queue drain leaves an `autoforward` child blind to everything its host delivers."
        )
    }
}
