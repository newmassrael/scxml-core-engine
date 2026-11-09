// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import SceTestProfile
import Testing

// The canonical form of a value in the test log, a message's value, and the records of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), each written in the JSON Canonicalization Scheme. Each case is the
// twin of the Rust case of the same name in backends/rust/runtime/src/test/canonical.rs, port.rs and
// test_log.rs (`an_integer_is_the_text_of_its_decimal_digits` is `anIntegerIsTheTextOfItsDecimalDigits`)
// and of the Kotlin one in TestLogTest.kt, with the same bytes. Every text is compared by its UTF-8
// bytes, which is what two logs agree by.

/// The bytes `writeCanonicalJSON` writes of `value`.
private func written(_ value: JSONValue) throws -> [UInt8] {
    Array(try writeCanonicalJSON(value).utf8)
}

/// The bytes of `text`.
private func bytes(_ text: String) -> [UInt8] {
    Array(text.utf8)
}

struct CanonicalTests {
    /// An integer is the text of its digits at every width, the ends of the 64-bit ones among them,
    /// which a double does not hold exactly.
    @Test func anIntegerIsTheTextOfItsDecimalDigits() throws {
        #expect(try written(Canonical.integer(UInt8(0))) == bytes("\"0\""))
        #expect(try written(Canonical.integer(Int8.min)) == bytes("\"-128\""))
        #expect(try written(Canonical.integer(UInt64.max)) == bytes("\"18446744073709551615\""))
        #expect(try written(Canonical.integer(Int64.min)) == bytes("\"-9223372036854775808\""))
    }

    @Test func aBoolIsATruthValueAndATextAText() throws {
        #expect(try written(Canonical.boolean(true)) == bytes("true"))
        #expect(try written(Canonical.boolean(false)) == bytes("false"))
        #expect(try written(Canonical.text("a\"b")) == bytes("\"a\\\"b\""))
    }

    /// Two hex digits an octet, upper case, the leading zero kept.
    @Test func aByteStringIsItsOctetsInUpperCaseHex() throws {
        #expect(try written(Canonical.bytes([0x00, 0xAB, 0x0F, 0xFF])) == bytes("\"00AB0FFF\""))
        #expect(try written(Canonical.bytes([])) == bytes("\"\""))
    }

    /// A closed enum by its declared name; an open one by its name, or by its number where no variant
    /// declares it.
    @Test func anEnumIsItsDeclaredName() throws {
        #expect(try written(Canonical.enumerated("red")) == bytes("\"red\""))
        #expect(try written(Canonical.openEnumerated("red", UInt8(0))) == bytes("\"red\""))
        #expect(try written(Canonical.openEnumerated(nil, UInt8(7))) == bytes("\"7\""))
    }

    /// An absent field is omitted, and the fields are sorted when written, whatever order they were
    /// given in.
    @Test func aRecordOmitsAnAbsentField() throws {
        let value = Canonical.record(
            ("quantity", Canonical.integer(UInt16(2))),
            ("note", nil),
            ("color", Canonical.enumerated("red")))
        #expect(try written(value) == bytes("{\"color\":\"red\",\"quantity\":\"2\"}"))
        #expect(try written(Canonical.record(("note", nil))) == bytes("{}"))
    }

    @Test func aListIsItsElementsInOrder() throws {
        let value = Canonical.list([UInt8(3), 1, 2].map { Canonical.integer($0) })
        #expect(try written(value) == bytes("[\"3\",\"1\",\"2\"]"))
        #expect(try written(Canonical.list([])) == bytes("[]"))
    }

    /// An absent element of a list is `null` in its place, and a present one its value: the array keeps
    /// every position.
    @Test func anAbsentElementOfAListIsNull() throws {
        let tags: [UInt8?] = [1, nil]
        let value = Canonical.list(tags.map { tag -> JSONValue in
            Canonical.optionalElement(tag.map { held -> JSONValue in Canonical.integer(held) })
        })
        #expect(try written(value) == bytes("[\"1\",null]"))
    }

    /// A union is one member named by its case's variant: the case's record, or the empty object for a
    /// case that holds none.
    @Test func aUnionIsOneMemberNamedByItsVariant() throws {
        let card = Canonical.unionCase("card", Canonical.record(("amount", Canonical.integer(UInt32(1)))))
        #expect(try written(card) == bytes("{\"card\":{\"amount\":\"1\"}}"))
        #expect(try written(Canonical.unionCase("cash", nil)) == bytes("{\"cash\":{}}"))
    }
}

struct MessageValueTests {
    /// A message's value is read back by its type, and by no other, and its canonical form is the one
    /// its writer gives, whichever reader asks.
    @Test func aMessageValueIsReadByItsTypeAndWrittenByItsWriter() {
        let message = Message.of(
            event: "number", value: UInt32(7), typeName: "number", canonical: { Canonical.integer($0) })
        #expect(message.valueAs(UInt32.self) == 7)
        #expect(message.valueAs(Int.self) == nil)
        #expect(message.canonicalValue() == .text("7"))
        #expect(message.typeName == "number")
        #expect(String(describing: message.value) == "TestValue(typeName=number, ..)")
    }

    /// A send's request carries a message's value as its `HostSendValue`: two are equal where they hold
    /// equal values of one type under one type name, whatever their writers, and the processor that
    /// serves the send reads it back as a `TestValue`.
    @Test func aMessageValueIsTheValueOfASendsRequest() throws {
        let three = HostSendValue(messageValue(UInt8(3), "n", { Canonical.integer($0) }))
        #expect(three == HostSendValue(messageValue(UInt8(3), "n", { _ in Canonical.record() })))
        #expect(three != HostSendValue(messageValue(UInt8(4), "n", { Canonical.integer($0) })))
        #expect(three != HostSendValue(messageValue(UInt8(3), "m", { Canonical.integer($0) })))
        #expect(three != HostSendValue(messageValue(UInt16(3), "n", { Canonical.integer($0) })))
        let served = try #require(three.value as? any TestValue)
        #expect(served.typeName == "n")
        #expect(served.canonical() == .text("3"))
    }
}

struct TestLogRecordTests {
    private func record(_ event: TestLogEvent) throws -> [UInt8] {
        try written(TestLogRecord(at: 250, event: event).toValue())
    }

    /// A record of each shape: `at` and every integer as the text of its digits, an absent reason
    /// omitted, the members sorted when written.
    @Test func aRecordIsWrittenWithTheMembersOfItsKind() throws {
        #expect(
            try record(.timerStarted(component: "mtc", timer: "reply", duration: 1_000_000))
                == bytes(
                    "{\"at\":\"250\",\"component\":\"mtc\",\"duration_us\":\"1000000\","
                        + "\"kind\":\"timer-started\",\"timer\":\"reply\"}"))
        #expect(
            try record(.verdictSet(component: "echo", verdict: .inconclusive, reason: nil))
                == bytes("{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\",\"verdict\":\"inconc\"}"))
        #expect(
            try record(.verdictSet(component: "echo", verdict: .fail, reason: "late"))
                == bytes(
                    "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\","
                        + "\"reason\":\"late\",\"verdict\":\"fail\"}"))
        let job = Canonical.record(("job", Canonical.integer(UInt32(7))))
        #expect(
            try record(.messageReceived(component: "mtc", port: "sut", messageType: "job_reply", value: job))
                == bytes(
                    "{\"at\":\"250\",\"component\":\"mtc\",\"kind\":\"message-received\","
                        + "\"port\":\"sut\",\"type\":\"job_reply\",\"value\":{\"job\":\"7\"}}"))
    }

    /// Every kind the runtime writes, each spelled as the schema names it.
    @Test func everyKindIsSpelledAsTheSchemaNamesIt() {
        let events: [TestLogEvent] = [
            .testCaseStarted,
            .testCaseEnded(verdict: .none),
            .componentCreated(component: "", componentType: ""),
            .componentDone(component: "", verdict: .none),
            .messageSent(component: "", port: "", messageType: "", value: .text("")),
            .messageReceived(component: "", port: "", messageType: "", value: .text("")),
            .timerStarted(component: "", timer: "", duration: 0),
            .timerStopped(component: "", timer: ""),
            .timerTimedOut(component: "", timer: ""),
            .verdictSet(component: "", verdict: .none, reason: nil),
            .log(component: "", text: ""),
        ]
        #expect(
            events.map(\.kind) == [
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
            ])
    }

    /// The log of a run: its shape, suite, test case and verdict before its records, all sorted, with no
    /// whitespace.
    @Test func aLogIsItsShapeSuiteCaseVerdictAndRecords() {
        let result = TestCaseResult(
            verdict: .error, endedAt: 0, log: [],
            records: [
                TestLogRecord(at: 0, event: .testCaseStarted),
                TestLogRecord(at: 0, event: .testCaseEnded(verdict: .error)),
            ])
        #expect(
            bytes(result.canonicalLog(suite: "a_suite", testCase: "a_case"))
                == bytes(
                    "{\"records\":[{\"at\":\"0\",\"kind\":\"test-case-started\"},"
                        + "{\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}],"
                        + "\"schema\":\"sce-test-log.v1\",\"suite\":\"a_suite\",\"test_case\":\"a_case\","
                        + "\"verdict\":\"error\"}"))
    }
}
