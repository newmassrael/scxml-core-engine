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

    /// A saved state records the configuration it was taken in (docs/adr/0057, section 7).
    @Test func aSavedStateNamesTheFeaturesThisBuildTurnsOn() {
        #expect(lifted([]).saved.contains(#""features":["session-unicast"]"#))
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
