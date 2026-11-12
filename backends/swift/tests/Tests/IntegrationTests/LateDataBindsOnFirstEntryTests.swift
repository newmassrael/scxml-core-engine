// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is bound on that state's
// FIRST entry and never again. Swift AOT path.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its <onentry> records the `v`
// it sees each time. A processor that bound late data on every entry would show the second entry
// 1 again.
//
// Fixture: integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct LateDataBindsOnFirstEntry {
    @Test func aStateBindsItsDataOnlyOnItsFirstEntry() throws {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel machine.
        let policy = LateDataBindsOnFirstEntryPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        #expect(engine.getCurrentState() == .idle, "the run has to start in `idle`")

        // Enter `s`, change its `v`, leave it, enter it again: one step each.
        for event in [LateDataBindsOnFirstEntryEvent.go, .bump, .back, .go] {
            engine.processEvent(event)
        }

        let seen =
            "entries=\(String(describing: policy.readScriptValue("entries"))) "
            + "seen=\(String(describing: policy.readScriptValue("seen"))) "
            + "contentSeen=\(String(describing: policy.readScriptValue("contentSeen"))) (wanted 2 / 15 / 7)"
        #expect(
            engine.getCurrentState() == .s,
            "the second `go` has to leave the machine in `s`. \(seen)")
        #expect(
            policy.readScriptValue("entries") == .integer(2),
            "both entries of `s` must have run. \(seen)")
        #expect(
            policy.readScriptValue("seen") == .integer(15),
            "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: 11 is a processor that binds late data on every entry, and no value at all one that never binds it. \(seen)"
        )
        #expect(
            policy.readScriptValue("contentSeen") == .integer(7),
            "`c` is bound from inline content, not an expr, and must be bound too. \(seen)")
    }
}
