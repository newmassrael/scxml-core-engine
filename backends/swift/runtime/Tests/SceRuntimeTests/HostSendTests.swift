// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceRuntime

// The send half of the host processor (§scxml-6.2.5): what the engine gives the handler a host
// registered for a type, which events of its answer reach the machine and in what order, what a
// type nobody registered raises, and which types a host may not register.

/// What a host processor was asked, in order, for a test to read back.
private final class RecordingProcessor {
    var requests: [HostSendRequest] = []
    var answer: [HostSendResponse] = []

    func handle(_ request: HostSendRequest) -> [HostSendResponse] {
        requests.append(request)
        return answer
    }
}

private func request(
    _ type: String = "x-host-test", event: String = "go", sendID: String = "s1"
) -> HostSendRequest {
    HostSendRequest(processorType: type, eventName: event, sendID: sendID)
}

/// A machine that takes `go` from `idle` to `busy`, `again` from `busy` to `done`, and an
/// `error.execution` from `idle` to `busy`; the processor `x-host-test` is registered.
private func makeHostSendEngine() throws -> (Engine<ScriptedPolicy>, RecordingProcessor, ManualClock) {
    let (engine, _, clock) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(event: .go, target: .busy),
            FlatTransition(event: .errorExecution, target: .busy),
        ]
        policy.transitions[.busy] = [FlatTransition(event: .again, target: .done)]
    }
    let processor = RecordingProcessor()
    try engine.registerEventProcessor("x-host-test", handler: processor.handle)
    try engine.initialize()
    return (engine, processor, clock)
}

@Test func aSendIsGivenToTheHandlerRegisteredForItsTypeAsTheDocumentWroteIt() throws {
    let (engine, processor, _) = try makeHostSendEngine()
    let sent = HostSendRequest(
        processorType: "x-host-test", eventName: "ping", target: "t", content: "c",
        params: ["k": ["1", "2"]], sendID: "s7", eventData: "{\"k\":[1,2]}", invokeID: "inv")
    engine.performHostSend(sent)
    #expect(processor.requests == [sent])
}

@Test func theEventsAnAnswerHoldsAreRaisedInTheOrderItGaveThem() throws {
    // `go` then `again` takes the machine through `busy` to `done`; the other order loses `again`
    // in `idle`, where nothing takes it, and ends in `busy`.
    let (engine, processor, _) = try makeHostSendEngine()
    processor.answer = [HostSendResponse(eventName: "go"), HostSendResponse(eventName: "again")]
    engine.performHostSend(request())
    engine.tick()
    #expect(engine.getCurrentState() == .done)

    let (other, reversed, _) = try makeHostSendEngine()
    reversed.answer = [HostSendResponse(eventName: "again"), HostSendResponse(eventName: "go")]
    other.performHostSend(request())
    other.tick()
    #expect(other.getCurrentState() == .busy)
}

@Test func anEmptyAnswerIsPerformedAndNothingToReport() throws {
    let (engine, processor, _) = try makeHostSendEngine()
    engine.performHostSend(request())
    engine.tick()
    #expect(processor.requests.count == 1)
    #expect(engine.getCurrentState() == .idle)
}

@Test func anEventTheMachineDoesNotDeclareIsDropped() throws {
    let (engine, processor, _) = try makeHostSendEngine()
    processor.answer = [HostSendResponse(eventName: "no.such.event")]
    engine.performHostSend(request())
    engine.tick()
    #expect(engine.getCurrentState() == .idle)
}

@Test func aTypeNobodyRegisteredRaisesErrorExecutionAndIsNotPerformed() throws {
    let (engine, processor, _) = try makeHostSendEngine()
    #expect(!engine.hasEventProcessor("x-unregistered"))
    engine.performHostSend(request("x-unregistered"))
    engine.tick()
    // `error.execution` took the machine from `idle` to `busy`.
    #expect(engine.getCurrentState() == .busy)
    #expect(processor.requests.isEmpty)
}

/// What `_event.sendid` and `_event.data` the `error.execution` the engine raises carry, read by the
/// transition that takes it out of `idle`: `nil` until one is taken.
private final class ErrorSeen {
    var sendID: String?
    var data: String?
}

/// A machine whose `error.execution` from `idle` records what the event carries, and no processor
/// registered: every host send it is handed is an act nobody performs.
private func makeErrorRecordingEngine() throws -> (Engine<ScriptedPolicy>, ErrorSeen, ManualClock) {
    let seen = ErrorSeen()
    let (engine, _, clock) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(
                event: .errorExecution, target: .busy,
                content: { engine in
                    seen.sendID = engine.policy.eventSendID
                    seen.data = engine.policy.eventData
                }),
        ]
    }
    try engine.initialize()
    return (engine, seen, clock)
}

/// §scxml-6.2.4 + §scxml-5.10 (test 332): the error a send to a type nobody registered raises names
/// that send by its id, as every other backend's does, and says what failed in the sentence they use.
@Test func theErrorOfASendNobodyServesCarriesItsSendID() throws {
    let (engine, seen, _) = try makeErrorRecordingEngine()
    engine.performHostSend(request("x-unregistered", sendID: "__send_3"))
    engine.tick()
    #expect(engine.getCurrentState() == .busy)
    #expect(seen.sendID == "__send_3", "the error names the send that caused it")
    #expect(seen.data == unregisteredHostProcessorMessage("x-unregistered"))
}

/// The same error for a delayed send, raised when its wait is over: the request the scheduler holds
/// is the one the send made, its id included.
@Test func theErrorOfADelayedSendNobodyServesCarriesItsSendID() throws {
    let (engine, seen, clock) = try makeErrorRecordingEngine()
    engine.scheduleHostSend(
        delay: .milliseconds(100), request: request("x-unregistered", sendID: "late"), sender: "o")
    engine.tick()
    #expect(seen.sendID == nil, "nothing is raised while the send waits")
    clock.advance(100)
    // One tick performs the act and a second takes whatever it raised, however the engine orders
    // its scheduler and its queues inside one tick; a second tick with nothing to take does nothing.
    engine.tick()
    engine.tick()
    #expect(engine.getCurrentState() == .busy)
    #expect(seen.sendID == "late", "the error names the delayed send that caused it")
}

@Test func registeringATypeTwiceReplacesTheHandler() throws {
    let (engine, first, _) = try makeHostSendEngine()
    let second = RecordingProcessor()
    try engine.registerEventProcessor("x-host-test", handler: second.handle)
    engine.performHostSend(request())
    #expect(first.requests.isEmpty)
    #expect(second.requests.count == 1)
}

@Test func aDelayedSendIsPerformedWhenItsWaitIsOverWithTheRequestOfTheMomentItWasMade() throws {
    let (engine, processor, clock) = try makeHostSendEngine()
    let sent = HostSendRequest(
        processorType: "x-host-test", eventName: "go", sendID: "late", eventData: "d", invokeID: "inv-at-send")
    #expect(engine.scheduleHostSend(delay: .milliseconds(500), request: sent, sender: "o") == "late")
    #expect(processor.requests.isEmpty)
    clock.advance(499)
    engine.tick()
    #expect(processor.requests.isEmpty)
    clock.advance(1)
    engine.tick()
    #expect(processor.requests == [sent])
}

@Test func aSendWhoseDelayIsZeroIsPerformedNowAndQueuesNothing() throws {
    let (engine, processor, _) = try makeHostSendEngine()
    engine.scheduleHostSend(delay: .zero, request: request(), sender: "o")
    #expect(processor.requests.count == 1)
}

@Test func aCancelReachesAHostSendWhileItWaits() throws {
    let (engine, processor, clock) = try makeHostSendEngine()
    engine.scheduleHostSend(delay: .seconds(1), request: request(sendID: "gone"), sender: "o")
    #expect(engine.cancelEvent(sendID: "gone"))
    clock.advance(2000)
    engine.tick()
    #expect(processor.requests.isEmpty)
}

@Test func theMeshRouterIsTheOneHandlerOfTheReservedType() throws {
    let (engine, _, _) = try makeHostSendEngine()
    #expect(!engine.hasEventProcessor(meshProcessorType))
    let router = RecordingProcessor()
    engine.registerMeshRouter(router.handle)
    #expect(engine.hasEventProcessor(meshProcessorType))
    engine.performHostSend(HostSendRequest(processorType: meshProcessorType, eventName: "go", target: "#peer"))
    #expect(router.requests.map(\.target) == ["#peer"])
}

@Test func theTypesAHostMayNotRegisterAreTheOnesTheSharedTableReserves() throws {
    let url = repositoryRoot().appendingPathComponent(
        "sce-build/tests/fixtures/host_processor/reserved_type_cases.json")
    let data = try Data(contentsOf: url)
    let object = try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any])
    let cases = try #require(object["cases"] as? [[String: Any]])
    #expect(!cases.isEmpty)
    let (engine, processor, _) = try makeHostSendEngine()
    for entry in cases {
        let type = try #require(entry["type"] as? String)
        let reserved = try #require(entry["reserved"] as? Bool)
        #expect(isReservedProcessorType(type) == reserved, "\(type.debugDescription)")
        if reserved {
            #expect(
                throws: HostRegistrationError.reservedType(call: "registerEventProcessor", type: type)
            ) { try engine.registerEventProcessor(type, handler: processor.handle) }
            #expect(!engine.hasEventProcessor(type), "\(type.debugDescription) was registered")
        } else {
            try engine.registerEventProcessor(type, handler: processor.handle)
            #expect(engine.hasEventProcessor(type), "\(type.debugDescription) was refused")
        }
    }
}
