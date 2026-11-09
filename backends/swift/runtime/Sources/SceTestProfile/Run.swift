// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md §2 to §5, §12): its entities,
// its ports and timers, the round scheduler on the logical clock, and the test case verdict.
//
// The module `SceTestProfile` is the runtime of the test profile (§12.2): the round scheduler, `NOW`,
// port queues and snapshots, timer bindings, verdicts, the entity table and the guard, written once
// for every suite. A suite's generated units reach it from here: a test component's machine through
// its `ComponentContext`, which it keeps from `ComponentMachine.start` on; a configuration as a
// `Configuration` and a `MachineFactory`; a test case and a suite as a `TestCase` and a `Suite`, which
// `runSuite` runs. A template's length restriction on a text counts it with `textLength`. The
// algorithm is the one the document specifies and the C++ runtime (`sce/include/test/`), the Rust one
// (`sce_rust_runtime::test`), the Kotlin one (`com.sce.runtime.test`), the Go one (`testprofile`), the
// Python one (`sce_runtime.testprofile`) and the C11 one (`sce/test/`) implement, type for type. When
// this module and the Rust one diverge, Rust wins.
//
// A component's machine is generated from its `test-component` and reaches this run only through its
// context (§12.3): the snapshot its branches' guards read, the statement each taken branch runs first,
// the statements of the profile, and the doors of a send and of a PTC. The run knows nothing of the
// engine or of the record types, so it is tested on its own.
//
// Ownership. The state of a run is shared between the run and a context per component, as the C++
// runtime's context holds a reference to its run: the machine keeps its context from `start` on, and
// so may the host processors it registers on its engine for the doors of a send and of a PTC. A run is
// driven on one thread (§12.4), so the state takes no lock, where the Rust runtime's takes a mutex
// because its engine holds those processors as `Send` closures. The state holds each machine and each
// machine its context, which holds the state: the run lets every machine go when it ends, so a run
// that has ended holds nothing that holds it.
//
// The run never holds its state across a call into a machine: every step of it marks the state held
// for its own length (`RunState.access`), and a machine is called between steps. So every call a
// machine makes into its context finds the state free. A call that finds it held is a call made while
// the run, or the same component, is already inside the state, which on one thread can only be a
// re-entrant call (a snapshot reader that calls its context, say), and it stops the program naming the
// component, as the Rust runtime panics. So does a call through the context of a component that was
// removed, and a receive or a timeout the component type does not declare: each is a machine that does
// not keep the profile's rules, not a test case that fails.

import SceRuntime

/// The name of the MTC in a configuration; no PTC takes it (§11.6).
public let mtcName = "mtc"

/// A component port named in a configuration: the component's name (`mtc` or a PTC's id) and the
/// port's id.
public struct Endpoint: Hashable, Sendable {
    /// The component's name.
    public var component: String
    /// The port's id.
    public var port: String

    public init(component: String, port: String) {
        self.component = component
        self.port = port
    }

    /// Whether this is the port `port` of the component `component`.
    func names(_ component: String, _ port: String) -> Bool {
        self.component == component && self.port == port
    }
}

/// One `<sce:connect>` pair (-4 §9.10).
public struct Connection: Hashable, Sendable {
    public var a: Endpoint
    public var b: Endpoint

    public init(_ a: Endpoint, _ b: Endpoint) {
        self.a = a
        self.b = b
    }
}

/// One `<sce:map>` entry: a component port and its system port (-4 §9.32).
public struct Mapping: Hashable, Sendable {
    public var port: Endpoint
    public var systemPort: String

    public init(_ port: Endpoint, systemPort: String) {
        self.port = port
        self.systemPort = systemPort
    }
}

/// A `test-configuration` as the run reads it (§11.6).
///
/// Generated from the configuration document. Each port takes part in one connection or one mapping
/// (§2.2), which the build has checked.
public struct Configuration: Equatable, Sendable {
    /// The MTC's component type.
    public var mtcType: String
    /// Each PTC's name and its component type.
    public var ptcTypes: [String: String]
    /// The port ids each component type declares.
    public var portsOfType: [String: [String]]
    /// The timer ids each component type declares.
    public var timersOfType: [String: [String]]
    /// The `<sce:connect>` pairs.
    public var connections: [Connection]
    /// The `<sce:map>` entries.
    public var mappings: [Mapping]

    public init(
        mtcType: String,
        ptcTypes: [String: String] = [:],
        portsOfType: [String: [String]] = [:],
        timersOfType: [String: [String]] = [:],
        connections: [Connection] = [],
        mappings: [Mapping] = []
    ) {
        self.mtcType = mtcType
        self.ptcTypes = ptcTypes
        self.portsOfType = portsOfType
        self.timersOfType = timersOfType
        self.connections = connections
        self.mappings = mappings
    }
}

/// What a line of the run's log records (§12.5): the run as a reader follows it, with the removals of
/// PTCs and the cause of an `error` in the runtime's words. The test log, which two runtimes compare, is
/// `TestCaseResult.records`.
public enum LogKind: String, Sendable {
    /// The MTC was created and started.
    case start = "Start"
    /// A PTC was created and started.
    case create = "Create"
    /// A component sent on a port.
    case send = "Send"
    /// A receive branch removed the head of a port.
    case receive = "Receive"
    /// A timeout branch took a timer.
    case timeout = "Timeout"
    /// A PTC terminated (P10).
    case done = "Done"
    /// A PTC was removed.
    case kill = "Kill"
    /// A component set its local verdict.
    case verdict = "Verdict"
    /// A `<log>`.
    case log = "Log"
    /// A dynamic error ended the run (P13).
    case error = "Error"
}

/// One line of the run's log.
public struct LogRecord: Equatable, Sendable {
    /// `NOW` when it was recorded.
    public var at: TestInstant
    /// The component it is of.
    public var component: String
    /// What it records.
    public var kind: LogKind
    /// The port and event, the timer, the verdict and reason, the text or the error, by kind; empty for
    /// a kind that has none.
    public var detail: String

    public init(at: TestInstant, component: String, kind: LogKind, detail: String) {
        self.at = at
        self.component = component
        self.kind = kind
        self.detail = detail
    }
}

/// The outcome of one test case: its verdict, the instant it ended at, the lines of its log, and its
/// test log.
public struct TestCaseResult: Equatable, Sendable {
    /// The test case verdict (P14).
    public var verdict: Verdict
    /// `NOW` when the run ended.
    public var endedAt: TestInstant
    /// The lines of the run's log, for a reader.
    public var log: [LogRecord]
    /// The test log (docs/adr/0091, C3): the records of TCI's test logging interface, in the order the
    /// run produced them, which `canonicalLog` writes.
    public var records: [TestLogRecord]

    public init(verdict: Verdict, endedAt: TestInstant, log: [LogRecord], records: [TestLogRecord] = []) {
        self.verdict = verdict
        self.endedAt = endedAt
        self.log = log
        self.records = records
    }

    /// The test log of this run of the test case `testCase` of the suite `suite`, in its one canonical
    /// text: the JSON Canonicalization Scheme of `schema`, `suite`, `test_case`, `verdict` and `records`,
    /// each record in the order the run produced it.
    ///
    /// A message's value that is not in the canonical form (`TestValue.canonical`), which no writer of
    /// `Canonical` builds, stops the program.
    public func canonicalLog(suite: String, testCase: String) -> String {
        writeTestLog(suite, testCase, verdict, records)
    }
}

/// One input of the SUT: the system port it arrives on and the message.
public struct SystemInput: Sendable {
    public var systemPort: String
    public var message: Message

    public init(systemPort: String, message: Message) {
        self.systemPort = systemPort
        self.message = message
    }
}

/// The SUT's side of the mapped ports (§12.2).
///
/// Shaped after TRI (ES 201 873-5): `send` is `triSend`, and the inputs `takeInputsDue` returns are what
/// `triEnqueueMsg` would enqueue. The adapter owns the codec and the transport; the run sees values only
/// (ADR 0091 C2). The instants are the logical clock's, so a recorded trace replays to the same run
/// (§3.1).
public protocol SystemAdapter: AnyObject {
    /// A component sent `message` on a port mapped to `systemPort`.
    func send(systemPort: String, message: Message, now: TestInstant)

    /// The instant of the earliest input not yet taken, if any.
    func nextInput() -> TestInstant?

    /// Every input due at or before `now`, in arrival order.
    func takeInputsDue(now: TestInstant) -> [SystemInput]
}

/// A component's machine, as the run drives it (§12.3).
///
/// Generated from a `test-component`: the backend's statechart machine, with each branch a transition on
/// the round's snapshot event.
public protocol ComponentMachine: AnyObject {
    /// Enter the initial configuration and complete the first macrostep. The machine keeps `context`,
    /// and every call it makes, now and on each later delivery, goes to it.
    func start(_ context: ComponentContext)

    /// Deliver the round's snapshot event: the machine takes the first branch whose guard holds on the
    /// snapshot (P3) and completes its macrostep (P4), or takes none.
    func deliverSnapshot()

    /// Whether the machine has entered a top-level `<final>` (P10).
    var terminated: Bool { get }
}

/// Builds the machine of a component type, or answers nil for a type it does not make, which ends the
/// run with `error` where a component of that type is created.
public typealias MachineFactory = (_ componentType: String) -> (any ComponentMachine)?

/// What one component holds in the run.
final class ComponentState {
    var ports: [String: TestPort] = [:]
    var timers: [String: TestTimer] = [:]
    var doneSnapshot: Set<String> = []
    var verdict = Verdict.none
    var branchTaken = false

    /// The port `id`, made empty where the component has none of that id yet.
    func port(_ id: String) -> TestPort {
        if let port = ports[id] {
            return port
        }
        let port = TestPort()
        ports[id] = port
        return port
    }

    /// The timer `id`, made idle where the component has none of that id yet.
    func timer(_ id: String) -> TestTimer {
        if let timer = timers[id] {
            return timer
        }
        let timer = TestTimer()
        timers[id] = timer
        return timer
    }
}

/// A component of the run's entity table.
final class Entity {
    /// Which creation this is: a PTC removed and created again in one round is a new component, which
    /// took no snapshot in it.
    let serial: Int64
    let state: ComponentState
    /// The machine, or nil while the run has it out to call it.
    var machine: (any ComponentMachine)?
    var terminated = false

    init(serial: Int64, state: ComponentState) {
        self.serial = serial
        self.state = state
    }
}

/// The state of one run, shared by the run and every component's context.
final class RunState {
    let configuration: Configuration
    let factory: MachineFactory
    let adapter: any SystemAdapter
    let guardAt: TestDuration?
    let moduleParameters: ModuleParameters

    var now: TestInstant = 0
    private var created: Int64 = 0

    /// The entities, by name; a name's place here is not the creation order, `order` is.
    private var entities: [String: Entity] = [:]
    private var order: [String] = []
    private var done: Set<String> = []
    private var removedVerdict = Verdict.none
    private var mtcTerminated = false
    /// A dynamic error ended the run (P13).
    private var failed = false
    private var log: [LogRecord] = []
    private var records: [TestLogRecord] = []

    /// A step of the run is inside the state (`access`).
    private var held = false

    init(
        configuration: Configuration, factory: @escaping MachineFactory, adapter: any SystemAdapter,
        guardAt: TestDuration?, moduleParameters: ModuleParameters
    ) {
        self.configuration = configuration
        self.factory = factory
        self.adapter = adapter
        self.guardAt = guardAt
        self.moduleParameters = moduleParameters
    }

    /// `step` on the state, for `component`, with the state held for its length; a call that finds it
    /// held already is re-entrant, and stops the program naming the component.
    func access<R>(_ component: String, _ step: (RunState) -> R) -> R {
        if held {
            fatalError("re-entrant access to the test run from \(component)")
        }
        held = true
        defer { held = false }
        return step(self)
    }

    // MARK: The round (§3.2, §12.4)

    /// Step 2, the update: inputs and timer expiries due at `NOW`. Then step 3, the snapshot: every
    /// waiting component at once. Answers the components that took it, in creation order, each with
    /// its serial.
    func updateAndSnapshot() -> [(name: String, serial: Int64)] {
        for input in adapter.takeInputsDue(now: now) {
            deliverInput(input.systemPort, input.message)
        }
        for entity in entities.values {
            for timer in entity.state.timers.values {
                timer.update(now: now)
            }
        }
        var waiting: [(name: String, serial: Int64)] = []
        for name in order {
            guard let entity = entities[name], !entity.terminated else {
                continue
            }
            for port in entity.state.ports.values {
                port.takeSnapshot()
            }
            for timer in entity.state.timers.values {
                timer.takeSnapshot()
            }
            entity.state.doneSnapshot = done
            waiting.append((name: name, serial: entity.serial))
        }
        return waiting
    }

    /// The machine of a component that took this round's snapshot, out of its entity to be delivered;
    /// nil for one removed or terminated earlier in the round, which does not select (§3.2 step 5).
    func takeForDelivery(_ name: String, _ serial: Int64) -> (any ComponentMachine)? {
        guard let entity = entities[name], entity.serial == serial, !entity.terminated else {
            return nil
        }
        entity.state.branchTaken = false
        let machine = entity.machine
        entity.machine = nil
        return machine
    }

    func branchTaken(_ name: String, _ serial: Int64) -> Bool {
        guard let entity = entities[name], entity.serial == serial else {
            return false
        }
        return entity.state.branchTaken
    }

    /// No branch fired: time advances to the next instant something can happen at (§3.3), or the test
    /// case can never end (P13).
    func advance() {
        var next = adapter.nextInput()
        for entity in entities.values where !entity.terminated {
            for timer in entity.state.timers.values {
                if let expiry = timer.expiry {
                    next = next.map { min($0, expiry) } ?? expiry
                }
            }
        }
        if let guardAt {
            next = next.map { min($0, guardAt) } ?? guardAt
        }
        guard let at = next else {
            fail(mtcName, "no branch can fire and no instant is pending: the test case can never end")
            return
        }
        now = max(now, at)
        if let guardAt, now >= guardAt {
            fail(mtcName, "the guard of the test case was reached")
        }
    }

    /// A component's machine has just run and is put back. If it terminated, P10 applies.
    ///
    /// The entity is still there: a component whose machine is running is not removed (`kill`), and
    /// nothing else removes one mid-round.
    func settle(_ name: String, _ serial: Int64, _ machine: any ComponentMachine, terminated: Bool) {
        guard let entity = entities[name], entity.serial == serial else {
            fatalError("the component \(name) was removed while its machine ran")
        }
        entity.machine = machine
        if entity.terminated || !terminated {
            return
        }
        entity.terminated = true
        trace(.componentDone(component: name, verdict: entity.state.verdict))
        if name == mtcName {
            mtcTerminated = true
            return
        }
        done.insert(name)
        record(name, .done, "")
    }

    func ended() -> Bool {
        mtcTerminated || failed
    }

    /// The MTC terminated or the run ended in error: every PTC still present is removed, in ascending
    /// order of name as the other runtimes remove them, and the test case verdict is the worst local
    /// verdict (P10, P14). Every machine is let go, so the run holds none that holds a context of it.
    func finish() -> TestCaseResult {
        for ptc in entities.keys.filter({ $0 != mtcName }).sorted() {
            removeEntity(ptc)
        }
        var verdict = removedVerdict
        if let mtc = entities[mtcName] {
            verdict = worse(verdict, mtc.state.verdict)
        }
        if failed {
            verdict = .error
        }
        entities.removeAll()
        order.removeAll()
        trace(.testCaseEnded(verdict: verdict))
        return TestCaseResult(verdict: verdict, endedAt: now, log: log, records: records)
    }

    // MARK: Entities (§2.1, P9)

    /// Enters a new component of the type `componentType` into the table, with its ports and timers,
    /// and logs its creation. Answers its serial.
    func enter(_ name: String, _ componentType: String) -> Int64 {
        created += 1
        let state = ComponentState()
        for port in configuration.portsOfType[componentType] ?? [] {
            _ = state.port(port)
        }
        for timer in configuration.timersOfType[componentType] ?? [] {
            _ = state.timer(timer)
        }
        entities[name] = Entity(serial: created, state: state)
        order.append(name)
        record(name, name == mtcName ? .start : .create, componentType)
        trace(.componentCreated(component: name, componentType: componentType))
        return created
    }

    /// The type of the PTC `ptc` that `creator` creates, or nil, with the dynamic error, when it cannot
    /// be created.
    func ptcToCreate(_ creator: String, _ ptc: String) -> String? {
        guard let componentType = configuration.ptcTypes[ptc] else {
            fail(creator, "creates '\(ptc)', which the configuration does not declare")
            return nil
        }
        if entities[ptc] != nil {
            fail(creator, "creates '\(ptc)', which is already present")
            return nil
        }
        done.remove(ptc)
        return componentType
    }

    /// The kill of the PTC `ptc` by `killer` (P9). A component whose machine is running, up the call
    /// stack of this one, is not removed under it: that is a dynamic error.
    func kill(_ killer: String, _ ptc: String) {
        if let entity = entities[ptc], entity.machine == nil {
            fail(killer, "kills '\(ptc)', whose machine is running")
            return
        }
        removeEntity(ptc)
    }

    /// Removes a PTC: its local verdict still counts (-4 §9.29a.2), and its ports are gone, so a send to
    /// one of them is an error.
    private func removeEntity(_ name: String) {
        guard let entity = entities.removeValue(forKey: name) else {
            return
        }
        removedVerdict = worse(removedVerdict, entity.state.verdict)
        record(name, .kill, "")
        order.removeAll { $0 == name }
        done.remove(name)
    }

    // MARK: Ports (§2.2, P6)

    func send(_ sender: String, _ port: String, _ message: Message) {
        record(sender, .send, "\(port) \(message.event)")
        trace(
            .messageSent(
                component: sender, port: port, messageType: message.typeName, value: message.canonicalValue()))
        let peer = configuration.connections.lazy.compactMap { connection -> Endpoint? in
            if connection.a.names(sender, port) {
                return connection.b
            }
            if connection.b.names(sender, port) {
                return connection.a
            }
            return nil
        }.first
        if let peer {
            if let entity = entities[peer.component] {
                entity.state.port(peer.port).enqueue(message)
                return
            }
            fail(sender, "sends on '\(port)', whose peer '\(peer.component)' is not present")
            return
        }
        if let mapped = configuration.mappings.first(where: { $0.port.names(sender, port) }) {
            adapter.send(systemPort: mapped.systemPort, message: message, now: now)
            return
        }
        fail(sender, "sends on '\(port)', which is neither connected nor mapped")
    }

    private func deliverInput(_ systemPort: String, _ message: Message) {
        guard let endpoint = configuration.mappings.first(where: { $0.systemPort == systemPort })?.port else {
            fail(mtcName, "an input on system port '\(systemPort)', which no port maps")
            return
        }
        if let entity = entities[endpoint.component] {
            entity.state.port(endpoint.port).enqueue(message)
            return
        }
        fail(endpoint.component, "an input on system port '\(systemPort)' found no component")
    }

    // MARK: A component's own state, through its context

    func component(_ name: String, _ serial: Int64) -> ComponentState {
        guard let entity = entities[name], entity.serial == serial else {
            fatalError("the component \(name) was removed, and its context outlived it")
        }
        return entity.state
    }

    func receive(_ name: String, _ serial: Int64, _ port: String) -> Message {
        let owner = component(name, serial)
        owner.branchTaken = true
        guard let queue = owner.ports[port] else {
            fatalError("\(name) receives on '\(port)', which its component type does not declare")
        }
        let message = queue.removeHead()
        record(name, .receive, "\(port) \(message.event)")
        trace(
            .messageReceived(
                component: name, port: port, messageType: message.typeName, value: message.canonicalValue()))
        return message
    }

    func timeout(_ name: String, _ serial: Int64, _ timer: String) {
        let owner = component(name, serial)
        owner.branchTaken = true
        guard let binding = owner.timers[timer] else {
            fatalError("\(name) times out on '\(timer)', which its component type does not declare")
        }
        binding.timeoutTaken()
        record(name, .timeout, timer)
        trace(.timerTimedOut(component: name, timer: timer))
    }

    func setverdict(_ name: String, _ serial: Int64, _ verdict: Verdict, _ reason: String) {
        let owner = component(name, serial)
        owner.verdict = worse(owner.verdict, verdict)
        let detail = reason.isEmpty ? verdict.spelling : "\(verdict.spelling) \(reason)"
        record(name, .verdict, detail)
        trace(.verdictSet(component: name, verdict: verdict, reason: reason.isEmpty ? nil : reason))
    }

    // MARK: The verdict and the log (§5, §12.5)

    /// A dynamic error (P13): the run ends with `error`. The test log records the verdict without `why`,
    /// which is this runtime's own prose.
    func fail(_ component: String, _ why: String) {
        failed = true
        record(component, .error, why)
        trace(.verdictSet(component: component, verdict: .error, reason: nil))
    }

    func record(_ component: String, _ kind: LogKind, _ detail: String) {
        log.append(LogRecord(at: now, component: component, kind: kind, detail: detail))
    }

    /// A record of the test log, at `NOW`.
    func trace(_ event: TestLogEvent) {
        records.append(TestLogRecord(at: now, event: event))
    }
}

/// Creates the component `name` of the type `componentType` and starts its machine, with the state let
/// go while the factory and the machine run.
private func createEntity(_ run: RunState, _ name: String, _ componentType: String) {
    guard let machine = run.factory(componentType) else {
        run.access(name) {
            $0.fail(
                name,
                "is of the component type '\(componentType)', which the factory of machines does not make")
        }
        return
    }
    let serial = run.access(name) { $0.enter(name, componentType) }
    machine.start(ComponentContext(run: run, name: name, serial: serial))
    let terminated = machine.terminated
    run.access(name) { $0.settle(name, serial, machine, terminated: terminated) }
}

/// One round (§3.2, §12.4).
private func runRound(_ run: RunState) {
    let waiting = run.access(mtcName) { $0.updateAndSnapshot() }
    // Steps 4 and 5: one component at a time, in creation order, each selects on its own snapshot and
    // runs the macrostep of the branch it took. A component removed or terminated earlier in the round
    // does not select, and nothing does once the MTC has terminated (§3.2).
    var anyTaken = false
    for (name, serial) in waiting {
        if run.access(name, { $0.ended() }) {
            break
        }
        guard let machine = run.access(name, { $0.takeForDelivery(name, serial) }) else {
            continue
        }
        machine.deliverSnapshot()
        let terminated = machine.terminated
        let taken = run.access(name) { state -> Bool in
            let branch = state.branchTaken(name, serial)
            state.settle(name, serial, machine, terminated: terminated)
            return branch
        }
        anyTaken = anyTaken || taken
    }
    run.access(mtcName) { state in
        if !state.ended() && !anyTaken {
            state.advance()
        }
    }
}

/// What a component's machine reads and does through the run (§12.3).
///
/// The machine keeps it from `start` on, and so may the host processors the machine registers for the
/// doors of a send and of a PTC. Each call takes the run's state for its own length and lets it go
/// before it returns, so no call holds it across a call into a machine. A call made while the state is
/// held, from inside a snapshot reader say, is re-entrant and stops the program naming the component.
public final class ComponentContext: CustomStringConvertible {
    private let run: RunState
    /// The component's name: `mtc`, or the PTC's id.
    public let name: String
    private let serial: Int64

    init(run: RunState, name: String, serial: Int64) {
        self.run = run
        self.name = name
        self.serial = serial
    }

    private func access<R>(_ step: (RunState) -> R) -> R {
        run.access(name, step)
    }

    public var description: String {
        "ComponentContext(name=\(name), serial=\(serial), ..)"
    }

    // MARK: The snapshot, read by the branches' guards (P2)

    /// Hands `read` the message the snapshot saw at the head of `port`, or nil, and answers what it
    /// answers. `read` runs while the run's state is held, so it must not call this context: such a call
    /// stops the program.
    public func withSnapshot<R>(_ port: String, _ read: (Message?) -> R) -> R {
        access { state in
            read(state.component(name, serial).ports[port]?.snapshot())
        }
    }

    /// Whether the snapshot saw a message at the head of `port`: the guard of a receive any branch.
    public func snapshotPresent(_ port: String) -> Bool {
        withSnapshot(port) { head in head != nil }
    }

    /// Whether the snapshot saw `timer` timed out.
    public func timedOut(_ timer: String) -> Bool {
        access { state in
            state.component(name, serial).timers[timer]?.snapshotTimedOut == true
        }
    }

    /// Whether the snapshot saw the PTC `ptc` in `DONE`.
    public func done(_ ptc: String) -> Bool {
        access { state in
            state.component(name, serial).doneSnapshot.contains(ptc)
        }
    }

    // MARK: Taking a branch (P4 step 1), the first statement of each

    /// A receive branch: remove the head of `port`, which its content reads as `_event.data`.
    ///
    /// A component type that declares no port `port`, or a queue that is empty, stops the program: a
    /// receive branch is taken only on a snapshot that saw a message there.
    public func receive(_ port: String) -> Message {
        access { state in state.receive(name, serial, port) }
    }

    /// A timeout branch: `timer` becomes idle. A component type that declares no timer `timer` stops
    /// the program.
    public func timeout(_ timer: String) {
        access { state in state.timeout(name, serial, timer) }
    }

    /// A done branch on the PTC `ptc`: nothing is removed (-4 §9.16); the component took a branch in
    /// this round.
    public func doneTaken(_ ptc: String) {
        access { state in state.component(name, serial).branchTaken = true }
    }

    // MARK: Statements of the profile

    /// `<sce:setverdict>` (P12): the local verdict becomes the worse of the two, and the verdict and its
    /// reason are logged.
    public func setverdict(_ verdict: Verdict, _ reason: String = "") {
        access { state in state.setverdict(name, serial, verdict, reason) }
    }

    /// `<sce:start-timer>` (P7), for `duration` microseconds from `NOW`.
    public func startTimer(_ timer: String, _ duration: TestDuration) {
        access { state in
            state.component(name, serial).timer(timer).start(now: state.now, duration: duration)
            state.trace(.timerStarted(component: name, timer: timer, duration: duration))
        }
    }

    /// `<sce:stop-timer>` (P7). The test log records the stop of a running timer; the stop of one that is
    /// not running changes nothing.
    public func stopTimer(_ timer: String) {
        access { state in
            let binding = state.component(name, serial).timer(timer)
            let running = binding.status == .running
            binding.stop()
            if running {
                state.trace(.timerStopped(component: name, timer: timer))
            }
        }
    }

    /// A `<log>` (P8): recorded in the test log with `NOW` and the component.
    public func log(_ text: String) {
        access { state in
            state.record(name, .log, text)
            state.trace(.log(component: name, text: text))
        }
    }

    // MARK: Doors (§12.3)

    /// A send on `port` (P6): to the queue of the connected port, or to the system adapter when the
    /// port is mapped.
    public func send(_ port: String, _ message: Message) {
        access { state in state.send(name, port, message) }
    }

    /// The start of the invocation that creates the PTC `ptc` (P9): the PTC is created and its machine
    /// started before this returns.
    public func create(_ ptc: String) {
        guard let componentType = access({ state in state.ptcToCreate(name, ptc) }) else {
            return
        }
        createEntity(run, ptc, componentType)
    }

    /// The cancel of that invocation: the PTC is removed (P9).
    public func kill(_ ptc: String) {
        access { state in state.kill(name, ptc) }
    }

    // MARK: Module parameters (§11.5, §11.8)

    /// Hands `read` the module parameters of the run, which it reads into the variables the component
    /// declares as module parameters, before the component's machine starts.
    ///
    /// A value the run was not given (`ModuleParameters.of`), or one `read` refuses as not of the type
    /// the component declares, is thrown, which is a dynamic error (P13): the run ends with `error` and
    /// false is answered, so the machine is not started on a value made up in its place. A reader
    /// throws a `StateRefusal`, whose sentence the log records; any other error is recorded as it
    /// describes itself. `read` runs with the run's state let go.
    public func readModuleParameters(_ read: (ModuleParameters) throws -> Void) -> Bool {
        let given = access { state in state.moduleParameters }
        do {
            try read(given)
            return true
        } catch {
            let why = (error as? StateRefusal)?.message ?? String(describing: error)
            access { state in state.fail(name, why) }
            return false
        }
    }

    /// `NOW`.
    public func now() -> TestInstant {
        access { state in state.now }
    }
}

/// Whether a receive branch with a template matches the snapshot of `context` (P2,
/// docs/SCE_TEST_PROFILE.md §12.3).
///
/// The snapshot saw a message at the head of `port`, under `event`, the event name of the template's
/// record, whose value is a `V`, and `matches`, the template's `matches` bound to its arguments, holds
/// of that value. A message of another record, or one whose value is of another type, does not match.
/// `matches` runs while the run's state is held, as a reader of `ComponentContext.withSnapshot` does.
public func receives<V>(
    _ context: ComponentContext, _ port: String, _ event: String, as type: V.Type,
    _ matches: (V) -> Bool
) -> Bool {
    context.withSnapshot(port) { head in
        guard let head, head.event == event, let value = head.valueAs(type) else {
            return false
        }
        return matches(value)
    }
}

/// One run of one test case (§2.3, §3, §4.9 to §4.11, §5).
///
/// The components run on the calling thread, in the order they were created; §3.2 shows that the order
/// within a round does not change the result.
///
/// `guardAt` is the test case's guard duration (P11), if it has one, and `moduleParameters` the values of
/// the module parameters the components read (§11.8): the suite's, with any the caller overrides.
///
/// Not copyable, and `run` consumes it, as the Rust run is consumed by its `run`: a run has read its
/// adapter and holds its components' end, so the compiler refuses a second run of it, which the Kotlin
/// and Python runs refuse when it is made.
public struct TestCaseRun: ~Copyable {
    private let state: RunState

    public init(
        _ configuration: Configuration, _ factory: @escaping MachineFactory, _ adapter: any SystemAdapter,
        guardAt: TestDuration? = nil, moduleParameters: ModuleParameters = ModuleParameters()
    ) {
        state = RunState(
            configuration: configuration, factory: factory, adapter: adapter, guardAt: guardAt,
            moduleParameters: moduleParameters)
    }

    /// Runs the test case to its end and answers its verdict and logs.
    public consuming func run() -> TestCaseResult {
        state.access(mtcName) { $0.trace(.testCaseStarted) }
        createEntity(state, mtcName, state.configuration.mtcType)
        while !state.access(mtcName, { $0.ended() }) {
            runRound(state)
        }
        return state.access(mtcName) { $0.finish() }
    }
}
