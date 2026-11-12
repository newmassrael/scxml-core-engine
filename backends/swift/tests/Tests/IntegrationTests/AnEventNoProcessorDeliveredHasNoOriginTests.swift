// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor delivered has no
// origin. Swift AOT path.
//
// An event the host posts itself and the reply a host processor answers a `<send>` with, both
// naming no party, read `_event.origin` and `_event.origintype` as the empty text. No engine invents
// an origin for them: `raiseExternal` used to give a host-posted event the SCXML processor's type
// with an empty address.
//
// Fixture: integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml
// (canonical, shared with the other channels), generated with `--host-processor x-sce-host`.
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// How many times the host's processor was handed a send.
private final class Served {
    var count = 0
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnEventNoProcessorDeliveredHasNoOrigin {
    @Test func anEventNoProcessorDeliveredHasNoOrigin() throws {
        let policy = AnEventNoProcessorDeliveredHasNoOriginPolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        let served = Served()
        // The host's processor answers the document's send with a reply that names no party.
        // Registered before the machine starts, as every host registers.
        try engine.registerEventProcessor("x-sce-host") { _ in
            served.count += 1
            return [HostSendResponse(eventName: "host.reply")]
        }
        try engine.initialize()
        engine.tick()

        // The host raises an event of its own, naming no origin.
        engine.raiseExternal(.posted)
        engine.tick()
        engine.tick()

        #expect(served.count == 1, "the host's processor was not handed the document's send once")
        #expect(engine.terminalState == .done, "the run must end in `done`; it stopped in \(engine.getCurrentState())")
        let wrong = mismatches([("postedBlank", 1), ("replyBlank", 1)]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "an event read an origin it was not given: \(wrong)")
    }
}
