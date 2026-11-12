// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: an `<invoke>` names its child through an expression, and the expression cannot be
// evaluated. The clause places the obligation on the Processor: evaluation happens when the invoke
// fires, and a failure raises `error.execution`. No child starts. Swift AOT path.
//
// The invoke declares no candidates, so the child it would start is the stub the build writes for it
// (an immediate `<final>`): a successful evaluation would deliver `done.invoke` whatever the value
// was, which is why the fixture is built on the failure and not on the value.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokeExpressionFailureIsReported {
    @Test func anInvokeExpressionThatCannotBeEvaluatedRaisesErrorExecution() throws {
        let engine = try initializedOnVirtualTime(InvokeExpressionFailureIsReportedPolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState())). W3C SCXML 6.4.3 requires the expression to be evaluated when the <invoke> fires; parking means neither the raise nor the child arrived"
        )
        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)): reaching `fail` means the child started on an expression that cannot be evaluated, so nothing evaluated it"
        )
    }
}
