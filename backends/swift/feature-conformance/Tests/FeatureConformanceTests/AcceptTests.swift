// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import AloneAccept
import LiftedAccept
import SceRuntime
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

    /// A saved state records the configuration it was taken in (docs/adr/0057, section 7), sorted.
    @Test func aSavedStateNamesTheFeaturesThisBuildTurnsOn() {
        #expect(lifted([]).saved.contains(#""features":["session-unicast","session-unicast-accept"]"#))
    }

    /// A restore into another configuration is refused by the features that differ: a build with more features has
    /// data and invocations the saved run never initialised or started.
    @Test func aStateOfAnotherConfigurationIsRefusedByTheFeaturesThatDiffer() throws {
        let saving = Engine(policy: LiftedAccept.SessionPolicy(actions: LiftedAccept.RecordingSessionActions()))
        try saving.initialize()
        var state = try saving.save(wallNowMs: 0)
        state.features = ["session-unicast"]
        let restoring = Engine(policy: LiftedAccept.SessionPolicy(actions: LiftedAccept.RecordingSessionActions()))
        do {
            try restoring.restore(state, wallNowMs: 0)
            Issue.record("a state of another configuration was restored")
        } catch let refusal as StateRefusal {
            #expect(refusal.message.contains("[session-unicast-accept] on only in this build"), "\(refusal.message)")
        }
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
