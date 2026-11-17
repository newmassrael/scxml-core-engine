// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// docs/adr/0106 #111: the `_event.data` of an `error.*` event the engine raises is
// the record SCE_MESH.md section 10.7.1 pins, `errorName`, `reason` and `detail`, in
// the bytes every engine writes; the Rust twin is
// `event::tests::a_communication_code_names_its_event_and_its_detail_is_escaped`.

@Test func anExecutionCodeIsRaisedAsErrorExecutionWithItsDetail() {
    let raised = EventWithMetadata<Int>.platformError(
        7, reason: .assignmentFailed, message: "<assign> to 'x' failed")
    #expect(
        raised.metadata.data
            == #"{"errorName":"execution","reason":"assignmentFailed","detail":"<assign> to 'x' failed"}"#)
}

@Test func aCommunicationCodeNamesItsEventAndItsDetailIsEscaped() {
    #expect(errorName(.expressionFailed) == "execution")
    #expect(errorName(.payloadRefused) == "execution")
    #expect(errorName(.targetUnreachable) == "communication")
    #expect(
        errorData(.targetUnreachable, detail: "say \"hi\"\n")
            == #"{"errorName":"communication","reason":"targetUnreachable","detail":"say \"hi\"\n"}"#)
}
