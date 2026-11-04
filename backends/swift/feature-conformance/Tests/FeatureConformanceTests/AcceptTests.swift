// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import AloneAccept
import LiftedAccept
import Testing

/// The build that turns `session-unicast` and `session-unicast-accept` on (see
/// `Observation.swift`).
@Suite struct WithAccept {
    @Test func everyRunIsObservedAlike() {
        expectEveryRunIsObservedAlike(lifted: lifted, alone: alone)
    }

    /// `probe` is the first child exactly where `session-unicast` is on.
    @Test func theDefaultStateIsThisBuilds() {
        #expect(lifted([]).active == "probe;")
    }

    private func lifted(_ run: [String]) -> Observation {
        let host = LiftedAccept.RecordingSessionActions()
        return observe(
            LiftedAccept.SessionPolicy(actions: host), calls: { host.calls.map { "\($0)" } },
            run: run)
    }

    private func alone(_ run: [String]) -> Observation {
        let host = AloneAccept.RecordingSessionActions()
        return observe(
            AloneAccept.SessionPolicy(actions: host), calls: { host.calls.map { "\($0)" } },
            run: run)
    }
}
