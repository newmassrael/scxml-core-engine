# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""docs/adr/0106 #111: the `_event.data` of an `error.*` event the engine raises
is the record SCE_MESH.md section 10.7.1 pins, `errorName`, `reason` and
`detail`, in the bytes every engine writes; the Rust twin is
`event::tests::a_communication_code_names_its_event_and_its_detail_is_escaped`.
"""

from __future__ import annotations

from sce_runtime import ErrorReason, error_data, error_name, platform_error


def test_an_execution_code_is_raised_as_error_execution_with_its_detail() -> None:
    metadata = platform_error(ErrorReason.ASSIGNMENT_FAILED, "<assign> to 'x' failed", send_id="s1")
    assert metadata.data == (
        '{"errorName":"execution","reason":"assignmentFailed","detail":"<assign> to \'x\' failed"}'
    )
    assert metadata.event_type == "platform"
    assert metadata.send_id == "s1"


def test_a_communication_code_names_its_event_and_its_detail_is_escaped() -> None:
    assert error_name(ErrorReason.EXPRESSION_FAILED) == "execution"
    assert error_name(ErrorReason.PAYLOAD_REFUSED) == "execution"
    assert error_name(ErrorReason.TARGET_UNREACHABLE) == "communication"
    assert error_data(ErrorReason.TARGET_UNREACHABLE, 'say "hi"\n') == (
        '{"errorName":"communication","reason":"targetUnreachable","detail":"say \\"hi\\"\\n"}'
    )
