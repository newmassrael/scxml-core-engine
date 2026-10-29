// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// The transforms `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a formula per output, a static function of the caseless enum its own unit declares. The units
// are of the conformance corpus; each test reads a document's formula at the values that tell a
// right spelling from a near one.
final class TransformUnitTests: XCTestCase {
    // `transform_single_sum.scxml`: two `float32`, summed in `float32`. 2^24 + 1 is not a `float32`,
    // where a `double` makes it 16777217.
    func testASumOfSingleReadsIsRoundedToBinary32() {
        XCTAssertEqual(TransformSingleSum.computeTotal(1.5, 2.25), 3.75)
        XCTAssertEqual(TransformSingleSum.computeTotal(16_777_216, 1), 16_777_216)
    }

    // `transform_multi_output.scxml`: two outputs of one input, each its own function.
    func testEachOutputOfADocumentIsItsOwnFunctionOfTheSameInput() {
        XCTAssertEqual(TransformMultiOutput.computeFahrenheit(100), 212, accuracy: 1e-9)
        XCTAssertEqual(TransformMultiOutput.computeFahrenheit(-40), -40, accuracy: 1e-9)
        XCTAssertEqual(TransformMultiOutput.computeKelvin(0), 273.15, accuracy: 1e-9)
    }

    // `transform_bitwise.scxml`: a nibble of a byte, by shift and mask.
    func testANibbleIsTakenByShiftAndMask() {
        XCTAssertEqual(TransformBitwise.computeHighNibble(0xAB), 0x0A)
        XCTAssertEqual(TransformBitwise.computeLowNibble(0xAB), 0x0B)
        XCTAssertEqual(TransformBitwise.computeHighNibble(0xFF), 0x0F)
        XCTAssertEqual(TransformBitwise.computeLowNibble(0x00), 0x00)
    }

    // `transform_narrowing_wraps.scxml`: a conversion to a narrower integer keeps the low bits.
    func testANarrowingConversionKeepsTheLowBits() {
        XCTAssertEqual(TransformNarrowingWraps.computeLow(0x1FF, -1, 0), 0xFF)
        XCTAssertEqual(TransformNarrowingWraps.computeLowFromSigned(0, -1, 0), 0xFF)
        XCTAssertEqual(TransformNarrowingWraps.computeNarrowSigned(0, 0, 0x1_0000_0001), 1)
        XCTAssertEqual(TransformNarrowingWraps.computeNarrowSigned(0, 0, 0xFFFF_FFFF), -1)
    }

    // `transform_int_div.scxml`: an integer division truncates toward zero.
    func testAnIntegerDivisionTruncatesTowardZero() {
        XCTAssertEqual(TransformIntDiv.computeQuotient(7, 2), 3)
        XCTAssertEqual(TransformIntDiv.computeQuotient(-7, 2), -3)
        XCTAssertEqual(TransformIntDiv.computeQuotient(7, -2), -3)
    }

    // `transform_temperature.scxml`: a raw reading in tenths of a degree, `raw * 0.1 - 40`.
    func testARawReadingIsConvertedToItsPhysicalUnit() {
        XCTAssertEqual(TransformTemperature.computeTemperature(400), 0.0, accuracy: 1e-9)
        XCTAssertEqual(TransformTemperature.computeTemperature(0), -40.0, accuracy: 1e-9)
        XCTAssertEqual(TransformTemperature.computeTemperature(UInt16.max), 6513.5, accuracy: 1e-9)
    }
}
