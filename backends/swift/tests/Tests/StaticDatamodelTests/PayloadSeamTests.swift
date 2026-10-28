// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// The per-event inject seam of the typed `_event.data` channel (NL-to-IR Item C1 Path
// A): one call binds the event and the values of its payload, so the pairing of a name
// and a type cannot be built inconsistently. The scenarios send JSON, which is the
// other producer; these send the typed carrier.

private func started() -> (StaticPayloadPolicy, Engine<StaticPayloadPolicy>) {
    let policy = StaticPayloadPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    return (policy, engine)
}

@Test func theSeamDeliversTheTypedPayloadToTheGuardAndTheAssignments() {
    let (policy, engine) = started()
    engine.raiseDayPicked(year: 2026, month: 10, dayOfMonth: 20)
    engine.step()
    #expect(policy.late)
    #expect(policy.day == 20)
    #expect(policy.sinceEpoch == 26)
    #expect(policy.refusals == 0)
}

@Test func theSeamsValueThatDoesNotFitAnAssignmentIsRefusedAsItIsFromJSON() {
    let (policy, engine) = started()
    // 1999 - 2000 does not fit a uint16: the assignment is skipped and error.execution
    // says so, as it does for the same payload sent as JSON.
    engine.raiseDayPicked(year: 1999, month: 12, dayOfMonth: 3)
    engine.step()
    #expect(!policy.late)
    #expect(policy.day == 3)
    #expect(policy.sinceEpoch == 0)
    #expect(policy.refusals == 1)
}

@Test func anEventWithNoDataIsRefusedAndItsGuardDoesNotFire() {
    let (policy, engine) = started()
    // No typed carrier and no data: the lift refuses, the engine raises
    // error.execution, and neither day.picked transition reads a payload it has not.
    engine.raiseExternalByName("day.picked")
    engine.step()
    #expect(policy.refusals >= 1)
    #expect(policy.day == 0)
}
