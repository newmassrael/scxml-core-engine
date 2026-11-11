// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import SceTestProfile
import StaticDatamodelMachines
import XCTest

// The reader and the writer of the canonical form a test configuration's unit generates for each record
// its system ports take (docs/SCE_TTCN3_LANE.md §10.3, §10.5; docs/SCE_TEST_PROFILE.md §12.5), compiled
// here over records of every form Swift holds: order_placed (a list of records with a closed enum, an
// optional text and integer, a record with a truth value, a union of a case with a record, one with
// none and one with a byte string), job_status (a list of integers, optional integers) and
// closed_enum_form_extras (a list of optional elements, a 64-bit integer, a list with a lower count, an
// optional list and a list of lists). The units are generated from
// `sce-build/tests/fixtures/test_profile/trace_forms/` into the target `StaticDatamodelMachines` (regen:
// `scripts/regen_static_datamodel_swift.sh`).
//
// A value written by `canonical_<record>` and read back by `read_<record>` is the value it was, through
// the canonical text; each malformed spelling of a form is refused, naming where it stands; and a trace
// of one record of each type replays to a log that holds each value as the trace does.
//
// The twin of `backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs` and of the Kotlin
// `ARecordReadsBackTheFormItWritesTest.kt`, with the same values and the same refusals. Their
// configuration, forms_configuration, takes form_extras, whose value of an enum of an open set a Swift
// record does not hold, so the two refusals of that form have no twin here; the C++ twin takes
// order_taken in order_placed's place for the same kind of reason.

private typealias Forms = ClosedEnumFormsConfiguration
private typealias Extras = ClosedEnumFormExtrasPayload

/// An order paid by `payment`: two lines, one with a text of a character of two bytes and an escaped
/// quote, the widest order id, a note and no priority.
private func order(_ payment: OrderPayment) -> OrderPlacedPayload {
    OrderPlacedPayload(
        order_id: UInt32.max,
        lines: [
            OrderLinePayload(sku: "A-1", quantity: 2, color: .green),
            OrderLinePayload(sku: "\"q\" \u{E9}", quantity: UInt16.max, color: .blue),
        ],
        note: "ring twice",
        priority: nil,
        customer: OrderCustomerPayload(full_name: "Ada", vip: true),
        payment: payment)
}

/// The three cases of the payment: one of a record, one of none, one of a byte string.
private func payments() -> [OrderPayment] {
    [
        .card(CardPaymentPayload(last4: "4242", amount: 1)),
        .cash,
        .voucher(VoucherPaymentPayload(voucher_code: [0x00, 0xAB, 0xFF])),
    ]
}

private func statuses() -> [JobStatusPayload] {
    [
        JobStatusPayload(elapsed_ms: 0, note: nil, codes: [], tag: UInt8.max, retry: nil, count: 7),
        JobStatusPayload(elapsed_ms: 12, note: "late", codes: [3, 9, 5, 0], tag: nil, retry: 0, count: 0),
    ]
}

/// The extras: a list with an absent element, the least 64-bit integer, a list at its lower count and
/// at its bound, an optional list present and absent, and a list of lists with an empty one.
private func extras() -> [Extras] {
    [
        Extras(slots: [1, nil, UInt16.max], least: Int64.min, flags: [true], spare: [0, UInt8.max], grid: [[1, 2], []]),
        Extras(slots: [], least: Int64.max, flags: [false, true], spare: nil, grid: []),
    ]
}

/// The canonical text of `value`.
private func text(_ value: JSONValue) throws -> String {
    try writeCanonicalJSON(value)
}

/// `value` written by `write` in its canonical text, and read back from that text by `read`.
private func roundTrip<V: Equatable>(
    _ value: V, _ write: (V) -> JSONValue, _ read: (JSONValue) throws(TraceRefusal) -> V,
    file: StaticString = #filePath, line: UInt = #line
) throws {
    let written = try text(write(value))
    let parsed = try parseJSON(written)
    XCTAssertEqual(try read(parsed), value, written, file: file, line: line)
}

/// The reason `read` refuses the canonical text of `value` with `from` replaced once by `to` for.
private func refused<V>(
    _ value: V, _ write: (V) -> JSONValue, _ read: (JSONValue) throws(TraceRefusal) -> V, _ from: String,
    _ to: String
) throws -> String {
    let written = try text(write(value))
    guard let range = written.range(of: from) else {
        XCTFail("`\(from)` in \(written)")
        return ""
    }
    let changed = written.replacingCharacters(in: range, with: to)
    let parsed = try parseJSON(changed)
    do throws(TraceRefusal) {
        _ = try read(parsed)
    } catch {
        return error.reason
    }
    XCTFail("\(changed) is read")
    return ""
}

final class ARecordReadsBackTheFormItWritesTests: XCTestCase {
    /// docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the text its writer gives as
    /// the value it was.
    func testAValueOfEachFormReadsBackFromItsCanonicalText() throws {
        for payment in payments() {
            try roundTrip(order(payment), Forms.canonical_order_placed, Forms.read_order_placed)
        }
        for status in statuses() {
            try roundTrip(status, Forms.canonical_job_status, Forms.read_job_status)
        }
        for extra in extras() {
            try roundTrip(extra, Forms.canonical_closed_enum_form_extras, Forms.read_closed_enum_form_extras)
        }
    }

    /// docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is refused, naming the place it
    /// stands at: the record's type, each field by its id and `[]` for an element of a list.
    func testAMalformedSpellingOfEachFormIsRefusedByName() throws {
        let all = payments()
        let card = order(all[0])
        let cash = order(all[1])
        let voucher = order(all[2])
        func ofOrder(_ value: OrderPlacedPayload, _ from: String, _ to: String) throws -> String {
            try refused(value, Forms.canonical_order_placed, Forms.read_order_placed, from, to)
        }
        func ofExtras(_ from: String, _ to: String) throws -> String {
            try refused(
                extras()[0], Forms.canonical_closed_enum_form_extras, Forms.read_closed_enum_form_extras, from, to)
        }
        func ofStatus(_ from: String, _ to: String) throws -> String {
            try refused(statuses()[0], Forms.canonical_job_status, Forms.read_job_status, from, to)
        }
        let longName = "\"full_name\":\"\(String(repeating: "x", count: 25))\""
        let fiveLines =
            "\"lines\":[{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
            + "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
            + "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
        let cases: [(String, String)] = try [
            (
                ofOrder(card, "\"order_id\":\"4294967295\"", "\"order_id\":\"007\""),
                "order_placed.order_id: '007' is not the text of an integer's decimal digits"
            ),
            (
                ofOrder(card, "\"quantity\":\"2\"", "\"quantity\":\"65536\""),
                "order_placed.lines[].quantity: 65536 is past the range of its type"
            ),
            (
                ofOrder(card, "\"vip\":true", "\"vip\":\"true\""),
                "order_placed.customer.vip: a text is not a truth value"
            ),
            (
                ofOrder(card, "\"full_name\":\"Ada\"", longName),
                "order_placed.customer.full_name: a text of 25 bytes is past the 24 its type holds"
            ),
            (
                ofOrder(card, "\"color\":\"green\"", "\"color\":\"purple\""),
                "order_placed.lines[].color: 'purple' names no variant of its enum"
            ),
            (
                ofOrder(card, "\"note\":\"ring twice\"", "\"note\":null"),
                "order_placed.note: null is not a text"
            ),
            (
                ofOrder(card, "{\"customer\":", "{\"rush\":\"1\",\"customer\":"),
                "order_placed: 'rush' is no field of its record"
            ),
            (
                ofOrder(card, "\"order_id\":\"4294967295\",", ""),
                "order_placed.order_id: the field is absent, and its type is not optional"
            ),
            (
                ofOrder(card, "\"lines\":[", fiveLines),
                "order_placed.lines: a list of 5 elements is outside its bounds, 0 to 4"
            ),
            (
                ofOrder(card, "{\"card\":", "{\"cheque\":"),
                "order_placed.payment: 'cheque' names no case of its union"
            ),
            (
                ofOrder(card, "\"last4\":\"4242\"}}", "\"last4\":\"4242\"},\"cash\":{}}"),
                "order_placed.payment: a value of a union is an object of one member, and this one has 2"
            ),
            (
                ofOrder(cash, "{\"cash\":{}}", "{\"cash\":{\"x\":\"1\"}}"),
                "order_placed.payment.cash: a case that holds no record is the empty object"
            ),
            (
                ofOrder(voucher, "\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"00abff\""),
                "order_placed.payment.voucher_code: '00abff' is not octets in upper-case hex, two digits each"
            ),
            (
                ofOrder(voucher, "\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"0011223344\""),
                "order_placed.payment.voucher_code: a byte string of 5 octets is past the 4 its type holds"
            ),
            (
                ofStatus("\"tag\":\"255\"", "\"tag\":\"256\""),
                "job_status.tag: 256 is past the range of its type"
            ),
            (
                ofStatus("\"codes\":[]", "\"codes\":[\"x\"]"),
                "job_status.codes[]: 'x' is not the text of an integer's decimal digits"
            ),
            (
                ofExtras("\"slots\":[\"1\",null,", "\"slots\":[\"1\",\"x\","),
                "closed_enum_form_extras.slots[]: 'x' is not the text of an integer's decimal digits"
            ),
            (
                ofExtras("\"flags\":[true]", "\"flags\":[]"),
                "closed_enum_form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2"
            ),
            (
                ofExtras("\"grid\":[[\"1\",\"2\"]", "\"grid\":[[\"1\",\"2\",\"3\"]"),
                "closed_enum_form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2"
            ),
            (
                ofExtras("\"least\":\"-9223372036854775808\"", "\"least\":\"-9223372036854775809\""),
                "closed_enum_form_extras.least: -9223372036854775809 is past the range of its type"
            ),
            (
                ofExtras("\"spare\":[\"0\",\"255\"]", "\"spare\":null"),
                "closed_enum_form_extras.spare: null is not a list, which is an array"
            ),
        ]
        for (reason, expected) in cases {
            XCTAssertEqual(reason, expected)
        }
    }

    /// docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the system port takes, two at the
    /// start and one later, replays to the listener, which takes one a round and passes; the log holds
    /// each value as the trace does.
    func testATraceOfEveryFormReplaysToTheValuesItHolds() throws {
        let input = { (at: TestInstant, messageType: String, value: JSONValue) in
            TraceInput(at: at, port: "sut", messageType: messageType, value: value)
        }
        let trace = Trace(
            suite: "forms_suite", testCase: ReplaysEveryClosedEnumForm.name,
            inputs: [
                input(0, "order_placed", Forms.canonical_order_placed(order(payments()[0]))),
                input(0, "job_status", Forms.canonical_job_status(statuses()[0])),
                input(250, "closed_enum_form_extras", Forms.canonical_closed_enum_form_extras(extras()[0])),
            ])
        let written = trace.canonicalText()
        let suite = Suite(name: "forms_suite", executes: [Execute(ReplaysEveryClosedEnumForm.testCase())])
        let results = try runSuiteWithTraces(
            suite, world: [], traces: { (_: String) throws(TraceRefusal) -> Trace in try Trace.parse(written) })
        XCTAssertEqual(results.count, 1)
        let only = try XCTUnwrap(results.first)
        XCTAssertEqual(only.result.verdict, .pass, "\(only.result.log)")
        XCTAssertEqual(only.result.endedAt, 250)
        let log = only.canonicalLog(suite: "forms_suite")
        for each in trace.inputs {
            let value = try text(each.value)
            let received =
                "\"kind\":\"message-received\",\"port\":\"sut\",\"type\":\"\(each.messageType)\",\"value\":\(value)"
            XCTAssertTrue(log.contains(received), "`\(received)` in \(log)")
        }
    }
}
