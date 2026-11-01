// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Written by hand, and shared by the drivers of this directory that start a child session: what a
// driver means by "the run reached it" is said once here and not once per stem.

import Foundation
import SceRuntime

/// Lets the machine tick, for at most `timeout`, until `reached` holds, and says whether it did.
///
/// A machine that invokes a child session, or sends itself a delayed event, is only moved on by the
/// ticks: the engine advances its children and judges its scheduler there. The wait is a bound on a
/// run that never arrives, not a pace the verdict depends on, so a driver passes the state it is
/// waiting for rather than a count of ticks.
func tickUntil<Policy: StatePolicy>(
    _ engine: Engine<Policy>, timeout: Duration = .seconds(3),
    _ reached: () -> Bool
) -> Bool {
    let deadline = ContinuousClock.now + timeout
    while !reached() {
        if ContinuousClock.now >= deadline {
            return false
        }
        engine.tick()
        Thread.sleep(forTimeInterval: 0.005)
    }
    return true
}
