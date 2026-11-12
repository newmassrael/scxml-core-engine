// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as the same value written in `target` is, at once
// or after a delay. Swift AOT path.
//
// Fixture: integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ATargetExpressionIsRoutedAsItsLiteralIs {
    @Test func aTargetExpressionIsRoutedAsItsLiteralIs() throws {
        let policy = ATargetExpressionIsRoutedAsItsLiteralIsPolicy()
        let engine = try initializedOnVirtualTime(policy)
        _ = driveOnVirtualTime(engine, budgetMs: 3000)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState()))")
        #expect(engine.terminalState == .done, "the run must end in `done`")
        let wrong = mismatches([
            ("internalNow", 1), ("internalLater", 1), ("kidNow", 1), ("kidLater", 1),
            ("sessNow", 1), ("sessLater", 1), ("commErrors", 4), ("execErrors", 2),
            ("afterStranger", 0), ("afterStrangerLater", 0), ("afterOrphan", 0),
            ("afterOrphanLater", 0), ("afterBogus", 0), ("afterBogusLater", 0),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
