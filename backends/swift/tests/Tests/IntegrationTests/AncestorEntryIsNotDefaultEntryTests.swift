// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: an ancestor on the entry chain takes no default child. Swift AOT
// path.
//
// Fixture: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AncestorEntryIsNotDefaultEntry {
    @Test func anAncestorEnteredOnTheWayToATargetTakesNoDefaultChild() throws {
        // The fixture counts entries with `<assign>`, so this is an ECMAScript-datamodel machine.
        let engine = Engine(policy: AncestorEntryIsNotDefaultEntryPolicy())
        try engine.initialize()

        // `cross` enters the `<parallel>` itself, so `run` is a parallel ancestor and
        // `drive`/`outer` are compound ones; `back` then `again` repeat the entry with the
        // parallel already active, which is a different branch of the entry walk and the one a
        // running machine takes.
        engine.processEvent(.cross)
        engine.processEvent(.back)
        engine.processEvent(.again)
        engine.processEvent(.check)

        // The document checks its four clauses in document order and lands each in a `<final>` of
        // its own, so the state reported here names which one broke rather than saying only "not
        // settled".
        #expect(
            engine.getCurrentState() == .settled,
            "`check` did not carry the machine to `settled`: `failDefaulted` is a default child nobody targeted, `failLobbied` is `drive`'s default taken while it was only an ancestor, `failIdled` is the untouched region not getting its default, `failTargeted` is a pass that never reached the target"
        )
    }
}
