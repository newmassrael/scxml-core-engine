// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import XCTest

@testable import StaticDatamodelMachines

// The Swift units of the test profile's templates (docs/SCE_TEST_PROFILE.md §6, §11.4, §12.6 slice 6),
// matched against concrete values: each mechanism of ES 201 873-1 Annex B.1 the profile holds, true and
// false, and the value of each specific template matching its own template.
//
// The Swift half of the Rust suite's `a_test_template_matches_its_values.rs`, of the Kotlin
// `ATestTemplateMatchesItsValuesTest.kt`, of the Go `a_test_template_matches_its_values_test.go`, of the
// Python `test_a_test_template_matches_its_values.py` and of the C11
// `test_a_test_template_matches_its_values.c`, which hold the same templates to the same values, each
// test named after the Rust case. Swift lowers no union, so the two templates over one, and their
// cases, have no twin here: the generator refuses them on Swift
// (`sce-build/tests/a_test_template_is_generated_in_swift.rs`). The units are generated from the
// fixtures under `sce-build/tests/fixtures/test_profile/` by `scripts/regen_static_datamodel_swift.sh`
// into the target `StaticDatamodelMachines`, each template an enum named by its document. Every
// character outside ASCII is written as its escape.
final class ATestTemplateMatchesItsValuesTests: XCTestCase {
    /// U+00E9, one character of two UTF-8 bytes.
    private let eAcute = "\u{E9}"

    /// The limit `every_matcher`'s range and reference are given.
    private let limit: UInt32 = 100

    /// A job status `every_matcher` matches with the limit 100: each field at a value its matcher holds,
    /// each optional field absent.
    private func matching() -> JobStatusPayload {
        JobStatusPayload(elapsed_ms: 1, note: nil, codes: [3, 9, 5], tag: nil, retry: nil, count: 100)
    }

    func testAStatusEachOfWhoseFieldsItsMatcherHoldsMatches() {
        XCTAssertTrue(EveryMatcher.matches(matching(), limit))
    }

    /// value-list (B.1.2.1) of a specific value and a range (B.1.2.5), both bounds inclusive, the upper
    /// one a formal parameter.
    func testAValueListMatchesAnyOfItsItemsAndARangeHoldsItsBounds() {
        for elapsed: UInt32 in [1, 10, 50, limit] {
            var status = matching()
            status.elapsed_ms = elapsed
            XCTAssertTrue(EveryMatcher.matches(status, limit), "\(elapsed)")
        }
        for elapsed: UInt32 in [0, 2, 9, limit + 1] {
            var status = matching()
            status.elapsed_ms = elapsed
            XCTAssertFalse(EveryMatcher.matches(status, limit), "\(elapsed)")
        }
    }

    /// any-value with ifpresent (B.1.4.2) and a length restriction (B.1.4.1) on an optional text: absent,
    /// or present with at most eight characters.
    func testIfpresentTakesAnAbsentFieldAndALengthCountsCharacters() {
        let cases: [(String?, Bool)] = [
            (nil, true),
            ("abcdefgh", true),
            ("abcdefghi", false),
            // Eight characters of two bytes each: sixteen bytes, eight characters.
            (String(repeating: eAcute, count: 8), true),
            (String(repeating: eAcute, count: 9), false),
        ]
        for (note, held) in cases {
            var status = matching()
            status.note = note
            XCTAssertEqual(EveryMatcher.matches(status, limit), held, "\(String(describing: note))")
        }
    }

    /// list-match (B.1.1): exactly three elements, the first 3, the second any value (B.1.3.1), the third
    /// not 0 (complement, B.1.2.2).
    func testAListMatchHoldsItsCountAndEachElementInOrder() {
        let matched: [[UInt8]] = [[3, 9, 5], [3, 0, 1]]
        for items in matched {
            var status = matching()
            status.codes = items
            XCTAssertTrue(EveryMatcher.matches(status, limit), "\(items)")
        }
        let refused: [[UInt8]] = [[3, 9, 0], [4, 9, 5], [3, 9], [3, 9, 5, 5], []]
        for items in refused {
            var status = matching()
            status.codes = items
            XCTAssertFalse(EveryMatcher.matches(status, limit), "\(items)")
        }
    }

    /// any-value-or-none (B.1.2.4) and omit (B.1.2.8).
    func testAnyValueOrNoneTakesBothAndOmitOnlyAnAbsentField() {
        var tagged = matching()
        tagged.tag = 7
        XCTAssertTrue(EveryMatcher.matches(tagged, limit), "a present tag matches any-value-or-none")
        XCTAssertTrue(EveryMatcher.matches(matching(), limit), "an absent tag matches any-value-or-none")
        var retried = matching()
        retried.retry = 1
        XCTAssertFalse(EveryMatcher.matches(retried, limit), "a present retry does not match omit")
    }

    /// template-ref (-1 §15.1): the referenced template's match, its formal parameter bound to this
    /// template's own.
    func testAReferenceMatchesByItsTemplateWithTheArgumentItBinds() {
        var status = matching()
        status.count = 150
        XCTAssertFalse(EveryMatcher.matches(status, limit), "a count past the limit")
        XCTAssertTrue(EveryMatcher.matches(status, 150), "a count at the limit")
        XCTAssertTrue(ElapsedAtMost.matches(150, 150))
        XCTAssertFalse(ElapsedAtMost.matches(151, 150))
    }

    /// A specific template's value (§6.2) matches the template, and a value that differs in one field
    /// does not.
    func testASpecificTemplatesValueMatchesItsTemplate() {
        let report = JobReport.value()
        XCTAssertEqual(
            report,
            JobStatusPayload(elapsed_ms: 0, note: "started on time", codes: [7], tag: nil, retry: nil, count: 1))
        XCTAssertTrue(JobReport.matches(report))
        var unnoted = report
        unnoted.note = nil
        XCTAssertFalse(JobReport.matches(unnoted), "an absent note")
        var retried = report
        retried.retry = 0
        XCTAssertFalse(JobReport.matches(retried), "a present retry")
        var counted = report
        counted.count = 2
        XCTAssertFalse(JobReport.matches(counted), "another count")
    }

    /// A specific template whose values are partly its parameters.
    func testASpecificTemplatesValueHoldsTheParametersItIsGiven() {
        let report = JobReportOf.value(5, "late", 9)
        XCTAssertEqual(
            report,
            JobStatusPayload(elapsed_ms: 5, note: "late", codes: [9, 255], tag: nil, retry: 2, count: 4_294_967_295))
        XCTAssertTrue(JobReportOf.matches(report, 5, "late", 9))
        XCTAssertFalse(JobReportOf.matches(report, 6, "late", 9), "another elapsed time")
        XCTAssertFalse(JobReportOf.matches(report, 5, "early", 9), "another note")
        XCTAssertFalse(JobReportOf.matches(report, 5, "late", 8), "another code")
    }

    /// An enum literal, and a reference to a specific template, which builds the field with that
    /// template's value.
    func testAnEnumLiteralAndAReferenceToASpecificTemplateBuildAndMatch() {
        let line = OrderLineExact.value()
        XCTAssertEqual(line, OrderLinePayload(sku: "A-1", quantity: 2, color: .red))
        XCTAssertTrue(OrderLineExact.matches(line))
        var blue = line
        blue.color = .blue
        XCTAssertFalse(OrderLineExact.matches(blue), "another color")
        var three = line
        three.quantity = 3
        XCTAssertFalse(OrderLineExact.matches(three), "another quantity")
    }

    /// A reference whose template matches a range up to the argument it binds.
    func testAReferenceToARangeHoldsTheBoundItsArgumentGives() {
        let line = OrderLinePayload(sku: "A-1", quantity: 5, color: .blue)
        XCTAssertTrue(OrderLineBounded.matches(line, 5), "a quantity at the bound")
        XCTAssertFalse(OrderLineBounded.matches(line, 4), "a quantity past the bound")
        var green = line
        green.color = .green
        XCTAssertFalse(OrderLineBounded.matches(green, 5), "another color")
    }

    /// A template that is specific in itself and references one that is not matches by the referenced
    /// template's range.
    func testATemplateMatchesThroughAReferenceToOneThatIsNotSpecific() {
        var status = JobReport.value()
        status.count = 60
        XCTAssertTrue(ReportWithin.matches(status, 60), "a count at the limit")
        XCTAssertFalse(ReportWithin.matches(status, 59), "a count past the limit")
    }

    /// A length restriction on a text counts characters, so a text whose bytes pass its upper bound
    /// matches while its characters do not; a range with a literal lower bound.
    func testALengthOnATextCountsAMultiByteCharacterAsOne() {
        func card(_ last4: String, _ amount: UInt32) -> CardPaymentPayload {
            CardPaymentPayload(last4: last4, amount: amount)
        }
        // Two characters in three bytes, and in four.
        for last4 in [eAcute + "1", eAcute + eAcute, "123"] {
            XCTAssertTrue(CardWithin.matches(card(last4, 50), 100), last4)
        }
        for last4 in ["1", eAcute, "1234"] {
            XCTAssertFalse(CardWithin.matches(card(last4, 50), 100), last4)
        }
        let amounts: [(UInt32, Bool)] = [(1, true), (0, false), (100, true), (101, false)]
        for (amount, held) in amounts {
            XCTAssertEqual(CardWithin.matches(card("12", amount), 100), held, "\(amount)")
        }
    }

    /// A byte string written in hex, compared by its octets.
    func testAByteStringLiteralIsItsOctets() {
        let voucher = VoucherIs.value()
        XCTAssertEqual(voucher.voucher_code, [0x00, 0xFF, 0x10, 0xA0])
        XCTAssertTrue(VoucherIs.matches(voucher))
        // Another array of the same octets matches too.
        XCTAssertTrue(
            VoucherIs.matches(VoucherPaymentPayload(voucher_code: [0x00, 0xFF, 0x10, 0xA0])),
            "another array of the same octets matches")
        var other = voucher.voucher_code
        other[3] = 0xA1
        XCTAssertFalse(
            VoucherIs.matches(VoucherPaymentPayload(voucher_code: other)), "other octets do not match")
    }

    /// The bytes of the canonical text of `value`, which two logs agree by.
    private func canonicalBytes(_ value: JSONValue) throws -> [UInt8] {
        Array(try writeCanonicalJSON(value).utf8)
    }

    /// docs/SCE_TEST_PROFILE.md §12.5: each unit's `canonical` writes its record by the types its
    /// documents declare, in the JSON Canonicalization Scheme: every integer as the text of its digits, a
    /// list as an array in order, an absent field omitted and a present one written, an enum by its
    /// declared name, a byte string in upper-case hex. `TYPE` is the unit name of the record's event
    /// schema.
    func testARecordsCanonicalFormIsWrittenByTheTypesItsDocumentsDeclare() throws {
        XCTAssertEqual(
            try canonicalBytes(JobReport.canonical(JobReport.value())),
            Array("{\"codes\":[\"7\"],\"count\":\"1\",\"elapsed_ms\":\"0\",\"note\":\"started on time\"}".utf8))
        let retried = JobStatusPayload(
            elapsed_ms: 4_294_967_295, note: nil, codes: [], tag: 3, retry: 2, count: 0)
        XCTAssertEqual(
            try canonicalBytes(JobReport.canonical(retried)),
            Array(
                "{\"codes\":[],\"count\":\"0\",\"elapsed_ms\":\"4294967295\",\"retry\":\"2\",\"tag\":\"3\"}".utf8))
        XCTAssertEqual(
            try canonicalBytes(OrderLineExact.canonical(OrderLineExact.value())),
            Array("{\"color\":\"red\",\"quantity\":\"2\",\"sku\":\"A-1\"}".utf8))
        XCTAssertEqual(
            try canonicalBytes(VoucherIs.canonical(VoucherIs.value())),
            Array("{\"voucher_code\":\"00FF10A0\"}".utf8))
        XCTAssertEqual(JobReport.TYPE, "job_status")
        XCTAssertEqual(OrderLineExact.TYPE, "order_line")
        XCTAssertEqual(VoucherIs.TYPE, "voucher_payment")
    }
}
