// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation

/// What the engine did with one event it offered to the active configuration.
///
/// Its own type and not a bool, because the obvious bool ("the configuration
/// changed") answers false for two unrelated outcomes: an event no transition matched
/// at all, and a microstep of targetless transitions that ran their content in place.
/// Only the first is the discard §scxml-3.1.2 describes, and a count keyed off that
/// bool would report a handled event as one. Mirrors the Rust runtime's
/// `EventOutcome`.
private enum EventOutcome {
    /// No transition matched the event in any active state.
    case discarded

    /// A transition was selected and its microstep taken.
    case taken
}

/// Why `Engine.initializeAsRoot` refused to start a machine.
public enum RootStartRefusal: Error, Sendable, Equatable, CustomStringConvertible {
    /// §scxml-6.2.4: the document sends to `#_parent`, and a session its host started
    /// has no parent to reach.
    case needsParent

    /// The refusal as a sentence, for a host to report: the same words on every
    /// engine.
    public var description: String {
        switch self {
        case .needsParent:
            return "the machine sends to #_parent and was started with no parent session"
        }
    }
}

/// What became of a `<send>` handed to `Engine.sendToTarget`.
public enum TargetSendOutcome: Sendable, Equatable {
    /// Delivered, or scheduled to be when its delay elapses.
    case sent

    /// The value is not a target: the caller raises `error.execution`
    /// (§scxml-6.2.4).
    case notSupported

    /// Nothing the value names is there: the caller raises `error.communication`
    /// (§scxml-C-1).
    case notReachable
}

/// A refusal of an argument a host gave an engine.
public struct EngineError: Error, Equatable, CustomStringConvertible {
    public var message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String {
        message
    }
}

/// How many links an `error.*` chain may have before the engine stops feeding it; see
/// `Engine.errorCascadeEvents`.
///
/// §scxml-3.12.2 says what to do with an error event nothing matches. It does not say
/// what to do when something does match it and that handler fails too: the failure
/// raises the same error, the same transition answers it, and the machine has no way
/// out. Nothing in the specification bounds that, so the number is this engine's to
/// choose, and it is chosen beside `maxMacrostepMicrosteps`: the sibling case of a
/// document that cannot finish a macrostep, decided the same way for the same reason.
///
/// A hundred links is far past any repair strategy a document plausibly spells (a
/// handler that tries a fallback, then a second one, is three) and far short of a
/// number a host would wait through: measured 2026-08-19, the Python engine ran 37,000
/// links a second on a two-line document, so an unattended supervisor did not hang; it
/// burned a core until it was killed.
private let maxErrorCascadeDepth: UInt32 = 100

/// How many external events one invocation of the main event loop may take off the
/// queue before this engine hands control back; see `Engine.truncatedEventChains` and
/// ARCHITECTURE.md "External-Event Budget", the contract every engine here holds.
///
/// A machine that answers an event by sending itself the next one, with no target,
/// never lets the external queue empty. Every macrostep of it ends, so
/// `maxMacrostepMicrosteps` never applies, and the loop takes the next event whenever
/// the queue is not empty: the host call that drove it did not return. §scxml-3.13
/// lets a macrostep fail to end and says nothing of a chain of macrosteps, so, as with
/// the microstep ceiling, the number is this engine's to choose and the decline has to
/// be visible.
///
/// The default is the one the contract states: three orders of magnitude above the
/// longest invocation measured over the authoring suite (6 external events). It is a
/// margin and not a proof, and a host that hands a machine a backlog it means to work
/// through in one call can choose another (`setMaxExternalEventsPerCall`).
private let defaultMaxExternalEventsPerCall: UInt32 = 10000

/// How many microsteps one macrostep may take before this engine stops taking them;
/// see `Engine.truncatedMacrosteps`.
///
/// The specification defines a macrostep as a chain of microsteps ending in a
/// configuration where nothing is enabled by NULL and the internal queue is empty, and
/// its Principles and Constraints say in as many words that such a chain need not
/// exist: "A microstep always terminates. A macrostep may not. A macrostep that does
/// not terminate may be said to consist of an infinitely long sequence of microsteps.
/// This is currently allowed."
///
/// So the ceiling is not conformance: it is this engine declining a document the
/// specification permits, which is exactly why the decline has to be visible.
///
/// One budget for the whole inner loop, not one per branch. Appendix D's loop takes a
/// microstep on an eventless transition or on an internal event, and a document
/// alternating the two is one chain, not two: budgeting the branches separately leaves
/// that chain unbounded, which is what a per-call counter on the eventless branch alone
/// did until 2026-08-20.
///
/// Ten times `maxErrorCascadeDepth`, and deliberately not equal to it. This is the
/// backstop; the cascade ceiling is a diagnostic that names the error a handler keeps
/// failing on, and a backstop that fires first makes that diagnostic unreachable.
/// Measured 2026-08-20: with both at a hundred, a handler that raises one event of its
/// own per link (two microsteps a link, which is what a document that logs before it
/// fails looks like) was cut at fifty links by this ceiling and `errorCascadeEvents`
/// never moved. The factor of ten is the headroom that keeps the specific report
/// reachable for a handler raising up to eight events a link; a busier one is cut here
/// instead, which is coarser but still reported.
private let maxMacrostepMicrosteps = 1000

/// The SCXML execution engine.
///
/// Generic over the generated policy, which carries the State and Event types and
/// encodes the state machine structure. Matches Rust `Engine<P>` from
/// `backends/rust/runtime/src/engine.rs` and C++ `StaticExecutionEngine<StatePolicy>`.
///
/// # Threading model
///
/// An engine is not safe for concurrent use, and is not `Sendable`: the host runs it
/// on one thread, as the C++ and Rust single-threaded microstep loop does. The two
/// places state is shared with another thread are `ParentEventQueue` and the id
/// counters, which synchronize themselves.
public final class Engine<Policy: StatePolicy> {
    public typealias State = Policy.State
    public typealias Event = Policy.Event

    /// The generated per-machine policy (datamodel, last-transition flags, etc.).
    public let policy: Policy

    /// The currently active state, or the deepest active state for parallel machines.
    private var currentState: State

    /// The §scxml-C-1 internal event queue (high priority).
    private var internalQueue = EventQueueManager<EventWithMetadata<Event>>()

    /// The §scxml-C-1 external event queue (low priority).
    private var externalQueue = EventQueueManager<EventWithMetadata<Event>>()

    /// Whether the engine is currently running.
    public private(set) var isRunning = false

    /// The §scxml-D-enterStates top-level `<final>` the run ended in: set on entering
    /// it, cleared when a run starts. See `terminalState`.
    public private(set) var terminalState: State?

    /// Says §scxml-D-exitInterpreter has run for this run: the configuration is
    /// empty. `currentState` still names the state the run was last in (`State` has
    /// no "none"), so `getActiveStates` answers from this flag and not from it.
    private var interpreterExited = false

    /// The states whose `<data>` this run has already bound under late binding:
    /// `s.isFirstEntry` (§scxml-D-enterStates) is false for exactly these. The rule
    /// lives here, once, and the generated entry code asks it
    /// (`claimLateBindingFirstEntry`). Cleared when a run starts; nil until the first
    /// claim.
    private var lateBoundStates: Set<State>?

    /// The §scxml-6.4 callback invoked when reaching a final state.
    private var completionCallback: (() -> Void)?

    /// The §scxml-C-2 HTTP send dispatch callback. Returns the response when real
    /// HTTP is used; nil for fire-and-forget.
    private var onHTTPSend: ((HttpSendRequest) -> HttpSendResponse?)?

    /// The host-run `<invoke>` bookkeeping (§scxml-6.4.1): the invokers a host
    /// registered, the starts still running and the token counter. A stored property
    /// cannot live in an extension, so the state is held here and the operations on it
    /// are `HostInvoke.swift`'s.
    var hostInvokes = HostInvokeBook()

    /// How many ids this machine has generated for a `<send idlocation>`: the number
    /// the last one carried. Counted per machine and not per process, so a machine's
    /// ids do not depend on what else ran beside it. See `nextAutoSendID`.
    private var autoSendSeq: UInt64 = 0

    /// The §scxml-6.2 delayed event scheduler.
    private var scheduler = PullScheduler<Event>()

    /// Where this engine reads "now" from; see `SceClock`. Nil until first use, so an
    /// engine constructed before a clock is installed still gets the
    /// `MonotonicClock` default.
    private var installedClock: (any SceClock)?

    /// The reading the clock gave when the current turn began, valid only while
    /// `inTurn`; see `beginTurn`.
    private var turnNowMs: Int64 = 0
    private var inTurn = false

    /// Counts events taken off the external queue that no transition matched
    /// (§scxml-3.1.2); see `discardedExternalEvents`.
    public private(set) var discardedExternalEvents: UInt32 = 0

    /// The most recent event counted above; nil while the count is zero, because the
    /// first value of an event enum is a real event.
    public private(set) var lastDiscardedEvent: Event?

    /// Counts external events this machine never dequeued because it had already
    /// stopped; see `unseenExternalEvents`.
    public private(set) var unseenExternalEvents: UInt32 = 0
    public private(set) var lastUnseenEvent: Event?

    /// Counts deliveries whose payload announced structure and that the datamodel
    /// could not read as one (§scxml-B-2-8-1); see `undecodablePayloads`.
    public private(set) var undecodablePayloads: UInt32 = 0
    public private(set) var lastUndecodablePayload: Event?

    /// Counts `error.*` events this engine raised that no transition matched
    /// (§scxml-3.12.2); see `unhandledErrorEvents`.
    public private(set) var unhandledErrorEvents: UInt32 = 0
    public private(set) var lastUnhandledError: Event?

    /// Says the drain is currently executing a transition selected by an `error.*`
    /// event: the state in which a newly raised error is a link in a chain and not a
    /// first failure. It is the whole discriminator behind `errorCascadeEvents`: a
    /// document answering five hundred separate failures cleanly never sets it twice
    /// in a row.
    private var handlingErrorEvent = false

    /// How many links the current chain has, reset the moment the drain does anything
    /// else; see `errorCascadeEvents`.
    private var errorCascadeDepth: UInt32 = 0

    /// Counts `error.*` events refused because the chain that raised them had reached
    /// `maxErrorCascadeDepth`; see `errorCascadeEvents`.
    public private(set) var errorCascadeEvents: UInt32 = 0
    public private(set) var lastErrorCascadeEvent: Event?

    /// Counts macrosteps this engine stopped at `maxMacrostepMicrosteps` with the
    /// chain still going; see `truncatedMacrosteps`.
    public private(set) var truncatedMacrosteps: UInt32 = 0

    /// Counts microsteps taken by the macrostep now in progress, against
    /// `maxMacrostepMicrosteps`. A field and not a local, for the reason Appendix D's
    /// loop is one loop: the eventless branch and the internal-event branch take turns
    /// inside a single macrostep, so a counter that lives in either one alone is reset
    /// by the other and bounds nothing.
    private var macrostepMicrostepsTaken = 0

    /// The state the drain was in when it last stopped that way; nil while
    /// `truncatedMacrosteps` is zero.
    public private(set) var lastTruncatedMacrostepState: State?

    /// The budget of one invocation of the main event loop, set by
    /// `setMaxExternalEventsPerCall`; zero is "not chosen", which is the default: the
    /// setter refuses zero, so the two cannot be confused.
    private var maxExternalEventsPerCallChosen: UInt32 = 0

    /// Counts invocations of the main event loop this engine handed back with an
    /// external event still queued; see `truncatedEventChains`.
    public private(set) var truncatedEventChains: UInt32 = 0

    /// The event at the head of the queue the last time a call was handed back that
    /// way; nil while `truncatedEventChains` is zero.
    public private(set) var lastTruncatedEvent: Event?

    /// Says the macrostep now in progress has already been stopped at the ceiling.
    /// Every host call runs Appendix D's main event loop again, so without this the
    /// ceiling is not a ceiling: each call would re-enter the inner loop with the
    /// chain it just refused, walk it to the budget again, and count each refusal
    /// separately. Cleared where the algorithm starts a macrostep, which is the
    /// external dequeue.
    private var macrostepTruncated = false

    /// The §scxml-5.5 + 6.3.1 stashed donedata payload for a top-level `<final>`.
    /// Entry actions stash it here; an invoking parent reads it back via
    /// `donedataAtFinal` to lift onto `done.invoke.<id>._event.data`. Mirrors the C++
    /// AOT `stashDonedataAtFinal` contract and the Rust `Engine::donedata_at_final`
    /// field.
    public private(set) var donedataAtFinal = ""

    /// Constructs a new engine with the given policy instance.
    ///
    /// The initial state is set to `policy.initialState()`. The engine is not yet
    /// running: call `initialize()` to enter the initial configuration and begin
    /// processing events.
    ///
    /// Matches Rust `Engine::new`.
    public init(policy: Policy) {
        self.policy = policy
        self.currentState = policy.initialState()
    }

    // MARK: - Lifecycle (matches Rust/C++ public API)

    /// Whether a host running this machine as a root (started by the host, not by an
    /// `<invoke>`) should refuse to start it, and why; nil when it need not.
    ///
    /// §scxml-6.2.4: `#_parent` names the session that invoked this one, and a root
    /// has none. `initialize` still runs such a machine (each such send then raises
    /// `error.communication`, which is the default). This answers for a host that would
    /// rather not start it at all.
    public func rootStartRefusal() -> RootStartRefusal? {
        policy.needsParent ? .needsParent : nil
    }

    /// `initialize` for a host that refuses to start a machine needing a parent as a
    /// root (`rootStartRefusal`). A refusal starts nothing: no datamodel is declared
    /// and no state is entered.
    public func initializeAsRoot() throws {
        if let refusal = rootStartRefusal() {
            throw refusal
        }
        initialize()
    }

    /// Enters the initial configuration and runs the macrostep loop until stable.
    ///
    /// Matches Rust `Engine::initialize`. §scxml-5.3 guarantees datamodel
    /// initialization happens before any state entry.
    public func initialize() {
        // §scxml-3.13: entering the initial configuration is one turn, and the
        // `<onentry>` handlers it runs arm their `<send delay>`s against one instant;
        // see `beginTurn` for what reading the clock per `<send>` did to two of them.
        let opened = beginTurn()
        defer { endTurn(opened) }

        isRunning = true
        clearTerminalState()
        interpreterExited = false
        lateBoundStates = nil

        // §scxml-5.3: Initialize datamodel before any state entry
        if policy.needsDataModelInit {
            policy.initializeDataModel(engine: self)
        }

        // §scxml-D-interpret: enterStates([doc.initialTransition]): the document's own
        // initial transition, whose source is the <scxml> element, entered by the same
        // procedure every microstep enters by. It may name several states and a
        // <history>, so it is a target list, not a leaf to walk down to.
        enterStates([EntryTransition(source: nil, targets: policy.getDocumentInitialTargets())])

        // W3C SCXML Appendix D: hand over to the outer loop. The macrostep completes
        // on eventless transitions and internal events, then the invokes for the
        // states just entered run, and only then is anything taken off the external
        // queue, so an autoforward child is live for every event <onentry> queued on
        // the way in.
        runMainEventLoop()

        // §scxml-6.4: Fire completion callback if we reached a final state during
        // init. The main event loop has already run exitInterpreter, so the child's
        // <onexit> sends are queued ahead of the done.invoke this raises.
        if isInFinalState, let completionCallback {
            completionCallback()
        }
    }

    /// Enters a configuration this document already describes, without re-running the
    /// `<onentry>` actions an earlier run already ran (§scxml-3.2).
    ///
    /// # What it is for
    ///
    /// A host that persisted where a machine was and is bringing it back in a new
    /// process. `initialize` replays the document from its initial state, which runs
    /// every `<onentry>` on the way in: a resumed session would send its greeting
    /// twice, re-arm timers, and re-issue acts whose recipients already received them.
    ///
    /// # What it takes, and why two arguments
    ///
    /// Exactly what the two readers on this engine publish: `getActiveStates` and
    /// `getCurrentState`. Handing back both is not redundancy: for a machine with
    /// `<parallel>` states the configuration does not determine the current state.
    /// `currentState` is the leaf the engine descended to, so which region it sits in
    /// is a fact about the transition history and not about the configuration, and a
    /// set alone cannot recover it.
    ///
    /// # What it refuses
    ///
    /// Every set that is not a configuration of this document; see
    /// `validateConfiguration` for the rules and `ConfigurationRejection` for what each
    /// refusal names. Validation runs before any mutation, so a refused call leaves the
    /// engine exactly as it was; entering "near" the requested configuration is the one
    /// outcome this door must never produce, because a host has no way to detect it
    /// afterwards.
    ///
    /// # What it does not do
    ///
    ///   - No `<onentry>`, and no `<onexit>`: no state is entered or left.
    ///   - No macrostep. `initialize` settles the machine before returning; this does
    ///     not, because the configuration handed in was already a settled one: running
    ///     the loop here could take an eventless transition the earlier run had no
    ///     reason to take, and fire the `<send>`s on the way. The host drives the
    ///     machine on with `step` or `tick` as it otherwise would.
    ///   - No datamodel restore. §scxml-5.3 declaration still runs, so the variables
    ///     exist with their document defaults and a host can then put its saved values
    ///     back through the script engine: the engine does not persist datamodel state
    ///     and does not pretend to.
    ///
    /// The Swift twin of the Rust runtime's `Engine::enter_at` and of the C++
    /// `StaticExecutionEngine::enterAt`.
    ///
    /// Returns `.accepted` on success; the rule that was broken otherwise, with the
    /// engine untouched.
    public func enterAt(configuration: [State], current: State) -> ConfigurationRejection {
        // Before anything is touched: a rejection must not half-enter.
        let verdict = validateConfiguration(policy: policy, configuration: configuration, current: current)
        if verdict != .accepted {
            sceLog("Engine.enterAt: refused -- \(verdict)")
            return verdict
        }

        // §scxml-5.3: the datamodel is declared before anything can read it. This is
        // not a state entry action (<datamodel> holds <data>, not executable content)
        // so it runs here for the same reason it runs in `initialize`: a cond or an
        // assign evaluated after this call would otherwise reference variables that
        // were never declared.
        if policy.needsDataModelInit {
            policy.initializeDataModel(engine: self)
        }

        currentState = current
        interpreterExited = false
        // The restored states were entered by the run being resumed, so their
        // late-bound <data> is that run's: the host restores it, and a later re-entry
        // must not bind it again (§scxml-D-enterStates isFirstEntry).
        lateBoundStates = Set(configuration)
        // A configuration restored at a top-level <final> is a run that has already
        // ended there; any other is still running.
        clearTerminalState()
        recordTerminalState(current)

        // §scxml-3.4: a machine that keeps its own active set is handed it back. The
        // condition is the one the generator emits `setActiveStates` under, so a
        // policy reached here has the override.
        if policy.hasActiveStates {
            policy.setActiveStates(configuration)
        }

        isRunning = true
        return .accepted
    }

    /// Processes one macrostep: drains queues and runs eventless transitions.
    ///
    /// Matches Rust `Engine::step`. Used by parent SMs to explicitly drive children
    /// after sending them events (§scxml-6.4).
    public func step() {
        // §scxml-3.13: one host call, one reading. The macrostep below can enter a
        // state whose <onentry> arms several <send delay>s, and they are one instant's
        // worth of executable content however long the host takes to run them; see
        // `beginTurn`.
        let opened = beginTurn()
        defer { endTurn(opened) }

        runMainEventLoop()

        if isInFinalState, let completionCallback {
            completionCallback()
        }
    }

    /// Polls the scheduler for ready delayed events, then runs a macrostep.
    ///
    /// Matches Rust `Engine::tick`. Called periodically by callers that have delayed
    /// `<send>` operations.
    public func tick() {
        // §scxml-3.13: one turn, one reading. Everything below judges due against the
        // instant this tick began, and everything the macrosteps below arm is measured
        // from it, so a tick dispatches what was due when the host called it, and
        // cannot be extended by how long it takes to run (see `beginTurn`).
        let opened = beginTurn()
        defer { endTurn(opened) }

        if !isRunning {
            return
        }
        if isInFinalState {
            if let completionCallback {
                completionCallback()
            }
            return
        }

        // The same-instant bound on the scheduler pops below. A scheduler that can
        // deliver an entry due at the instant it is popping is a drain that refills
        // itself: an entry a handler re-arms at that same reading (a `delay` of zero,
        // which reaches this scheduler here, or a `delayexpr` that evaluates to zero)
        // is due now, so this loop pops it in the same call, and each pass takes one
        // event, so the budget on the external drain never trips. The pops of entries
        // due at the reading are bounded by the same budget (ARCHITECTURE.md
        // "External-Event Budget", rule 5).
        //
        // Entries due at the reading, not every entry popped at it: the reading is
        // latched for the whole tick, so a host that jumps the clock a long way pops
        // every entry that came due on the way at one reading, and those are due at
        // earlier instants. A handler arms relative to the latched reading, so what it
        // arms is due at the reading itself or after it, and only the first kind can be
        // popped by this tick. A clock the host moved is bounded by how far it moved
        // and is not cut; a heartbeat across a long jump arms its next beat after the
        // reading and is not popped here at all.
        var poppedDueAtReading: UInt32 = 0

        // §scxml-6.2: dispatch the ready scheduled events, earliest deadline first and
        // one macrostep apart.
        //
        // One at a time, not all at once. <cancel> drops an event that has not been
        // dispatched yet, and a host that woke late holds several past their
        // deadlines: promoting them together makes every later one undroppable before
        // the earlier one's transitions have had a chance to run. That is how a settle
        // timer (arm a long <send delay>, cancel it when the short signal arrives
        // first) delivers the event it was told to cancel. Measured 2026-08-19 on the
        // Go, Rust and Python backends alike.
        //
        // "Due" is judged against the instant this tick began, not against a clock
        // re-read on every pass: a tick that chased its own slowness would dispatch
        // entries the host had not yet reached, in a loop the host cannot get between
        // (see `beginTurn`).
        while true {
            if let next = scheduler.nextReadyAt(turnNowMs), next.dueMs == turnNowMs {
                if poppedDueAtReading >= maxExternalEventsPerCall {
                    recordTruncatedEventChain(head: next.event)
                    break
                }
                poppedDueAtReading += 1
            }
            guard let act = scheduler.popReadyAct(at: turnNowMs) else {
                break
            }
            if act.route != nil {
                // §scxml-6.2: to the target the send named when it was made.
                deliverRouted(act)
            } else if let event = act.event {
                // §scxml-5.10.1 + §scxml-C-1: the metadata the immediate send stamps,
                // restored when the wait ends, and §scxml-5.10: the name it was sent
                // under.
                var due = EventWithMetadata.withFields(
                    event,
                    data: act.data,
                    origin: act.origin,
                    sendID: act.sendID,
                    eventType: .externalEvent,
                    originType: scxmlEventProcessorType,
                    invokeID: "",
                    target: ""
                )
                due.metadata.name = act.name
                externalQueue.raise(due)
            }
            // The macrostep this act drives may <cancel> a later one, so the queue is
            // re-consulted after it and not before.
            runMainEventLoop()
            if !isRunning || isInFinalState {
                break
            }
        }

        // §scxml-6.4: Tick child state machines
        if policy.hasChildTick {
            policy.tickChildren(engine: self)
        }

        // Delegate to `step()` for the main event loop + completion callback.
        // §scxml-6.4's invokes are part of that loop and run there, ahead of the
        // external dequeue and not after it.
        step()
    }

    /// Stops the engine. Subsequent `tick` and `processEvent` calls become no-ops.
    ///
    /// §scxml-D-exitInterpreter: a run the host stops is exited as one that ended:
    /// every active state's `<onexit>` runs, innermost first, and the configuration
    /// ends empty. A run that already ended has been exited.
    public func stop() {
        if isRunning {
            exitInterpreter()
        }
        isRunning = false
    }

    /// Appendix D's `exitInterpreter` (§scxml-D-exitInterpreter): exit every state
    /// still in the configuration, innermost first, each as `exitStates` exits one
    /// (its `<onexit>`, then its invocations cancelled, then it leaves the
    /// configuration) and stop running.
    ///
    /// Reached two ways, the two the procedure names: the main event loop ends because
    /// the run entered a top-level `<final>`, or the host stops a run that has not
    /// ended. Runs once per run.
    private func exitInterpreter() {
        if interpreterExited {
            return
        }
        let configuration = getActiveStates()
        // configuration.toList().sort(exitOrder): reverse document order.
        let statesToExit = configuration.enumerated()
            .map { (offset: $0.offset, order: policy.getDocumentOrder($0.element), state: $0.element) }
            .sorted { ($0.order, $1.offset) > ($1.order, $0.offset) }
            .map(\.state)
        for state in statesToExit {
            policy.executeExitActions(state, engine: self, configurationBeforeExit: configuration)
        }
        interpreterExited = true
        isRunning = false
    }

    /// The current active (leaf) state.
    public func getCurrentState() -> State {
        currentState
    }

    /// The full list of active states (§scxml-3.11).
    ///
    /// Non-parallel machines: the hierarchy `[leaf, parent, grandparent, ..., root]`.
    /// Parallel machines: the union of all active regions via
    /// `StatePolicy.getActiveStates()`.
    public func getActiveStates() -> [State] {
        if policy.hasActiveStates {
            return policy.getActiveStates()
        }
        // §scxml-D-exitInterpreter: a run that has ended has no configuration. (A
        // policy that keeps its own active set emptied it state by state above; this
        // one derives the set from `currentState`, which still names the final, so the
        // answer has to be given here.)
        if interpreterExited {
            return []
        }
        // Walk from the current state up to the root.
        var active: [State] = []
        active.reserveCapacity(8)
        var current: State? = currentState
        while let state = current {
            active.append(state)
            current = policy.getParent(state)
        }
        return active
    }

    /// Binds the dequeued event's typed `_event.data` view and reports a refusal as
    /// `error.execution` (NL-to-IR Item C1 Path A).
    ///
    /// The raise lives here and not in each generated policy so that every machine
    /// answers a payload it cannot read the same way, and so the answer is the script
    /// engine's: §scxml-3.13 gives `error.execution` for a guard that cannot be
    /// evaluated and treats it as false, which is exactly what a policy with no lifted
    /// payload does: its tag stays `none` and the typed guards fail.
    private func liftPayload(of event: Event, metadata: EventMetadata) {
        do {
            try policy.liftTypedPayload(event: event, metadata: metadata)
        } catch {
            guard let errorEvent = policy.getEventFromName("error.execution") else {
                // §scxml-3.12.2 leaves a document that declares no error.execution
                // transition nothing to answer with; the guard still does not fire.
                return
            }
            raise(
                .platformError(
                    errorEvent,
                    message: "`\(policy.getEventName(event))` payload: \(error)"
                )
            )
        }
    }

    /// Whether this session has ended: whether the run has entered a `<final>` whose
    /// parent is the `<scxml>` element (§scxml-3.7).
    ///
    /// Appendix D `enterStates` sets `running = false` for a `<final>` only when
    /// `isSCXMLElement(s.parent)`; a nested one queues `done.state.<parent>` and the
    /// machine carries on. So the structural question, "is this state a `<final>`
    /// element", is `StatePolicy.isFinalState`, and it is not the completion criterion
    /// on its own. Everything that means "the machine is done" keys on this property:
    /// `runUntilCompletion`, the completion callback, and the `done.invoke.<id>` a
    /// parent emits for an invoked child. The same answer as `terminalState` having a
    /// value.
    public var isInFinalState: Bool {
        terminalState != nil
    }

    /// Appendix D's `s.isFirstEntry`, taken (§scxml-D-enterStates): true exactly once
    /// per run for `state` (the entry on which a late-binding document binds that
    /// state's `<data>`) and false on every entry after it. The generated entry code
    /// asks this and does not keep its own flag, so the rule is written once for every
    /// machine.
    public func claimLateBindingFirstEntry(_ state: State) -> Bool {
        if lateBoundStates?.contains(state) == true {
            return false
        }
        var claimed = lateBoundStates ?? []
        claimed.insert(state)
        lateBoundStates = claimed
        return true
    }

    /// Records `state` as the run's terminal state when it is a `<final>` whose parent
    /// is the `<scxml>` element: the one kind of state whose entry ends the run
    /// (§scxml-D-enterStates).
    fileprivate func recordTerminalState(_ state: State) {
        if !policy.isFinalState(state) {
            return
        }
        if policy.getParent(state) != nil {
            return
        }
        terminalState = state
    }

    private func clearTerminalState() {
        terminalState = nil
    }

    // MARK: - Event submission (matches Rust raise / raiseExternal overloads)

    /// Enqueues an internal event with full metadata (high priority) (§scxml-C-1).
    ///
    /// Matches Rust `Engine::raise`.
    ///
    /// An `error.*` event raised while an error handler is running is refused once the
    /// chain reaches `maxErrorCascadeDepth`; see `errorCascadeEvents` for why the engine
    /// is the one that has to stop it. Only the engine's own error events are refused:
    /// an author's `<raise>` inside an error handler is the document doing its job and
    /// rides the queue like any other.
    public func raise(_ event: EventWithMetadata<Event>) {
        // §scxml-3.12.2 names the error events this refuses; the clause itself is
        // silent on a handler that fails, which is why the ceiling is a choice this
        // engine documents and not a rule it implements.
        if handlingErrorEvent && isErrorEvent(policy.getEventName(event.event)) {
            errorCascadeDepth += 1
            if errorCascadeDepth >= maxErrorCascadeDepth {
                errorCascadeEvents += 1
                lastErrorCascadeEvent = event.event
                if errorCascadeEvents == 1 {
                    sceLog(
                        "Engine.raise: an error handler has raised an error \(maxErrorCascadeDepth) times over; "
                            + "refusing to feed the chain -- the document's error handling is failing"
                    )
                }
                return
            }
        }
        internalQueue.raise(event)
    }

    /// Enqueues an external event with optional data and origin (§scxml-C-1 / 6.2).
    ///
    /// Matches Rust `Engine::raise_external`.
    public func raiseExternal(_ event: Event, eventData: String = "", origin: String = "") {
        // §scxml-5.10.1: what the host hands the external queue is typed "external"
        // here, where it enters that queue.
        externalQueue.raise(
            .withFields(
                event,
                data: eventData,
                origin: origin,
                sendID: "",
                eventType: .externalEvent,
                originType: scxmlEventProcessorType,
                invokeID: "",
                target: ""
            )
        )
    }

    /// The event an event that arrives by name is delivered as (§scxml-3.12.1).
    ///
    /// An event arrives by name from outside the document (a child's autoforward, a
    /// host) so the names it can arrive under are open, while the enum holds only the
    /// names the document writes. A transition's descriptor matches an event by whole
    /// tokens: `request` matches `request.new` whether or not the document writes
    /// `request.new`. Every descriptor that matches an arriving name is a token prefix
    /// of it, so is one of the document's names, so is a prefix of the longest of
    /// those, which therefore matches exactly what the arriving name would. That is
    /// the event this answers: the name itself when the document writes it, else the
    /// longest token prefix of it the document does, else the wildcard member (the one
    /// `event="*"` stands for, which the exact table keeps under the name `*`; no
    /// document can write that as an event's name) when the document has such a
    /// transition, else nil: no transition the document has could match it.
    ///
    /// `getEventFromName` stays the exact table; the rule is written once, here, and
    /// the Rust twin is `StatePolicy::resolve_event_by_name`.
    public func resolveEventByName(_ name: String) -> Event? {
        var candidate = Array(name.utf8)
        while true {
            if let event = policy.getEventFromName(String(decoding: candidate, as: UTF8.self)) {
                return event
            }
            guard let at = candidate.lastIndex(of: UInt8(ascii: ".")) else {
                return policy.getEventFromName("*")
            }
            candidate = Array(candidate[..<at])
        }
    }

    /// Raises an external event by name (§scxml-6.4.1, for child autoforward).
    ///
    /// The name is delivered as the event `resolveEventByName` answers. A name no
    /// transition of the document could match is silently ignored. Matches Rust
    /// `Engine::raise_external_by_name`.
    public func raiseExternalByName(_ eventName: String, eventData: String = "") {
        guard let event = resolveEventByName(eventName) else {
            sceLog("Engine.raiseExternalByName: event '\(eventName)' matches no event of the document, ignoring")
            return
        }
        // §scxml-5.10.1: what a host hands the external queue is typed "external"
        // here, where it enters that queue; and `_event.name` is the name it gave.
        var meta = EventWithMetadata.withFields(
            event,
            data: eventData,
            origin: "",
            sendID: "",
            eventType: .externalEvent,
            originType: scxmlEventProcessorType,
            invokeID: "",
            target: ""
        )
        meta.metadata.name = arrivalName(of: event, name: eventName)
        externalQueue.raise(meta)
    }

    /// What `EventMetadata.name` is for an event that arrived as `name` and was
    /// delivered as `event` (§scxml-5.10): empty when `event` is the document's own
    /// name for it, so an event the document raises or sends carries no copy of a name
    /// its value already holds, and `name` when `resolveEventByName` had to cut it
    /// back to a shorter one.
    private func arrivalName(of event: Event, name: String) -> String {
        policy.getEventName(event).utf8.elementsEqual(name.utf8) ? "" : name
    }

    /// The event an event that arrives by name is delivered as, with the name it
    /// arrived under carried beside it (§scxml-3.12.1 + §scxml-5.10).
    ///
    /// The one constructor for every door that turns a name from outside the machine
    /// into an event of it (a child's `<send target="#_parent">`, a completion
    /// `done.invoke.<id>`, a host's reply, an HTTP response) so none of them resolves a
    /// name its own way. Nil when no transition the document has could match the name:
    /// it is dropped. `metadata` is what the caller knows (data, origin, invoke id); the
    /// arrival name replaces whatever it carried, since a name matched against another
    /// machine's says nothing here. The Rust twin is `Engine::arriving_event`.
    func arrivingEvent(name: String, metadata: EventMetadata) -> EventWithMetadata<Event>? {
        guard let event = resolveEventByName(name) else {
            return nil
        }
        var metadata = metadata
        metadata.name = arrivalName(of: event, name: name)
        return EventWithMetadata(event, metadata: metadata)
    }

    /// Raises an autoforwarded external event that is name-addressed but carries the
    /// source event's `_event` fields (§scxml-6.4).
    ///
    /// Autoforward is the one path where an event leaves the machine owning its enum,
    /// so it crosses by name while the metadata travels with it. Unknown names degrade
    /// silently: a child need not declare every event its parent forwards.
    public func raiseExternalByNameWithMeta(_ eventName: String, metadata: EventMetadata) {
        guard let event = resolveEventByName(eventName) else {
            sceLog(
                "Engine.raiseExternalByNameWithMeta: event '\(eventName)' matches no event of the document, ignoring"
            )
            return
        }
        // §scxml-5.10: the copy is exact, but what this machine is told the event is
        // called is a fact about this machine's names: the source's arrival name was
        // matched against the source's, and says nothing here.
        var metadata = metadata
        metadata.name = arrivalName(of: event, name: eventName)
        // The target stays empty: the copy is delivered to this machine, never
        // re-routed to the original event's target.
        raiseExternalWithMeta(EventWithMetadata(event, metadata: metadata))
    }

    /// Raises an external event with full metadata (W3C SCXML 6.4.1, for
    /// child-to-parent events).
    ///
    /// Preserves the invoke id for parent finalize handlers. Matches Rust
    /// `Engine::raise_external_with_meta`.
    public func raiseExternalWithMeta(_ event: EventWithMetadata<Event>) {
        externalQueue.raise(event)
    }

    /// Processes an external event (convenience API, runs one macrostep)
    /// (§scxml-3.12).
    ///
    /// Matches Rust `Engine::process_event`.
    public func processEvent(_ event: Event) {
        if !isRunning {
            // Refused and not queued, so the drain in `runMainEventLoop` never sees
            // it, which is why the count is taken here as well as there.
            noteUnseenEvent(event)
            return
        }
        raiseExternal(event)
        step()
    }

    /// Processes an external event with metadata (§scxml-5.10).
    ///
    /// Matches Rust `Engine::process_event_with_meta`.
    public func processEventWithMeta(_ event: Event, metadata: EventMetadata) {
        if !isRunning {
            // Same refusal as `processEvent`, same reason it is counted here.
            noteUnseenEvent(event)
            return
        }
        externalQueue.raise(EventWithMetadata(event, metadata: metadata))
        step()
    }

    // MARK: - Donedata stash (§scxml-5.5 + 6.3.1)

    /// Records the donedata payload evaluated on a top-level `<final>` entry so an
    /// invoking parent can lift it onto `done.invoke.<id>._event.data`.
    ///
    /// Matches C++ `StaticExecutionEngine::stashDonedataAtFinal` and Rust
    /// `Engine::stash_donedata_at_final`.
    public func stashDonedataAtFinal(_ data: String) {
        donedataAtFinal = data
    }

    // MARK: - Scheduler passthrough

    /// Schedules an event for delayed delivery. Returns the send id.
    ///
    /// The deadline is this turn's instant plus the delay; see `beginTurn` for why the
    /// reading is the turn's and not this statement's.
    @discardableResult
    public func scheduleEvent(
        _ event: Event,
        delay: Duration,
        sendID: String,
        eventData: String,
        origin: String
    ) -> String {
        scheduler.scheduleEvent(
            event,
            readyAtMs: schedNowMs() + delay.wholeMilliseconds,
            sendID: sendID,
            eventData: eventData,
            origin: origin
        )
    }

    /// A `<send eventexpr>` this session addresses to its own external queue, now or
    /// after `delay` (§scxml-6.2 + §scxml-3.12.1 + §scxml-5.10).
    ///
    /// The name is computed at run time, so the document cannot have written it: it is
    /// delivered as the event `resolveEventByName` gives it (the document's own event
    /// of that name, the longest token prefix of it the document writes, or its
    /// wildcard) and a name no descriptor of the document could match is dropped, as
    /// one that arrives from outside is. `_event.name` is the whole name whenever the
    /// member is not called that, a delayed send's included.
    public func sendNamedExternal(
        _ name: String,
        delay: Duration,
        sendID: String,
        eventData: String,
        origin: String
    ) {
        guard let event = resolveEventByName(name) else {
            return
        }
        let arrival = arrivalName(of: event, name: name)
        if delay > .zero {
            scheduler.scheduleEvent(
                event,
                readyAtMs: schedNowMs() + delay.wholeMilliseconds,
                sendID: sendID,
                eventData: eventData,
                origin: origin,
                name: arrival
            )
            return
        }
        var meta = EventWithMetadata(event, metadata: .external(sendID: sendID, origin: origin))
        meta.metadata.data = eventData
        meta.metadata.name = arrival
        raiseExternalWithMeta(meta)
    }

    /// The internal-queue twin of `sendNamedExternal`: a
    /// `<send target="#_internal" eventexpr>`, now or after `delay`. A delay postpones
    /// it and does not change the queue (§scxml-6.2).
    public func sendNamedInternal(
        _ name: String,
        delay: Duration,
        sendID: String,
        eventData: String,
        origin: String
    ) {
        guard let event = resolveEventByName(name) else {
            return
        }
        if delay > .zero {
            scheduleRoutedEvent(
                event,
                delay: delay,
                sendID: sendID,
                eventData: eventData,
                origin: origin,
                route: ScheduledRoute(kind: .internalQueue, eventName: name)
            )
            return
        }
        var meta = EventWithMetadata(event)
        meta.metadata.data = eventData
        meta.metadata.name = arrivalName(of: event, name: name)
        raise(meta)
    }

    /// Settles the children of the policy after a delivery made to one of them, which
    /// the child took at once and may have answered.
    private func settleChildren() {
        policy.settleChildren(engine: self)
    }

    /// Sends to a target value read at run time (§scxml-6.2.4, §scxml-C-1).
    ///
    /// A `targetexpr` is a target: whatever value it yields is routed as the same value
    /// written in `target` is (`classifyTarget` reads it, the table C++
    /// `SendHelper::classifyTarget` holds) sent at once or, with a positive delay, from
    /// the scheduler through `deliverRouted` when it comes due. An invocation or a
    /// session the value names that is not running now is reported now; one that ends
    /// while a delayed send waits is reported when it comes due.
    ///
    /// `event` is this machine's spelling of the event, for its own queues; nil when
    /// the document has no such event, and a child or a parent resolves `eventName`.
    public func sendToTarget(
        event: Event?,
        eventName: String,
        target: String,
        ownSessionID: String,
        delay: Duration,
        sendID: String,
        eventData: String,
        origin: String
    ) -> TargetSendOutcome {
        let resolved = classifyTarget(target, ownSessionID: ownSessionID)
        let delayed = delay > .zero
        let route: ScheduledRoute
        switch resolved.kind {
        case .unsupported, .mesh:
            return .notSupported
        case .unreachable:
            return .notReachable
        case .selfExternal:
            if let event {
                // §scxml-5.10: `event` is the member the sender's name resolved to in
                // this machine's document, so its own queues are told the name it was
                // sent under when the member is not called that.
                let arrival = arrivalName(of: event, name: eventName)
                if delayed {
                    scheduler.scheduleEvent(
                        event,
                        readyAtMs: schedNowMs() + delay.wholeMilliseconds,
                        sendID: sendID,
                        eventData: eventData,
                        origin: origin,
                        name: arrival
                    )
                } else {
                    var meta = EventWithMetadata.withFields(
                        event,
                        data: eventData,
                        origin: origin,
                        sendID: sendID,
                        eventType: .externalEvent,
                        originType: scxmlEventProcessorType,
                        invokeID: "",
                        target: ""
                    )
                    meta.metadata.name = arrival
                    raiseExternalWithMeta(meta)
                }
            }
            return .sent
        case .internalQueue:
            route = ScheduledRoute(kind: .internalQueue, eventName: eventName)
        case .parent:
            // §scxml-C-1: a session nothing invoked has no parent to address, delayed
            // or not.
            if !policy.hasParentSession {
                return .notReachable
            }
            route = ScheduledRoute(kind: .parent, eventName: eventName)
        case .invocation:
            if !policy.isInvocationRunning(resolved.id) {
                return .notReachable
            }
            route = ScheduledRoute(kind: .invocation, eventName: eventName, invokeID: resolved.id)
        case .session:
            if !policy.isChildSessionRunning(resolved.id) {
                return .notReachable
            }
            route = ScheduledRoute(kind: .session, eventName: eventName, sessionID: resolved.id)
        }
        if route.kind == .internalQueue && event == nil {
            return .sent
        }
        if delayed {
            scheduleRoutedEvent(
                event,
                delay: delay,
                sendID: sendID,
                eventData: eventData,
                origin: origin,
                route: route
            )
            return .sent
        }
        if deliverRoutedNow(event: event, route: route, eventData: eventData, sendID: sendID, origin: origin) {
            return .sent
        }
        return .notReachable
    }

    /// Delivers a send to the target a route names, now: the one delivery a send made
    /// at once and a delayed send whose wait is over both go through (§scxml-6.2.4: a
    /// delay postpones a send, it does not change where it goes). Whether the target
    /// was there to take the event.
    private func deliverRoutedNow(
        event: Event?,
        route: ScheduledRoute,
        eventData: String,
        sendID: String,
        origin: String
    ) -> Bool {
        switch route.kind {
        case .internalQueue:
            guard let event else {
                return false
            }
            var meta = EventWithMetadata(event)
            meta.metadata.data = eventData
            meta.metadata.sendID = sendID
            meta.metadata.origin = origin
            if !route.eventName.isEmpty {
                meta.metadata.name = arrivalName(of: event, name: route.eventName)
            }
            raise(meta)
            return true
        case .invocation:
            let delivered = policy.deliverToInvocation(
                invokeID: route.invokeID,
                eventName: route.eventName,
                eventData: eventData,
                sendID: sendID
            )
            settleChildren()
            return delivered
        case .session:
            let delivered = policy.deliverToChildSession(
                childSessionID: route.sessionID,
                eventName: route.eventName,
                eventData: eventData
            )
            settleChildren()
            return delivered
        case .parent:
            return policy.deliverToParent(eventName: route.eventName, eventData: eventData)
        }
    }

    /// Schedules a delayed send whose target is not this session's own external queue
    /// (§scxml-6.2). Returns the send id.
    ///
    /// A delay postpones a send; it does not change where the send goes. The generated
    /// send site resolves the target when the send is made and hands it over here, so
    /// the delivery at the end of the delay reaches it, in the same queue as every
    /// other delayed send, which is what lets one `<cancel sendid>` (§scxml-6.3) reach
    /// it. `event` is this machine's own spelling of the event, used for the internal
    /// queue; a child or a parent resolves `route.eventName`.
    @discardableResult
    public func scheduleRoutedEvent(
        _ event: Event?,
        delay: Duration,
        sendID: String,
        eventData: String,
        origin: String,
        route: ScheduledRoute
    ) -> String {
        scheduler.scheduleRouted(
            event,
            readyAtMs: schedNowMs() + delay.wholeMilliseconds,
            sendID: sendID,
            eventData: eventData,
            origin: origin,
            route: route
        )
    }

    /// Delivers a delayed send whose wait is over to the target it named.
    ///
    /// §scxml-C-1: "If the SCXML Processor cannot dispatch the event to the target, it
    /// MUST place the error error.communication on the internal event queue of the
    /// session that attempted to send the event." For a delayed send the dispatch is
    /// this delivery, so an invocation that has ended in the meantime is reported here,
    /// not when the send was made.
    private func deliverRouted(_ act: ReadyAct<Event>) {
        guard let route = act.route else {
            return
        }
        if deliverRoutedNow(event: act.event, route: route, eventData: act.data, sendID: act.sendID, origin: act.origin) {
            return
        }
        if let communicationError = policy.getEventFromName("error.communication") {
            var failure = EventWithMetadata.platformError(
                communicationError,
                message: "<send> named a target that was no longer there when its delay elapsed"
            )
            failure.metadata.sendID = act.sendID
            raise(failure)
        }
    }

    /// The id for a `<send idlocation>`, which the document keeps and may name in a
    /// `<cancel>` later: `_auto_send_` and the number of ids this machine has
    /// generated, counted from one. Unique within the machine, as the send's id must
    /// be, and the same text on every backend (at most 31 bytes: the largest `UInt64`
    /// is twenty digits).
    public func nextAutoSendID() -> String {
        autoSendSeq += 1
        return "_auto_send_\(autoSendSeq)"
    }

    /// Cancels a previously scheduled event by send id.
    public func cancelEvent(sendID: String) -> Bool {
        scheduler.cancelEvent(sendID: sendID)
    }

    /// Whether the scheduler has events ready to fire.
    public func hasReadyEvents() -> Bool {
        scheduler.hasReadyEvents(at: schedNowMs())
    }

    /// How long until this machine next needs `tick`. Zero means something is due now;
    /// nil when the scheduler is empty and no clock-driven wake-up is owed.
    ///
    /// `needsEventScheduler` tells a host which entry point to drive the machine with.
    /// This tells it when, and a host that cannot ask has only one move left: pick a
    /// polling interval. That guess is not free in either direction: measured on a
    /// document whose `<send delay="200ms">` is cancelled by a 100 ms signal, a 1 ms
    /// interval spends 176 wasted ticks to be on time, a 500 ms one fires 300 ms late,
    /// and a 250 ms one steps over both deadlines at once. An interval cannot straddle
    /// two deadlines it was never told about.
    ///
    /// The answer feeds a host loop directly: a sleep, a timer, a deadline.
    ///
    /// §scxml-6.4: the session's own scheduler is not the whole answer. This machine's
    /// `tick` advances every running child, so a child's deadline is a deadline of this
    /// machine's clock, and the answer is the nearest of the two. Counting only the
    /// parent's own made a host that walked time by it step over a timer its child had
    /// armed, or, for a parent that arms nothing itself, be told there was nothing to
    /// wait for at all.
    public func timeUntilNextScheduled() -> Duration? {
        var soonest: Duration?
        if let nextMs = scheduler.nextReadyAtMs {
            soonest = .milliseconds(max(0, nextMs - schedNowMs()))
        }
        if let due = policy.nextChildDeadline(), soonest == nil || due < soonest! {
            soonest = due
        }
        return soonest
    }

    // MARK: - Clock (§scxml-6.2.2)

    /// Where this engine reads "now" from; see `SceClock`.
    ///
    /// Never nil: an engine that was never given one reads a `MonotonicClock`, which is
    /// installed on first use.
    public func clock() -> any SceClock {
        if installedClock == nil {
            installedClock = MonotonicClock()
        }
        return installedClock!
    }

    /// Installs the `SceClock` this engine measures its `<send delay>` deadlines
    /// against.
    ///
    /// Must be called before `initialize`: the entry configuration's `<onentry>` can
    /// arm delayed sends, and swapping the clock under deadlines already computed from
    /// another one would leave the queue holding two incomparable time bases. That is a
    /// programming error and not a recoverable condition, so it traps.
    public func setClock(_ clock: any SceClock) {
        precondition(
            !isRunning,
            "sce: Engine.setClock must be called before initialize(): this engine has already armed its "
                + "entry configuration against the previous clock, and deadlines from two clocks do not compare"
        )
        installedClock = clock
    }

    /// Moves this engine's clock forward by `ms` and runs whatever that made due
    /// (§scxml-6.2).
    ///
    /// The host-owned twin of `tick`: `tick` asks a clock that moves on its own what
    /// time it is, this one sets what time it is and then ticks. A machine driven
    /// exclusively through here has no dependency on the load of the machine it runs
    /// on: the same sequence of calls produces the same configuration every time.
    ///
    /// Traps unless the clock is a `ManualClock`, because that is the only kind of
    /// clock a host can move. Calling it against the monotonic default is a programming
    /// error, not a no-op: it means the caller believes it owns time and it does not,
    /// and the events it is waiting for will arrive on a schedule it did not choose.
    public func advanceTimeMs(_ ms: Int64) {
        guard let manual = clock() as? ManualClock else {
            preconditionFailure(
                "sce: Engine.advanceTimeMs needs a ManualClock as clock(); this engine's time is not the "
                    + "host's to move. Call setClock(ManualClock()) before initialize(), or drive this machine "
                    + "with tick() and timeUntilNextScheduled()"
            )
        }
        manual.advance(ms)
        tick()
    }

    /// This engine's current reading of its clock, in milliseconds since that clock's
    /// origin.
    ///
    /// The absolute counterpart of `timeUntilNextScheduled`'s relative answer. A host
    /// owning time through a `ManualClock` uses it to say where in the run it is; a
    /// host on the wall clock uses it to correlate an engine's deadlines with its own
    /// log.
    public func nowMs() -> Int64 {
        schedNowMs()
    }

    /// Answers §scxml-3.13's question: what time it is, for everything this turn arms
    /// or judges.
    ///
    /// The clause executes a microstep's executable content as one unit and a
    /// macrostep as a chain of those, so "now" is a property of the turn the engine is
    /// in and not of the statement asking for it. Between turns there is no turn for it
    /// to be a property of, and the host's queries (`timeUntilNextScheduled`, `nowMs`)
    /// read the clock live.
    private func schedNowMs() -> Int64 {
        inTurn ? turnNowMs : clock().elapsedMs()
    }

    /// Opens a turn: it takes the single clock reading that everything inside it uses.
    ///
    /// Returns whether this call is the one that opened it, which `endTurn` needs so a
    /// nested entry point (`processEvent` delegating to `step`, `tick` doing the same)
    /// does not close the outer turn early.
    ///
    /// §scxml-6.2.2 makes a delay the wait the document asks for: "how long the
    /// processor should wait before dispatching the message". Time the host spent
    /// descheduled between two statements of one microstep is not part of any delay the
    /// document wrote, so it must not reach the deadline. Reading the clock per
    /// statement instead was two defects at once, both measured on the Go backend:
    ///
    ///   - Two `<send delay>`s executed by one `<onentry>` took a reading each, so a
    ///     host descheduled between them by more than the gap between their delays got
    ///     the later send's deadline first, and the engine then dispatched them in that
    ///     order, so the document's `<cancel>` arrived after the event it named. Which
    ///     of two events the author ordered arrives first became a fact about the host's
    ///     scheduler.
    ///   - The dispatch loop in `tick` re-read it on every pass, so a tick slow enough
    ///     to cross the next deadline dispatched that entry too, then the one after it:
    ///     the engine chasing deadlines its own slowness created, in a loop the host
    ///     cannot get between.
    ///
    /// Neither is reachable from a clock that is read once per turn.
    private func beginTurn() -> Bool {
        if inTurn {
            return false
        }
        turnNowMs = clock().elapsedMs()
        inTurn = true
        return true
    }

    /// Closes a turn opened by `beginTurn`.
    private func endTurn(_ opened: Bool) {
        if opened {
            inTurn = false
        }
    }

    // MARK: - Diagnostics

    /// Notes which §scxml-B-2-8-1 rung the payload just bound got.
    ///
    /// Called by generated code immediately after it binds `_event`, because that is
    /// the only moment the rung is known. Four of the five readings are the ladder
    /// working and are recorded by being ignored; the fifth is the one a host is wrong
    /// about.
    public func notePayloadReading(_ event: Event, reading: PayloadReading) {
        if reading == .undecodable {
            undecodablePayloads += 1
            lastUndecodablePayload = event
        }
    }

    /// Records one external event this machine will never look at.
    private func noteUnseenEvent(_ event: Event) {
        // W3C SCXML Appendix D's main event loop: the loop that would have dequeued
        // this has ended, so the event is not "pending"; it is over.
        unseenExternalEvents += 1
        lastUnseenEvent = event
    }

    /// Empties the external queue into the count above, at the moment the main event
    /// loop ends.
    ///
    /// Drained and not left in place so each event is counted exactly once: a host
    /// that keeps calling `step()` on a halted machine would otherwise re-count the
    /// same queue on every call, and a count that grows while nothing arrives is a
    /// count nobody can use.
    private func recordUnseenExternalEvents() {
        while let meta = externalQueue.pop() {
            noteUnseenEvent(meta.event)
        }
    }

    // `discardedExternalEvents` counts what a machine discarded (§scxml-3.1.2).
    //
    // Discarding is what the clause requires. The count is the part the clause does
    // not cover: the host that queued the event cannot otherwise tell that outcome from
    // a handled one, because a self transition, a targetless internal transition and a
    // discard all leave the configuration alone. Comparing the count across a drive is
    // what turns "the machine ignored what I sent" into something the program can see.
    //
    // The C++ Interpreter has answered this all along (`processEvent`'s
    // `TransitionResult.success`, and `getStatistics().failedTransitions`); this is the
    // generated engines' side of the same question.
    //
    // Counts external-queue events only. An internal `<raise>` that matches nothing is
    // discarded too, but both ends of that are inside the document.
    //
    // `undecodablePayloads` counts events that arrived carrying a payload that
    // announced itself as structure and that the datamodel could not read as one.
    // §scxml-B-2-8-1 requires the fallback: content the processor cannot interpret
    // becomes a space-normalized string. What it does not require is any way for the
    // host that sent that payload to learn its fields have stopped existing. Counts
    // only the reading a host can act on: prose delivered as text is the ladder working
    // (W3C test 562) and is not counted, because a diagnostic that fires when nothing is
    // wrong is one nobody reads.
    //
    // `unseenExternalEvents` counts external events the host handed this machine that
    // it never looked at, because it had already stopped. W3C SCXML Appendix D's main
    // event loop exits when the machine reaches a top-level final state, and §scxml-3.13
    // is explicit that the interpreter is then done. Refusing the event is therefore
    // correct, and, as with `discardedExternalEvents` and `undecodablePayloads`, being
    // unable to say it happened is not part of the clause. This is the count that
    // separates the third explanation from the other two. A host that sent an event and
    // saw nothing move has three candidates:
    //
    //     dequeued, no transition matched          -> discardedExternalEvents
    //     dequeued, a transition matched but its
    //       guard was false                        -> neither
    //     never dequeued: the machine had stopped  -> this one
    //
    // `unhandledErrorEvents` counts `error.*` events this engine raised that no
    // transition in any active state answered. §scxml-3.12.2 requires the processor to
    // signal its own failures as `error.*` events on the internal queue, and says in
    // the same breath that "they are ignored if no transition is found that matches
    // them". Being ignored is the clause. Being unable to say it happened is not, and
    // the difference matters to exactly one party: the host, which did not write the
    // document, cannot see the failure anywhere in the configuration, and is the only
    // one positioned to do something about it. A supervisor driving a machine whose
    // `<assign>` silently fails every round reads `isRunning == true` and a plausible
    // state forever. It is the sibling of `discardedExternalEvents`, and the two are
    // deliberately separate counts: that one stops at the external queue because an
    // author's unmatched `<raise>` has both ends inside the document; an error event's
    // sender is the engine, so the same reasoning does not reach it. An error the
    // document did answer is not counted either: the document dealt with it, and its
    // handling is visible in the configuration the host can already read.
    //
    // `errorCascadeEvents` counts `error.*` events this engine refused to queue because
    // the error handler that raised them had been failing for `maxErrorCascadeDepth`
    // links running. §scxml-3.12.2 says an unmatched error event is ignored, and
    // `unhandledErrorEvents` is that case. This is its opposite and its worse half: the
    // document does match the error, and the handler fails the same way every time. The
    // failure raises `error.execution`, the same transition answers it, and the drain
    // never empties. Left to run, that is not a hang: it is a core at 100% forever, with
    // `isRunning` staying true, the exact reading an unattended supervisor takes as
    // healthy. So the engine stops feeding the chain and says how often it had to, which
    // is the one fact that separates "this machine is idle" from "this machine's error
    // handling is broken". A document that fails five hundred times cleanly counts zero
    // here: the chain is measured from handler to handler, not from failure to failure,
    // and any other internal event resets it.
    //
    // `truncatedMacrosteps` counts macrosteps this engine stopped short because their
    // chain was still going after `maxMacrostepMicrosteps` microsteps. A document with a
    // cyclic eventless transition is not malformed, and neither is one whose `<raise>`
    // answers itself; both are documents whose macrostep is infinite, and an engine that
    // runs either to the letter never returns. Both are counted here, because they are
    // the same fact to a host: the macrostep it just drove did not reach a stable
    // configuration. Which chain it was is what `lastTruncatedMacrostepState` points at.
    // A document whose chain is a hundred microsteps long and then settles counts zero:
    // the ceiling is on microsteps taken, and the macrostep is only counted here when
    // the loop still had work after them.
    //
    // `truncatedEventChains` counts invocations of the main event loop this engine
    // handed back with an external event still queued, because the invocation had
    // already taken `maxExternalEventsPerCall` of them (ARCHITECTURE.md "External-Event
    // Budget"). `truncatedMacrosteps` counts a macrostep that does not end; this counts
    // the other way a call fails to return: every macrostep ends, and each one queues
    // the event that starts the next. Counted when the loop still had work after the
    // budget: a call that takes exactly the budget and empties the queue counts zero.
    // The events left queued stay queued, so a host that calls again gets another budget
    // and the machine goes on from where it was. A cut of the same-instant bound on the
    // scheduler counts here too (see `tick`).

    /// The most external events one invocation of the main event loop may take.
    public var maxExternalEventsPerCall: UInt32 {
        maxExternalEventsPerCallChosen == 0 ? defaultMaxExternalEventsPerCall : maxExternalEventsPerCallChosen
    }

    /// Lets a host choose the budget of one invocation. A host that hands the machine a
    /// backlog it means the machine to work through in one call knows its size, and
    /// this engine does not. Settable at any time.
    ///
    /// A budget below one is refused and changes nothing: a budget that takes no event
    /// is a machine that cannot run, not a stricter one, since every call would hand
    /// control back with the queue untouched and say it had been cut. So is one the
    /// counter cannot hold.
    public func setMaxExternalEventsPerCall(_ limit: Int) throws {
        if limit < 1 || UInt64(limit) > UInt64(UInt32.max) {
            throw EngineError(
                "sce: the budget of one call is a whole number of events, at least one and at most "
                    + "\(UInt32.max); got \(limit)"
            )
        }
        maxExternalEventsPerCallChosen = UInt32(limit)
    }

    /// Says that a call was handed back with work still waiting: the count, and what
    /// it was still taking. The bound itself is enforced by the caller; this is only
    /// the report.
    private func recordTruncatedEventChain(head: Event?) {
        truncatedEventChains += 1
        lastTruncatedEvent = head
    }

    // MARK: - Callbacks

    /// Registers a callback invoked when the engine reaches a final state (§scxml-6.4).
    public func setCompletionCallback(_ callback: (() -> Void)?) {
        completionCallback = callback
    }

    /// Registers an HTTP send dispatcher callback (§scxml-C-2).
    ///
    /// The callback returns the response when real HTTP is used. The engine injects the
    /// response event into the external queue. Return nil for fire-and-forget sends.
    public func setHTTPSendCallback(_ callback: ((HttpSendRequest) -> HttpSendResponse?)?) {
        onHTTPSend = callback
    }

    /// Dispatches a BasicHTTP send through the registered callback (§scxml-C-2).
    ///
    /// The callback is the sole dispatch mechanism. If it returns a response, the
    /// engine injects the response event into the external queue. The engine has no
    /// knowledge of HTTP transport: callers supply the implementation via
    /// `setHTTPSendCallback`.
    public func performHTTPSend(
        target: String,
        eventName: String,
        content: String,
        params: [String: [String]],
        sendID: String
    ) {
        guard let onHTTPSend else {
            return
        }
        let response = onHTTPSend(
            HttpSendRequest(target: target, eventName: eventName, content: content, params: params, sendID: sendID)
        )
        if let response {
            var metadata = EventMetadata.external(sendID: "", origin: "")
            metadata.data = response.eventData
            if let arriving = arrivingEvent(name: response.eventName, metadata: metadata) {
                externalQueue.raise(arriving)
            }
        }
    }

    /// Runs the state machine to completion or timeout (§scxml-6.2).
    ///
    /// Calls `tick()` in a loop until either the final state is reached or `timeout`
    /// elapses. Returns true on completion, false on timeout.
    ///
    /// `pollInterval` is a ceiling on the wait between ticks, not the interval actually
    /// slept: a nearer scheduler deadline (`timeUntilNextScheduled`) shortens it, so a
    /// caller passing an interval coarser than the document's delays no longer steps
    /// over them.
    ///
    /// Matches Rust `Engine::run_until_completion`.
    public func runUntilCompletion(timeout: Duration, pollInterval: Duration) -> Bool {
        // W3C SCXML: if already stopped but reached final state during initialize(),
        // return true
        if !isRunning {
            return isInFinalState
        }

        let start = ContinuousClock.now
        while !isInFinalState {
            if start.duration(to: ContinuousClock.now) > timeout {
                return false
            }
            // The scheduler's own answer wins whenever it is nearer: sleeping past a
            // deadline is what turns a coarse interval into a document that behaves
            // differently, and waking on it costs nothing extra.
            var wait = pollInterval
            if let next = timeUntilNextScheduled(), next < wait {
                wait = next
            }
            let (seconds, attoseconds) = wait.components
            Thread.sleep(forTimeInterval: Double(seconds) + Double(attoseconds) / 1e18)
            tick()
        }
        return true
    }

    // MARK: - Internal: microstep + macrostep implementation

    /// The W3C SCXML Appendix D outer loop (§scxml-D-mainEventLoop), and the only place
    /// the three exported entry points express macrostep semantics.
    ///
    /// Appendix D names the external queue exactly once per iteration and it is after
    /// `invoke(inv)`:
    ///
    ///     while running:
    ///         while running and not macrostepDone:      # eventless + internal only
    ///             ... selectEventlessTransitions() / internalQueue.dequeue() ...
    ///         for state in statesToInvoke.sort(entryOrder):
    ///             for inv in state.invoke.sort(documentOrder):
    ///                 invoke(inv)
    ///         statesToInvoke.clear()
    ///         if not internalQueue.isEmpty(): continue
    ///         externalEvent = externalQueue.dequeue()
    ///
    /// Folding the external drain into the macrostep-completion loop instead is a
    /// different algorithm, not a shorter one. The invoked children do not exist yet
    /// while that drain runs, so everything `<onentry>` queued for this session on the
    /// way in is consumed with no autoforward child to receive it, and there is no later
    /// point at which it is delivered. One external event per iteration for the same
    /// reason: a state entered by event N's transition must have its invokes started
    /// before N+1 comes off the queue.
    ///
    /// Matches Rust `Engine::run_main_event_loop`.
    private func runMainEventLoop() {
        // External events this invocation has taken off the queue, against
        // `maxExternalEventsPerCall`. A local, because the budget is one invocation's:
        // a cut leaves the queue as it is and the next invocation gets a budget of its
        // own (ARCHITECTURE.md "External-Event Budget").
        var taken: UInt32 = 0
        while true {
            // W3C SCXML Appendix D: complete the macrostep on eventless transitions
            // and internal events alone.
            completeMacrostep()

            if !isRunning || isInFinalState {
                // W3C SCXML Appendix D's main event loop ends here, and whatever the
                // host put on the external queue ends with it. That is the clause: a
                // machine that reached a top-level final state has exited the
                // interpreter. Saying nothing about it is not the clause; see
                // `unseenExternalEvents`.
                recordUnseenExternalEvents()
                // §scxml-D-exitInterpreter: the loop is over because the run entered a
                // top-level final, so every state still active is exited now, before
                // the completion callback tells a parent.
                if isInFinalState {
                    exitInterpreter()
                }
                break
            }

            // §scxml-6.4: invokes for states entered during this macrostep.
            if policy.hasInvokeSupport {
                policy.executePendingInvokes(engine: self)
            }

            // W3C SCXML Appendix D: invoking may have raised internal error events (and
            // a child that completed synchronously may already have raised
            // done.invoke); handle them before touching the external queue.
            //
            // Not when this macrostep was already stopped at the ceiling: the queue is
            // non-empty because the drain refused it, so looping back is a spin that
            // takes no microstep, logs nothing, and never ends. Falling through to the
            // external dequeue instead is what keeps a machine inside an endless chain
            // reachable at all: the event that rescues it is on that queue, and the
            // clause's priority would otherwise hold it behind a chain that never ends.
            if !macrostepTruncated && internalQueue.hasEvents {
                continue
            }

            guard let head = externalQueue.front() else {
                break
            }
            // Every macrostep this loop runs ends, so `maxMacrostepMicrosteps` bounds
            // none of this: a machine that sends itself an external event on every one
            // never empties the queue. Past the budget the queue is left exactly as it
            // is and the call hands back, counted.
            if taken >= maxExternalEventsPerCall {
                recordTruncatedEventChain(head: head.event)
                break
            }
            taken += 1
            if !processNextExternalEvent() {
                break
            }
        }
    }

    /// Appendix D's inner loop: take microsteps on eventless transitions and internal
    /// events until nothing is enabled by NULL and the internal queue is empty (the
    /// stable configuration a macrostep ends in) or until the macrostep's budget is
    /// spent.
    ///
    ///     while running and not macrostepDone:
    ///         enabledTransitions = selectEventlessTransitions()
    ///         if enabledTransitions.isEmpty():
    ///             if internalQueue.isEmpty():
    ///                 macrostepDone = true
    ///             else:
    ///                 internalEvent = internalQueue.dequeue()
    ///                 datamodel["_event"] = internalEvent
    ///                 enabledTransitions = selectTransitions(internalEvent)
    ///         if not enabledTransitions.isEmpty():
    ///             microstep(enabledTransitions.toList())
    ///
    /// Eventless transitions are selected again after every microstep, whatever took
    /// it. A targetless transition that only ran content can enable one as surely as a
    /// transition that entered a state, and the internal event queued behind it waits
    /// until that has been asked. So does the turn after an internal event no
    /// transition matched: it took no microstep, but it was bound as `_event`, and an
    /// eventless guard may read `_event`.
    ///
    /// One budget for both branches; see `maxMacrostepMicrosteps`. A `<raise>` answered
    /// by a transition that raises again is a macrostep that never ends, exactly as a
    /// cyclic eventless transition is; until 2026-08-20 the internal branch had no
    /// ceiling in any of the seven engines here, so that document did not return at all.
    ///
    /// Matches Rust `Engine::complete_macrostep`.
    private func completeMacrostep() {
        // §scxml-D-enterStates sets running = false on entering a top-level <final>;
        // this engine keeps `isRunning` for the host's `stop` and asks the
        // configuration for the other half.
        while isRunning && !isInFinalState && !macrostepTruncated {
            let enabled = selectTransitions(policy.nullEvent())
            if !enabled.isEmpty {
                if macrostepMicrostepsTaken == maxMacrostepMicrosteps {
                    // The chain is still going one microstep past the budget, so this
                    // is the case the specification calls a macrostep that does not
                    // terminate. Refuse the microstep and not take it, and publish the
                    // refusal: the configuration left behind is not a stable one and
                    // only the count says so.
                    refuseMacrostep(stoppedIn: currentState)
                    return
                }
                macrostepMicrostepsTaken += 1
                microstep(enabled)
                continue
            }

            if !internalQueue.hasEvents {
                // The queue emptied, so the chain, refused or merely finished, is
                // over. A machine whose next macrostep starts a new one starts it from
                // zero, and the count of what was refused stays where the host reads
                // it.
                errorCascadeDepth = 0
                return
            }
            if macrostepMicrostepsTaken == maxMacrostepMicrosteps {
                // Work is still queued one microstep past the budget. Refuse the
                // microstep and not take it: the event stays on the queue, which is
                // where the next macrostep will find it.
                refuseMacrostep(stoppedIn: currentState)
                return
            }
            guard let eventWithMeta = internalQueue.pop() else {
                return
            }
            takeInternalEvent(eventWithMeta)
        }
    }

    /// Stops the macrostep now in progress at `maxMacrostepMicrosteps`, from whichever
    /// branch of the inner loop found the budget spent, naming `stoppedIn`: the state
    /// the machine is standing in, which is on the walk that cannot end, and not one
    /// the refused microstep would have entered.
    private func refuseMacrostep(stoppedIn: State) {
        recordTruncatedMacrostep(stoppedIn)
        sceLog("Engine.completeMacrostep: macrostep still going after \(maxMacrostepMicrosteps) microsteps; stopped")
    }

    /// Binds one event taken off the internal queue as `_event`, selects, and takes the
    /// microstep it selects (§scxml-D-mainEventLoop).
    private func takeInternalEvent(_ eventWithMeta: EventWithMetadata<Event>) {
        // §scxml-5.10: Populate policy metadata from event
        policy.populateEventMetadata(eventWithMeta.metadata)
        liftPayload(of: eventWithMeta.event, metadata: eventWithMeta.metadata)
        // §scxml-3.12.2: the processor raises error.* into this queue and the clause
        // says they "are ignored if no transition is found that matches them". Ignoring
        // them is the clause; staying silent about it is not. `discardedExternalEvents`
        // deliberately stops at the external queue because an unmatched <raise> has both
        // ends inside the document, but the sender of an error event is this engine, so
        // that reasoning does not reach it. The host never wrote the document, cannot
        // see the failure in the configuration, and is the only party able to act on it.
        //
        // The selection runs first and unconditionally: it is what processes every
        // internal event, and folding it into the condition below would skip it for
        // everything that is not an error.
        //
        // An error raised from here on is raised by an error handler, which is the one
        // situation the engine cannot leave to the document: the handler that failed is
        // the same one that will answer the failure. The flag is what `raise` reads to
        // tell that apart from a first failure, and it is cleared before anything else
        // can run so a chain cannot be attributed to the wrong event.
        let isError = isErrorEvent(policy.getEventName(eventWithMeta.event))
        // The chain is not ended by the drain doing something else. An earlier draft
        // reset the depth on every non-error event, which reads as the careful choice
        // and is the opposite: a handler that raises its own event before failing (a
        // document that logs, then fails, which is most of them) leaves the queue
        // alternating tick, error, tick, error..., and each tick put the ceiling back out
        // of reach. The count needs no such guard, because it only ever rises while an
        // error handler is running.
        handlingErrorEvent = isError
        let outcome = takeEvent(eventWithMeta.event)
        handlingErrorEvent = false
        if outcome == .taken {
            // Appendix D: the loop turn that selects nothing takes no microstep, so it
            // spends no budget. Only a turn that answered the event moved the machine,
            // and only those are what a ceiling on microsteps can be counted in.
            macrostepMicrostepsTaken += 1
        }
        if outcome == .discarded && isError {
            unhandledErrorEvents += 1
            lastUnhandledError = eventWithMeta.event
            sceLog("Engine.takeInternalEvent: error event matched no transition; unhandled")
        }
        policy.clearEventMetadata()
    }

    /// Takes exactly one event off the external queue, runs the preliminary
    /// `<finalize>` and autoforward step against it, then selects transitions. Reports
    /// whether an event was processed.
    ///
    /// One event, not a drain: Appendix D returns to the top of the outer loop after
    /// each external event, so a state entered by this event's transition gets its
    /// invokes started before the next one is dequeued.
    ///
    /// Matches Rust `Engine::process_next_external_event`.
    private func processNextExternalEvent() -> Bool {
        guard var eventWithMeta = externalQueue.pop() else {
            return false
        }
        // Taking an event off the external queue is where a macrostep begins, so it is
        // where the previous one's ceiling stops applying. A machine left inside an
        // endless chain gets a full budget for each event it is given, and each refusal
        // is counted separately, which is what tells a host that spins once from one
        // that spins on everything.
        //
        // Here and not at the entry to the loop above, which reads like the more
        // general boundary and is not one: a machine whose chain was refused would spend
        // a whole budget re-walking it before it ever looked at the event the host sent
        // to get it out. The refused events stay queued either way; this is where the
        // budget that drains them comes back.
        macrostepTruncated = false
        macrostepMicrostepsTaken = 0

        // §scxml-6.5: Execute finalize before parent's own transition matching
        if policy.hasFinalize {
            policy.executeFinalizeForChildEvent(&eventWithMeta, engine: self)
        }
        // W3C SCXML Appendix D mainEventLoop: autoforward belongs to the same
        // preliminary step as <finalize> above: both run against the event this drain
        // has just popped off the external queue, and both run before transition
        // selection. §scxml-6.4.2 fixes the position in prose as well: the parent
        // forwards "at the point at which it removes it from the external event queue".
        //
        // Forwarding where the event is enqueued instead is a different algorithm, not
        // an earlier one. `raiseExternalWithMeta` runs inside whatever executable
        // content produced the event, so a transition body that queues two events hands
        // the child both of them before the parent has processed either: the child runs
        // a whole event ahead and the two sessions stop agreeing on what has happened.
        // Run-to-completion is a property of this loop's shape, so the forward has to
        // live in the loop.
        //
        // §scxml-6.4 mandates an exact copy, so the source event's metadata rides along
        // with the name. The target is not forwarded: it is a routing decision owned by
        // the originating <send>, and inheriting it would re-route the child's copy.
        if policy.hasAutoforward {
            // §scxml-5.10: under the name the event arrived under, which is the value's
            // own unless a door cut a longer one back to it.
            var name = eventWithMeta.metadata.name
            if name.isEmpty {
                name = policy.getEventName(eventWithMeta.event)
            }
            policy.forwardToAutoforwardChildren(eventName: name, metadata: eventWithMeta.metadata, engine: self)
        }
        // §scxml-5.10: Populate policy metadata from event
        policy.populateEventMetadata(eventWithMeta.metadata)
        liftPayload(of: eventWithMeta.event, metadata: eventWithMeta.metadata)
        // §scxml-3.1.2: "If no transition matches in any state, the event is
        // discarded." Discarding it is the rule; being unable to say so is not part of
        // the rule. The host that put this event on the queue is the one party that
        // cannot see the outcome (a discard leaves the configuration exactly as a self
        // transition does) and it is the party that got the event wrong. Recorded for
        // the external queue only: an internal <raise> that matches nothing is the
        // document's own business, and both ends of it are in the document.
        if takeEvent(eventWithMeta.event) == .discarded {
            discardedExternalEvents += 1
            lastDiscardedEvent = eventWithMeta.event
        }
        policy.clearEventMetadata()
        return true
    }

    /// Publishes a macrostep this engine stopped short, from whichever branch of
    /// Appendix D's inner loop ran out of budget.
    ///
    /// One function, two callers, for the reason the budget is one number: a host reads
    /// a macrostep that did not reach a stable configuration, and the branch it died in
    /// is a detail of the document, not of the contract. Two copies of this would be
    /// two chances for one of them to stop setting the flag that keeps the same chain
    /// from being handed a second budget.
    private func recordTruncatedMacrostep(_ state: State) {
        truncatedMacrosteps += 1
        lastTruncatedMacrostepState = state
        macrostepTruncated = true
    }

    // MARK: - Appendix D over the generated policy
    //
    // The policy answers what only the document knows: which of a state's transitions
    // an event enables, what a transition's content is, what a state's onentry and
    // onexit do, what a history recorded. What Appendix D does with those answers is
    // `Microstep.swift`, written once for every machine; `EngineHost` is how this engine
    // hands it the policy.
    //
    // A machine with a <parallel> used to take a second microstep written into its
    // generated code while every other machine took an LCA walk here. Neither was the
    // appendix's, and they disagreed about eventless selection (one never looked above
    // the atomic state, the other looked at every active state without walking), about
    // the order transition content runs in, about which states a transition to an
    // ancestor exits, and about how many targets a transition has: both kept only the
    // first.

    /// Appendix D's `selectTransitions`, or its `selectEventlessTransitions` for the
    /// null event: the optimal enabled transition set, in selection order.
    private func selectTransitions(_ event: Event) -> [EnabledTransition<State>] {
        // §scxml-5.10: the event whose transitions are about to be selected is the
        // _event their guards read: bound before the first guard runs.
        policy.bindCurrentEvent(event, engine: self)
        return SceRuntime.selectTransitions(EngineHost(engine: self), event: event)
    }

    /// Appendix D's `microstep`: exit, run the transitions' content, enter.
    private func microstep(_ transitions: [EnabledTransition<State>]) {
        let entry = SceRuntime.microstep(EngineHost(engine: self), transitions: transitions)
        settleCurrentState(targets: transitions.map(\.targets), entered: entry.statesToEnter)
    }

    /// Appendix D's `enterStates`: for the document's initial transition, the whole of
    /// §scxml-D-interpret's entry into the initial configuration.
    private func enterStates(_ transitions: [EntryTransition<State>]) {
        let entry = SceRuntime.enterStates(EngineHost(engine: self), transitions: transitions)
        settleCurrentState(targets: transitions.map(\.targets), entered: entry.statesToEnter)
    }

    /// Offers one event to the configuration and takes the microstep it selects.
    private func takeEvent(_ event: Event) -> EventOutcome {
        let enabled = selectTransitions(event)
        if enabled.isEmpty {
            return .discarded
        }
        microstep(enabled)
        return .taken
    }

    /// Points `currentState` at the atomic state the last transition brought the
    /// machine into.
    ///
    /// Without a `<parallel>` the configuration is one chain and the entry set ends on
    /// its atomic state. With one, `currentState` is the first state the last targeted
    /// transition names, dereferenced, then down through the configuration to an atomic
    /// state: the answer every backend gives. A microstep of targetless transitions
    /// entered nothing and moves nothing.
    ///
    /// `targets` are the target lists as written, in selection order: the microstep's
    /// enabled set, or the initial transition.
    private func settleCurrentState(targets: [[EntryTarget<State>]], entered: [State]) {
        guard let last = entered.last else {
            return
        }
        if !policy.hasParallelStates {
            currentState = last
            return
        }
        for list in targets.reversed() {
            let effective = effectiveTargetStates(PolicyDocument(policy), targets: list)
            if let first = effective.first {
                currentState = first
                resolveCurrentStateToLeaf()
                return
            }
        }
    }

    /// Settles `currentState` on an atomic state.
    ///
    /// A transition may target a compound state, or a `<parallel>`, and `getCurrentState`
    /// must still name the atomic state the machine is in and not one it is within.
    ///
    /// The descent reads the configuration and does not recompute initial or history
    /// children: the configuration is what the microstep actually entered, so a descent
    /// through it cannot disagree with what happened. Under a `<parallel>` it takes the
    /// first region in document order, so the answer does not depend on the order states
    /// happened to become active.
    private func resolveCurrentStateToLeaf() {
        let active = getActiveStates()
        // A region holds one atomic state, so this descends once per level; the bound is
        // the parent walk's own, so a parent chain that loops is reported and not walked
        // forever.
        for _ in 0..<maxHierarchyDepth {
            if !policy.isCompoundState(currentState) && !policy.isParallelState(currentState) {
                return
            }
            // A compound state with no active child is not a configuration this can
            // repair, so it is left as it is and not guessed at.
            guard let child = policy.getChildStates(currentState).first(where: { active.contains($0) }) else {
                return
            }
            currentState = child
        }
        sceLog(
            "Engine.resolveCurrentStateToLeaf: exceeded \(maxHierarchyDepth) descents from a compound state -- "
                + "the parent chain does not terminate"
        )
    }
}

/// An engine as `Microstep.swift` reads it: that file's `Document` and `Run`
/// protocols state what each member answers.
///
/// The document is the policy's static tables; the history values and the
/// configuration are run-time state, which is why this holds the engine.
private final class EngineHost<Policy: StatePolicy>: Run {
    typealias State = Policy.State
    typealias Event = Policy.Event

    unowned let engine: Engine<Policy>
    let document: PolicyDocument<Policy>

    init(engine: Engine<Policy>) {
        self.engine = engine
        self.document = PolicyDocument(engine.policy)
    }

    func parentOf(_ state: State) -> State? { document.parentOf(state) }

    func isCompound(_ state: State) -> Bool { document.isCompound(state) }

    func isParallel(_ state: State) -> Bool { document.isParallel(state) }

    func isFinal(_ state: State) -> Bool { document.isFinal(state) }

    func childStates(_ state: State) -> [State] { document.childStates(state) }

    func initialTargets(_ state: State) -> [EntryTarget<State>] { document.initialTargets(state) }

    func historyParent(_ history: HistoryID) -> State { document.historyParent(history) }

    func historyValue(_ history: HistoryID) -> [State]? { document.historyValue(history) }

    func historyDefaultTargets(_ history: HistoryID) -> [EntryTarget<State>] {
        document.historyDefaultTargets(history)
    }

    func documentOrder(_ state: State) -> Int { document.documentOrder(state) }

    /// A copy, because the procedures hold it across the exits and entries that change
    /// the policy's own set.
    func configuration() -> [State] {
        engine.getActiveStates()
    }

    func firstEnabledTransition(_ state: State, event: Event) -> EnabledTransition<State>? {
        engine.policy.firstEnabledTransition(state, event: event, engine: engine)
    }

    func exitState(_ state: State, configurationBeforeExit: [State]) {
        engine.policy.executeExitActions(state, engine: engine, configurationBeforeExit: configurationBeforeExit)
    }

    func executeTransitionContent(_ transition: EnabledTransition<State>) {
        engine.policy.executeTransitionContent(
            source: transition.source,
            transitionIndex: transition.transitionIndex,
            engine: engine
        )
    }

    func enterState(_ state: State, isDefaultEntry: Bool) {
        engine.policy.executeEntryActions(state, engine: engine, isDefaultEntry: isDefaultEntry)
        // §scxml-D-enterStates: a top-level <final> ends the run, and entering one is
        // where the run's terminal state is recorded.
        engine.recordTerminalState(state)
    }

    func executeHistoryDefaultContent(_ history: HistoryID) {
        engine.policy.executeHistoryDefaultContent(history, engine: engine)
    }
}
