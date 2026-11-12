// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1: an `<invoke>` naming an unsupported `type` raises `error.execution`. Swift AOT
// path.
//
// The spec defines the case ("the processor MUST place error.execution in the internal event
// queue"), so the document is valid SCXML with one observable: that raise. No child session starts
// and `done.invoke.<id>` never fires. A backend that renders this fixture without the raise rests in
// `probe` instead of reaching `pass`.
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InvokeUnsupportedType {
    @Test func anUnsupportedInvokeTypeRaisesErrorExecution() throws {
        let engine = try initializedOnVirtualTime(InvokeUnsupportedTypePolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState())). W3C SCXML 6.4.1 requires an <invoke> whose `type` names no supported processor to place error.execution on the internal queue; parking in `probe` means the <invoke> was dropped rather than lowered"
        )
        #expect(
            engine.terminalState == .pass,
            "the machine completed somewhere other than the error.execution target")
    }
}
