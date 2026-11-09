# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The canonical form of a value in the test log, a message's value, and the
records of the test log (docs/SCE_TEST_PROFILE.md §12.5), each written in the
JSON Canonicalization Scheme.

Each case is the twin of the Rust case of the same name in
``backends/rust/runtime/src/test/canonical.rs``, ``port.rs`` and
``test_log.rs``, and of the Kotlin case in ``TestLogTest.kt``, named
``test_<case>`` after the Rust one, with the same bytes. The last case of
``TestCanonical`` holds what only a Python value has to: a ``bool`` is an
``int`` there, and is no integer of the profile.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime.saved_json import write_jcs  # noqa: E402 -- path inserted above
from sce_runtime.testprofile import (  # noqa: E402
    TEST_LOG_SCHEMA,
    Message,
    TestCaseResult,
    TestLogRecord,
    Verdict,
    canonical,
)
from sce_runtime.testprofile.testlog import (  # noqa: E402
    ComponentCreated,
    ComponentDone,
    Log,
    MessageReceived,
    MessageSent,
    TestCaseEnded,
    TestCaseStarted,
    TimerStarted,
    TimerStopped,
    TimerTimedOut,
    VerdictSet,
)


def written(value: object) -> str:
    return write_jcs(value)


class TestCanonical:
    def test_an_integer_is_the_text_of_its_decimal_digits(self) -> None:
        """An integer is the text of its digits at every width, the ends of the
        64-bit ones among them, which a double does not hold exactly."""
        assert written(canonical.integer(0)) == '"0"'
        assert written(canonical.integer(-128)) == '"-128"'
        assert written(canonical.integer(18446744073709551615)) == '"18446744073709551615"'
        assert written(canonical.integer(-9223372036854775808)) == '"-9223372036854775808"'

    def test_a_bool_is_a_truth_value_and_a_text_a_text(self) -> None:
        assert written(canonical.boolean(True)) == "true"
        assert written(canonical.boolean(False)) == "false"
        assert written(canonical.text('a"b')) == '"a\\"b"'

    def test_a_byte_string_is_its_octets_in_upper_case_hex(self) -> None:
        """Two hex digits an octet, upper case, the leading zero kept."""
        assert written(canonical.byte_string(b"\x00\xab\x0f\xff")) == '"00AB0FFF"'
        assert written(canonical.byte_string(b"")) == '""'

    def test_an_enum_is_its_declared_name(self) -> None:
        """A closed enum by its declared name; an open one by its name, or by its
        number where no variant declares it."""
        assert written(canonical.enumerated("red")) == '"red"'
        assert written(canonical.open_enumerated("red", 0)) == '"red"'
        assert written(canonical.open_enumerated(None, 7)) == '"7"'

    def test_a_record_omits_an_absent_field(self) -> None:
        """An absent field is omitted, and the fields are sorted when written,
        whatever order they were given in."""
        value = canonical.record(
            ("quantity", canonical.integer(2)),
            ("note", None),
            ("color", canonical.enumerated("red")),
        )
        assert written(value) == '{"color":"red","quantity":"2"}'
        assert written(canonical.record(("note", None))) == "{}"

    def test_a_list_is_its_elements_in_order(self) -> None:
        value = canonical.list_of(canonical.integer(n) for n in [3, 1, 2])
        assert written(value) == '["3","1","2"]'
        assert written(canonical.list_of([])) == "[]"

    def test_an_absent_element_of_a_list_is_null(self) -> None:
        """An absent element of a list is ``null`` in its place, and a present one
        its value: the array keeps every position."""
        value = canonical.list_of(
            canonical.optional_element(tag, canonical.integer) for tag in [1, None]
        )
        assert written(value) == '["1",null]'

    def test_a_union_is_one_member_named_by_its_variant(self) -> None:
        """A union is one member named by its case's variant: the case's record,
        or the empty object for a case that holds none."""
        card = canonical.union_case("card", canonical.record(("amount", canonical.integer(1))))
        assert written(card) == '{"card":{"amount":"1"}}'
        assert written(canonical.union_case("cash", None)) == '{"cash":{}}'

    def test_a_bool_is_no_integer_of_the_profile(self) -> None:
        """Python holds a ``bool`` as an ``int``; written as an integer it would
        be the text ``1``, so it is refused there."""
        with pytest.raises(TypeError):
            canonical.integer(True)


class TestPort:
    def test_a_message_value_is_read_by_its_type_and_written_by_its_writer(self) -> None:
        """A message's value is read as it was given, and its canonical form is
        the one its writer gives, with its type's name, whichever holder asks."""
        message = Message.of("number", 7, "number", canonical.integer)
        assert message.value == 7
        assert message.event == "number"
        assert message.canonical_value() == "7"
        assert message.type_name == "number"
        assert "number" in repr(message.test_value), "a value is shown by its type's name"


def record(event: object) -> str:
    return written(TestLogRecord(250, event).to_value())


class TestTestLogRecord:
    def test_a_record_is_written_with_the_members_of_its_kind(self) -> None:
        """A record of each shape: ``at`` and every integer as the text of its
        digits, an absent reason omitted, the members sorted when written."""
        assert record(TimerStarted("mtc", "reply", 1_000_000)) == (
            '{"at":"250","component":"mtc","duration_us":"1000000",'
            '"kind":"timer-started","timer":"reply"}'
        )
        assert record(VerdictSet("echo", Verdict.INCONCLUSIVE, None)) == (
            '{"at":"250","component":"echo","kind":"verdict-set","verdict":"inconc"}'
        )
        assert record(VerdictSet("echo", Verdict.FAIL, "late")) == (
            '{"at":"250","component":"echo","kind":"verdict-set","reason":"late",'
            '"verdict":"fail"}'
        )
        assert record(
            MessageReceived("mtc", "sut", "job_reply", canonical.record(("job", canonical.integer(7))))
        ) == (
            '{"at":"250","component":"mtc","kind":"message-received","port":"sut",'
            '"type":"job_reply","value":{"job":"7"}}'
        )

    def test_every_kind_is_spelled_as_the_schema_names_it(self) -> None:
        """Every kind the runtime writes, each spelled as the schema names it."""
        kinds = [
            TestCaseStarted(),
            TestCaseEnded(Verdict.NONE),
            ComponentCreated("", ""),
            ComponentDone("", Verdict.NONE),
            MessageSent("", "", "", ""),
            MessageReceived("", "", "", ""),
            TimerStarted("", "", 0),
            TimerStopped("", ""),
            TimerTimedOut("", ""),
            VerdictSet("", Verdict.NONE, None),
            Log("", ""),
        ]
        assert [event.kind for event in kinds] == [
            "test-case-started",
            "test-case-ended",
            "component-created",
            "component-done",
            "message-sent",
            "message-received",
            "timer-started",
            "timer-stopped",
            "timer-timed-out",
            "verdict-set",
            "log",
        ]

    def test_a_log_is_its_shape_suite_case_verdict_and_records(self) -> None:
        """The log of a run: its shape, suite, test case and verdict before its
        records, all sorted, with no whitespace."""
        result = TestCaseResult(
            verdict=Verdict.ERROR,
            ended_at=0,
            log=[],
            records=[
                TestLogRecord(0, TestCaseStarted()),
                TestLogRecord(0, TestCaseEnded(Verdict.ERROR)),
            ],
        )
        assert result.canonical_log("a_suite", "a_case") == (
            '{"records":[{"at":"0","kind":"test-case-started"},'
            '{"at":"0","kind":"test-case-ended","verdict":"error"}],'
            '"schema":"sce-test-log.v1","suite":"a_suite","test_case":"a_case",'
            '"verdict":"error"}'
        )
        assert TEST_LOG_SCHEMA == "sce-test-log.v1"

    def test_a_value_not_in_the_canonical_form_is_refused(self) -> None:
        """A message's value with a number in it, which no writer of
        ``canonical`` builds, has no canonical text: the log is refused, naming
        the suite and the test case, rather than written with a number another
        runtime would spell otherwise."""
        result = TestCaseResult(
            verdict=Verdict.PASS,
            ended_at=0,
            log=[],
            records=[TestLogRecord(0, MessageSent("mtc", "p", "number", 7))],
        )
        with pytest.raises(ValueError) as refused:
            result.canonical_log("a_suite", "a_case")
        assert "a_suite a_case" in str(refused.value)
