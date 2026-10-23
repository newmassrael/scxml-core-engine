// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D's microstep: which transitions an event selects, which of
// them survive preemption, which states they exit and in which order, the order
// their content runs in, which states they enter. Written once for every machine
// this runtime runs.
//
// The procedures are transcribed and not paraphrased, one function per procedure
// and under the appendix's names, so a reader can hold this file against the
// specification line by line, and against the other engines' transcriptions of
// it: the C++ engines' `sce/include/core/MicrostepAlgorithms.h`, the Rust
// runtime's `backends/rust/runtime/src/helpers/microstep.rs`, the Go runtime's
// `microstep.go` and the Python runtime's `sce_runtime/microstep.py`, which this
// file follows function for function.
//
// What differs between machines is injected. A `Document` describes the
// document: its structure, and what each `<history>` recorded. A `Run` is the
// running machine: its configuration, which of a state's transitions an event
// enables, and what exiting a state, running a transition's content and entering
// a state do. Nothing here knows how a machine stores either, so a table written
// by hand answers them as well as a generated policy does.
//
// A document whose `<history>` defaults name one another in a cycle has no entry
// set (dereferencing never reaches a state), and a hierarchy whose parent links
// cycle has no domain; both are generator defects. The procedures assume a legal
// document, and the parent walks below trap and do not spin when handed one that
// is not.

/// The maximum supported state hierarchy depth. It bounds every walk up the
/// parent links here, so a cyclic parent relationship (a generator defect) is
/// reported and not walked forever. Matches Rust `MAX_HIERARCHY_DEPTH` = 16.
///
/// W3C SCXML has no normative depth limit; 16 covers every real-world document
/// (typical: 1-5, complex: up to 10).
public let maxHierarchyDepth = 16

/// Names one `<history>` pseudo-state of a generated document.
///
/// A history is not a state (it is never in a configuration), so a generated
/// machine numbers its histories apart from its states, and a target list names
/// one through `EntryTarget.history`. The number means nothing outside the
/// policy that issued it.
public struct HistoryID: Hashable, Sendable, CustomStringConvertible {
    public var rawValue: Int

    public init(_ rawValue: Int) {
        self.rawValue = rawValue
    }

    public var description: String {
        String(rawValue)
    }
}

/// One token of a target list, as the document wrote it.
///
/// A transition's target, a state's initial transition and a `<history>`'s
/// default transition all name either states or `<history>` pseudo-states. The
/// two kinds are kept apart and not folded into one identifier: the entry
/// procedures dereference a history (to what it recorded, or to its default) and
/// add a state.
public enum EntryTarget<State: Hashable>: Hashable, CustomStringConvertible {
    /// A `<state>`, `<parallel>` or `<final>`.
    case state(State)

    /// A `<history>` pseudo-state.
    case history(HistoryID)

    /// The state this token names; nil when it names a history.
    public var asState: State? {
        if case .state(let state) = self {
            return state
        }
        return nil
    }

    /// The history this token names; nil when it names a state.
    public var asHistory: HistoryID? {
        if case .history(let history) = self {
            return history
        }
        return nil
    }

    /// The token spelt for diagnostics.
    public var description: String {
        switch self {
        case .state(let state): return "state(\(state))"
        case .history(let history): return "history(\(history))"
        }
    }
}

extension EntryTarget: Sendable where State: Sendable {}

/// A transition selection enabled: one member of Appendix D's
/// `enabledTransitions`.
///
/// What the microstep reads of it (its source, its target list as written,
/// whether it is internal), plus the index its source state knows it by, so the
/// machine that owns its executable content can run it. A transition with no
/// targets exits and enters nothing and only runs its content.
///
/// `targets` is the document's own table: a generated machine hands out arrays of
/// its tables, because a transition is a fact about the document and not about the
/// run. Nothing here writes to it.
public struct EnabledTransition<State: Hashable> {
    /// The state whose transition this is.
    public var source: State

    /// The target list as written; empty for a targetless transition.
    public var targets: [EntryTarget<State>]

    /// The position of this transition among its source's own transitions.
    public var transitionIndex: Int

    /// Whether the transition has executable content to run.
    public var hasActions: Bool

    /// Whether the transition was written `type="internal"`.
    public var isInternal: Bool

    public init(
        source: State,
        targets: [EntryTarget<State>],
        transitionIndex: Int,
        hasActions: Bool,
        isInternal: Bool
    ) {
        self.source = source
        self.targets = targets
        self.transitionIndex = transitionIndex
        self.hasActions = hasActions
        self.isInternal = isInternal
    }

    /// Whether the transition has no targets. §scxml-D-computeExitSet guards the
    /// whole computation with `if t.target`: a transition without targets exits
    /// and enters nothing.
    public var isTargetless: Bool {
        targets.isEmpty
    }

    /// The same transition as the entry procedures read it.
    public var entryTransition: EntryTransition<State> {
        EntryTransition(source: source, targets: targets, isInternal: isInternal)
    }
}

extension EnabledTransition: Sendable where State: Sendable {}

/// The part of a transition the entry procedures read.
///
/// `source` is nil for the document's own initial transition, whose source is the
/// `<scxml>` element, which has no state identifier here and whose domain is the
/// whole document.
public struct EntryTransition<State: Hashable> {
    /// The transition's source state; nil when the source is the `<scxml>` element.
    public var source: State?

    /// The target list as written.
    public var targets: [EntryTarget<State>]

    /// Whether the transition was written `type="internal"`.
    public var isInternal: Bool

    public init(source: State?, targets: [EntryTarget<State>], isInternal: Bool = false) {
        self.source = source
        self.targets = targets
        self.isInternal = isInternal
    }
}

extension EntryTransition: Sendable where State: Sendable {}

/// What a microstep enters: the appendix's three out-parameters.
///
/// `statesToEnter` is already sorted into entry order, so a machine enters it
/// front to back. `statesForDefaultEntry` answers whether a state's initial
/// transition content runs; `defaultHistoryContent` whether, and whose,
/// `<history>` default transition content runs after that state's own entry,
/// keyed by the history's parent as the appendix keys it.
public struct EntrySet<State: Hashable> {
    /// The states to enter, in entry order.
    public var statesToEnter: [State] = []

    /// The compound states whose initial state is entered by default.
    public var statesForDefaultEntry: [State] = []

    /// Maps a history's parent to the history whose default content runs when
    /// that parent is entered.
    public var defaultHistoryContent: [State: HistoryID] = [:]

    public init() {}

    /// Whether this microstep enters `state`.
    public func contains(_ state: State) -> Bool {
        statesToEnter.contains(state)
    }

    /// Whether `state`'s initial state is entered by default, so its initial
    /// transition's executable content runs.
    public func isDefaultEntry(_ state: State) -> Bool {
        statesForDefaultEntry.contains(state)
    }

    /// The `<history>` whose default transition content runs after `state` is
    /// entered, if a history of `state` was taken with nothing recorded.
    public func defaultHistoryContentOf(_ state: State) -> HistoryID? {
        defaultHistoryContent[state]
    }
}

extension EntrySet: Sendable where State: Sendable {}

/// The document, as the entry and exit procedures read it.
///
/// Structure only, plus the one piece of run-time state the entry procedures need:
/// what each `<history>` recorded.
public protocol Document {
    associatedtype State: Hashable

    /// The state's parent; nil when its parent is the `<scxml>` element.
    func parentOf(_ state: State) -> State?

    /// Whether the state is a `<state>` with child states. A `<parallel>` answers
    /// false: this is the appendix's `isCompoundState`, and with the `<scxml>`
    /// element it is the set `findLCCA` chooses a domain from.
    func isCompound(_ state: State) -> Bool

    /// Whether the state is a `<parallel>`.
    func isParallel(_ state: State) -> Bool

    /// Whether the state is a `<final>` element; not whether it is in a final
    /// state, which is `isInFinalState`.
    func isFinal(_ state: State) -> Bool

    /// §scxml-D-getChildStates: the state's `<state>`, `<parallel>` and `<final>`
    /// children, in document order.
    func childStates(_ state: State) -> [State]

    /// A compound state's initial transition target, as written; the first child
    /// state when the document names none.
    func initialTargets(_ state: State) -> [EntryTarget<State>]

    /// The state a `<history>` is declared in.
    func historyParent(_ history: HistoryID) -> State

    /// What the history recorded when its parent was last exited; nil before that
    /// ever happened.
    func historyValue(_ history: HistoryID) -> [State]?

    /// A `<history>`'s default transition target, as written.
    func historyDefaultTargets(_ history: HistoryID) -> [EntryTarget<State>]

    /// The state's position in document order, which is also entry order.
    func documentOrder(_ state: State) -> Int
}

/// The running machine, as the microstep drives it.
public protocol Run: AnyObject, Document {
    associatedtype Event

    /// The active states, in any order. The procedures hold the answer across the
    /// exits and entries they drive, so it must not alias storage those mutate.
    func configuration() -> [State]

    /// This state's first transition, in document order, that the event enables
    /// and whose guard holds; for the machine's "no event", its first eventless
    /// transition whose guard holds. The only place a guard is evaluated.
    func firstEnabledTransition(_ state: State, event: Event) -> EnabledTransition<State>?

    /// Records this state's histories from the configuration as it stood before
    /// the microstep's first exit, runs its onexit, cancels its invocations and
    /// removes it from the configuration.
    func exitState(_ state: State, configurationBeforeExit: [State])

    /// Runs one transition's executable content.
    func executeTransitionContent(_ transition: EnabledTransition<State>)

    /// Adds the state to the configuration and schedules its invocations, runs its
    /// onentry, then its initial transition's content when `isDefaultEntry`; for a
    /// `<final>`, what the appendix does on entering one.
    func enterState(_ state: State, isDefaultEntry: Bool)

    /// Runs a `<history>`'s default transition content.
    func executeHistoryDefaultContent(_ history: HistoryID)
}

// MARK: - Selection

/// Appendix D's `selectTransitions`, or its `selectEventlessTransitions` when
/// `event` is the machine's "no event".
///
/// Returns the optimal enabled transition set, in selection order.
public func selectTransitions<R: Run>(_ run: R, event: R.Event) -> [EnabledTransition<R.State>] {
    let configuration = run.configuration()

    let atomicStates = stableSorted(
        configuration.filter { !run.isCompound($0) && !run.isParallel($0) },
        by: { run.documentOrder($0) }
    )

    var enabled: [EnabledTransition<R.State>] = []
    for atomic in atomicStates {
        // §scxml-D-selectTransitions: the atomic state first, then its proper
        // ancestors, and the first enabled transition in document order ends the
        // walk for this atomic state. The set is ordered and a set: two atomic
        // states under one ancestor both reach its transition, and it is one
        // transition, taken once. With the machine's "no event" this is
        // §scxml-D-selectEventlessTransitions, the same walk over transitions
        // that have no event.
        var current: R.State? = atomic
        var depth = 0
        while let state = current {
            if depth == maxHierarchyDepth {
                preconditionFailure("selectTransitions: cyclic parent relationship detected walking from \(atomic)")
            }
            if let found = run.firstEnabledTransition(state, event: event) {
                let seen = enabled.contains {
                    $0.source == found.source && $0.transitionIndex == found.transitionIndex
                }
                if !seen {
                    enabled.append(found)
                }
                break
            }
            current = run.parentOf(state)
            depth += 1
        }
    }
    return removeConflictingTransitions(run, enabled: enabled, configuration: configuration)
}

/// Appendix D's `removeConflictingTransitions`.
///
/// Two transitions conflict when their exit sets intersect, and the one whose
/// source is a descendant of the other's wins; otherwise the one selected first
/// does.
public func removeConflictingTransitions<D: Document>(
    _ doc: D,
    enabled: [EnabledTransition<D.State>],
    configuration: [D.State]
) -> [EnabledTransition<D.State>] {
    var filtered: [EnabledTransition<D.State>] = []
    for t1 in enabled {
        let t1Exit = computeExitSet(doc, transition: t1, configuration: configuration)
        let conflicts = { (t2: EnabledTransition<D.State>) -> Bool in
            intersects(t1Exit, computeExitSet(doc, transition: t2, configuration: configuration))
        }

        // §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
        // transition it conflicts with whose source it does not descend from. A
        // transition that exits nothing (a targetless one) conflicts with nothing
        // and can never be preempted.
        let preempted = filtered.contains { t2 in
            conflicts(t2) && !isDescendant(doc, t1.source, of: t2.source)
        }
        if preempted {
            continue
        }
        // Not preempted means every kept transition t1 conflicts with has a source
        // t1 descends from: the appendix removes exactly those.
        filtered = filtered.filter { !conflicts($0) } + [t1]
    }
    return filtered
}

// MARK: - Domains and exit sets

/// Appendix D's `getEffectiveTargetStates`, over a target list.
///
/// The states the list stands for, histories dereferenced (to what each recorded
/// or, before its parent was ever exited, to its default), each state once and in
/// the order first named. The domain, and so the exit set, is a question about
/// these.
public func effectiveTargetStates<D: Document>(_ doc: D, targets: [EntryTarget<D.State>]) -> [D.State] {
    var effective: [D.State] = []
    addEffectiveTargetStates(doc, targets: targets, effective: &effective)
    return effective
}

private func addEffectiveTargetStates<D: Document>(
    _ doc: D,
    targets: [EntryTarget<D.State>],
    effective: inout [D.State]
) {
    for target in targets {
        switch target {
        case .state(let state):
            addOnce(&effective, state)
        case .history(let history):
            // §scxml-D-getEffectiveTargetStates: a history stands for the
            // configuration it recorded, and before its parent was ever exited for
            // the targets of its default transition, which may name histories
            // themselves, hence the recursion.
            if let recorded = doc.historyValue(history), !recorded.isEmpty {
                for state in recorded {
                    addOnce(&effective, state)
                }
            } else {
                addEffectiveTargetStates(doc, targets: doc.historyDefaultTargets(history), effective: &effective)
            }
        }
    }
}

/// Appendix D's `getTransitionDomain`, for a transition that has targets.
///
/// `effectiveTargets` are the transition's effective targets: a history that
/// recorded a state deep inside its parent is a target that deep. The domain; nil
/// when it is the `<scxml>` element, which it always is for the document's initial
/// transition.
public func transitionDomain<D: Document>(
    _ doc: D,
    transition: EntryTransition<D.State>,
    effectiveTargets: [D.State]
) -> D.State? {
    guard let source = transition.source else {
        return nil
    }

    // §scxml-D-getTransitionDomain: an internal transition whose targets all lie
    // below a compound source has the source as its domain, so the source stays
    // active and only its active descendants are exited. That is not the same as
    // exiting nothing: a transition rooted at one of those descendants exits it
    // too, and the appendix expects the two to be found in conflict.
    if transition.isInternal && doc.isCompound(source) && allDescend(doc, effectiveTargets, of: source) {
        return source
    }
    return findLCCA(doc, head: source, tail: effectiveTargets)
}

/// Appendix D's `findLCCA` over `[head] + tail`: the first proper ancestor of
/// `head` that is a compound state, or the `<scxml>` element (nil), and contains
/// every state of `tail`.
///
/// Asked of the source and every target at once: combining pairwise answers can
/// only widen the domain.
private func findLCCA<D: Document>(_ doc: D, head: D.State, tail: [D.State]) -> D.State? {
    var current = head
    for _ in 0..<maxHierarchyDepth {
        guard let ancestor = doc.parentOf(current) else {
            return nil
        }
        // §scxml-D-findLCCA filters the ancestors with
        // isCompoundStateOrScxmlElement: a <parallel> is never a domain.
        if doc.isCompound(ancestor) && allDescend(doc, tail, of: ancestor) {
            return ancestor
        }
        current = ancestor
    }
    preconditionFailure("findLCCA: cyclic parent relationship detected walking from \(head)")
}

/// Appendix D's `computeExitSet`, for one transition: the active proper
/// descendants of its domain, in the configuration's own order.
public func computeExitSet<D: Document>(
    _ doc: D,
    transition: EnabledTransition<D.State>,
    configuration: [D.State]
) -> [D.State] {
    // §scxml-D-computeExitSet: the appendix guards the whole computation with
    // `if t.target`, so a transition without one exits nothing at all and can
    // therefore never be preempted.
    if transition.isTargetless {
        return []
    }

    let targets = effectiveTargetStates(doc, targets: transition.targets)
    let domain = transitionDomain(doc, transition: transition.entryTransition, effectiveTargets: targets)
    return configuration.filter { state in
        // The domain itself is not exited; everything active below it is. When the
        // domain is the <scxml> element every active state is a descendant of it,
        // the sibling regions of an enclosing <parallel> included.
        guard let domain else {
            return true
        }
        return isDescendant(doc, state, of: domain)
    }
}

/// Appendix D's `computeExitSet` over the microstep's transitions, in `exitOrder`:
/// the states `exitStates` exits.
public func computeStatesToExit<D: Document>(
    _ doc: D,
    transitions: [EnabledTransition<D.State>],
    configuration: [D.State]
) -> [D.State] {
    var statesToExit: [D.State] = []
    // §scxml-D-computeExitSet takes the microstep's whole transition list and
    // unions the exit sets; §scxml-D-exitStates exits that union in exitOrder:
    // descendants before their ancestors and reverse document order among the rest,
    // which together are exactly reverse document order.
    for transition in transitions {
        for state in computeExitSet(doc, transition: transition, configuration: configuration) {
            addOnce(&statesToExit, state)
        }
    }
    return stableSorted(statesToExit, by: { -doc.documentOrder($0) })
}

// MARK: - The microstep

/// Appendix D's `microstep`: exit, run the transitions' content, enter.
///
/// Returns what was entered: the entry set, `statesToEnter` in entry order.
@discardableResult
public func microstep<R: Run>(_ run: R, transitions: [EnabledTransition<R.State>]) -> EntrySet<R.State> {
    // §scxml-D-microstepProcedure: every exit, then every transition's content,
    // then every entry, for the whole set at once, which is what lets the regions
    // of a <parallel> each take their own transition in one step.
    exitStates(run, transitions: transitions)
    executeTransitionContent(run, transitions: transitions)
    return enterStates(run, transitions: transitions.map(\.entryTransition))
}

/// Appendix D's `exitStates`.
public func exitStates<R: Run>(_ run: R, transitions: [EnabledTransition<R.State>]) {
    // §scxml-D-exitStates: the union of the transitions' exit sets, exited in
    // exitOrder. Every history is recorded from the configuration as it stood
    // before the first exit, which is the snapshot each exit is handed.
    let configuration = run.configuration()
    for state in computeStatesToExit(run, transitions: transitions, configuration: configuration) {
        run.exitState(state, configurationBeforeExit: configuration)
    }
}

/// Appendix D's `executeTransitionContent`.
public func executeTransitionContent<R: Run>(_ run: R, transitions: [EnabledTransition<R.State>]) {
    // §scxml-D-executeTransitionContent: in the order the transitions were
    // selected, which is not their sources' document order once an ancestor's
    // transition is reached from a later region.
    for transition in transitions where transition.hasActions {
        run.executeTransitionContent(transition)
    }
}

/// Appendix D's `enterStates`.
///
/// Also the whole of the appendix's entry into the initial configuration
/// (§scxml-D-interpret): hand it the document's initial transition, whose source
/// is the `<scxml>` element (a nil `source`).
@discardableResult
public func enterStates<R: Run>(_ run: R, transitions: [EntryTransition<R.State>]) -> EntrySet<R.State> {
    let entry = computeEntrySet(run, transitions: transitions)
    for state in entry.statesToEnter {
        // §scxml-D-enterStates: onentry, then the initial transition's content if
        // and only if this state's initial state is being entered by default, then
        // a history's default content owed to it.
        run.enterState(state, isDefaultEntry: entry.isDefaultEntry(state))
        if let history = entry.defaultHistoryContentOf(state) {
            run.executeHistoryDefaultContent(history)
        }
    }
    return entry
}

// MARK: - The entry set

/// Appendix D's `computeEntrySet`.
///
/// `transitions` are the microstep's, in the order they were selected; a
/// transition without targets enters nothing. Returns the entry set, with
/// `statesToEnter` sorted into entry order.
public func computeEntrySet<D: Document>(_ doc: D, transitions: [EntryTransition<D.State>]) -> EntrySet<D.State> {
    var entry = EntrySet<D.State>()
    for transition in transitions {
        // §scxml-D-computeEntrySet: first every target with its default
        // descendants, then the ancestors that are entered inside the domain;
        // ancestors outside it were never exited. The order matters for a target
        // set: the regions of a <parallel> that another target already descends
        // into must be seen as taken before the ancestor walk fills the rest with
        // defaults.
        for target in transition.targets {
            addDescendantStatesToEnter(doc, target: target, entry: &entry)
        }
        let effective = effectiveTargetStates(doc, targets: transition.targets)
        if effective.isEmpty {
            continue
        }
        let domain = transitionDomain(doc, transition: transition, effectiveTargets: effective)
        for state in effective {
            addAncestorStatesToEnter(doc, target: .state(state), ancestor: domain, entry: &entry)
        }
    }

    // §scxml-D-enterStates: states are entered in entryOrder, ancestors before
    // descendants and document order between the rest, which is exactly document
    // order, because that is a pre-order walk.
    entry.statesToEnter = stableSorted(entry.statesToEnter, by: { doc.documentOrder($0) })
    return entry
}

/// Appendix D's `addDescendantStatesToEnter`.
///
/// Adds `target` and every descendant entering it enters: a history's recorded or
/// default configuration, a compound state's initial state(s), every region of a
/// `<parallel>` not already on the set.
public func addDescendantStatesToEnter<D: Document>(
    _ doc: D,
    target: EntryTarget<D.State>,
    entry: inout EntrySet<D.State>
) {
    switch target {
    case .history(let history):
        let parent = doc.historyParent(history)
        // §scxml-3.10: a transition to a history behaves as a transition to the
        // configuration it stored, or, before its parent was ever visited, to its
        // default stored configuration.
        if let recorded = doc.historyValue(history), !recorded.isEmpty {
            // §scxml-D-addDescendantStatesToEnter: a history that has recorded a
            // configuration enters it, and the ancestors between it and the
            // history's parent; a deep history records atomic states, which need
            // not be children.
            for state in recorded {
                addDescendantStatesToEnter(doc, target: .state(state), entry: &entry)
            }
            for state in recorded {
                addAncestorStatesToEnter(doc, target: .state(state), ancestor: parent, entry: &entry)
            }
            return
        }
        // §scxml-D-addDescendantStatesToEnter: nothing recorded yet, so the default
        // transition is taken, and its content is owed once the parent has been
        // entered, keyed by the parent, a later history of the same parent
        // replacing it.
        entry.defaultHistoryContent[parent] = history
        let defaults = doc.historyDefaultTargets(history)
        for target in defaults {
            addDescendantStatesToEnter(doc, target: target, entry: &entry)
        }
        for target in defaults {
            addAncestorStatesToEnter(doc, target: target, ancestor: parent, entry: &entry)
        }

    case .state(let state):
        addOnce(&entry.statesToEnter, state)
        if doc.isCompound(state) {
            // §scxml-D-addDescendantStatesToEnter: a compound state entered as a
            // target is entered by default, the one condition under which its
            // initial transition's content runs. Its initial transition names the
            // child or children it enters (§scxml-3.3), possibly several and
            // possibly deep (§scxml-3.6).
            addOnce(&entry.statesForDefaultEntry, state)
            let initial = doc.initialTargets(state)
            for child in initial {
                addDescendantStatesToEnter(doc, target: child, entry: &entry)
            }
            for child in initial {
                addAncestorStatesToEnter(doc, target: child, ancestor: state, entry: &entry)
            }
        } else if doc.isParallel(state) {
            addRegionDefaults(doc, parallel: state, entry: &entry)
        }
    }
}

/// Appendix D's `addAncestorStatesToEnter`.
///
/// Adds the proper ancestors of `target` up to, not including, `ancestor` (nil: up
/// to the `<scxml>` element), filling in the regions of every `<parallel>` among
/// them.
public func addAncestorStatesToEnter<D: Document>(
    _ doc: D,
    target: EntryTarget<D.State>,
    ancestor: D.State?,
    entry: inout EntrySet<D.State>
) {
    for anc in properAncestors(doc, target: target, ancestor: ancestor) {
        // §scxml-D-addAncestorStatesToEnter: an ancestor is entered without its
        // default initial state, since the set already holds the descendant it
        // leads to. A <parallel> still gives its other regions their defaults,
        // because all of them are entered.
        addOnce(&entry.statesToEnter, anc)
        if doc.isParallel(anc) {
            addRegionDefaults(doc, parallel: anc, entry: &entry)
        }
    }
}

/// Appendix D's `getProperAncestors`: the ancestors of `target` in ancestry order
/// up to, not including, `ancestor`, and nothing at all when `ancestor` is not
/// above it.
///
/// A `<history>` sits in its parent, so the parent is its first proper ancestor. A
/// nil `ancestor` is the `<scxml>` element, which is above every state and is
/// never entered, so it is not in the list.
private func properAncestors<D: Document>(
    _ doc: D,
    target: EntryTarget<D.State>,
    ancestor: D.State?
) -> [D.State] {
    var chain: [D.State] = []
    var current: D.State?
    switch target {
    case .history(let history):
        current = doc.historyParent(history)
    case .state(let state):
        current = doc.parentOf(state)
    }
    for _ in 0..<maxHierarchyDepth {
        guard let at = current else {
            // §scxml-D-getProperAncestors: `ancestor` is the state's parent, the
            // state itself, or one of its descendants, none of which the walk can
            // meet, so the answer is the empty set.
            return ancestor == nil ? chain : []
        }
        if let ancestor, at == ancestor {
            return chain
        }
        chain.append(at)
        current = doc.parentOf(at)
    }
    preconditionFailure("properAncestors: cyclic parent relationship detected walking from \(target)")
}

private func addRegionDefaults<D: Document>(_ doc: D, parallel: D.State, entry: inout EntrySet<D.State>) {
    // §scxml-3.4: every child of an active <parallel> is active, so a region
    // nothing on the set descends into is entered by default.
    for child in doc.childStates(parallel) {
        let taken = entry.statesToEnter.contains { isDescendant(doc, $0, of: child) }
        if !taken {
            addDescendantStatesToEnter(doc, target: .state(child), entry: &entry)
        }
    }
}

// MARK: - History

/// Appendix D's `exitStates`, its history half: what a `<history>` of `parent`
/// records as `parent` is exited. For a deep history the active atomic states
/// below `parent`, for a shallow one its active children.
///
/// Read off `configurationBeforeExit`, the configuration as it stood before the
/// microstep's first exit: the appendix records every history before it exits any
/// state, so every history of one microstep reads the same configuration. Returned
/// in that configuration's order.
public func recordedHistory<D: Document>(
    _ doc: D,
    parent: D.State,
    deep: Bool,
    configurationBeforeExit: [D.State]
) -> [D.State] {
    configurationBeforeExit.filter { state in
        if deep {
            // §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`.
            return !doc.isCompound(state) && !doc.isParallel(state) && isDescendant(doc, state, of: parent)
        }
        // §scxml-D-exitStates: `s0.parent == s`.
        return doc.parentOf(state) == parent
    }
}

// MARK: - Predicates

/// Appendix D's `isInFinalState`: a compound state is in a final state when one of
/// its `<final>` children is active; a `<parallel>` when every one of its child
/// states is, asked recursively, so a region that is itself a `<parallel>` counts
/// only once all of its regions do. Nothing else ever is.
public func isInFinalState<D: Document>(_ doc: D, state: D.State, configuration: [D.State]) -> Bool {
    if doc.isCompound(state) {
        return doc.childStates(state).contains { doc.isFinal($0) && configuration.contains($0) }
    }
    if doc.isParallel(state) {
        return doc.childStates(state).allSatisfy { isInFinalState(doc, state: $0, configuration: configuration) }
    }
    return false
}

/// Appendix D's `isDescendant`: whether `state` lies strictly below `ancestor`.
public func isDescendant<D: Document>(_ doc: D, _ state: D.State, of ancestor: D.State) -> Bool {
    var current = state
    for _ in 0..<maxHierarchyDepth {
        guard let parent = doc.parentOf(current) else {
            return false
        }
        if parent == ancestor {
            return true
        }
        current = parent
    }
    preconditionFailure("isDescendant: cyclic parent relationship detected walking from \(state)")
}

private func allDescend<D: Document>(_ doc: D, _ states: [D.State], of ancestor: D.State) -> Bool {
    states.allSatisfy { isDescendant(doc, $0, of: ancestor) }
}

private func intersects<State: Hashable>(_ a: [State], _ b: [State]) -> Bool {
    a.contains { b.contains($0) }
}

private func addOnce<State: Hashable>(_ set: inout [State], _ state: State) {
    if !set.contains(state) {
        set.append(state)
    }
}

/// `values` sorted ascending by `key`, equal keys keeping the order they had.
///
/// The microstep's orders are defined with ties kept in selection order, so the
/// sort is made stable here and not left to the standard library's current
/// implementation, which does not promise it.
private func stableSorted<Element>(_ values: [Element], by key: (Element) -> Int) -> [Element] {
    values.enumerated()
        .map { (offset: $0.offset, key: key($0.element), element: $0.element) }
        .sorted { ($0.key, $0.offset) < ($1.key, $1.offset) }
        .map(\.element)
}
