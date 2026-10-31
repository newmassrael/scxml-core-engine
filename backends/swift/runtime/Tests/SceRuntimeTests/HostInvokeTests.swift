// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceRuntime

// The invoker half of the host processor: what the engine records of a host-run
// invocation, which completion it accepts, and what the document is told.

/// What a host invoker was asked, in order, for a test to read back.
private final class RecordingInvoker {
    var events: [HostInvokeEvent] = []
    var answer: HostInvokeResponse?

    func handle(_ event: HostInvokeEvent) -> HostInvokeResponse? {
        events.append(event)
        return answer
    }

    var starts: [HostInvokeRequest] {
        events.compactMap { event in
            if case .start(let request) = event { return request }
            return nil
        }
    }

    var cancels: [HostInvokeCancel] {
        events.compactMap { event in
            if case .cancel(let cancel) = event { return cancel }
            return nil
        }
    }
}

private func request(_ id: String = "probe") -> HostInvokeRequest {
    HostInvokeRequest(processorType: "sce-host-test", invokeID: id)
}

/// A machine that answers the completion of the invoke `probe` by going to `done`, its
/// failure and an `error.execution` by going to `busy`.
private func makeHostEngine() -> (Engine<ScriptedPolicy>, RecordingInvoker) {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(event: .doneInvokeProbe, target: .done),
            FlatTransition(event: .errorInvokeProbe, target: .busy),
            FlatTransition(event: .errorExecution, target: .busy),
        ]
    }
    let invoker = RecordingInvoker()
    engine.registerInvoker("sce-host-test", handler: invoker.handle)
    engine.initialize()
    return (engine, invoker)
}

@Test func aTypeNobodyRegisteredIsNotRun() {
    let (engine, _, _) = makeEngine { _ in }
    engine.initialize()
    #expect(!engine.hasInvoker("sce-host-test"))
    #expect(!engine.performHostInvoke(request()))
}

@Test func aStartCarriesATokenAndEveryStartGetsItsOwn() {
    let (engine, invoker) = makeHostEngine()
    #expect(engine.performHostInvoke(request()))
    #expect(engine.performHostInvoke(request()))
    let tokens = invoker.starts.map(\.token)
    #expect(tokens.count == 2)
    #expect(tokens[0] != tokens[1])
}

@Test func aCompletionRaisesTheDoneEventOfTheInvokeAndEndsIt() {
    let (engine, invoker) = makeHostEngine()
    engine.performHostInvoke(request())
    let token = invoker.starts[0].token
    #expect(engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, doneData: "{}"))
    engine.tick()
    #expect(engine.getCurrentState() == .done)
    // It ended once: a second report of it is refused.
    #expect(
        !engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, doneData: "{}")
    )
}

@Test func aCompletionOfAnEarlierStartOfTheSameIdIsRefused() {
    let (engine, invoker) = makeHostEngine()
    engine.performHostInvoke(request())
    engine.performHostInvoke(request())
    let first = invoker.starts[0].token
    let second = invoker.starts[1].token
    #expect(!engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: first, doneData: ""))
    engine.tick()
    #expect(engine.getCurrentState() == .idle)
    #expect(engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: second, doneData: ""))
}

@Test func aCompletionTheHostGivesBeforeReturningTakesTheSameDoor() {
    let (engine, invoker) = makeHostEngine()
    invoker.answer = .done("{}")
    #expect(engine.performHostInvoke(request()))
    engine.tick()
    #expect(engine.getCurrentState() == .done)
    // The start is over, so a cancel has nothing to stop.
    #expect(!engine.cancelHostInvoke(processorType: "sce-host-test", invokeID: "probe"))
    #expect(invoker.cancels.isEmpty)
}

@Test func aRefusedStartRaisesErrorExecutionAndNeverStarted() {
    let (engine, invoker) = makeHostEngine()
    invoker.answer = .refused("unavailable")
    #expect(engine.performHostInvoke(request()))
    engine.tick()
    #expect(engine.getCurrentState() == .busy)
    let token = invoker.starts[0].token
    #expect(!engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, doneData: ""))
    #expect(!engine.cancelHostInvoke(processorType: "sce-host-test", invokeID: "probe"))
    #expect(invoker.cancels.isEmpty)
}

@Test func aFailureRaisesTheErrorEventOfTheInvoke() {
    let (engine, invoker) = makeHostEngine()
    engine.performHostInvoke(request())
    let token = invoker.starts[0].token
    #expect(engine.failHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, errorData: "deadline"))
    engine.tick()
    #expect(engine.getCurrentState() == .busy)
}

@Test func aCancelReachesTheHostOnceAndOnlyForARunningStart() {
    let (engine, invoker) = makeHostEngine()
    // Nothing started: nothing to stop.
    #expect(!engine.cancelHostInvoke(processorType: "sce-host-test", invokeID: "probe"))
    engine.performHostInvoke(request())
    let token = invoker.starts[0].token
    #expect(engine.cancelHostInvoke(processorType: "sce-host-test", invokeID: "probe"))
    #expect(invoker.cancels == [HostInvokeCancel(processorType: "sce-host-test", invokeID: "probe", token: token)])
    #expect(!engine.cancelHostInvoke(processorType: "sce-host-test", invokeID: "probe"))
    // What a cancelled run sends afterwards is ignored.
    #expect(!engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, doneData: ""))
}

// MARK: - The deadline

private func deadlineRequest(_ written: String) -> HostInvokeRequest {
    HostInvokeRequest(
        processorType: "sce-host-test",
        invokeID: "probe",
        params: [hostInvokeDeadlineParam: [written]]
    )
}

@Test func theDeadlineIsTheEnginesAndNotTheHosts() {
    let (engine, invoker) = makeHostEngine()
    #expect(engine.performHostInvoke(deadlineRequest("5000")))
    #expect(invoker.starts.count == 1)
    #expect(invoker.starts[0].params[hostInvokeDeadlineParam] == nil)
}

@Test func anInvocationThatOutlivesItsDeadlineIsStoppedAndTheDocumentToldOnce() {
    let (engine, invoker) = makeHostEngine()
    engine.performHostInvoke(deadlineRequest("5000"))
    let token = invoker.starts[0].token
    engine.advanceTimeMs(4999)
    engine.tick()
    #expect(engine.getCurrentState() == .idle)
    #expect(invoker.cancels.isEmpty)
    engine.advanceTimeMs(1)
    engine.tick()
    // `error.invoke.<id>` took the document where the deadline's transition goes, and the
    // host was told to stop the run.
    #expect(engine.getCurrentState() == .busy)
    #expect(invoker.cancels == [HostInvokeCancel(processorType: "sce-host-test", invokeID: "probe", token: token)])
    // The run is over: a late completion is refused.
    #expect(!engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, doneData: ""))
}

@Test func aCompletionBeforeTheDeadlineTakesTheDeadlineWithIt() {
    let (engine, invoker) = makeHostEngine()
    engine.performHostInvoke(deadlineRequest("5000"))
    let token = invoker.starts[0].token
    #expect(engine.completeHostInvoke(processorType: "sce-host-test", invokeID: "probe", token: token, doneData: "{}"))
    engine.tick()
    #expect(engine.getCurrentState() == .done)
    engine.advanceTimeMs(10_000)
    engine.tick()
    #expect(invoker.cancels.isEmpty)
    #expect(engine.timeUntilNextScheduled() == nil)
}

@Test func aCancelTakesTheDeadlineWithIt() {
    let (engine, _) = makeHostEngine()
    engine.performHostInvoke(deadlineRequest("5000"))
    #expect(engine.timeUntilNextScheduled() == .milliseconds(5000))
    #expect(engine.cancelHostInvoke(processorType: "sce-host-test", invokeID: "probe"))
    #expect(engine.timeUntilNextScheduled() == nil)
}

@Test func aDeadlineThatIsNotAWholeNumberOfMillisecondsStartsNothing() {
    let (engine, invoker) = makeHostEngine()
    // Answered true: the report has been made, and false is kept for "no invoker ran it".
    #expect(engine.performHostInvoke(deadlineRequest("5x")))
    engine.tick()
    #expect(invoker.starts.isEmpty)
    #expect(engine.getCurrentState() == .busy)
}

@Test func aDeadlineGivenTwiceStartsNothing() {
    let (engine, invoker) = makeHostEngine()
    let twice = HostInvokeRequest(
        processorType: "sce-host-test",
        invokeID: "probe",
        params: [hostInvokeDeadlineParam: ["5", "6"]]
    )
    #expect(engine.performHostInvoke(twice))
    #expect(invoker.starts.isEmpty)
}

@Test func aRefusedStartTakesItsDeadlineBack() {
    let (engine, invoker) = makeHostEngine()
    invoker.answer = .refused("unavailable")
    engine.performHostInvoke(deadlineRequest("5000"))
    #expect(engine.timeUntilNextScheduled() == nil)
    _ = invoker
}

@Test func theDeadlineIsReadAsTheTableEveryRuntimeIsHeldToReadsIt() throws {
    // <root>/backends/swift/runtime/Tests/SceRuntimeTests/HostInvokeTests.swift
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    let path = url.appendingPathComponent("sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json").path
    let table = try parseJSON(String(contentsOfFile: path, encoding: .utf8))
    guard case .array(let accepted)? = table.member("accepted"), case .array(let refused)? = table.member("refused") else {
        Issue.record("the table holds an accepted and a refused list")
        return
    }
    #expect(!accepted.isEmpty && !refused.isEmpty)
    for pair in accepted {
        guard case .array(let items) = pair, case .text(let written) = items[0], case .text(let ms) = items[1] else {
            Issue.record("an accepted entry is a pair of texts")
            continue
        }
        #expect(parseHostInvokeDeadlineMs(written) == Int64(ms), "\(written.debugDescription) is accepted")
    }
    for entry in refused {
        guard case .text(let written) = entry else {
            Issue.record("a refused entry is a text")
            continue
        }
        #expect(parseHostInvokeDeadlineMs(written) == nil, "\(written.debugDescription) is refused")
    }
}

// MARK: - A completion by another door

@Test func aDoneInvokeThatDidNotComeThroughTheCompletionDoorIsRefusedAndCounted() {
    let (engine, policy, _) = makeEngine { policy in
        policy.hostInvokeIDs = ["probe"]
        policy.transitions[.idle] = [FlatTransition(event: .doneInvokeProbe, target: .done)]
    }
    engine.initialize()
    // Raised as any event is, with no token: a cancelled run's late reply looks the same.
    engine.raiseExternal(.doneInvokeProbe)
    engine.tick()
    #expect(engine.getCurrentState() == .idle)
    #expect(engine.refusedHostInvokeCompletions == 1)
    _ = policy
}

@Test func aDoneInvokeOfAnInvokeNoHostRunsIsNotRefused() {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .doneInvokeProbe, target: .done)]
    }
    engine.initialize()
    engine.raiseExternal(.doneInvokeProbe)
    engine.tick()
    #expect(engine.getCurrentState() == .done)
    #expect(engine.refusedHostInvokeCompletions == 0)
}

// MARK: - The fields of a typed request

@Test func aTypedRequestIsReadBackAsTheTypesItsRecordDeclares() {
    let request = HostInvokeRequest(
        processorType: "sce-host-test",
        invokeID: "probe",
        params: [
            "small": ["-128"],
            "large": ["18446744073709551615"],
            "wide": ["-9223372036854775808"],
            "ratio": ["0.5"],
            "flag": ["true"],
            "name": ["key"],
            "raw": ["A\u{FF}"],
        ]
    )
    #expect(request.requestInteger("small", as: Int8.self) == -128)
    #expect(request.requestInteger("large", as: UInt64.self) == UInt64.max)
    #expect(request.requestInteger("wide", as: Int64.self) == Int64.min)
    #expect(request.requestReal("ratio", as: Double.self) == 0.5)
    #expect(request.requestReal("ratio", as: Float.self) == 0.5)
    #expect(request.requestBool("flag"))
    #expect(request.requestString("name") == "key")
    #expect(request.requestBytes("raw") == [0x41, 0xFF])
}
