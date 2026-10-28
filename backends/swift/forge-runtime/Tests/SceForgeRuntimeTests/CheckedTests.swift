// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// Each failure the contract names is reported by its contract name, and every value
// that fits comes back unchanged. The cases are those of
// backends/go/forge-runtime/algorithm/algorithm_test.go.

private struct Case: Sendable {
    var name: String
    var run: @Sendable (inout Failure) -> Int64
    var want: Int64
    var fail: String?
}

private let cases: [Case] = [
    Case(name: "max sum", run: { Int64(Checked.add(&$0, UInt32.max - 1, 1)) }, want: Int64(UInt32.max), fail: nil),
    Case(name: "sum past max", run: { Int64(Checked.add(&$0, UInt32.max, 1)) }, want: 0, fail: "overflow"),
    Case(name: "below zero", run: { Int64(Checked.sub(&$0, UInt8(0), 1)) }, want: 0, fail: "overflow"),
    Case(name: "product past max", run: { Int64(Checked.mul(&$0, Int16(256), 128)) }, want: 0, fail: "overflow"),
    Case(name: "truncated", run: { Int64(Checked.div(&$0, Int32(-7), 2)) }, want: -3, fail: nil),
    Case(name: "dividend sign", run: { Int64(Checked.rem(&$0, Int32(-7), 3)) }, want: -1, fail: nil),
    Case(name: "divide by zero", run: { Int64(Checked.div(&$0, UInt8(1), 0)) }, want: 0, fail: "divide-by-zero"),
    Case(name: "remainder by zero", run: { Checked.rem(&$0, Int64(1), 0) }, want: 0, fail: "divide-by-zero"),
    Case(name: "MIN / -1", run: { Int64(Checked.div(&$0, Int8.min, -1)) }, want: 0, fail: "overflow"),
    Case(name: "MIN % -1", run: { Int64(Checked.rem(&$0, Int8.min, -1)) }, want: 0, fail: "overflow"),
    Case(name: "-MIN", run: { Int64(Checked.neg(&$0, Int32.min)) }, want: 0, fail: "overflow"),
    Case(name: "negate", run: { Int64(Checked.neg(&$0, Int32(5))) }, want: -5, fail: nil),
    Case(name: "negate a nonzero unsigned", run: { Int64(Checked.neg(&$0, UInt8(1))) }, want: 0, fail: "overflow"),
    Case(name: "negate zero unsigned", run: { Int64(Checked.neg(&$0, UInt8(0))) }, want: 0, fail: nil),
    Case(name: "64-bit product", run: { Checked.mul(&$0, Int64.max, 2) }, want: 0, fail: "overflow"),
    Case(name: "64-bit sum", run: { Checked.add(&$0, Int64.max, 1) }, want: 0, fail: "overflow"),
    Case(name: "64-bit difference", run: { Checked.sub(&$0, Int64.min, 1) }, want: 0, fail: "overflow"),
    Case(name: "unsigned 64-bit sum", run: { Int64(truncatingIfNeeded: Checked.add(&$0, UInt64.max, 1)) }, want: 0, fail: "overflow"),
    Case(name: "unsigned 64-bit exact", run: { Int64(truncatingIfNeeded: Checked.add(&$0, UInt64.max - 1, 1)) }, want: -1, fail: nil),
]

@Test(arguments: cases.map(\.name))
func eachFailureIsNamedAndEveryValuePasses(name: String) throws {
    let c = try #require(cases.first { $0.name == name })
    var f = Failure()
    let got = c.run(&f)
    if let fail = c.fail {
        #expect(f.error?.contractName == fail, "\(name): got \(got), want failure \(fail)")
        #expect(got == 0, "\(name): a failed operation yields 0")
    } else {
        #expect(!f.failed, "\(name): failed with \(String(describing: f.error))")
        #expect(got == c.want, "\(name)")
    }
}

// The first failure is the one the caller learns of.
@Test func theFirstFailureIsKept() {
    var f = Failure()
    _ = Checked.div(&f, Int32(1), 0)
    _ = Checked.add(&f, Int32.max, 1)
    #expect(f.error == .divideByZero)
    #expect(throws: AlgorithmError.divideByZero) { try f.throwIfFailed() }
}

@Test func aBodyThatRecordedNothingThrowsNothing() throws {
    var f = Failure()
    #expect(Checked.add(&f, 2, 3) == 5)
    try f.throwIfFailed()
    #expect(f.error == nil)
}

@Test func everyFailureHasTheNameTheContractGivesIt() {
    let names = [
        AlgorithmError.overflow, .divideByZero, .capacityExceeded, .precondition, .outOfRange,
    ].map(\.contractName)
    #expect(names == ["overflow", "divide-by-zero", "capacity-exceeded", "precondition", "out-of-range"])
}

// A value stored where a narrower integer type is declared is the same value, or 0 and
// an overflow when the type cannot hold it, and never a wrapped one.
@Test func aNarrowingKeepsTheValueOrRefusesItAndNeverWraps() {
    var f = Failure()
    #expect(Checked.narrow(&f, Int64(200), to: UInt8.self) == 200)
    #expect(Checked.narrow(&f, UInt64(127), to: Int8.self) == 127)
    #expect(!f.failed)
    #expect(Checked.narrow(&f, Int64(256), to: UInt8.self) == 0)
    #expect(f.error == .overflow)

    var g = Failure()
    #expect(Checked.narrow(&g, Int64(-1), to: UInt32.self) == 0)
    #expect(g.error == .overflow)
    var h = Failure()
    #expect(Checked.narrow(&h, UInt64.max, to: Int64.self) == 0)
    #expect(h.error == .overflow)
    var i = Failure()
    #expect(Checked.narrow(&i, Int64(-129), to: Int8.self) == 0)
    #expect(i.error == .overflow)
}

// A string is held to its UTF-8 bytes: two characters of four bytes fit a bound of four
// and not of three, and a count of characters would be told apart from one of bytes.
private struct BoundCase {
    var value: String
    var capacity: UInt32
    var want: String
    var fail: Bool
}

@Test(arguments: [
    BoundCase(value: "abcd", capacity: 4, want: "abcd", fail: false),
    BoundCase(value: "abcde", capacity: 4, want: "", fail: true),
    BoundCase(value: "\u{E9}\u{E9}", capacity: 4, want: "\u{E9}\u{E9}", fail: false),
    BoundCase(value: "\u{E9}\u{E9}", capacity: 3, want: "", fail: true),
    BoundCase(value: "\u{E9}\u{20AC}", capacity: 4, want: "", fail: true),
    BoundCase(value: "", capacity: 1, want: "", fail: false),
    // One grapheme cluster, several bytes: `String.count` is 1.
    BoundCase(value: "e\u{301}", capacity: 2, want: "", fail: true),
])
private func aStringIsHeldToItsBytes(c: BoundCase) {
    var f = Failure()
    let got = Checked.bounded(&f, c.value, capacity: c.capacity)
    #expect(f.failed == c.fail)
    #expect(got == c.want)
    if c.fail {
        #expect(f.error == .capacityExceeded)
    }
}

@Test func aByteStringIsHeldToItsBytes() {
    for (value, capacity, fail) in [
        (Array("abcd".utf8), UInt32(4), false),
        (Array("abcde".utf8), 4, true),
        ([0xE9, 0xFF, 0x80, 0x00], 4, false),
        (Array("\u{E9}".utf8), 1, true),
        ([], 1, false),
    ] as [([UInt8], UInt32, Bool)] {
        var f = Failure()
        let got = Checked.bounded(&f, value, capacity: capacity)
        #expect(f.failed == fail, "\(value) held to \(capacity)")
        #expect(got == (fail ? [] : value))
        if fail {
            #expect(f.error == .capacityExceeded)
        }
    }
}

@Test func aListIsHeldToItsOwnBound() {
    var f = Failure()
    #expect(Checked.within(&f, [1, 2, 3], capacity: 3) == [1, 2, 3])
    #expect(!f.failed)
    #expect(Checked.within(&f, [1, 2, 3, 4], capacity: 3).isEmpty)
    #expect(f.error == .capacityExceeded)
}

@Test func anIndexOutsideTheCollectionIsAFailureAndNeverATrap() {
    var f = Failure()
    let xs: [UInt8] = [10, 20, 30]
    #expect(Checked.at(&f, xs, Int64(2), fallback: 0) == 30)
    #expect(Checked.at(&f, xs, UInt64(0), fallback: 0) == 10)
    #expect(!f.failed)
    #expect(Checked.at(&f, xs, Int64(3), fallback: 99) == 99)
    #expect(f.error == .outOfRange)
    var g = Failure()
    #expect(Checked.at(&g, xs, Int64(-1), fallback: 7) == 7)
    #expect(g.error == .outOfRange)
    var h = Failure()
    #expect(Checked.at(&h, xs, UInt64.max, fallback: 7) == 7)
    #expect(h.error == .outOfRange)
}

// A call to another may-fail algorithm is its value, or its failure recorded in the
// caller's Failure under the callee's own name.
@Test func takePassesACalleeFailureOn() {
    func callee(_ fail: Bool) throws -> UInt32 {
        if fail {
            throw AlgorithmError.divideByZero
        }
        return 7
    }
    var f = Failure()
    #expect(Checked.take(&f, fallback: 0) { try callee(false) } == 7)
    #expect(!f.failed)
    #expect(Checked.take(&f, fallback: 0) { try callee(true) } == 0)
    #expect(f.error == .divideByZero)
}

private enum Level: UInt8, CaseIterable {
    case low = 0
    case high = 7
}

// A reading that found no variant is an out-of-range failure, which a call that may
// fail passes on, and an enum's first case stands where it failed.
@Test func foundThrowsOutOfRangeForAReadingThatFoundNothingAndTakeCaseNamesTheFirstCase() throws {
    #expect(try Checked.found(Level(rawValue: 7)) == .high)
    var f = Failure()
    #expect(Checked.takeCase(&f) { try Checked.found(Level(rawValue: 7)) } == .high)
    #expect(!f.failed)
    #expect(Checked.takeCase(&f) { try Checked.found(Level(rawValue: 3)) } == .low)
    #expect(f.error == .outOfRange)
}
