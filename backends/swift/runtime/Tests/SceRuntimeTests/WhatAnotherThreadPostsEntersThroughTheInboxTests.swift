// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceRuntime

// docs/adr/0084, ARCHITECTURE.md "Engine Threading": a Swift engine is confined to its owner by
// the compiler, since it is not `Sendable`, and what another thread has for it enters through its
// inbox. The cases hold the inbox to its contract:
//
//   * many threads post endings and external events while the owner ticks, and what the machine
//     takes, and what is left running, are those of a single-threaded replay of the same posts;
//   * the external queue takes events in the order they arrived: a post made while a macrostep
//     runs is applied before an event that macrostep then puts on the queue itself, a `<send>` to
//     itself or a host-run invocation's `done.invoke` (W3C test 201's shape);
//   * an ending posted for a start that was cancelled, or started again, is refused by the token
//     check, as a direct one is (§scxml-6.4);
//   * a post is taken before the run starts and wakes the owner; the inbox closes with a run that
//     was stopped or ended, after what it held is applied, and opens with the next run.

private let probeType = "x-sce-probe"
private let probeID = "probe"

/// What the machine took and what its host's invoker was asked, in order. Written by the owner
/// alone.
private final class ProbeRecord {
    var taken: [String] = []
    var invoker: [String] = []
}

/// The machine of these cases, on the flat scripted policy: `idle` (initial) starts the host-run
/// invocation `probe` on entry and cancels it on exit; `go` leaves for `busy`, `again` comes back
/// and `request` ends the run in `done`. `ping`, `done.invoke.probe` and `error.invoke.probe` are
/// taken in `idle` and `busy` alike, and recorded as `ping:`, `done:` and `failed:` followed by
/// their `_event.data`. The invoker records `start:<token>` and `cancel:<token>` and runs the work
/// past the call.
private func makeProbe() -> (Engine<ScriptedPolicy>, ProbeRecord) {
    let record = ProbeRecord()
    let (engine, _, _) = makeEngine { policy in
        policy.hostInvokeIDs = [probeID]
        let recorded: [FlatTransition] = [
            FlatTransition(event: .ping, target: nil, content: { record.taken.append("ping:" + $0.policy.eventData) }),
            FlatTransition(
                event: .doneInvokeProbe, target: nil, content: { record.taken.append("done:" + $0.policy.eventData) }),
            FlatTransition(
                event: .errorInvokeProbe, target: nil,
                content: { record.taken.append("failed:" + $0.policy.eventData) }),
        ]
        policy.transitions[.idle] =
            recorded + [FlatTransition(event: .go, target: .busy), FlatTransition(event: .request, target: .done)]
        policy.transitions[.busy] = recorded + [FlatTransition(event: .again, target: .idle)]
        policy.onEntry[.idle] = { engine in
            _ = engine.performHostInvoke(HostInvokeRequest(processorType: probeType, invokeID: probeID))
        }
        policy.onExit[.idle] = { engine in
            _ = engine.cancelHostInvoke(processorType: probeType, invokeID: probeID)
        }
    }
    do {
        try engine.registerInvoker(probeType) { event in
            switch event {
            case .start(let request):
                record.invoker.append("start:\(request.token)")
            case .cancel(let cancel):
                record.invoker.append("cancel:\(cancel.token)")
            }
            return nil
        }
    } catch {
        Issue.record("the probe invoker was refused: \(error)")
    }
    return (engine, record)
}

/// The token of the latest start the invoker was asked for; nil when `probe` never started.
private func startedToken(_ record: ProbeRecord) -> UInt64? {
    guard let start = record.invoker.last(where: { $0.hasPrefix("start:") }) else {
        return nil
    }
    return UInt64(start.dropFirst("start:".count))
}

private func ending(_ token: UInt64, done: Bool, data: String) -> HostInvokeEnding {
    HostInvokeEnding(processorType: probeType, invokeID: probeID, token: token, done: done, data: data)
}

/// The post `k` of producer `producer`: external events, typed and by name, and now and then an
/// ending of `token`, completed or failed. Each data names its producer and its place, so the
/// record says which post it came from.
private func postOf(producer: Int, k: Int, token: UInt64) -> EngineInboxInput<FlatEvent> {
    let data = "p\(producer)-\(k)"
    switch k % 7 {
    case 3:
        return .hostInvokeEnding(ending(token, done: true, data: data))
    case 5:
        return .hostInvokeEnding(ending(token, done: false, data: data))
    case 1:
        return .externalEventByName("ping", eventData: data)
    default:
        return .externalEvent(.ping, eventData: data)
    }
}

/// What the machine must take from `posted`, in the order the posts were taken: every event, and
/// of the endings only the first, since it ends `probe` and the token check refuses the rest.
private func expectedFrom(_ posted: [EngineInboxInput<FlatEvent>]) -> [String] {
    var ended = false
    var expected: [String] = []
    for input in posted {
        switch input {
        case let .externalEvent(_, eventData, _):
            expected.append("ping:" + eventData)
        case let .externalEventByName(_, eventData, _):
            expected.append("ping:" + eventData)
        case let .hostInvokeEnding(ending):
            if !ended {
                ended = true
                expected.append((ending.done ? "done:" : "failed:") + ending.data)
            }
        }
    }
    return expected
}

/// `posted` replayed on a machine of its own by one thread: every post, then one tick. What it
/// took and what its invoker was asked.
private func replay(_ posted: [EngineInboxInput<FlatEvent>]) throws -> (taken: [String], invoker: [String]) {
    let (engine, record) = makeProbe()
    try engine.initialize()
    let inbox = engine.inbox
    for input in posted {
        #expect(inbox.post(input), "the replay's inbox refused a post")
    }
    engine.tick()
    engine.stop()
    return (record.taken, record.invoker)
}

/// What the producers of one run share: the order the inbox took their posts in, recorded under
/// one lock with the post so it is the order of the inbox itself; the posts it refused; and how
/// many producers are still posting.
private final class Producers: Sendable {
    let posted = Mutex<[EngineInboxInput<FlatEvent>]>([])
    let refused = Mutex<[String]>([])
    let running: Atomic<Int>

    init(count: Int) {
        running = Atomic<Int>(count)
    }
}

/// Whether a post made on another thread was taken.
private final class PostAnswer: Sendable {
    let taken = Atomic<Bool>(false)
}

/// How many times an inbox woke its owner.
private final class WakeCount: Sendable {
    let count = Atomic<Int>(0)
}

/// Posts `input` from a thread of its own, as a host's worker does, and answers whether it was
/// taken.
private func postFromAnotherThread(_ inbox: EngineInbox<FlatEvent>, _ input: EngineInboxInput<FlatEvent>) -> Bool {
    let answer = PostAnswer()
    let done = DispatchSemaphore(value: 0)
    Thread {
        answer.taken.store(inbox.post(input), ordering: .releasing)
        done.signal()
    }.start()
    if done.wait(timeout: .now() + .seconds(10)) == .timedOut {
        Issue.record("the post on another thread never returned")
        return false
    }
    return answer.taken.load(ordering: .acquiring)
}

@Test func postsFromManyThreadsWhileTheOwnerTicksAreTakenAsASingleThreadedReplayTakesThem() throws {
    let (engine, record) = makeProbe()
    let inbox = engine.inbox
    // The waker is what tells this owner there is something to apply; the owner sleeps on it
    // between ticks rather than on a clock.
    let wake = DispatchSemaphore(value: 0)
    inbox.wakeWith { _ = wake.signal() }
    try engine.initialize()
    let token = try #require(startedToken(record), "`probe` never started")

    let producerCount = 8
    let perProducer = 60
    let producers = Producers(count: producerCount)
    for producer in 0..<producerCount {
        Thread {
            for k in 0..<perProducer {
                let input = postOf(producer: producer, k: k, token: token)
                producers.posted.withLock { posted in
                    if inbox.post(input) {
                        posted.append(input)
                    } else {
                        producers.refused.withLock { $0.append("post \(producer)/\(k)") }
                    }
                }
            }
            // Counted out before the last wake, so an owner asleep on it is not left waiting.
            _ = producers.running.wrappingSubtract(1, ordering: .releasing)
            wake.signal()
        }.start()
    }
    let deadline = ContinuousClock.now + .seconds(30)
    while producers.running.load(ordering: .acquiring) > 0 {
        if ContinuousClock.now > deadline {
            Issue.record("the producers never finished posting")
            return
        }
        _ = wake.wait(timeout: .now() + .milliseconds(50))
        engine.tick()
    }
    // What the last producer posted after the owner's last tick.
    engine.tick()
    // The invocation table: the first ending ended `probe`, so the stop that exits `idle` has
    // nothing to cancel.
    engine.stop()

    let posted = producers.posted.withLock { $0 }
    let refused = producers.refused.withLock { $0 }
    #expect(refused.isEmpty, "an open inbox refused posts: \(refused)")
    #expect(posted.count == producerCount * perProducer)
    #expect(record.taken == expectedFrom(posted), "the machine did not take the posts in the order the inbox took them")
    #expect(record.invoker == ["start:\(token)"], "an ending was not applied, or a refused one was")
    let replayed = try replay(posted)
    #expect(record.taken == replayed.taken, "the run differs from a single-threaded replay of its posts")
    #expect(record.invoker == replayed.invoker, "the invocations differ from a single-threaded replay of the posts")
}

/// W3C test 201's shape, without the HTTP listener: inside the macrostep that enters `idle`, once
/// `probe` is started, the owner waits for a thread that posts `ping` (as a BasicHTTP send waits
/// for the listener that delivers its event), and then `queueOwn` puts an event of the owner's own
/// on the external queue. What the machine took. The semaphore the post signals is the only
/// ordering; nothing sleeps.
private func takenWhenTheOwnerQueuesItsOwnEventAfterAPost(
    _ queueOwn: @escaping (Engine<ScriptedPolicy>, ProbeRecord) -> Void
) throws -> [String] {
    let (engine, record) = makeProbe()
    let inbox = engine.inbox
    let enterIdle = engine.policy.onEntry[.idle]
    engine.policy.onEntry[.idle] = { engine in
        enterIdle?(engine)
        #expect(postFromAnotherThread(inbox, .externalEvent(.ping, eventData: "posted")))
        queueOwn(engine, record)
    }
    try engine.initialize()
    return record.taken
}

@Test func anEventPostedBeforeTheOwnersOwnSendIsTakenFirst() throws {
    let taken = try takenWhenTheOwnerQueuesItsOwnEventAfterAPost { engine, _ in
        // What the generated `<send event="ping"/>` with no target raises.
        var own = EventMetadata.external(sendID: "", sender: "")
        own.data = "own"
        engine.raiseExternalWithMeta(EventWithMetadata<FlatEvent>(.ping, metadata: own))
    }
    #expect(taken == ["ping:posted", "ping:own"], "the owner's own send overtook an event posted before it")
}

@Test func anEventPostedBeforeAHostRunInvocationsDoneInvokeIsTakenFirst() throws {
    let taken = try takenWhenTheOwnerQueuesItsOwnEventAfterAPost { engine, record in
        guard let token = startedToken(record) else {
            Issue.record("`probe` never started")
            return
        }
        // The host completes `probe` on the owner, inside the same macrostep.
        #expect(engine.completeHostInvoke(processorType: probeType, invokeID: probeID, token: token, doneData: "own"))
    }
    #expect(taken == ["ping:posted", "done:own"], "the owner's own done.invoke overtook an event posted before it")
}

@Test func aPostedEndingOfACancelledOrRestartedStartIsRefusedAsADirectOneIs() throws {
    let (engine, record) = makeProbe()
    let inbox = engine.inbox
    try engine.initialize()
    let first = try #require(startedToken(record), "`probe` never started")
    // `go` leaves `idle`, which cancels the first start.
    engine.processEvent(.go)
    #expect(record.invoker == ["start:\(first)", "cancel:\(first)"])

    #expect(postFromAnotherThread(inbox, .hostInvokeEnding(ending(first, done: true, data: "after-cancel"))))
    engine.tick()
    #expect(record.taken.isEmpty, "a cancelled start's posted ending was taken")
    #expect(
        !engine.completeHostInvoke(processorType: probeType, invokeID: probeID, token: first, doneData: "direct"),
        "a cancelled start's direct ending was taken")

    // `again` enters `idle` again: the same invocation, a new start.
    engine.processEvent(.again)
    let second = try #require(startedToken(record), "`probe` never started again")
    #expect(first != second, "a restart reused the first start's token")

    #expect(postFromAnotherThread(inbox, .hostInvokeEnding(ending(first, done: false, data: "stale"))))
    #expect(postFromAnotherThread(inbox, .hostInvokeEnding(ending(second, done: true, data: "fresh"))))
    engine.tick()
    #expect(record.taken == ["done:fresh"], "the first start's ending was taken for the second's")
    #expect(
        !engine.failHostInvoke(processorType: probeType, invokeID: probeID, token: first, errorData: "direct"),
        "the first start's direct ending was taken")
}

@Test func aPostBeforeTheRunIsTakenByItAndOneAfterTheStopIsRefused() throws {
    let (engine, record) = makeProbe()
    let inbox = engine.inbox
    let woken = WakeCount()
    inbox.wakeWith { _ = woken.count.wrappingAdd(1, ordering: .relaxed) }

    #expect(inbox.post(.externalEvent(.ping, eventData: "early")))
    #expect(woken.count.load(ordering: .relaxed) == 1, "a taken post did not wake the owner")
    #expect(record.taken.isEmpty, "a post was applied before the run started")
    // Applied at the first macrostep boundary of the run.
    try engine.initialize()
    #expect(record.taken == ["ping:early"])

    engine.stop()
    #expect(!inbox.post(.externalEvent(.ping, eventData: "late")), "a stopped engine's inbox took a post")
    #expect(woken.count.load(ordering: .relaxed) == 1, "a refused post woke the owner")

    // A run entered again has an open inbox.
    try engine.initialize()
    #expect(inbox.post(.externalEventByName("ping", eventData: "again")))
    engine.tick()
    #expect(record.taken == ["ping:early", "ping:again"])
}

@Test func whatTheInboxHeldWhenTheRunEndedIsCountedUnseenAndLaterPostsAreRefused() throws {
    let (engine, record) = makeProbe()
    let inbox = engine.inbox
    // Posted while the run's last macrostep runs: after that macrostep's boundary was drained, and
    // before the run ended.
    let cancelOnExit = engine.policy.onExit[.idle]
    engine.policy.onExit[.idle] = { engine in
        cancelOnExit?(engine)
        #expect(inbox.post(.externalEvent(.ping, eventData: "during")))
    }
    try engine.initialize()
    engine.processEvent(.request)
    #expect(engine.isInFinalState)
    #expect(record.taken.isEmpty, "an ended run took an event")
    #expect(engine.unseenExternalEvents == 1, "what the inbox held when the run ended was not counted")
    #expect(!inbox.post(.externalEvent(.ping, eventData: "after")), "an ended run's inbox took a post")
    #expect(record.invoker == ["start:1", "cancel:1"])
}

@Test func anInboxFirstAskedForAfterAStopRefusesPostsUntilTheNextRun() throws {
    let (engine, _) = makeProbe()
    try engine.initialize()
    engine.stop()
    #expect(!engine.inbox.post(.externalEvent(.ping)), "an inbox made after the stop took a post")
    try engine.initialize()
    #expect(engine.inbox.post(.externalEvent(.ping)), "the next run's inbox refused a post")
}
