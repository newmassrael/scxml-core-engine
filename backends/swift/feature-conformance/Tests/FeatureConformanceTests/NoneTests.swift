// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import AloneNone
import LiftedNone
import Testing

/// The build that turns no feature on (see `Observation.swift`).
@Suite struct WithNoFeature {
    @Test func everyRunIsObservedAlike() {
        expectEveryRunIsObservedAlike(lifted: lifted, alone: alone)
    }

    /// The default state is a value the document computes, and differs between builds:
    /// `probe` is the first child exactly where `session-unicast` is on.
    @Test func theDefaultStateIsThisBuilds() {
        #expect(lifted([]).active == "idle;")
    }

    private func lifted(_ run: [String]) -> Observation {
        let host = LiftedNone.RecordingSessionActions()
        return observe(
            LiftedNone.SessionPolicy(actions: host), calls: { host.calls.map { "\($0)" } },
            run: run)
    }

    private func alone(_ run: [String]) -> Observation {
        let host = AloneNone.RecordingSessionActions()
        return observe(
            AloneNone.SessionPolicy(actions: host), calls: { host.calls.map { "\($0)" } },
            run: run)
    }
}
