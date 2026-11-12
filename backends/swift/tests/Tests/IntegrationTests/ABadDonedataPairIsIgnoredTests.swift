// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises error.execution and is
// ignored; the done events are raised all the same. Swift AOT path.
//
// Fixture: integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ABadDonedataPairIsIgnored {
    @Test func theBadPairsAreDroppedAndTheDoneEventsStillArrive() throws {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel machine. The run
        // needs nothing from the host.
        let policy = ABadDonedataPairIsIgnoredPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()

        let seen =
            "errors=\(String(describing: policy.readScriptValue("errors"))) "
            + "shape=\(String(describing: policy.readScriptValue("shape"))) (wanted 2 / 1)"
        #expect(
            engine.getCurrentState() == .done,
            "done.state.p must still arrive and carry the run to `done`. \(seen)")
        #expect(
            policy.readScriptValue("errors") == .integer(2),
            "each ignored pair raises its own error.execution. \(seen)")
        #expect(
            policy.readScriptValue("shape") == .integer(1),
            "done.state.r1 must carry the surviving pair and neither ignored one. \(seen)")
    }
}
