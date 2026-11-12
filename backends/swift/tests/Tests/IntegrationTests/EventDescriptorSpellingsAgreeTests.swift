// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1: every spelling of one event descriptor matches the same events. Swift AOT path.
//
// Fixture: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct EventDescriptorSpellingsAgree {
    @Test func everySpellingOfOneDescriptorCatchesTheSameEvents() throws {
        let engine = Engine(policy: EventDescriptorSpellingsAgreePolicy())
        try engine.initialize()
        // The machine raises its own events, so it is finished by the ticks: the run is let go on
        // for as long as the other backends give a `simple` fixture.
        _ = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))

        // Each failure final names the case, so this one assertion says which spelling disagreed
        // with the clause:
        //   failSuffixed   `wild.*` did not catch bare `wild`
        //   failDotted     `dot.` did not catch bare `dot`
        //   failBounded    `wild.*` caught `wilder`, across a token boundary
        //   failUniversal  a bare `.*` did not catch `any.token.sequence`
        #expect(
            engine.getCurrentState() == .pass,
            "the machine rested in \(engine.getCurrentState()), not in `pass`: `wild.*` must catch bare `wild` and `dot.` must catch bare `dot` (the clause calls the spellings functionally equivalent), a bare `.*` must catch every event, and `wild.*` must NOT catch `wilder` because the prefix is a whole token"
        )
    }
}
