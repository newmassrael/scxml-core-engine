// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10: `_event.name` is the name the event was sent under. Swift AOT path.
//
// Fixture: integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnEventKeepsTheNameItWasSentUnder {
    @Test func aNameTheDocumentDoesNotWriteIsToldWhole() throws {
        let engine = try initializedOnVirtualTime(AnEventKeepsTheNameItWasSentUnderPolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)); the child reports `arrivedShortened` when `request.new` took the transition on `request` but `_event.name` told the child `request`. W3C 5.10 makes the name the one the event was sent under, and 3.12.1 only decides which transition takes it."
        )
    }
}
