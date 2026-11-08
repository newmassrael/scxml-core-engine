// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import SceTestProfile
import Testing

// The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the rules of §2 to §5 with
// components written here by hand, before any machine is generated: each one keeps its context from
// `start` on, reads its snapshot and calls its context as a generated machine does (§12.3). The runner
// of a suite is held to P11 over test cases made of the same components.
//
// Each suite and case is the twin of the Kotlin class and case of the same name in
// backends/kotlin/tests/src/test/kotlin/com/sce/runtime/test/TestCaseRunTest.kt, itself the twin of the
// Rust module in backends/rust/runtime/tests/a_test_case_runs_as_the_test_profile_says.rs and of the
// C++ gtest in tests/test_runtime/TestCaseRunTest.cpp, with the same scenario, verdict, `NOW` and log.
// The cases of `TheSharedHandleTests` hold what the Rust runtime's handle holds, which the Swift context
// holds too: re-entrant use is refused, and a context outlives no component it names. Each such misuse
// stops the program, as a panic does in Rust, so each is run as an exit test, in a process of its own,
// and read by what the process wrote. The Kotlin `TheRunIsRunOnceTest` has no twin: a `TestCaseRun` is
// not copyable and its `run` consumes it, so a second run of it does not compile, as in Rust. The case
// of `TheDoorsTests` reaches the run only through the two host processors a generated component
// registers on its engine (§12.3), written here with the types the engine registers them under.

private let second: TestDuration = 1_000_000

/// What a hand-written component does when it starts, or on a snapshot.
private typealias Step = (ComponentContext, Scripted) -> Void

private func idle() -> Step {
    { _, _ in }
}

/// A component written by hand: what it does when it starts, and what it does on each snapshot. Either
/// may end it, as entering a top-level final does.
private final class Scripted: ComponentMachine {
    private let onStart: Step
    private let onSnapshot: Step
    private var context: ComponentContext?
    private var ended = false

    init(_ onStart: @escaping Step, _ onSnapshot: @escaping Step) {
        self.onStart = onStart
        self.onSnapshot = onSnapshot
    }

    func end() {
        ended = true
    }

    func start(_ context: ComponentContext) {
        self.context = context
        onStart(context, self)
    }

    func deliverSnapshot() {
        guard let context else {
            fatalError("a snapshot is delivered only to a started machine")
        }
        onSnapshot(context, self)
    }

    var terminated: Bool {
        ended
    }
}

/// The machines of a test, by component type.
private final class Machines {
    private var types: [String: (Step, Step)] = [:]

    func with(_ componentType: String, _ steps: (Step, Step)) -> Machines {
        types[componentType] = steps
        return self
    }

    func factory() -> MachineFactory {
        let known = types
        return { componentType in
            guard let steps = known[componentType] else {
                return nil
            }
            return Scripted(steps.0, steps.1)
        }
    }
}

/// A value of no record, which a message whose value no test reads carries.
private struct Blank: Sendable {}

private func message(_ event: String) -> Message {
    Message(event: event, value: Blank())
}

/// One input of a trace, or one send the adapter was handed.
private struct Input: Equatable {
    let at: TestInstant
    let systemPort: String
    let event: String
}

/// A system adapter that replays a trace and records what was sent to it.
private final class TraceAdapter: SystemAdapter {
    private let trace: [Input]
    private var next = 0
    private(set) var sent: [Input] = []

    init(_ trace: [Input] = []) {
        self.trace = trace
    }

    func send(systemPort: String, message: Message, now: TestInstant) {
        sent.append(Input(at: now, systemPort: systemPort, event: message.event))
    }

    func nextInput() -> TestInstant? {
        next < trace.count ? trace[next].at : nil
    }

    func takeInputsDue(now: TestInstant) -> [SystemInput] {
        var due: [SystemInput] = []
        while next < trace.count && trace[next].at <= now {
            let input = trace[next]
            due.append(SystemInput(systemPort: input.systemPort, message: message(input.event)))
            next += 1
        }
        return due
    }
}

private func logged(_ result: TestCaseResult, _ component: String, _ kind: LogKind) -> Bool {
    result.log.contains { $0.component == component && $0.kind == kind }
}

/// The ids each component type declares, from `(type, id)` pairs.
private func byType(_ entries: (String, String)...) -> [String: [String]] {
    Dictionary(grouping: entries, by: { $0.0 }).mapValues { pairs in pairs.map { $0.1 } }
}

/// An MTC and one PTC joined by one connection, the MTC's `p` to the PTC's `q`.
private func mtcAndEcho() -> Configuration {
    Configuration(
        mtcType: "Tester",
        ptcTypes: ["echo": "Echo"],
        portsOfType: byType(("Tester", "p"), ("Echo", "q")),
        timersOfType: byType(("Tester", "reply")),
        connections: [Connection(Endpoint(component: "mtc", port: "p"), Endpoint(component: "echo", port: "q"))])
}

/// An MTC alone, of the type `componentType`, whose port `p` is mapped to the system port `sut`.
private func mapped(_ componentType: String) -> Configuration {
    Configuration(
        mtcType: componentType,
        portsOfType: byType((componentType, "p")),
        mappings: [Mapping(Endpoint(component: "mtc", port: "p"), systemPort: "sut")])
}

/// An MTC alone, of the type `componentType`, with no port and no timer.
private func lone(_ componentType: String) -> Configuration {
    Configuration(mtcType: componentType)
}

/// The MTC creates the echo, sends to it, and waits for the answer within a second: a receive branch
/// passes, a timeout branch fails.
private func tester() -> (Step, Step) {
    (
        { context, _ in
            context.create("echo")
            context.send("p", message("ping"))
            context.startTimer("reply", second)
        },
        repliesInTime()
    )
}

private func repliesInTime() -> Step {
    { context, me in
        if context.snapshotPresent("p") {
            #expect(context.receive("p").event == "pong")
            context.stopTimer("reply")
            context.setverdict(.pass)
            me.end()
        } else if context.timedOut("reply") {
            context.timeout("reply")
            context.setverdict(.fail, "no reply in time")
            me.end()
        }
    }
}

private func runCase(
    _ configuration: Configuration, _ machines: Machines, _ adapter: any SystemAdapter,
    guardAt: TestDuration? = nil, parameters: ModuleParameters = ModuleParameters()
) -> TestCaseResult {
    TestCaseRun(configuration, machines.factory(), adapter, guardAt: guardAt, moduleParameters: parameters).run()
}

/// A `uint32` module parameter's value, as a generated suite writes its default.
private func saved(_ value: UInt32) -> ModuleParameterValue {
    SavedValues.of(value)
}

/// What a component read of a module parameter, if it read one.
private final class Slot {
    var value: UInt32?
}

/// §11.5, §11.8: a component reads the module parameters of its run into its variables before it starts,
/// as a generated machine reads each through the reader of the type it declares.
private func readsJob(_ job: Slot) -> (Step, Step) {
    (
        { context, me in
            let read = context.readModuleParameters { given in
                job.value = try SavedValues.uint32(given.of("job"), "job")
            }
            if read {
                context.setverdict(.pass)
                me.end()
            }
        },
        idle()
    )
}

/// The names a test records as it runs, in order.
private final class Recorded {
    var names: [String] = []
}

struct TestVerdictTests {
    @Test func onlyEverBecomesWorse() {
        #expect(worse(.pass, Verdict.none) == .pass)
        #expect(worse(.pass, .inconclusive) == .inconclusive)
        #expect(worse(.fail, .pass) == .fail)
        #expect(worse(.fail, .error) == .error)
        #expect(Verdict.inconclusive.spelling == "inconc")
    }
}

struct TestPortTests {
    @Test func aSnapshotSeesOnlyWhatWasQueuedWhenItWasTaken() {
        let port = TestPort()
        port.takeSnapshot()
        port.enqueue(message("late"))
        #expect(port.snapshot()?.event == nil, "a message that arrives after the snapshot is not in it")
        port.takeSnapshot()
        #expect(port.snapshot()?.event == "late")
        port.enqueue(message("behind"))
        #expect(port.snapshot()?.event == "late", "a later message goes behind the head")
        #expect(port.removeHead().event == "late")
        #expect(port.count == 1)
    }
}

struct TestTimerTests {
    @Test func timesOutAtItsInstantAndRestartsWhenStartedAgain() {
        let timer = TestTimer()
        timer.start(now: 0, duration: 10)
        timer.update(now: 9)
        #expect(timer.status == .running)
        timer.start(now: 5, duration: 10)
        #expect(timer.expiry == 15, "a running timer that is started again restarts")
        timer.update(now: 15)
        #expect(timer.status == .timeout)
        timer.takeSnapshot()
        #expect(timer.snapshotTimedOut)
        timer.timeoutTaken()
        #expect(timer.status == .idle)
        timer.stop()
        #expect(timer.status == .idle, "stopping a timer that is not running does nothing")
    }
}

/// The SUT's four messages, all at `NOW = 0`: a record of another type, another event, another value,
/// and the one the template matches.
private final class FourInputs: SystemAdapter {
    private var given = false

    func send(systemPort: String, message: Message, now: TestInstant) {}

    func nextInput() -> TestInstant? {
        nil
    }

    func takeInputsDue(now: TestInstant) -> [SystemInput] {
        if now != 0 || given {
            return []
        }
        given = true
        return [
            SystemInput(systemPort: "sut", message: Message(event: "number", value: "7")),
            SystemInput(systemPort: "sut", message: Message(event: "other", value: 7)),
            SystemInput(systemPort: "sut", message: Message(event: "number", value: 6)),
            SystemInput(systemPort: "sut", message: Message(event: "number", value: 7)),
        ]
    }
}

struct TestCaseRunTests {
    /// P6, P9, §3.2: a message sent in a round is taken in a later round, and the whole exchange happens
    /// at one instant, because a branch fired in each round.
    @Test func anAnsweredRequestPassesWithoutTimeMoving() {
        let machines = Machines().with("Tester", tester()).with(
            "Echo",
            (
                idle(),
                { context, _ in
                    if context.snapshotPresent("q") {
                        #expect(context.receive("q").event == "ping")
                        context.send("q", message("pong"))
                    }
                }
            ))
        let result = runCase(mtcAndEcho(), machines, TraceAdapter())
        #expect(result.verdict == .pass)
        #expect(result.endedAt == 0)
        #expect(logged(result, "echo", .kill), "the MTC's end removes the PTC (P10)")
    }

    /// P7, §3.3: with nothing to receive, time advances to the timer's instant.
    @Test func anUnansweredRequestFailsWhenItsTimerExpires() {
        let machines = Machines().with("Tester", tester()).with("Echo", (idle(), idle()))
        let result = runCase(mtcAndEcho(), machines, TraceAdapter())
        #expect(result.verdict == .fail)
        #expect(result.endedAt == second)
    }

    /// P13: no branch can fire and no instant is pending, so the test case can never end; the runtime
    /// ends it with error.
    @Test func aTestCaseThatCanNeverEndIsAnError() {
        let machines = Machines().with("Waiter", (idle(), idle()))
        let result = runCase(mapped("Waiter"), machines, TraceAdapter())
        #expect(result.verdict == .error)
        #expect(logged(result, "mtc", .error))
    }

    /// P11: the guard instant ends a test case that has not ended, with error.
    @Test func theGuardEndsATestCaseWithError() {
        let machines = Machines().with("Waiter", ({ context, _ in context.setverdict(.pass) }, idle()))
        let result = runCase(mapped("Waiter"), machines, TraceAdapter(), guardAt: 5 * second)
        #expect(result.verdict == .error)
        #expect(result.endedAt == 5 * second)
    }

    /// §2.2, §12.2: a mapped port's sends reach the adapter, and the trace's inputs reach the port at
    /// their instants.
    @Test func aMappedPortTalksToTheSystemAdapterOnTheLogicalClock() {
        var configuration = mapped("Tester")
        configuration.timersOfType = byType(("Tester", "reply"))
        let machines = Machines().with(
            "Tester",
            (
                { context, _ in
                    context.send("p", message("ping"))
                    context.startTimer("reply", second)
                },
                repliesInTime()
            ))
        let adapter = TraceAdapter([Input(at: 500, systemPort: "sut", event: "pong")])
        let result = runCase(configuration, machines, adapter)
        #expect(result.verdict == .pass)
        #expect(result.endedAt == 500)
        #expect(adapter.sent == [Input(at: 0, systemPort: "sut", event: "ping")])
    }

    /// P9, P14: a removed PTC's verdict still counts in the test case verdict.
    @Test func aRemovedPtcsVerdictStillCounts() {
        let machines = Machines()
            .with(
                "Tester",
                (
                    { context, _ in
                        context.create("echo")
                        context.startTimer("reply", 10)
                    },
                    { context, me in
                        if context.timedOut("reply") {
                            context.timeout("reply")
                            context.kill("echo")
                            context.setverdict(.pass)
                            me.end()
                        }
                    }
                ))
            .with("Echo", ({ context, _ in context.setverdict(.fail) }, idle()))
        let result = runCase(mtcAndEcho(), machines, TraceAdapter())
        #expect(result.verdict == .fail)
    }

    /// §3.2 step 4: components select one at a time in creation order, so when the MTC terminates in a
    /// round, a PTC that would have selected after it in that round does not.
    @Test func nothingSelectsAfterTheMtcTerminatesInTheRound() {
        var configuration = mtcAndEcho()
        configuration.timersOfType["Echo"] = ["late"]
        let machines = Machines()
            .with(
                "Tester",
                (
                    { context, _ in
                        context.create("echo")
                        context.startTimer("reply", 10)
                    },
                    { context, me in
                        if context.timedOut("reply") {
                            context.timeout("reply")
                            context.setverdict(.pass)
                            me.end()
                        }
                    }
                ))
            .with(
                "Echo",
                (
                    { context, _ in context.startTimer("late", 10) },
                    { context, _ in
                        if context.timedOut("late") {
                            context.timeout("late")
                            context.setverdict(.fail)
                        }
                    }
                ))
        let result = runCase(configuration, machines, TraceAdapter())
        #expect(result.verdict == .pass)
        #expect(!logged(result, "echo", .timeout))
    }

    /// P2, P10: a PTC that terminates is in DONE from the next snapshot on, and a done branch removes
    /// nothing.
    @Test func aDoneBranchSeesAPtcThatTerminated() {
        let machines = Machines()
            .with(
                "Tester",
                (
                    { context, _ in context.create("echo") },
                    { context, me in
                        if context.done("echo") {
                            context.doneTaken("echo")
                            context.kill("echo")
                            me.end()
                        }
                    }
                ))
            .with(
                "Echo",
                (
                    { context, me in
                        context.setverdict(.pass)
                        me.end()
                    },
                    idle()
                ))
        let result = runCase(mtcAndEcho(), machines, TraceAdapter())
        #expect(result.verdict == .pass)
        #expect(logged(result, "echo", .done))
    }

    /// P2, §12.3: a receive branch with a template matches only a message of its record, under its
    /// record's event, whose value its `match` holds; the guard reads the snapshot and removes nothing.
    @Test func aReceiveWithATemplateMatchesOnlyItsRecordsValue() {
        let seen = Recorded()
        let machines = Machines().with(
            "Reader",
            (
                idle(),
                { context, me in
                    if receives(context, "p", "number", as: Int.self, { $0 == 7 }) {
                        seen.names.append("seven")
                        #expect(
                            receives(context, "p", "number", as: Int.self, { $0 == 7 }),
                            "a guard that matched removed the message")
                        _ = context.receive("p")
                        me.end()
                    } else if context.snapshotPresent("p") {
                        if let event = context.withSnapshot("p", { head in head?.event }) {
                            seen.names.append(event)
                        }
                        _ = context.receive("p")
                    }
                }
            ))
        let result = runCase(mapped("Reader"), machines, FourInputs())
        #expect(
            seen.names == ["number", "other", "number", "seven"],
            "a value of another type, another event and another value do not match")
        #expect(result.verdict == Verdict.none)
    }

    /// §11.8: the value the run is given is the one the component reads, and the component starts on it.
    @Test func aComponentReadsTheModuleParameterItsRunIsGiven() {
        let job = Slot()
        let machines = Machines().with("Reader", readsJob(job))
        var parameters = ModuleParameters()
        parameters.set("job", saved(7))
        let result = runCase(lone("Reader"), machines, TraceAdapter(), parameters: parameters)
        #expect(job.value == 7)
        #expect(result.verdict == .pass)
    }

    /// P13: a module parameter the run has no value for is a dynamic error, named in the log, and the
    /// component goes no further than the read.
    @Test func aModuleParameterTheRunIsNotGivenIsAnError() throws {
        let job = Slot()
        let machines = Machines().with("Reader", readsJob(job))
        let result = runCase(lone("Reader"), machines, TraceAdapter())
        #expect(result.verdict == .error)
        #expect(job.value == nil)
        let last = try #require(result.log.last, "the run logged its error")
        #expect(last.kind == .error)
        #expect(last.detail.contains("'job'"), "\(last.detail)")
    }

    /// P13: a value that is not of the type the component declares is refused as a restored value of
    /// another type is, and not converted.
    @Test func aModuleParameterOfAnotherTypeIsAnError() {
        let job = Slot()
        let machines = Machines().with("Reader", readsJob(job))
        var parameters = ModuleParameters()
        parameters.set("job", SavedValues.of("seven"))
        let result = runCase(lone("Reader"), machines, TraceAdapter(), parameters: parameters)
        #expect(result.verdict == .error)
        #expect(job.value == nil)
        #expect(logged(result, "mtc", .error))
    }

    /// P13: a component type the factory of machines makes no machine for ends the run with error where
    /// a component of it is created.
    @Test func aComponentTypeWithNoMachineIsAnError() {
        let result = TestCaseRun(lone("Ghost"), { _ in nil }, TraceAdapter()).run()
        #expect(result.verdict == .error)
        #expect(logged(result, "mtc", .error))
        #expect(!logged(result, "mtc", .start), "no component of the type was created")
    }

    /// P6, P13: a send on a port whose peer is not present is a dynamic error.
    @Test func aSendToAnAbsentPeerIsAnError() {
        let machines = Machines().with("Tester", ({ context, _ in context.send("p", message("ping")) }, idle()))
        let result = runCase(mtcAndEcho(), machines, TraceAdapter())
        #expect(result.verdict == .error)
        #expect(result.endedAt == 0)
    }
}

/// A test case whose MTC, of type `componentType`, sets `verdict` as it starts and ends there.
private func endsWith(_ name: String, _ componentType: String, _ verdict: Verdict) -> TestCase {
    let machines = Machines().with(
        componentType,
        (
            { context, me in
                context.setverdict(verdict)
                me.end()
            },
            idle()
        ))
    return TestCase(name: name, configuration: lone(componentType), machines: machines.factory())
}

/// The adapters of a suite's run, each a fresh `TraceAdapter`, recording the test case each was built
/// for.
private func tracing(_ built: Recorded) -> (String) -> (any SystemAdapter)? {
    { testCase in
        built.names.append(testCase)
        return TraceAdapter()
    }
}

/// The selection of a world that enables `feature`.
private func selects(_ feature: String) -> Selection {
    { world in enabled(world, feature) }
}

struct TestSuiteRunTests {
    /// P11: the runner runs the test cases a world selects, in the suite's order, and skips the others; a
    /// test case with no selection runs in every world.
    @Test func runsTheTestCasesAWorldSelectsInTheSuitesOrder() throws {
        let suite = Suite(
            name: "ordered",
            executes: [
                Execute(endsWith("first", "First", .pass)),
                Execute(endsWith("second", "Second", .fail), selection: selects("x")),
                Execute(endsWith("third", "Third", .inconclusive), selection: selects("y")),
            ])
        let built = Recorded()
        let results = try runSuite(suite, world: ["y"], adapters: tracing(built))
        #expect(results.map(\.testCase) == ["first", "third"])
        #expect(results.map(\.result.verdict) == [.pass, .inconclusive])
        #expect(built.names == ["first", "third"], "an adapter is built for each case run")
    }

    /// P11: each test case is a fresh run from NOW = 0 under its own guard.
    @Test func eachTestCaseIsAFreshRunUnderItsOwnGuard() throws {
        let waiting = Machines().with("Waiter", (idle(), idle()))
        let stuck = TestCase(name: "stuck", configuration: lone("Waiter"), machines: waiting.factory())
        let suite = Suite(
            name: "guarded",
            executes: [
                Execute(stuck, guardAt: 5 * second),
                Execute(endsWith("quick", "Quick", .pass)),
            ])
        let built = Recorded()
        let results = try runSuite(suite, world: [], adapters: tracing(built))
        #expect(results.count == 2)
        #expect(results[0].result.verdict == .error)
        #expect(results[0].result.endedAt == 5 * second, "the guard ends the first")
        #expect(results[1].result.verdict == .pass)
        #expect(results[1].result.endedAt == 0, "the second starts again at NOW = 0")
    }

    /// -1 §8.2.1: a value the caller gives a module parameter replaces the suite's default, and a
    /// parameter the suite does not declare cannot be given one.
    @Test func anOverrideReplacesTheDefaultOfAModuleParameterTheSuiteDeclares() throws {
        let job = Slot()
        let machines = Machines().with("Reader", readsJob(job))
        var defaults = ModuleParameters()
        defaults.set("job", saved(7))
        let suite = Suite(
            name: "parameters",
            moduleParameters: defaults,
            executes: [
                Execute(
                    TestCase(
                        name: "reads", configuration: lone("Reader"), machines: machines.factory(),
                        moduleParameters: ["job"]))
            ])
        let built = Recorded()
        let results = try runSuite(suite, world: [], adapters: tracing(built))
        #expect(results.map(\.result.verdict) == [.pass])
        #expect(job.value == 7, "the default")

        var overrides = ModuleParameters()
        overrides.set("job", saved(9))
        let overridden = try runSuite(suite, world: [], adapters: tracing(built), overrides: overrides)
        #expect(overridden.count == 1)
        #expect(job.value == 9, "the override")

        var stray = ModuleParameters()
        stray.set("limit", saved(1))
        let refusal = #expect(throws: OverrideRefusal.self, "an override of an undeclared parameter is refused") {
            _ = try runSuite(suite, world: [], adapters: tracing(built), overrides: stray)
        }
        #expect(refusal?.reason.contains("'limit'") == true, "\(String(describing: refusal))")
    }

    /// P13: a test case whose components read a module parameter the run has no value for ends with error
    /// before anything of it runs, its adapter included.
    @Test func aTestCaseThatReadsAModuleParameterWithNoValueIsAnError() throws {
        var reads = endsWith("reads", "Reader", .pass)
        reads.moduleParameters = ["job"]
        let suite = Suite(name: "undeclared", executes: [Execute(reads)])
        let built = Recorded()
        let results = try runSuite(suite, world: [], adapters: tracing(built))
        #expect(results.count == 1)
        #expect(results[0].result.verdict == .error)
        #expect(results[0].result.log.count == 1)
        #expect(results[0].result.log[0].detail.contains("'job'"))
        #expect(built.names.isEmpty, "no adapter is built for a case that cannot start")
    }

    /// P13: a test case the caller gives no system adapter ends with error.
    @Test func aTestCaseWithNoSystemAdapterIsAnError() throws {
        let suite = Suite(name: "unadapted", executes: [Execute(endsWith("only", "Only", .pass))])
        let results = try runSuite(suite, world: [], adapters: { _ in nil })
        #expect(results.map(\.result.verdict) == [.error])
    }
}

/// What a process an exit test ran wrote to its standard error, as text.
private func standardError(_ result: ExitTest.Result?) -> String {
    result.map { String(decoding: $0.standardErrorContent, as: UTF8.self) } ?? ""
}

struct TheSharedHandleTests {
    /// A snapshot reader runs while the run's state is held, so a call it makes into its context is
    /// re-entrant: it stops the program, naming the component, and does not go on inside the state.
    @Test func aCallIntoTheContextFromASnapshotReaderStopsTheProgram() async {
        let result = await #expect(processExitsWith: .failure, observing: [\.standardErrorContent]) {
            let machines = Machines().with(
                "Reader",
                ({ context, _ in context.withSnapshot("p") { _ in context.log("inside the reader") } }, idle()))
            _ = runCase(mapped("Reader"), machines, TraceAdapter())
        }
        #expect(standardError(result).contains("re-entrant access to the test run from mtc"))
    }

    /// P9, P13: a component whose machine is running is not removed under it. The PTC's start kills the
    /// MTC, whose own start is still running; that is a dynamic error, and the MTC stays.
    @Test func aKillOfAComponentWhoseMachineRunsIsAnError() {
        let machines = Machines()
            .with(
                "Tester",
                (
                    { context, _ in
                        context.create("echo")
                        context.setverdict(.pass)
                    },
                    idle()
                ))
            .with("Echo", ({ context, _ in context.kill("mtc") }, idle()))
        let result = runCase(mtcAndEcho(), machines, TraceAdapter())
        #expect(result.verdict == .error)
        #expect(logged(result, "echo", .error))
        #expect(!logged(result, "mtc", .kill), "the MTC was not removed")
        #expect(logged(result, "mtc", .verdict), "the MTC's start ran on after the refused kill")
    }

    /// A context names one creation of its component: once the PTC is removed, a call through a context
    /// it handed out stops the program instead of reaching another component.
    @Test func aContextDoesNotOutliveItsComponent() async {
        let result = await #expect(processExitsWith: .failure, observing: [\.standardErrorContent]) {
            final class Kept {
                var context: ComponentContext?
            }
            let kept = Kept()
            let machines = Machines()
                .with(
                    "Tester",
                    (
                        { context, _ in
                            context.create("echo")
                            context.kill("echo")
                            guard let echo = kept.context else {
                                fatalError("the echo kept its context")
                            }
                            echo.setverdict(.fail, "after its removal")
                        },
                        idle()
                    ))
                .with("Echo", ({ context, _ in kept.context = context }, idle()))
            _ = runCase(mtcAndEcho(), machines, TraceAdapter())
        }
        #expect(standardError(result).contains("the component echo was removed, and its context outlived it"))
    }
}

/// The test-port processor, of the type `Engine.registerEventProcessor` takes a handler of: a send (P6)
/// carries the template's value, which the run appends to the connected port, or hands the system
/// adapter.
private func testPortProcessor(_ context: ComponentContext) -> HostSendHandler {
    { request in
        guard let value = request.value else {
            fatalError("a test-port send carries its template's value")
        }
        context.send(request.target, Message(event: request.eventName, value: value.value))
        return []
    }
}

/// The test-component invoker, of the type `Engine.registerInvoker` takes a handler of: a PTC (P9), which
/// the start creates and the cancel removes.
private func testComponentInvoker(_ context: ComponentContext) -> HostInvokeHandler {
    { event in
        switch event {
        case .start(let request):
            context.create(request.invokeID)
        case .cancel(let cancel):
            context.kill(cancel.invokeID)
        }
        return nil
    }
}

/// The Event I/O Processor whose addresses are ports (docs/SCE_TEST_PROFILE.md §11.1).
private let testPort = "http://sce.dev/ext/test-port"

/// The invocation of a PTC (docs/SCE_TEST_PROFILE.md §11.1).
private let testComponent = "http://sce.dev/ext/test-component"

/// The record a template of the port type reads.
private struct Ping: Equatable, Sendable {
    let n: Int
}

/// An MTC wired as a generated one is: its start makes the two processors, each holding its context,
/// and it reaches the doors of the run only through them. Entering its state invokes `echo` and sends
/// it a `Ping`; a timeout leaves the state, which cancels the invoke.
private final class DoorsTester: ComponentMachine {
    private var invoker: HostInvokeHandler?
    private var context: ComponentContext?
    private var ended = false

    func start(_ context: ComponentContext) {
        let sends = testPortProcessor(context)
        let invokes = testComponentInvoker(context)
        _ = invokes(.start(HostInvokeRequest(processorType: testComponent, invokeID: "echo")))
        _ = sends(
            HostSendRequest(
                processorType: testPort, eventName: "ping", target: "p", value: HostSendValue(Ping(n: 3))))
        context.startTimer("reply", 10)
        invoker = invokes
        self.context = context
    }

    func deliverSnapshot() {
        guard let context else {
            fatalError("a snapshot is delivered only to a started machine")
        }
        guard context.timedOut("reply") else {
            return
        }
        context.timeout("reply")
        guard let invoker else {
            fatalError("the start made the invoker")
        }
        _ = invoker(.cancel(HostInvokeCancel(processorType: testComponent, invokeID: "echo", token: 0)))
        context.setverdict(.pass)
        ended = true
    }

    var terminated: Bool {
        ended
    }
}

private func position(_ result: TestCaseResult, _ component: String, _ kind: LogKind) throws -> Int {
    let found = result.log.firstIndex(where: { $0.component == component && $0.kind == kind })
    return try #require(found, "no \(kind.rawValue) of \(component) in \(result.log)")
}

struct TheDoorsTests {
    /// §12.3: the PTC the invoker's start creates receives the value the processor's send carried, by its
    /// type, and the invoker's cancel removes it at the timeout, before the MTC's verdict.
    @Test func aSendAndAPtcGoThroughTheHostProcessors() throws {
        let echo: Step = { context, _ in
            if receives(context, "q", "ping", as: Ping.self, { $0.n == 3 }) {
                _ = context.receive("q")
                context.setverdict(.pass, "the value arrived")
            } else if context.snapshotPresent("q") {
                _ = context.receive("q")
                context.setverdict(.fail, "another value arrived")
            }
        }
        let factory: MachineFactory = { componentType in
            switch componentType {
            case "Tester": return DoorsTester()
            case "Echo": return Scripted(idle(), echo)
            default: return nil
            }
        }
        let result = TestCaseRun(mtcAndEcho(), factory, TraceAdapter()).run()
        #expect(result.verdict == .pass, "\(result.log)")
        let created = try position(result, "echo", .create)
        let sent = try position(result, "mtc", .send)
        let received = try position(result, "echo", .receive)
        let killed = try position(result, "echo", .kill)
        let verdict = try position(result, "mtc", .verdict)
        #expect(created < sent && sent < received, "\(result.log)")
        #expect(result.log[sent].detail == "p ping")
        #expect(result.log[received].detail == "q ping")
        #expect(result.log[killed].at == 10, "the cancel came at the timeout")
        #expect(killed < verdict, "the cancel removed the PTC, not the end")
        #expect(logged(result, "echo", .verdict))
    }
}
