// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A filter answers from the samples it has seen, so each test feeds one in order and reads what
// every sample answers.

@Test func aMovingAverageIsTheMeanOfTheSamplesSoFarWhileTheWindowFills() {
    let filter = MovingAverage(window: 3)
    #expect(filter.update(3) == 3)
    #expect(filter.update(6) == 4.5)
    #expect(filter.update(9) == 6)
}

@Test func aMovingAverageDropsTheOldestSampleOnceTheWindowIsFull() {
    let filter = MovingAverage(window: 3)
    _ = filter.update(3)
    _ = filter.update(6)
    _ = filter.update(9)
    #expect(filter.update(12) == 9)
    #expect(filter.update(15) == 12)
}

@Test func aMovingAverageOfOneSampleIsTheSample() {
    let filter = MovingAverage(window: 1)
    #expect(filter.update(5) == 5)
    #expect(filter.update(-2) == -2)
}

@Test func aMovingAverageForgetsEverySampleOnReset() {
    let filter = MovingAverage(window: 2)
    _ = filter.update(10)
    _ = filter.update(20)
    filter.reset()
    #expect(filter.update(4) == 4)
}

@Test func aLowPassStartsAtItsFirstSampleAndMovesTowardTheNext() {
    let filter = LowPass(alpha: 0.5)
    #expect(filter.update(10) == 10)
    #expect(filter.update(20) == 15)
    #expect(filter.update(20) == 17.5)
}

@Test func aLowPassStartsAgainAtTheSampleAfterAReset() {
    let filter = LowPass(alpha: 0.1)
    _ = filter.update(100)
    filter.reset()
    #expect(filter.update(7) == 7)
}

@Test func aDebouncePassesTheLatestSampleThroughWhileTheWindowFills() {
    let filter = Debounce<Bool>(window: 3)
    #expect(filter.update(true) == true)
    #expect(filter.update(false) == false)
}

@Test func aDebounceLatchesOnlyAfterAWindowOfIdenticalSamples() {
    let filter = Debounce<Int>(window: 3)
    _ = filter.update(1)
    _ = filter.update(1)
    #expect(filter.update(1) == 1)
    // One different sample does not move a latched output.
    #expect(filter.update(2) == 1)
    #expect(filter.update(2) == 1)
    #expect(filter.update(2) == 2)
}

@Test func aDebounceOfOneSampleIsTheSample() {
    let filter = Debounce<Int>(window: 1)
    #expect(filter.update(4) == 4)
    #expect(filter.update(9) == 9)
}

@Test func aDebounceForgetsItsLatchOnReset() {
    let filter = Debounce<Int>(window: 2)
    _ = filter.update(5)
    #expect(filter.update(5) == 5)
    filter.reset()
    #expect(filter.update(8) == 8)
}
