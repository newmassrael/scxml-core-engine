// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10: two live sessions are issued different ids. Swift AOT path.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct SessionIdsAreDistinct {
    @Test func twoLiveSessionsAreIssuedDifferentIds() throws {
        let engine = Engine(policy: SessionIdsAreDistinctPolicy())
        try engine.initialize()
        // Two children report their `_sessionid` to the parent, which is only finished by the
        // ticks, so the run is let go on for as long as the other backends give a `simple`
        // fixture.
        _ = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))

        let reached = engine.getCurrentState()
        let why: String
        switch reached {
        case .fail:
            why =
                "two live sessions reported the same `_sessionid`. The clause binds it to the id of the current session, and C.1.1 publishes an address derived from it, so one id for two sessions is one address for two."
        case .pass:
            why = ""
        default:
            why =
                "parked in \(reached) rather than a verdict state: only one child reported its `_sessionid`, so the two ids were never compared."
        }

        #expect(reached == .pass, Comment(rawValue: why))
    }
}
