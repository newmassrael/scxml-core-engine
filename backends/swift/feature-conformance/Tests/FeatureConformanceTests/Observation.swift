// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

// docs/adr/0057 §6 and §10: the Swift machine generated for every build of a product
// line, compiled under one feature set, is the machine that set's variant generates alone
// (the commuting law `select(gen(D)) = gen(project(D))`).
//
// The generator proves it of the tokens, build by build, before it writes anything. Here
// the compiler resolves the `#if` itself: each feature set's tests drive the machine for
// every build, compiled with that set's `.define`s, and the set's own machine through
// every run of up to four of the events either one names, and the two must agree on every
// observation a host has of a machine: where it is, what it asked its host to do, which
// events it discarded, which names it had no event for, and where it ended. The same
// comparison as the C++ lane's (`tests/integration/TheMachineForEveryBuildIsEachBuildsOwnAotTest.cpp`).

/// Every event name either machine knows, and one neither does.
let events = ["ready", "inbound.start", "connect", "done", "close", "unknown"]
let longestRun = 4

/// What a host observes of a machine after a run of events.
struct Observation: Equatable, CustomStringConvertible {
    var active = ""
    var calls = ""
    var discarded: UInt32 = 0
    /// Names the machine has no event for: neither machine can be handed one.
    var unnamed: UInt32 = 0
    var terminal = ""
    /// The text the machine saves where it stands (docs/adr/0045), with the session id its draw gave it left out,
    /// which differs between two machines by chance; empty where a machine cannot be saved, one that has ended.
    var saved = ""

    var description: String {
        "active=\(active) calls=\(calls) discarded=\(discarded) unnamed=\(unnamed) terminal=\(terminal) saved=\(saved)"
    }
}

/// The observation of the machine `policy` describes after `run`, `calls` reading the
/// calls its host has recorded. The machine runs no script engine, so its start has no script
/// session to fail (docs/adr/0096); a start refused all the same is an issue of the test, and
/// the observation is of a machine that was not started.
func observe<Policy: SavedPolicy>(
    _ policy: Policy, calls: () -> [String], run: [String]
) -> Observation {
    let engine = Engine(policy: policy)
    do {
        try engine.initialize()
    } catch {
        Issue.record("the start of a machine that runs no script engine was refused: \(error)")
    }
    var seen = Observation()
    for name in run {
        guard let event = engine.resolveEventByName(name) else {
            seen.unnamed += 1
            continue
        }
        engine.raiseExternal(event)
        engine.step()
    }
    seen.active = engine.getActiveStates().map { policy.getStateName($0) + ";" }.joined()
    seen.calls = calls().map { $0 + ";" }.joined()
    seen.discarded = engine.discardedExternalEvents
    seen.terminal = engine.terminalState.map { policy.getStateName($0) } ?? ""
    if var state = try? engine.save(wallNowMs: 0) {
        state.sessionID = ""
        seen.saved = state.toJSON()
    }
    return seen
}

/// Every run of up to `longestRun` of `events`, observed of the machine for every build
/// (`lifted`) and of the build's own (`alone`): the two observe alike, and the runs held
/// them to something, a run that ended the machine and one that called the host among
/// them.
func expectEveryRunIsObservedAlike(
    lifted: ([String]) -> Observation, alone: ([String]) -> Observation
) {
    var runs = 0
    var ended = false
    var calledHost = false
    var saved = false
    for length in 0...longestRun {
        var index = [Int](repeating: 0, count: length)
        while true {
            let run = index.map { events[$0] }
            let ours = lifted(run)
            let theirs = alone(run)
            runs += 1
            #expect(
                ours == theirs,
                "the machine for every build and this build's own part on the run \(run)")
            if ours != theirs {
                return
            }
            ended = ended || !theirs.terminal.isEmpty
            calledHost = calledHost || !theirs.calls.isEmpty
            saved = saved || !theirs.saved.isEmpty
            // The next run of this length, the last event turning fastest.
            var digit = length
            while digit > 0 {
                index[digit - 1] += 1
                if index[digit - 1] < events.count {
                    break
                }
                digit -= 1
                index[digit] = 0
            }
            if digit == 0 {
                break
            }
        }
    }
    #expect(ended, "no run ended the machine, so the comparison held the machines to less than it says")
    #expect(calledHost, "no run called the host, so the comparison held the machines to less than it says")
    #expect(saved, "no run could be saved, so the comparison held the saved text to nothing")
    #expect(runs > 0)
}
