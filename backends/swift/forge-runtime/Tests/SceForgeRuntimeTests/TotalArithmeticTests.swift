// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// Every wrapping and saturating operation at every width is its definition
// (docs/adr/0063, section 1): the exact result `e` in the integers, then the `r` in
// `[m, M]` congruent to `e` modulo `2^W`, or `max(m, min(M, e))`. The expected values
// are computed here from that definition in `Int128`, which holds every exact result of
// two operands of a width up to 64 bits but the product of two large `UInt64` values,
// and that one in `UInt128`. The operands are each type's `m`, `M`, 0, 1 and -1 and
// their neighbours, and `2^(W/2)` and `2^(W/2) + 1`, whose products pass `2^W` and stay
// within `2^(2W)`. The pinned cases after the sweep spell the same points by hand.

private enum Rule {
    case wrapping
    case saturating
}

/// The exact result `e` brought into the range of `T` by `rule`.
private func reduce<T: FixedWidthInteger>(_ e: Int128, _ rule: Rule, into _: T.Type) -> T {
    let lo = Int128(T.min)
    let hi = Int128(T.max)
    switch rule {
    case .wrapping:
        let span = hi - lo + 1
        let offset = (e - lo) % span
        return T(offset < 0 ? offset + span + lo : offset + lo)
    case .saturating:
        return T(max(lo, min(hi, e)))
    }
}

/// The exact product of `a` and `b` brought into the range of `T` by `rule`. A product
/// past `Int128` is one of two `UInt64` values, non-negative and below `2^128`.
private func product<T: FixedWidthInteger>(_ a: T, _ b: T, _ rule: Rule) -> T {
    let (e, overflow) = Int128(a).multipliedReportingOverflow(by: Int128(b))
    if !overflow {
        return reduce(e, rule, into: T.self)
    }
    let exact = UInt128(a) * UInt128(b)
    switch rule {
    case .wrapping:
        return T(exact % (UInt128(1) << T.bitWidth))
    case .saturating:
        return T.max
    }
}

/// The operands of the sweep at `T`.
private func operands<T: FixedWidthInteger>(of _: T.Type) -> [T] {
    let half = T(1) << (T.bitWidth / 2)
    var values: [T] = [T.min, T.min + 1, 0, 1, 2, half, half + 1, T.max - 1, T.max]
    if T.isSigned {
        values += [T.zero - 1, T.zero - 2]
    }
    return values
}

/// Every operation but the saturating negation, on every pair of operands at `T`.
private func expectTheDefinition<T: FixedWidthInteger>(at type: T.Type) {
    let name = String(describing: type)
    for a in operands(of: type) {
        let x = Int128(a)
        #expect(Wrapping.neg(a) == reduce(-x, .wrapping, into: type), "\(name): wrapping -(\(a))")
        for b in operands(of: type) {
            let y = Int128(b)
            #expect(Wrapping.add(a, b) == reduce(x + y, .wrapping, into: type), "\(name): wrapping \(a) + \(b)")
            #expect(Wrapping.sub(a, b) == reduce(x - y, .wrapping, into: type), "\(name): wrapping \(a) - \(b)")
            #expect(Wrapping.mul(a, b) == product(a, b, .wrapping), "\(name): wrapping \(a) * \(b)")
            #expect(Saturating.add(a, b) == reduce(x + y, .saturating, into: type), "\(name): saturating \(a) + \(b)")
            #expect(Saturating.sub(a, b) == reduce(x - y, .saturating, into: type), "\(name): saturating \(a) - \(b)")
            #expect(Saturating.mul(a, b) == product(a, b, .saturating), "\(name): saturating \(a) * \(b)")
        }
    }
}

/// The saturating negation, a signed type's only, on every operand at `T`.
private func expectTheSaturatingNegation<T: FixedWidthInteger & SignedInteger>(at type: T.Type) {
    let name = String(describing: type)
    for a in operands(of: type) {
        #expect(Saturating.neg(a) == reduce(-Int128(a), .saturating, into: type), "\(name): saturating -(\(a))")
    }
}

private struct Width: Sendable {
    var name: String
    var check: @Sendable () -> Void
}

private let widths: [Width] = [
    Width(name: "Int8", check: {
        expectTheDefinition(at: Int8.self)
        expectTheSaturatingNegation(at: Int8.self)
    }),
    Width(name: "Int16", check: {
        expectTheDefinition(at: Int16.self)
        expectTheSaturatingNegation(at: Int16.self)
    }),
    Width(name: "Int32", check: {
        expectTheDefinition(at: Int32.self)
        expectTheSaturatingNegation(at: Int32.self)
    }),
    Width(name: "Int64", check: {
        expectTheDefinition(at: Int64.self)
        expectTheSaturatingNegation(at: Int64.self)
    }),
    Width(name: "UInt8", check: { expectTheDefinition(at: UInt8.self) }),
    Width(name: "UInt16", check: { expectTheDefinition(at: UInt16.self) }),
    Width(name: "UInt32", check: { expectTheDefinition(at: UInt32.self) }),
    Width(name: "UInt64", check: { expectTheDefinition(at: UInt64.self) }),
]

@Suite struct TotalArithmeticTests {
    @Test(arguments: widths.map(\.name))
    func everyOperationAtEveryWidthIsItsDefinition(width: String) throws {
        let w = try #require(widths.first { $0.name == width })
        w.check()
    }

    // A sum, a difference and a negation past either bound wrap to the other one.
    @Test func eachWrapPointWrapsToTheOtherBound() {
        #expect(Wrapping.add(UInt8.max, 1) == 0)
        #expect(Wrapping.sub(UInt8(0), 1) == UInt8.max)
        #expect(Wrapping.add(Int8.max, 1) == Int8.min)
        #expect(Wrapping.sub(Int8.min, 1) == Int8.max)
        #expect(Wrapping.add(Int8.min, -1) == Int8.max)
        #expect(Wrapping.sub(Int8.max, -1) == Int8.min)
        #expect(Wrapping.neg(Int8.min) == Int8.min)
        #expect(Wrapping.neg(UInt8(1)) == UInt8.max)
        #expect(Wrapping.neg(UInt8(0)) == 0)
        #expect(Wrapping.sub(UInt32(5), UInt32.max) == 6)
        #expect(Wrapping.add(Int64.max, 1) == Int64.min)
        #expect(Wrapping.sub(Int64.min, 1) == Int64.max)
        #expect(Wrapping.sub(UInt64(0), 1) == UInt64.max)
        #expect(Wrapping.neg(Int64.min) == Int64.min)
    }

    // A product past `2^W` keeps its residue modulo `2^W`: `(2^(W/2) + 1)^2` is
    // `2^W + 2^(W/2 + 1) + 1`, so it wraps to `2^(W/2 + 1) + 1`.
    @Test func aProductPastTheWidthKeepsItsResidue() {
        #expect(Wrapping.mul(UInt8(17), 17) == 33)
        #expect(Wrapping.mul(Int8(17), 17) == 33)
        #expect(Wrapping.mul(Int8(-17), 17) == -33)
        #expect(Wrapping.mul(UInt16(257), 257) == 513)
        #expect(Wrapping.mul(Int16(257), 257) == 513)
        #expect(Wrapping.mul(UInt32(65537), 65537) == 131_073)
        #expect(Wrapping.mul(Int32(65537), 65537) == 131_073)
        #expect(Wrapping.mul(UInt64(4_294_967_297), 4_294_967_297) == 8_589_934_593)
        #expect(Wrapping.mul(Int64(4_294_967_297), 4_294_967_297) == 8_589_934_593)
        #expect(Wrapping.mul(UInt64.max, UInt64.max) == 1)
        #expect(Wrapping.mul(UInt64.max, 2) == UInt64.max - 1)
        #expect(Wrapping.mul(Int8.min, -1) == Int8.min)
        #expect(Wrapping.mul(Int64.min, -1) == Int64.min)
    }

    // A result past either bound stands at that bound.
    @Test func aResultPastEitherBoundStandsAtIt() {
        #expect(Saturating.add(UInt8.max, 1) == UInt8.max)
        #expect(Saturating.sub(UInt8(0), 1) == 0)
        #expect(Saturating.sub(UInt8(3), 5) == 0)
        #expect(Saturating.add(Int8.max, 1) == Int8.max)
        #expect(Saturating.add(Int8.min, -1) == Int8.min)
        #expect(Saturating.sub(Int8.min, 1) == Int8.min)
        #expect(Saturating.sub(Int8.max, -1) == Int8.max)
        #expect(Saturating.sub(Int8(0), Int8.min) == Int8.max)
        #expect(Saturating.sub(Int8(-1), Int8.min) == Int8.max)
        #expect(Saturating.sub(Int8(-2), Int8.min) == 126)
        #expect(Saturating.add(Int64.max, 1) == Int64.max)
        #expect(Saturating.sub(Int64.min, 1) == Int64.min)
        #expect(Saturating.sub(UInt64(0), 1) == 0)
        #expect(Saturating.add(UInt64.max, UInt64.max) == UInt64.max)
    }

    // A product past either bound stands at the bound on the side of its sign.
    @Test func aProductPastEitherBoundStandsAtTheBoundOfItsSign() {
        #expect(Saturating.mul(UInt8(17), 17) == UInt8.max)
        #expect(Saturating.mul(Int8(17), 17) == Int8.max)
        #expect(Saturating.mul(Int8(-17), 17) == Int8.min)
        #expect(Saturating.mul(Int8(17), -17) == Int8.min)
        #expect(Saturating.mul(Int8(-17), -17) == Int8.max)
        #expect(Saturating.mul(Int8.min, -1) == Int8.max)
        #expect(Saturating.mul(Int8.min, 2) == Int8.min)
        #expect(Saturating.mul(Int8.max, -1) == -127)
        #expect(Saturating.mul(Int8.min, Int8.max) == Int8.min)
        #expect(Saturating.mul(Int8.min, Int8.min) == Int8.max)
        #expect(Saturating.mul(UInt64(4_294_967_297), 4_294_967_297) == UInt64.max)
        #expect(Saturating.mul(UInt64.max, UInt64.max) == UInt64.max)
        #expect(Saturating.mul(Int64.min, -1) == Int64.max)
        #expect(Saturating.mul(Int64(4_294_967_297), -4_294_967_297) == Int64.min)
    }

    // The one negation past the range, `-MIN`, stands at `M`.
    @Test func theNegationOfMinStandsAtMax() {
        #expect(Saturating.neg(Int8.min) == Int8.max)
        #expect(Saturating.neg(Int8(Int8.min + 1)) == Int8.max)
        #expect(Saturating.neg(Int8.max) == Int8.min + 1)
        #expect(Saturating.neg(Int8(-1)) == 1)
        #expect(Saturating.neg(Int8(0)) == 0)
        #expect(Saturating.neg(Int16.min) == Int16.max)
        #expect(Saturating.neg(Int32.min) == Int32.max)
        #expect(Saturating.neg(Int64.min) == Int64.max)
    }

    // Saturation is one reduction per operation and does not associate.
    @Test func saturationIsReducedOncePerOperation() {
        #expect(Saturating.add(Saturating.add(Int8.max, 1), -1) == 126)
        #expect(Saturating.add(Int8.max, Saturating.add(Int8(1), -1)) == Int8.max)
    }
}
