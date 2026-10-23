// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

@Test func aManualClockMovesOnlyWhenTheHostMovesIt() {
    let clock = ManualClock(startMs: 40)
    #expect(clock.elapsedMs() == 40)
    #expect(clock.elapsedMs() == 40)
    clock.advance(0)
    #expect(clock.elapsedMs() == 40)
    clock.advance(25)
    #expect(clock.elapsedMs() == 65)
}

@Test func oneManualClockIsReadByEveryEngineThatSharesIt() {
    let shared = ManualClock()
    let parent: any SceClock = shared
    let child: any SceClock = shared
    shared.advance(7)
    #expect(parent.elapsedMs() == 7)
    #expect(child.elapsedMs() == 7)
}

@Test func aMonotonicClockNeverReadsBackwards() {
    let clock = MonotonicClock()
    var last = clock.elapsedMs()
    #expect(last >= 0)
    for _ in 0..<1000 {
        let now = clock.elapsedMs()
        #expect(now >= last)
        last = now
    }
}
