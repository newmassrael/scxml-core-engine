// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Is a set of states a CONFIGURATION of the document, and is `current` its current
// state? The clauses that decide are cited on `validateConfiguration`, which is
// where they are answered.
//
// `Engine.getActiveStates` publishes the configuration a machine is in and
// `Engine.enterAt` is the door that takes one back. This file owns the question
// between them, and it is the only place that asks it.
//
// # Why this rejects instead of trapping
//
// Every other input to this engine is authored: the generator wrote the policy, and
// a hierarchy that does not walk is a generator defect. A restored configuration is
// the single exception: it arrives from outside the process, read back by a host from
// wherever it persisted it, recorded against a document revision that may since have
// moved. A host holding a stale record has to be able to handle that answer, so this
// returns a reason and the engine hands it on.
//
// The Swift twin of the Rust runtime's `helpers::configuration::validate`, of the Go
// `ValidateConfiguration` and of the C++ `SCE::Core::validateConfiguration`, asking
// the same questions of the same static hierarchy, so a configuration one engine
// accepts is one the others accept.

/// Why a configuration was refused; `accepted` is the accepting answer.
///
/// An enumerated reason and not a bool: a host handing back a journal it wrote itself
/// needs to know which rule its record broke, and "invalid" sends it looking at the
/// wrong half.
public enum ConfigurationRejection: Sendable, Equatable, CustomStringConvertible {
    /// The set is a configuration of this document.
    case accepted

    /// No states at all. A machine is never in nothing.
    case empty

    /// A state appears twice. Checked before every arity count below, which would
    /// otherwise read a duplicate as a second child and blame the wrong rule.
    case duplicate

    /// A state is present whose parent is not, so the set is not ancestor-closed.
    case ancestorMissing

    /// The set closes on the wrong number of roots. Exactly one is a configuration;
    /// more than one is two disjoint trees.
    case rootCount

    /// A compound state holds a number of active children that is not one
    /// (§scxml-3.11).
    case compoundChildCount

    /// A `<parallel>` region is absent, so the set is not one in which every region
    /// is simultaneously active (§scxml-3.11).
    case parallelRegionMissing

    /// A `<parallel>` holds a number of children that is not its region count
    /// (§scxml-3.11).
    case parallelChildCount

    /// An atomic state has a child in the set, so it is not atomic here.
    case atomicHasChildren

    /// The claimed current state is not in the configuration at all.
    case currentNotActive

    /// The claimed current state is compound or parallel. §scxml-3.11 makes the
    /// current state the atomic one the engine descended to.
    case currentNotAtomic

    /// The human-readable reason, for the message a refusal carries.
    public var description: String {
        switch self {
        case .accepted:
            return "accepted"
        case .empty:
            return "the configuration is empty; a machine is never in nothing"
        case .duplicate:
            return "a state appears twice"
        case .ancestorMissing:
            return "a state is present whose parent is not, so the set is not ancestor-closed"
        case .rootCount:
            return "a configuration closes on exactly one root (W3C SCXML 3.11)"
        case .compoundChildCount:
            return "a compound state holds exactly one active child (W3C SCXML 3.11)"
        case .parallelRegionMissing:
            return "a <parallel> holds every region and one is missing (W3C SCXML 3.11)"
        case .parallelChildCount:
            return "a <parallel> holds every region and nothing else (W3C SCXML 3.11)"
        case .atomicHasChildren:
            return "an atomic state has a child in the set"
        case .currentNotActive:
            return "the current state is not in the configuration"
        case .currentNotAtomic:
            return "the current state must be the atomic state the engine descended to"
        }
    }
}

/// Whether `configuration` is a configuration of `policy`'s document, with `current`
/// as its current state.
///
/// Pure: reads the policy's static hierarchy and nothing else. Cost is quadratic in
/// the set length, which is a handful of states, and this runs once per restore: the
/// shape is chosen for being obviously right and not for being fast, exactly as its
/// Rust and C++ twins are.
///
/// What cannot be wrong, and why it is not checked: a member is a `State`, a generated
/// state type with one value per state of this document, so "no such state" needs no
/// rejection case.
public func validateConfiguration<Policy: StatePolicy>(
    policy: Policy,
    configuration: [Policy.State],
    current: Policy.State
) -> ConfigurationRejection {
    if configuration.isEmpty {
        return .empty
    }

    for i in configuration.indices {
        for j in 0..<i where configuration[j] == configuration[i] {
            return .duplicate
        }
    }

    let holds = { (state: Policy.State) in configuration.contains(state) }

    var roots = 0
    for state in configuration {
        if let parent = policy.getParent(state) {
            if !holds(parent) {
                return .ancestorMissing
            }
        } else {
            roots += 1
        }
    }
    if roots != 1 {
        return .rootCount
    }

    for state in configuration {
        let verdict = childArity(policy: policy, among: configuration, state: state)
        if verdict != .accepted {
            return verdict
        }
    }

    if !holds(current) {
        return .currentNotActive
    }
    if policy.isCompoundState(current) {
        return .currentNotAtomic
    }
    if policy.hasParallelStates && policy.isParallelState(current) {
        return .currentNotAtomic
    }

    return .accepted
}

/// Whether `state`, in the set `among`, holds the number of children a configuration
/// gives it: one for a compound state (§scxml-3.11), every region and nothing else
/// for a `<parallel>` (§scxml-3.4), none for an atomic one.
private func childArity<Policy: StatePolicy>(
    policy: Policy,
    among: [Policy.State],
    state: Policy.State
) -> ConfigurationRejection {
    var children = 0
    for candidate in among where policy.getParent(candidate) == state {
        children += 1
    }

    if policy.hasParallelStates && policy.isParallelState(state) {
        let regions = policy.getChildStates(state)
        // §scxml-3.4: every region, simultaneously.
        for region in regions where !among.contains(region) {
            return .parallelRegionMissing
        }
        if children != regions.count {
            return .parallelChildCount
        }
        return .accepted
    }

    // §scxml-3.11: exactly one.
    if policy.isCompoundState(state) {
        if children != 1 {
            return .compoundChildCount
        }
    } else if children != 0 {
        return .atomicHasChildren
    }
    return .accepted
}

/// Why a recorded `<history>` value was refused (§scxml-3.10).
public enum HistoryRejectionKind: Sendable, Equatable {
    /// The value is one the `<history>` could have recorded.
    case accepted

    /// The value holds no state. A history records at least one.
    case empty

    /// A state appears more than once.
    case duplicate

    /// A state is not below the history's parent.
    case notBelow

    /// A shallow history records the children of its parent, and a state is deeper.
    case notAChild

    /// A deep history records atomic states, and one has children.
    case notAtomic

    /// What the value holds is not part of any configuration of the document: a
    /// compound state with two active children, a region missing.
    case arity
}

/// The verdict on a recorded `<history>` value: its kind, the state it is about and
/// the history's parent where the kind names them, and the configuration rule broken
/// for `.arity`.
public struct HistoryRejection<State: Hashable> {
    public var kind: HistoryRejectionKind
    public var state: State?
    public var parent: State?
    public var arity: ConfigurationRejection

    public init(
        kind: HistoryRejectionKind,
        state: State? = nil,
        parent: State? = nil,
        arity: ConfigurationRejection = .accepted
    ) {
        self.kind = kind
        self.state = state
        self.parent = parent
        self.arity = arity
    }
}

/// Whether `recorded` is a value the `<history>` of `parent` could have recorded: its
/// active children for a shallow one, the active atomic states below it for a deep
/// one (`deep`) (§scxml-3.10).
///
/// What a history recorded is part of a configuration, so a value read back from a
/// saved state is held to that, and the restored machine can only be taken through the
/// history to a configuration its document could have been in. It is read as a
/// configuration of the subtree under `parent`: the recorded states and every state
/// between each of them and `parent` satisfy the child arity a whole configuration
/// does. A shallow history records only the children, so the arity is judged on
/// `parent` alone.
public func validateHistory<Policy: StatePolicy>(
    policy: Policy,
    parent: Policy.State,
    deep: Bool,
    recorded: [Policy.State]
) -> HistoryRejection<Policy.State> {
    typealias Verdict = HistoryRejection<Policy.State>
    if recorded.isEmpty {
        return Verdict(kind: .empty)
    }
    for (at, state) in recorded.enumerated() where recorded[..<at].contains(state) {
        return Verdict(kind: .duplicate, state: state)
    }

    // The recorded states, and every state between each of them and parent.
    var subtree = [parent]
    for state in recorded {
        var chain: [Policy.State] = []
        var at = state
        while at != parent {
            chain.append(at)
            guard let above = policy.getParent(at) else {
                return Verdict(kind: .notBelow, state: state, parent: parent)
            }
            at = above
        }
        // A state equal to parent is below nothing.
        if chain.isEmpty {
            return Verdict(kind: .notBelow, state: state, parent: parent)
        }
        for between in chain where !subtree.contains(between) {
            subtree.append(between)
        }
    }

    if deep {
        for state in recorded where !policy.getChildStates(state).isEmpty {
            return Verdict(kind: .notAtomic, state: state)
        }
        for state in subtree {
            let verdict = childArity(policy: policy, among: subtree, state: state)
            if verdict != .accepted {
                return Verdict(kind: .arity, arity: verdict)
            }
        }
        return Verdict(kind: .accepted)
    }
    for state in recorded where policy.getParent(state) != parent {
        return Verdict(kind: .notAChild, state: state, parent: parent)
    }
    let verdict = childArity(policy: policy, among: subtree, state: parent)
    if verdict != .accepted {
        return Verdict(kind: .arity, arity: verdict)
    }
    return Verdict(kind: .accepted)
}
