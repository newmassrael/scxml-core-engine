// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096 — Swift AOT path: a generated machine whose script session cannot be established
// does not start, and its host is told why.
//
// A Swift machine owns its Lua state rather than being handed an engine, so the steps are failed
// through the seam the generated machine compiles into a debug build alone
// (`sceScriptSessionFault`): it is asked before each of the three steps that establish a session,
// creating the Lua state, loading the shared library into it, binding the system variables, with
// the id of the session being established, and a step it answers true for fails as a failure of
// the engine would. A parent hands it to the children it starts, so the same seam reaches a
// child's session. Every step it is asked about is kept, so "the start stopped at the step that
// failed" and "no child was started" are read from the steps a start took.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml,
// reused for its shape: a parent in `s0` with `<data ticks>` that invokes an inline child, whose
// `<onentry>` sends `tick` to the parent 110 times. The child declares `<data>` of its own, so it
// has a script session of its own to fail. A start that ran would initialize `ticks`, enter `s0`
// and start the child, establishing the child's session; a child that ran would send `tick`.
//
// Regeneration: scripts/regen_swift_integration.sh

@testable import IntegrationMachines
import SceLua
import SceRuntime
import Testing

private typealias Policy = AChildMaySendManyEventsInOneTickPolicy

/// The id the machine's session is given, so a child's is known before it is started.
private let parentID = "parent"

/// The id the parent starts the child of its one `<invoke>` under, on its first entry of `s0`.
private let childID = "parent.s0.1.chatty"

/// One step the seam was asked about: the session being established and the step.
private struct Ask: Equatable {
    let sessionID: String
    let stage: ScriptSessionStage
}

/// What the seam answers, and every step it was asked about, in order.
private final class Steps {
    private let fails: (_ sessionID: String, _ stage: ScriptSessionStage) -> Bool
    private(set) var asked: [Ask] = []

    init(failing fails: @escaping (_ sessionID: String, _ stage: ScriptSessionStage) -> Bool) {
        self.fails = fails
    }

    func ask(_ sessionID: String, _ stage: ScriptSessionStage) -> Bool {
        asked.append(Ask(sessionID: sessionID, stage: stage))
        return fails(sessionID, stage)
    }

    /// The sessions a start asked to establish: each asks `create` first.
    var created: [String] {
        asked.filter { $0.stage == .create }.map(\.sessionID)
    }
}

/// The fixture's machine under `parentID`, on a manual clock, whose seam fails the steps `fails`
/// names, and the record of what the seam was asked.
private func machine(
    failing fails: @escaping (_ sessionID: String, _ stage: ScriptSessionStage) -> Bool
) throws -> (Engine<Policy>, Policy, Steps) {
    let steps = Steps(failing: fails)
    let policy = Policy()
    try policy.setSessionID(parentID)
    policy.sceScriptSessionFault = { steps.ask($0, $1) }
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    return (engine, policy, steps)
}

/// A refused start left nothing behind: no run marked running, no state entered, no final state
/// reached, no session kept, no error raised; and only the machine's own session was asked for,
/// since entering `s0` would have started its child and established the child's.
private func expectNothingStarted(_ engine: Engine<Policy>, _ policy: Policy, _ steps: Steps, _ label: String) {
    #expect(!engine.isRunning, "\(label): a refused start marks no run running")
    #expect(engine.getActiveStates().isEmpty, "\(label): a refused start enters no state, and \(engine.getActiveStates()) is active")
    #expect(engine.terminalState == nil, "\(label): a refused start reaches no final state")
    #expect(policy.readScriptValue("1") == nil, "\(label): a refused start keeps no session")
    #expect(engine.unhandledErrorEvents == 0, "\(label): a refused start raises nothing in the machine")
    #expect(!steps.asked.isEmpty, "\(label): the start asked for its session")
    #expect(
        steps.asked.allSatisfy { $0.sessionID == parentID },
        "\(label): a refused start enters no state, and a child's session was asked for: \(steps.asked)")
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AMachineWhoseScriptSessionFailsDoesNotStart {
    /// The control: the seam fails nothing, so the machine is the fixture's and its child's ticks
    /// arrive. Without it the cases below could pass over a seam that breaks every machine.
    @Test func aStartWhoseSeamFailsNothingRunsTheFixture() throws {
        let (engine, policy, steps) = try machine { _, _ in false }
        try engine.initialize()
        engine.processEvent(.finish)

        #expect(engine.terminalState == .done, "`finish` carries the run to `done`")
        #expect(policy.readScriptValue("ticks") == .integer(110), "every tick the child sent arrived")
        #expect(steps.created == [parentID, childID], "the parent's session and its child's were established")
        #expect(
            steps.asked.prefix(3).map(\.stage) == [.create, .library, .systemVariables],
            "the parent's session is established in its three steps, in order: \(steps.asked)")
    }

    @Test func aStartWhoseSessionCannotBeEstablishedIsRefusedWithTheStep() throws {
        for (index, stage) in ScriptSessionStage.allCases.enumerated() {
            let (engine, policy, steps) = try machine { _, asked in asked == stage }
            do {
                try engine.initialize()
                Issue.record("\(stage): the start was not refused")
            } catch let failure as ScriptSessionFailure {
                #expect(failure.stage == stage, "\(stage): the refusal names the step that failed")
                #expect(failure.message == "the step '\(stage)' was failed on request", "\(stage): the engine's words")
                #expect(
                    failure.description
                        == "the script session could not be established at step '\(stage)': \(failure.message)",
                    "\(stage): the refusal is the sentence every backend writes")
            }
            #expect(
                steps.asked.map(\.stage) == Array(ScriptSessionStage.allCases.prefix(index + 1)),
                "\(stage): the start stopped at the step that failed: \(steps.asked)")
            expectNothingStarted(engine, policy, steps, "\(stage)")
        }
    }

    @Test func aRefusedStartIsRefusedAgainRatherThanRunWithoutASession() throws {
        // A session that failed is not taken as made: the next start asks for it again and is
        // refused again, rather than finding it there and running the machine over no session.
        for stage in ScriptSessionStage.allCases {
            let (engine, policy, steps) = try machine { _, asked in asked == stage }
            #expect(throws: ScriptSessionFailure.self, "\(stage)") { try engine.initialize() }
            #expect(throws: ScriptSessionFailure.self, "\(stage)") { try engine.initialize() }
            #expect(steps.created == [parentID, parentID], "\(stage): each start asks for the session once")
            expectNothingStarted(engine, policy, steps, "\(stage)")
        }
    }

    @Test func aSessionWhoseSystemVariablesCannotBeBoundIsReleased() throws {
        let (engine, policy, steps) = try machine { _, asked in asked == .systemVariables }
        #expect(throws: ScriptSessionFailure(stage: .systemVariables, message: "the step 'system-variables' was failed on request")) {
            try engine.initialize()
        }
        #expect(policy.readScriptValue("1") == nil, "the session the failed bind was made in is not kept")
        expectNothingStarted(engine, policy, steps, "system-variables")

        // The machine is as it was before the start: once the step no longer fails, a start runs it.
        policy.sceScriptSessionFault = nil
        try engine.initialize()
        #expect(policy.readScriptValue("1") == .integer(1), "the next start established a session of its own")
        #expect(engine.getCurrentState() == .s0)
    }

    @Test func aRefusedMachineTakesNoEvent() throws {
        let (engine, policy, steps) = try machine { _, asked in asked == .create }
        #expect(throws: ScriptSessionFailure.self) { try engine.initialize() }
        // The machine was not started, so the event is not taken and nothing reads it.
        engine.processEvent(.finish)
        #expect(engine.unseenExternalEvents == 1, "the event a refused machine was handed is counted unseen")
        #expect(engine.getCurrentState() == .s0, "a refused machine stays where it was built")
        expectNothingStarted(engine, policy, steps, "create")
    }

    @Test func theCheckedRootStartIsRefusedForTheSession() throws {
        let (engine, policy, steps) = try machine { _, asked in asked == .library }
        do {
            try engine.initializeAsRoot()
            Issue.record("the checked root start was not refused")
        } catch let refusal as RootStartRefusal {
            #expect(refusal != .needsParent, "a refusal for the session is not one for a parent")
            guard case .scriptSession(let failure) = refusal else {
                Issue.record("the checked root start was refused for something else: \(refusal)")
                return
            }
            #expect(failure.stage == .library)
            #expect(refusal.description == failure.description, "the refusal's words are the failure's sentence")
        }
        expectNothingStarted(engine, policy, steps, "library")
    }

    @Test func aResumedStartWhoseSessionCannotBeEstablishedIsRefused() throws {
        let (engine, policy, steps) = try machine { _, asked in asked == .create }
        let verdict = engine.enterAt(configuration: [.s0], current: .s0)
        #expect(verdict != .accepted)
        guard case .scriptSessionUnavailable(let failure) = verdict else {
            Issue.record("enterAt answered \(verdict)")
            return
        }
        #expect(failure.stage == .create, "the verdict carries the failure of the create step")
        expectNothingStarted(engine, policy, steps, "create")
    }

    @Test func aChildWhoseSessionFailsLeavesItsParentWithErrorExecutionAndNoChild() throws {
        let (engine, policy, steps) = try machine { sessionID, asked in sessionID != parentID && asked == .create }
        try engine.initialize()

        #expect(steps.created == [parentID, childID], "the parent asked for its own session and the child's")
        #expect(
            steps.asked.filter { $0.sessionID == childID }.map(\.stage) == [.create],
            "the child's start stopped at the step that failed: \(steps.asked)")
        // The invoke ends without a child: none is kept, so no send can name one.
        #expect(!policy.isInvocationRunning("chatty"), "the child whose start was refused is not kept")
        #expect(!policy.isChildSessionRunning(childID), "nothing answers at the refused child's id")
        // The parent is told with error.execution, which no transition of `s0` takes.
        #expect(engine.lastUnhandledError == .errorExecution, "the refused child is reported as error.execution")
        #expect(engine.unhandledErrorEvents == 1, "one error.execution for the one refused child")
        // No child ran, so no `tick` and no `done.invoke` ever arrived.
        #expect(policy.readScriptValue("ticks") == .integer(0), "no child ran, so no tick arrived")
        #expect(engine.discardedExternalEvents == 0, "no done.invoke was raised for the refused child")

        // The parent itself runs on.
        #expect(engine.getCurrentState() == .s0, "the parent stays in `s0`")
        engine.processEvent(.finish)
        #expect(engine.terminalState == .done, "`finish` carries the parent to `done`")
    }
}
