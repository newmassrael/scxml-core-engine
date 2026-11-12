// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + 6.2: a payload rides on its own event and on no other. Swift AOT path.
//
// Fixture: integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct APayloadRidesOnItsOwnEvent {
    @Test func eachPayloadArrivesOnItsOwnEvent() throws {
        let policy = APayloadRidesOnItsOwnEventPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        engine.processEvent(.finish)

        let seen =
            "got=\(String(describing: policy.readScriptValue("got"))) stolen=\(String(describing: policy.readScriptValue("stolen"))) plains=\(String(describing: policy.readScriptValue("plains"))) (wanted 4 / 0 / 4)"
        #expect(
            engine.terminalState == .done, "`finish` must carry the run to `done`. \(seen)")
        #expect(
            policy.readScriptValue("got") == .integer(4),
            "each payload event arrives carrying its own payload. \(seen)")
        #expect(
            policy.readScriptValue("stolen") == .integer(0),
            "no data-less event arrives carrying a payload. \(seen)")
        #expect(
            policy.readScriptValue("plains") == .integer(4),
            "every data-less event arrives, and arrives empty. \(seen)")
    }
}
