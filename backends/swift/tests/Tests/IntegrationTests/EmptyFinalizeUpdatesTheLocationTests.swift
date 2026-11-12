// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.5.2: an empty `<finalize>` updates; an absent one must not. Swift AOT path.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct EmptyFinalizeUpdatesTheLocation {
    @Test func anEmptyFinalizeUpdatesTheLocationAndAnAbsentOneDoesNot() throws {
        let engine = Engine(policy: EmptyFinalizeUpdatesTheLocationPolicy())
        try engine.initialize()

        // Each phase is settled by a 3 s delayed `<send>`: a child that never answers must reach a
        // verdict rather than hang.
        _ = tickUntil(engine, timeout: .seconds(15)) { engine.isInFinalState }

        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)). failNotUpdated: the empty <finalize/> left `tally` at its old value: an empty element means the automatic update, writing each namelist item's location as if by <assign> with the matching return value. failUpdatedWithoutFinalize: `guard` moved with no <finalize> element at all: the clause's note is a prohibition, not an omission. failUnmatchedNameWrote: an event carrying no matching name still wrote `keeper`: the clause says \"with ANY return value that has a name that matches\", so an unconditional write blanks the parent's data model on every unrelated answer. failEmptyChildSilent / failAbsentChildSilent / failUnmatchedChildSilent: a child never answered, so that half was never exercised."
        )
    }
}
