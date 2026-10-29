// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A table is interpolated between the two breakpoints an input lies between, and an input
// outside the axis answers the value at the nearer end.

private let axis: [Double] = [0, 1000, 2000]
private let values: [Double] = [100, 150, 120]

@Test func aBreakpointAnswersItsOwnValue() {
    #expect(Interpolation.linear(axis, values, 0) == 100)
    #expect(Interpolation.linear(axis, values, 1000) == 150)
    #expect(Interpolation.linear(axis, values, 2000) == 120)
}

@Test func anInputBetweenTwoBreakpointsIsInterpolated() {
    #expect(Interpolation.linear(axis, values, 500) == 125)
    #expect(Interpolation.linear(axis, values, 1500) == 135)
}

@Test func anInputOutsideTheAxisAnswersTheNearerEnd() {
    #expect(Interpolation.linear(axis, values, -5) == 100)
    #expect(Interpolation.linear(axis, values, 9999) == 120)
}

@Test func aSinglePointAndAnEmptyTableAnswerWhatTheyHold() {
    #expect(Interpolation.linear([5], [42], 0) == 42)
    #expect(Interpolation.linear([5], [42], 100) == 42)
    #expect(Interpolation.linear([], [], 3) == 0)
}

@Test func aBilinearTableIsInterpolatedAlongBothAxes() {
    let rows: [Double] = [0, 10]
    let cols: [Double] = [0, 100]
    let grid: [[Double]] = [[0, 100], [10, 110]]
    #expect(Interpolation.bilinear(rows, cols, grid, 0, 0) == 0)
    #expect(Interpolation.bilinear(rows, cols, grid, 10, 100) == 110)
    #expect(Interpolation.bilinear(rows, cols, grid, 5, 50) == 55)
    #expect(Interpolation.bilinear(rows, cols, grid, 5, 0) == 5)
}

@Test func aBilinearInputOutsideEitherAxisIsClamped() {
    let rows: [Double] = [0, 10]
    let cols: [Double] = [0, 100]
    let grid: [[Double]] = [[0, 100], [10, 110]]
    #expect(Interpolation.bilinear(rows, cols, grid, -3, -3) == 0)
    #expect(Interpolation.bilinear(rows, cols, grid, 99, 999) == 110)
    #expect(Interpolation.bilinear([], cols, grid, 1, 1) == 0)
}
