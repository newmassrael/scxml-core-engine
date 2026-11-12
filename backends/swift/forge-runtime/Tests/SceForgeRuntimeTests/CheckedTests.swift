// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// Each failure the contract names is thrown by its contract name and answers no value,
// and every value that fits comes back unchanged (docs/adr/0098). The cases are those of
// backends/go/forge-runtime/algorithm/algorithm_test.go.

/// What `body` completed with: its value, or the failure it threw.
private func outcome<T>(_ body: () throws(AlgorithmError) -> T) -> Result<T, AlgorithmError> {
    do throws(AlgorithmError) {
        return .success(try body())
    } catch {
        return .failure(error)
    }
}

private struct Case: Sendable {
    var name: String
    var run: @Sendable () throws(AlgorithmError) -> Int64
    var want: Int64?
    var fail: String?
}

private let cases: [Case] = [
    Case(name: "max sum", run: { Int64(try Checked.add(UInt32.max - 1, 1)) }, want: Int64(UInt32.max)),
    Case(name: "sum past max", run: { Int64(try Checked.add(UInt32.max, 1)) }, fail: "overflow"),
    Case(name: "below zero", run: { Int64(try Checked.sub(UInt8(0), 1)) }, fail: "overflow"),
    Case(name: "product past max", run: { Int64(try Checked.mul(Int16(256), 128)) }, fail: "overflow"),
    Case(name: "truncated", run: { Int64(try Checked.div(Int32(-7), 2)) }, want: -3),
    Case(name: "dividend sign", run: { Int64(try Checked.rem(Int32(-7), 3)) }, want: -1),
    Case(name: "divide by zero", run: { Int64(try Checked.div(UInt8(1), 0)) }, fail: "divide-by-zero"),
    Case(name: "remainder by zero", run: { try Checked.rem(Int64(1), 0) }, fail: "divide-by-zero"),
    Case(name: "MIN / -1", run: { Int64(try Checked.div(Int8.min, -1)) }, fail: "overflow"),
    Case(name: "MIN % -1", run: { Int64(try Checked.rem(Int8.min, -1)) }, fail: "overflow"),
    Case(name: "-MIN", run: { Int64(try Checked.neg(Int32.min)) }, fail: "overflow"),
    Case(name: "negate", run: { Int64(try Checked.neg(Int32(5))) }, want: -5),
    Case(name: "negate a nonzero unsigned", run: { Int64(try Checked.neg(UInt8(1))) }, fail: "overflow"),
    Case(name: "negate zero unsigned", run: { Int64(try Checked.neg(UInt8(0))) }, want: 0),
    Case(name: "64-bit product", run: { try Checked.mul(Int64.max, 2) }, fail: "overflow"),
    Case(name: "64-bit sum", run: { try Checked.add(Int64.max, 1) }, fail: "overflow"),
    Case(name: "64-bit difference", run: { try Checked.sub(Int64.min, 1) }, fail: "overflow"),
    Case(name: "unsigned 64-bit sum", run: { Int64(truncatingIfNeeded: try Checked.add(UInt64.max, 1)) }, fail: "overflow"),
    Case(name: "unsigned 64-bit exact", run: { Int64(truncatingIfNeeded: try Checked.add(UInt64.max - 1, 1)) }, want: -1),
]

@Test(arguments: cases.map(\.name))
func eachFailureIsNamedAndEveryValuePasses(name: String) throws {
    let c = try #require(cases.first { $0.name == name })
    switch outcome(c.run) {
    case .success(let got):
        #expect(c.fail == nil, "\(name): answered \(got), want failure \(c.fail ?? "")")
        #expect(got == c.want, "\(name)")
    case .failure(let error):
        #expect(error.contractName == c.fail, "\(name): failed with \(error)")
    }
}

// The first failure in evaluation order is the one the caller learns of, and nothing
// after it is evaluated: no value stands in for it to compute on.
@Test func theFirstFailureEndsTheExpression() {
    var evaluatedAfter = false
    let got = outcome { () throws(AlgorithmError) -> Int32 in
        let a = try Checked.div(Int32(1), 0)
        evaluatedAfter = true
        return try Checked.add(a, Int32.max)
    }
    #expect(got == .failure(.divideByZero))
    #expect(!evaluatedAfter)
}

@Test func everyFailureHasTheNameTheContractGivesIt() {
    let names = [
        AlgorithmError.overflow, .divideByZero, .capacityExceeded, .precondition, .outOfRange,
    ].map(\.contractName)
    #expect(names == ["overflow", "divide-by-zero", "capacity-exceeded", "precondition", "out-of-range"])
}

// A value stored where a narrower integer type is declared is the same value, or an
// overflow when the type cannot hold it, and never a wrapped one.
@Test func aNarrowingKeepsTheValueOrRefusesItAndNeverWraps() {
    #expect(outcome { () throws(AlgorithmError) in try Checked.narrow(Int64(200), to: UInt8.self) } == .success(200))
    #expect(outcome { () throws(AlgorithmError) in try Checked.narrow(UInt64(127), to: Int8.self) } == .success(127))
    #expect(outcome { () throws(AlgorithmError) in try Checked.narrow(Int64(256), to: UInt8.self) } == .failure(.overflow))
    #expect(outcome { () throws(AlgorithmError) in try Checked.narrow(Int64(-1), to: UInt32.self) } == .failure(.overflow))
    #expect(outcome { () throws(AlgorithmError) in try Checked.narrow(UInt64.max, to: Int64.self) } == .failure(.overflow))
    #expect(outcome { () throws(AlgorithmError) in try Checked.narrow(Int64(-129), to: Int8.self) } == .failure(.overflow))
}

// A string is held to its UTF-8 bytes: two characters of four bytes fit a bound of four
// and not of three, and a count of characters would be told apart from one of bytes.
private struct BoundCase {
    var value: String
    var capacity: UInt32
    var fits: Bool
}

@Test(arguments: [
    BoundCase(value: "abcd", capacity: 4, fits: true),
    BoundCase(value: "abcde", capacity: 4, fits: false),
    BoundCase(value: "\u{E9}\u{E9}", capacity: 4, fits: true),
    BoundCase(value: "\u{E9}\u{E9}", capacity: 3, fits: false),
    BoundCase(value: "\u{E9}\u{20AC}", capacity: 4, fits: false),
    BoundCase(value: "", capacity: 1, fits: true),
    // One grapheme cluster, several bytes: `String.count` is 1.
    BoundCase(value: "e\u{301}", capacity: 2, fits: false),
])
private func aStringIsHeldToItsBytes(c: BoundCase) {
    let got = outcome { () throws(AlgorithmError) in try Checked.bounded(c.value, capacity: c.capacity) }
    #expect(got == (c.fits ? .success(c.value) : .failure(.capacityExceeded)))
}

@Test func aByteStringIsHeldToItsBytes() {
    for (value, capacity, fits) in [
        (Array("abcd".utf8), UInt32(4), true),
        (Array("abcde".utf8), 4, false),
        ([0xE9, 0xFF, 0x80, 0x00], 4, true),
        (Array("\u{E9}".utf8), 1, false),
        ([], 1, true),
    ] as [([UInt8], UInt32, Bool)] {
        let got = outcome { () throws(AlgorithmError) in try Checked.bounded(value, capacity: capacity) }
        #expect(got == (fits ? .success(value) : .failure(.capacityExceeded)), "\(value) held to \(capacity)")
    }
}

@Test func aListIsHeldToItsOwnBound() {
    #expect(outcome { () throws(AlgorithmError) in try Checked.within([1, 2, 3], capacity: 3) } == .success([1, 2, 3]))
    #expect(outcome { () throws(AlgorithmError) in try Checked.within([1, 2, 3, 4], capacity: 3) } == .failure(.capacityExceeded))
}

@Test func anIndexOutsideTheCollectionIsAFailureAndNeverATrap() {
    let xs: [UInt8] = [10, 20, 30]
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(xs, Int64(2)) } == .success(30))
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(xs, UInt64(0)) } == .success(10))
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(xs, Int64(3)) } == .failure(.outOfRange))
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(xs, Int64(-1)) } == .failure(.outOfRange))
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(xs, UInt64.max) } == .failure(.outOfRange))
}

/// A sum type with no value to start at: the element type the old checked read could
/// not answer for (docs/adr/0098, section 5).
private enum Shape: Equatable {
    case point
    case circle(radius: UInt32)
}

// An element of any type is read alike, a sum type's included, since no value of the
// type stands where the index is outside the list.
@Test func anElementOfASumTypeIsReadAndAnIndexOutsideItsListFails() {
    let shapes: [Shape] = [.point, .circle(radius: 3)]
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(shapes, UInt8(1)) } == .success(.circle(radius: 3)))
    #expect(outcome { () throws(AlgorithmError) in try Checked.at(shapes, Int32(2)) } == .failure(.outOfRange))
}

// A call to another may-fail algorithm is its value, or its failure under the callee's
// own name, passed on by `try` alone.
@Test func aCalleeFailureIsTheCallersUnrenamed() {
    func callee(_ fail: Bool) throws(AlgorithmError) -> UInt32 {
        if fail {
            throw .divideByZero
        }
        return 7
    }
    #expect(outcome { () throws(AlgorithmError) in try callee(false) + 1 } == .success(8))
    #expect(outcome { () throws(AlgorithmError) in try callee(true) + 1 } == .failure(.divideByZero))
}

private enum Level: UInt8, CaseIterable {
    case low = 0
    case high = 7
}

// A reading that found no variant is an out-of-range failure, and no case stands in for
// it.
@Test func foundThrowsOutOfRangeForAReadingThatFoundNothing() {
    #expect(outcome { () throws(AlgorithmError) in try Checked.found(Level(rawValue: 7)) } == .success(.high))
    #expect(outcome { () throws(AlgorithmError) in try Checked.found(Level(rawValue: 3)) } == .failure(.outOfRange))
}

// The right operand of a short-circuit that can fail is evaluated only where the left
// one decides nothing, so its failure is the expression's only there.
@Test func aShortCircuitEvaluatesItsRightOperandOnlyWhereTheLeftDecidesNothing() {
    let failing = { () throws(AlgorithmError) -> Bool in try Checked.div(Int32(1), 0) > 0 }
    #expect(outcome { () throws(AlgorithmError) in try Checked.and(false, failing) } == .success(false))
    #expect(outcome { () throws(AlgorithmError) in try Checked.and(true, failing) } == .failure(.divideByZero))
    #expect(outcome { () throws(AlgorithmError) in try Checked.or(true, failing) } == .success(true))
    #expect(outcome { () throws(AlgorithmError) in try Checked.or(false, failing) } == .failure(.divideByZero))
    #expect(outcome { () throws(AlgorithmError) in try Checked.and(true) { true } } == .success(true))
    #expect(outcome { () throws(AlgorithmError) in try Checked.or(false) { false } } == .success(false))
}

// A fallback that can fail is evaluated only where the value is absent, and one that may
// be absent leaves the whole absent.
@Test func aCoalesceEvaluatesItsFallbackOnlyWhereTheValueIsAbsent() {
    let failing = { () throws(AlgorithmError) -> UInt8 in try Checked.add(UInt8.max, 1) }
    let present: UInt8? = 4
    let absent: UInt8? = nil
    #expect(outcome { () throws(AlgorithmError) in try Checked.coalesce(present, failing) } == .success(4))
    #expect(outcome { () throws(AlgorithmError) in try Checked.coalesce(absent, failing) } == .failure(.overflow))
    #expect(outcome { () throws(AlgorithmError) in try Checked.coalesce(absent) { () throws(AlgorithmError) -> UInt8 in 9 } } == .success(9))
    let maybe = { () throws(AlgorithmError) -> UInt8? in nil }
    #expect(outcome { () throws(AlgorithmError) in try Checked.coalesce(absent, maybe) } == .success(nil))
}
