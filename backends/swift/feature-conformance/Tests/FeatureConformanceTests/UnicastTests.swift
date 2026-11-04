// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import AloneUnicast
import LiftedUnicast
import Testing

/// The build that turns `session-unicast` on (see `Observation.swift`).
@Suite struct WithUnicast {
    @Test func everyRunIsObservedAlike() {
        expectEveryRunIsObservedAlike(lifted: lifted, alone: alone)
    }

    /// `probe` is the first child exactly where `session-unicast` is on.
    @Test func theDefaultStateIsThisBuilds() {
        #expect(lifted([]).active == "probe;")
    }

    private func lifted(_ run: [String]) -> Observation {
        let host = LiftedUnicast.RecordingSessionActions()
        return observe(
            LiftedUnicast.SessionPolicy(actions: host), calls: { host.calls.map { "\($0)" } },
            run: run)
    }

    private func alone(_ run: [String]) -> Observation {
        let host = AloneUnicast.RecordingSessionActions()
        return observe(
            AloneUnicast.SessionPolicy(actions: host), calls: { host.calls.map { "\($0)" } },
            run: run)
    }
}
