// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard transition keeps its own guard and its own type (W3C SCXML 3.12.1 + 3.13). Swift AOT
// path.
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct WildcardInDocumentOrder {
    @Test func aWildcardKeepsItsGuardAndItsType() throws {
        let engine = Engine(policy: WildcardInDocumentOrderPolicy())
        try engine.initialize()
        // The machine raises its own events, so it is finished by the ticks: the run is let go on
        // for as long as the other backends give a `simple` fixture.
        _ = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))

        // Each failure final names the case, so this one assertion says which property of the
        // wildcard was lost:
        //   failGuardIgnored              a wildcard fired with its guard false
        //   failGuardNeverFired           a wildcard did not fire with its guard true
        //   failGuardedInternalReentered  a guarded internal wildcard exited its source
        //   failSealedInternalReentered   an unguarded internal wildcard exited its source
        // A machine resting in `guardedFrom` or `sealedFrom` took no internal wildcard at all.
        #expect(
            engine.getCurrentState() == .pass,
            "the machine rested in \(engine.getCurrentState()), not in `pass`: a wildcard is enabled only when its guard is true, and an internal wildcard targeting a descendant of its compound source must not exit and re-enter that source"
        )
    }
}
