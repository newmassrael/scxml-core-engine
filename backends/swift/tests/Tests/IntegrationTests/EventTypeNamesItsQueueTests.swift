// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1: each event is typed by the queue it was taken from. Swift AOT path.
//
// Fixture: integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct EventTypeNamesItsQueue {
    @Test func eachEventIsTypedByTheQueueItWasTakenFrom() throws {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel machine. The
        // document queues its own events; the run needs nothing from the host.
        let policy = EventTypeNamesItsQueuePolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()

        let seen =
            "intCode=\(String(describing: policy.readScriptValue("intCode"))) "
            + "sendCode=\(String(describing: policy.readScriptValue("sendCode"))) "
            + "extCode=\(String(describing: policy.readScriptValue("extCode"))) "
            + "(1 internal, 2 external, 3 other; wanted 1 / 1 / 2)"
        #expect(engine.getCurrentState() == .done, "`ext` must carry the run to `done`. \(seen)")
        #expect(
            policy.readScriptValue("intCode") == .integer(1),
            "`int` came off the internal queue while `ext` waited on the external one. \(seen)")
        #expect(
            policy.readScriptValue("sendCode") == .integer(1),
            "a `<send target=\"#_internal\">` with a payload rides the internal queue too. \(seen)")
        #expect(
            policy.readScriptValue("extCode") == .integer(2),
            "`ext` came off the external queue. \(seen)")
    }
}
