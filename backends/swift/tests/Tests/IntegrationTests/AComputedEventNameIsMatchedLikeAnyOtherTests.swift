// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1 + 5.10: a computed event name is matched like any other. Swift AOT path.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AComputedEventNameIsMatchedLikeAnyOther {
    @Test func aComputedNameIsMatchedByTokenAndToldWhole() {
        let engine = initializedOnVirtualTime(AComputedEventNameIsMatchedLikeAnyOtherPolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.terminalState == .pass,
            "a computed name was dropped, or matched but told shorter than the name it was sent under (`request.new`, `other.thing`, `request.again`, `last.one`); ended in \(String(describing: engine.terminalState))"
        )
    }
}
