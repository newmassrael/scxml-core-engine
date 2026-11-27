// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//     was stopped or ended, after what it held is applied, and opens with the next run;
//   * each run of the inbox under posters on several threads is recorded as an inbox history
//     (docs/adr/0147, D3): every post with what the inbox answered, every item the owner put on
//     the external queue as the engine's inbox port tells it, and the end of the run, against one
//     clock. Each history is held here to the arrival order the drain before an event of the
//     owner's own keeps, and written where `SCE_INBOX_HISTORY_DIR` names, for
//     `sce-codegen check-inbox-history` to judge linearizable to docs/formal/InboxService.tla
//     (`scripts/gates/swift-backend.sh` runs it). The Swift inbox has one door, the session's own
//     handle: no input it takes names a session, so it keeps no directory.

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


// ── Each run of the inbox, as an inbox history (docs/adr/0147, D3) ──────────────────────────────

/// Threads that post in a recorded run, beside the owner, and the posts each makes.
private let historyPosters = 4
private let historyPerPoster = 120

/// Own events the owner raises at most in a recorded run, and the first one's value, past every
/// input a poster posts.
private let historyOwnPerRun = 40
private let historyOwnBase = 1_000_000

/// The input of the post made once the run has ended, past every other input of the run.
private let historyLateInput = 900_000

/// Runs recorded, each written as one history: the schedules that matter are a small fraction of
/// those a machine gives.
private let historyRuns = 4

/// The one clock every thread of a recorded run reads, immediately before and immediately after
/// each operation it records: one read-modify-write chain on one atomic orders the readings with
/// the calls between them, so "returned before invoked" in the record means it in the run, and no
/// two readings are equal.
private final class HistoryClock: Sendable {
    private let readings = Atomic<Int>(0)

    func tick() -> Int {
        readings.wrappingAdd(1, ordering: .sequentiallyConsistent).newValue
    }
}

/// One post a poster observed, through the session's own handle, taken or refused. The one
/// reason a Swift inbox refuses a post is that its run has ended.
private struct RecordedPost {
    let input: Int
    let taken: Bool
    let invoked: Int
    let returned: Int

    var json: String {
        let answered = taken ? "\"outcome\":\"taken\"" : "\"outcome\":\"refused\",\"reason\":\"ended\""
        return "{\"call\":\"post\",\"session\":0,\"input\":\(input),\"door\":\"handle\",\(answered),"
            + "\"invoked\":\(invoked),\"returned\":\(returned)}"
    }
}

/// One operation of the owner: an item that entered the session's external queue, or the end of
/// its run, which names no item (`item` nil).
private struct RecordedOwnerOperation {
    let item: Int?
    let own: Bool
    let invoked: Int
    let returned: Int

    var json: String {
        let call: String
        if let item {
            call = "\"call\":\"enqueue\",\"item\":\(item),\"source\":\"\(own ? "own" : "posted")\""
        } else {
            call = "\"call\":\"end\""
        }
        return "{\(call),\"session\":0,\"invoked\":\(invoked),\"returned\":\(returned)}"
    }
}

/// What the owner of a recorded run did, as its engine's inbox port tells it: each item it put on
/// the external queue, read from the run's clock around it, and the end of the run, read around
/// the close of the inbox, from which every post is refused. An event of the owner's own is
/// invoked where the port is first told it, before the drain it begins with, and the inputs that
/// drain applies lie within it. Told on the owner only.
private final class OwnerRecorder: InboxPort {
    typealias Event = FlatEvent

    private let clock: HistoryClock
    private(set) var operations: [RecordedOwnerOperation] = []
    private var ownInvoked = 0
    private var postedInvoked = 0
    private var closeInvoked = 0

    init(clock: HistoryClock) {
        self.clock = clock
    }

    func enqueueBegins(_ event: EventWithMetadata<FlatEvent>, posted: Bool) {
        let now = clock.tick()
        if posted {
            postedInvoked = now
        } else {
            ownInvoked = now
        }
    }

    func enqueueEnds(_ event: EventWithMetadata<FlatEvent>, posted: Bool) {
        operations.append(
            RecordedOwnerOperation(
                // -1, which the history format refuses, for an event whose data names no item.
                item: Int(event.metadata.data) ?? -1, own: !posted, invoked: posted ? postedInvoked : ownInvoked,
                returned: clock.tick()))
    }

    func closeBegins() {
        closeInvoked = clock.tick()
    }

    func closeEnds() {
        operations.append(RecordedOwnerOperation(item: nil, own: false, invoked: closeInvoked, returned: clock.tick()))
    }
}

/// Posts `ping` carrying `value` through `inbox`, recorded with `clock` read immediately around
/// the post.
private func recordPost(_ clock: HistoryClock, _ inbox: EngineInbox<FlatEvent>, _ value: Int) -> RecordedPost {
    let invoked = clock.tick()
    let taken = inbox.post(.externalEvent(.ping, eventData: "\(value)"))
    let returned = clock.tick()
    return RecordedPost(input: value, taken: taken, invoked: invoked, returned: returned)
}

/// What the posters of a recorded run share: each one's posts, handed over when it has posted
/// everything, how many posts were made, and how many posters are still posting.
private final class HistoryPosters: Sendable {
    let logs: Mutex<[[RecordedPost]]>
    let made = Atomic<Int>(0)
    let running: Atomic<Int>

    init(count: Int) {
        logs = Mutex(Array(repeating: [], count: count))
        running = Atomic<Int>(count)
    }
}

/// Runs `work` on a thread of its own and answers what it posted; nil, recorded as an issue, when
/// it never returned.
private func postsFromAnotherThread(_ work: @escaping @Sendable () -> [RecordedPost]) -> [RecordedPost]? {
    let answer = Mutex<[RecordedPost]?>(nil)
    let done = DispatchSemaphore(value: 0)
    Thread {
        let posts = work()
        answer.withLock { $0 = posts }
        done.signal()
    }.start()
    if done.wait(timeout: .now() + .seconds(10)) == .timedOut {
        Issue.record("the posts on another thread never returned")
        return nil
    }
    return answer.withLock { $0 }
}

/// One recorded run: the owner's operations and each poster's posts, each in program order; nil
/// when the run could not be made, with the reason recorded.
///
/// A post is taken and returns before the owner raises an event of its own, so every run holds
/// that order. Then `historyPosters` threads post through the session's own handle while the owner
/// ticks the machine, raises events of its own and stops the run once half the posts are made. A
/// post once the run has ended closes the record.
private func recordInboxRun() throws -> (owner: [RecordedOwnerOperation], posters: [[RecordedPost]])? {
    let clock = HistoryClock()
    let recorder = OwnerRecorder(clock: clock)
    let (engine, _) = makeProbe()
    let inbox = engine.inbox
    engine.setInboxPort(recorder)
    try engine.initialize()
    guard let first = postsFromAnotherThread({ [recordPost(clock, inbox, 1)] }) else {
        return nil
    }
    var posters = [first]
    engine.raiseExternal(.ping, eventData: "\(historyOwnBase)")

    let shared = HistoryPosters(count: historyPosters)
    let total = historyPosters * historyPerPoster
    for poster in 0..<historyPosters {
        Thread {
            var log: [RecordedPost] = []
            for k in 0..<historyPerPoster {
                log.append(recordPost(clock, inbox, 2 + poster * historyPerPoster + k))
                _ = shared.made.wrappingAdd(1, ordering: .releasing)
            }
            // Handed over before the poster counts itself out, so the owner reads every post.
            shared.logs.withLock { $0[poster] = log }
            _ = shared.running.wrappingSubtract(1, ordering: .releasing)
        }.start()
    }

    let deadline = ContinuousClock.now + .seconds(30)
    var own = historyOwnBase + 1
    var round = 0
    var stopped = false
    while shared.running.load(ordering: .acquiring) > 0 {
        if ContinuousClock.now > deadline {
            Issue.record("the posters never finished posting")
            return nil
        }
        if !stopped {
            engine.tick()
            round += 1
            if round % 3 == 0 && own < historyOwnBase + historyOwnPerRun {
                engine.raiseExternal(.ping, eventData: "\(own)")
                own += 1
            }
            if shared.made.load(ordering: .acquiring) * 2 >= total {
                engine.stop()
                stopped = true
            }
        }
        Thread.sleep(forTimeInterval: 0.000_001)
    }
    if !stopped {
        engine.tick()
        engine.stop()
    }
    posters += shared.logs.withLock { $0 }
    guard let late = postsFromAnotherThread({ [recordPost(clock, inbox, historyLateInput)] }) else {
        return nil
    }
    posters.append(late)
    return (recorder.operations, posters)
}

/// The run as an inbox history (`tests/forge/conformance/inbox_history.schema.json`): one
/// session, whose inbox closes with its run and has no bound.
private func historyJSON(owner: [RecordedOwnerOperation], posters: [[RecordedPost]]) -> String {
    let owned = "[" + owner.map(\.json).joined(separator: ",") + "]"
    let posted = "[" + posters.map { "[" + $0.map(\.json).joined(separator: ",") + "]" }.joined(separator: ",") + "]"
    return "{\"version\":2,\"sessions\":[0],\"closes\":true,\"owner\":\(owned),\"posters\":\(posted)}"
}

/// Holds a recorded run, named `what`, to what it was meant to exercise and to the order the door
/// keeps, as far as one run's record says it: posts taken and refused as ended, own events entered
/// and the run ended once; every input a post took entered once; and an event of the owner's own
/// entered behind every input whose post returned before the event's enqueue began (docs/adr/0084,
/// D2 as amended at step 9). Whether the whole record is linearizable is
/// `sce-codegen check-inbox-history`'s to judge.
private func expectRunKeepsTheDoorsOrder(_ what: String, owner: [RecordedOwnerOperation], posters: [[RecordedPost]]) {
    let posts = posters.flatMap { $0 }
    #expect(posts.contains { $0.taken }, "\(what): no post was taken")
    #expect(posts.contains { !$0.taken }, "\(what): no post was refused as ended")
    #expect(owner.contains { $0.item != nil && $0.own }, "\(what): the owner put no event of its own on the queue")
    #expect(owner.filter { $0.item == nil }.count == 1, "\(what): the run did not end once")

    let taken = posts.filter(\.taken).map(\.input).sorted()
    let entered = owner.filter { $0.item != nil && !$0.own }.compactMap(\.item).sorted()
    #expect(entered == taken, "\(what): the inputs that entered the queue are not the inputs the posts took")

    var enteredAt: [Int: Int] = [:]
    for (at, op) in owner.enumerated() where !op.own {
        if let item = op.item {
            enteredAt[item] = at
        }
    }
    for (at, op) in owner.enumerated() where op.own {
        for post in posts where post.taken && post.returned < op.invoked {
            guard let postedAt = enteredAt[post.input] else {
                Issue.record("\(what): input \(post.input) never entered")
                continue
            }
            #expect(
                postedAt < at,
                "\(what): own event \(op.item ?? -1) entered ahead of input \(post.input), whose post returned before the event's enqueue began"
            )
        }
    }
}

/// docs/adr/0147, D3: each recorded run of the inbox keeps the order the door promises as far as its
/// own record says it, and is written as an inbox history where `SCE_INBOX_HISTORY_DIR` names, one
/// file a run, for `sce-codegen check-inbox-history` to judge linearizable to
/// docs/formal/InboxService.tla. Unset, the runs are held here and written nowhere.
@Test func eachRunOfTheInboxIsRecordedAsAHistoryThatKeepsTheDoorsOrder() throws {
    let named = ProcessInfo.processInfo.environment["SCE_INBOX_HISTORY_DIR"].flatMap { $0.isEmpty ? nil : $0 }
    let directory = named.map { URL(fileURLWithPath: $0, isDirectory: true) }
    if let directory {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }
    for run in 0..<historyRuns {
        guard let recorded = try recordInboxRun() else {
            return
        }
        expectRunKeepsTheDoorsOrder("run \(run)", owner: recorded.owner, posters: recorded.posters)
        if let directory {
            try (historyJSON(owner: recorded.owner, posters: recorded.posters) + "\n").write(
                to: directory.appendingPathComponent("swift_inbox_run_\(run).json"), atomically: true, encoding: .utf8)
        }
    }
}
