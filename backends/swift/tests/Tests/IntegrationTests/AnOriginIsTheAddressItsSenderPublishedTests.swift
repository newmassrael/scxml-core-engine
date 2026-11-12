// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the SCXML Event I/O
// Processor delivers is the address its sender published, on every route a session sends by, and a
// reply to it arrives. Swift AOT path.
//
// A self-send (written with the alias `type="scxml"`), a child's send to `#_parent` and a parent's
// send to `#_<invokeid>`: each time `_event.origin` is the sender's `_ioprocessors` SCXML location,
// `_event.origintype` is the processor's URI, and a reply sent with `targetexpr="_event.origin"
// typeexpr="_event.origintype"` arrives, the child's reply to its parent's location included, which
// reaches the parent as `#_parent` does.
//
// Fixture: integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// (canonical, shared with the other channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnOriginIsTheAddressItsSenderPublished {
    @Test func anOriginIsTheAddressItsSenderPublished() throws {
        let policy = AnOriginIsTheAddressItsSenderPublishedPolicy()
        let engine = try initializedOnVirtualTime(policy)
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.terminalState == .done,
            "the run must end in `done`; it waits in \(engine.getCurrentState()), where a reply sent to an origin that reached nobody leaves it"
        )
        let wrong = mismatches([
            ("selfOk", 1), ("selfReplied", 1), ("childOk", 1), ("childReplied", 1), ("parentOk", 1),
            ("parentReplied", 1),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
