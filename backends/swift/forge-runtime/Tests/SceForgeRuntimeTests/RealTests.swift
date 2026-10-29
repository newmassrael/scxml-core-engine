// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// `round` is half away from zero and `floor` toward negative infinity, whatever the
// language's own rounding does at a tie, and a value the target cannot hold stands at
// the nearest one it can: the conversion never traps.

@Test func roundGoesHalfAwayFromZero() {
    #expect(Real.round(0.5, to: Int32.self) == 1)
    #expect(Real.round(-0.5, to: Int32.self) == -1)
    #expect(Real.round(2.5, to: Int32.self) == 3)
    #expect(Real.round(-2.5, to: Int32.self) == -3)
    #expect(Real.round(2.4, to: Int32.self) == 2)
    #expect(Real.round(-2.6, to: Int32.self) == -3)
}

@Test func floorGoesTowardNegativeInfinity() {
    #expect(Real.floor(2.7, to: Int32.self) == 2)
    #expect(Real.floor(-2.7, to: Int32.self) == -3)
    #expect(Real.floor(-0.1, to: Int32.self) == -1)
    #expect(Real.floor(3.0, to: Int32.self) == 3)
}

@Test func aValueTheTargetCannotHoldStandsAtTheNearestOneItCan() {
    #expect(Real.round(300.0, to: UInt8.self) == UInt8.max)
    #expect(Real.round(-1.0, to: UInt8.self) == 0)
    #expect(Real.floor(-129.0, to: Int8.self) == Int8.min)
    #expect(Real.floor(128.0, to: Int8.self) == Int8.max)
    #expect(Real.round(1e30, to: Int64.self) == Int64.max)
    #expect(Real.round(-1e30, to: Int64.self) == Int64.min)
    #expect(Real.round(1e30, to: UInt64.self) == UInt64.max)
}

@Test func theLargestValueTheTargetHoldsIsKept() {
    #expect(Real.floor(Double(Int32.max), to: Int32.self) == Int32.max)
    #expect(Real.floor(Double(Int32.min), to: Int32.self) == Int32.min)
    #expect(Real.floor(Double(UInt32.max), to: UInt32.self) == UInt32.max)
    // 2^63 is the first double past Int64.max, and 2^63 - 1024 the last one below it.
    #expect(Real.floor(9_223_372_036_854_775_808.0, to: Int64.self) == Int64.max)
    #expect(Real.floor(9_223_372_036_854_774_784.0, to: Int64.self) == 9_223_372_036_854_774_784)
}

@Test func notANumberIsZeroAndInfinityStandsAtTheEnd() {
    #expect(Real.round(.nan, to: Int32.self) == 0)
    #expect(Real.floor(.nan, to: UInt16.self) == 0)
    #expect(Real.round(.infinity, to: Int16.self) == Int16.max)
    #expect(Real.floor(-.infinity, to: Int16.self) == Int16.min)
}
