// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: `_event.origin` is the sender's published location, and a peer can send back to
// it. Swift AOT path.
//
// The clause has two halves, and the fixture puts a second session on the other end so each has
// its own signal. A mismatch lands the parent in `fail`: `_event.origin` did not equal the location
// the child published for itself. A routing failure parks the parent in `await_reply`: a target
// that resolves nowhere delivers no event to fail on.
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct EventOriginIsALocation {
    @Test func originIsTheSendersPublishedLocationAndRoutesBack() throws {
        let engine = try initializedOnVirtualTime(EventOriginIsALocationPolicy())
        _ = driveOnVirtualTime(engine)

        let reached = engine.terminalState
        let why: String
        switch reached {
        case .fail:
            why =
                "`_event.origin` did not carry the sender's published `_ioprocessors` location. Appendix C.1 requires the origin to match that location, which is what makes it an address a peer can answer; a bare session id or an invoke-instance path matches nothing the sender published."
        case .pass:
            why = ""
        default:
            why =
                "parked in \(engine.getCurrentState()) rather than a verdict state. The parent accepted `_event.origin` as an address and sent `reply` to it, and nothing came back: C.1 requires the published location to be a usable <send> target, so an origin that routes nowhere fails the half a self-addressed test cannot exercise."
        }

        #expect(reached == .pass, "\(why)")
    }
}
