# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix D's microstep — which transitions an event selects,
which of them survive preemption, which states they exit and in which order,
the order their content runs in, which states they enter — written once for
every machine this runtime runs.

The procedures are transcribed rather than paraphrased, one function per
procedure and under the appendix's names, so a reader can hold this file
against the specification line by line — and against the other engines'
transcriptions of it: the C++ engines' ``sce/include/core/MicrostepAlgorithms.h``
and the Rust runtime's ``backends/rust/runtime/src/helpers/microstep.rs``, which
this module follows function for function.

What differs between machines is injected. A *document* (``Document``)
describes the document — its structure, and what each ``<history>`` recorded.
A *run* (``Run``) is the running machine: its configuration, which of a state's
transitions an event enables, and what exiting a state, running a transition's
content and entering a state DO. Nothing here knows how a machine stores
either, so a table written by hand answers them as well as a generated policy
does.

A document whose ``<history>`` defaults name one another in a cycle has no
entry set — dereferencing never reaches a state — and a hierarchy whose parent
links cycle has no domain; both are generator defects, and the procedures
assume a legal document.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import (
    Dict,
    Generic,
    List,
    Optional,
    Protocol,
    Sequence,
    Tuple,
    TypeVar,
    Union,
)

S = TypeVar("S")
H = TypeVar("H")
E = TypeVar("E")


@dataclass(frozen=True)
class StateTarget(Generic[S]):
    """One token of a target list that names a ``<state>``, ``<parallel>`` or
    ``<final>``."""

    state: S


@dataclass(frozen=True)
class HistoryTarget(Generic[H]):
    """One token of a target list that names a ``<history>`` pseudo-state.

    A history is not a state — it is never in a configuration — so the two
    kinds are kept apart rather than folded into one identifier: the entry
    procedures dereference a history (to what it recorded, or to its default)
    and add a state.
    """

    history: H


# One token of a target list, as the document wrote it. A transition's
# `target`, a state's initial transition and a `<history>`'s default
# transition all name states or histories.
EntryTarget = Union[StateTarget, HistoryTarget]


@dataclass(frozen=True)
class EnabledTransition(Generic[S]):
    """A transition selection enabled: one member of Appendix D's
    ``enabledTransitions``.

    What the microstep reads of it — its source, its target list as written,
    whether it is internal — plus the index its source state knows it by, so
    the machine that owns its executable content can run it. A transition with
    no targets exits and enters nothing and only runs its content.
    """

    source: S
    targets: Tuple[EntryTarget, ...] = ()
    transition_index: int = 0
    is_internal: bool = False

    @property
    def is_targetless(self) -> bool:
        """§scxml-D-computeExitSet guards the whole computation with
        ``if t.target``: a transition without targets exits and enters
        nothing."""
        return not self.targets

    def entry_transition(self) -> "EntryTransition[S]":
        """The same transition as the entry procedures read it."""
        return EntryTransition(self.source, self.targets, self.is_internal)


@dataclass(frozen=True)
class EntryTransition(Generic[S]):
    """The part of a transition the entry procedures read.

    ``source`` is ``None`` for the document's own initial transition, whose
    source is the ``<scxml>`` element — which has no state identifier here,
    and whose domain is the whole document.
    """

    source: Optional[S]
    targets: Tuple[EntryTarget, ...]
    is_internal: bool = False


@dataclass
class EntrySet(Generic[S, H]):
    """What a microstep enters: the appendix's three out-parameters.

    ``states_to_enter`` is sorted into entry order, so a machine enters it
    front to back. ``states_for_default_entry`` answers whether a state's
    initial transition content runs; ``default_history_content`` whether, and
    whose, ``<history>`` default transition content runs after that state's own
    entry — keyed by the history's parent, as the appendix keys it.
    """

    states_to_enter: List[S] = field(default_factory=list)
    states_for_default_entry: List[S] = field(default_factory=list)
    default_history_content: Dict[S, H] = field(default_factory=dict)

    def is_default_entry(self, state: S) -> bool:
        """Whether ``state``'s initial state is entered by default, so its
        initial transition's executable content runs."""
        return state in self.states_for_default_entry


class Document(Protocol[S, H]):
    """The document, as the entry and exit procedures read it.

    Structure only, plus the one piece of run-time state the entry procedures
    need: what each ``<history>`` recorded.
    """

    def parent_of(self, state: S) -> Optional[S]:
        """The state's parent; ``None`` when its parent is the ``<scxml>``
        element."""

    def is_compound(self, state: S) -> bool:
        """Whether the state is a ``<state>`` with child states. A
        ``<parallel>`` answers ``False``: this is the appendix's
        ``isCompoundState``, and with the ``<scxml>`` element (``None``) the set
        ``findLCCA`` chooses a domain from."""

    def is_parallel(self, state: S) -> bool:
        """Whether the state is a ``<parallel>``."""

    def is_final(self, state: S) -> bool:
        """Whether the state is a ``<final>`` element — not whether it is IN a
        final state; that is ``is_in_final_state``."""

    def child_states(self, state: S) -> Sequence[S]:
        """Appendix D's getChildStates: the state's ``<state>``, ``<parallel>``
        and ``<final>`` children, in document order."""
        # §scxml-D-getChildStates: for a <parallel>, these are its regions.

    def initial_targets(self, state: S) -> Sequence[EntryTarget]:
        """A compound state's initial transition target, as written — the
        first child state when the document names none."""

    def history_parent(self, history: H) -> S:
        """The state a ``<history>`` is declared in."""

    def history_value(self, history: H) -> Optional[Sequence[S]]:
        """What the history recorded when its parent was last exited; ``None``
        before that ever happened."""

    def history_default_targets(self, history: H) -> Sequence[EntryTarget]:
        """A ``<history>``'s default transition target, as written."""

    def document_order(self, state: S) -> int:
        """The state's position in document order, which is also entry
        order."""


class Run(Document[S, H], Protocol[S, H, E]):
    """The running machine, as the microstep drives it."""

    def configuration(self) -> Sequence[S]:
        """The active states, in any order."""

    def first_enabled_transition(
        self, state: S, event: E
    ) -> Optional[EnabledTransition[S]]:
        """This state's first transition, in document order, that the event
        enables and whose guard holds; for the machine's "no event", its first
        eventless transition whose guard holds. The only place a guard is
        evaluated."""

    def exit_state(self, state: S, configuration_before_exit: Sequence[S]) -> None:
        """Record this state's histories from the configuration as it stood
        before the microstep's first exit, run its onexit, cancel its
        invocations, remove it from the configuration."""

    def execute_transition_content(self, transition: EnabledTransition[S]) -> None:
        """Run one transition's executable content."""

    def enter_state(self, state: S, is_default_entry: bool) -> None:
        """Add the state to the configuration and schedule its invocations, run
        its onentry, then its initial transition's content when
        ``is_default_entry``; for a ``<final>``, what the appendix does on
        entering one."""

    def execute_history_default_content(self, history: H) -> None:
        """Run a ``<history>``'s default transition content."""


# ════════════════════════════════════════════════════════════════════════
# Selection
# ════════════════════════════════════════════════════════════════════════


def select_transitions(run: Run, event) -> List[EnabledTransition]:
    """Appendix D's selectTransitions, or its selectEventlessTransitions when
    ``event`` is the machine's "no event".

    Returns the optimal enabled transition set, in selection order.
    """
    configuration = list(run.configuration())
    atomic_states = sorted(
        (
            state
            for state in configuration
            if not run.is_compound(state) and not run.is_parallel(state)
        ),
        key=run.document_order,
    )

    enabled: List[EnabledTransition] = []
    for atomic in atomic_states:
        # §scxml-D-selectTransitions: the atomic state first, then its proper
        # ancestors, and the first enabled transition in document order ends
        # the walk for this atomic state. The set is ORDERED and a set: two
        # atomic states under one ancestor both reach its transition, and it is
        # one transition, taken once. With the machine's "no event" this is
        # §scxml-D-selectEventlessTransitions, the same walk over transitions
        # that have no event.
        current: Optional = atomic
        while current is not None:
            found = run.first_enabled_transition(current, event)
            if found is not None:
                seen = any(
                    t.source == found.source
                    and t.transition_index == found.transition_index
                    for t in enabled
                )
                if not seen:
                    enabled.append(found)
                break
            current = run.parent_of(current)
    return remove_conflicting_transitions(run, enabled, configuration)


def remove_conflicting_transitions(
    doc: Document,
    enabled: Sequence[EnabledTransition],
    configuration: Sequence,
) -> List[EnabledTransition]:
    """Appendix D's removeConflictingTransitions.

    Two transitions conflict when their exit sets intersect, and the one whose
    source is a descendant of the other's wins; otherwise the one selected
    first does.
    """
    filtered: List[EnabledTransition] = []
    for t1 in enabled:
        t1_exit = set(compute_exit_set(doc, t1, configuration))

        def conflicts(t2: EnabledTransition) -> bool:
            return bool(t1_exit & set(compute_exit_set(doc, t2, configuration)))

        # §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
        # transition it conflicts with whose source it does not descend from.
        # A transition that exits nothing — a targetless one — conflicts with
        # nothing and can never be preempted.
        preempted = any(
            conflicts(t2) and not is_descendant(doc, t1.source, t2.source)
            for t2 in filtered
        )
        if not preempted:
            # Not preempted means every kept transition t1 conflicts with has
            # a source t1 descends from: the appendix removes exactly those.
            filtered = [t2 for t2 in filtered if not conflicts(t2)]
            filtered.append(t1)
    return filtered


# ════════════════════════════════════════════════════════════════════════
# Domains and exit sets
# ════════════════════════════════════════════════════════════════════════


def effective_target_states(doc: Document, targets: Sequence[EntryTarget]) -> List:
    """Appendix D's getEffectiveTargetStates, over a target list.

    Returns the states the list stands for, histories dereferenced — to what
    each recorded or, before its parent was ever exited, to its default — each
    state once and in the order first named. The domain, and so the exit set,
    is a question about these.
    """
    effective: List = []
    _add_effective_target_states(doc, targets, effective)
    return effective


def _add_effective_target_states(
    doc: Document, targets: Sequence[EntryTarget], effective: List
) -> None:
    for target in targets:
        if isinstance(target, StateTarget):
            _add_once(effective, target.state)
            continue
        # §scxml-D-getEffectiveTargetStates: a history stands for the
        # configuration it recorded, and before its parent was ever exited for
        # the targets of its default transition — which may name histories
        # themselves, hence the recursion.
        recorded = doc.history_value(target.history)
        if recorded:
            for state in recorded:
                _add_once(effective, state)
        else:
            _add_effective_target_states(
                doc, doc.history_default_targets(target.history), effective
            )


def transition_domain(
    doc: Document, source: Optional, targets: Sequence, is_internal: bool
) -> Optional:
    """Appendix D's getTransitionDomain, for a transition that has targets.

    ``targets`` are the transition's EFFECTIVE targets: a history that recorded
    a state deep inside its parent is a target that deep. Returns the domain,
    or ``None`` when it is the ``<scxml>`` element — always so for the
    document's initial transition (``source`` ``None``).
    """
    if source is None:
        return None
    # §scxml-D-getTransitionDomain: an internal transition whose targets all
    # lie below a compound source has the SOURCE as its domain, so the source
    # stays active and only its active descendants are exited. That is not the
    # same as exiting nothing: a transition rooted at one of those descendants
    # exits it too, and the appendix expects the two to be found in conflict.
    if (
        is_internal
        and doc.is_compound(source)
        and all(is_descendant(doc, target, source) for target in targets)
    ):
        return source
    return _find_lcca(doc, source, targets)


def _find_lcca(doc: Document, head, tail: Sequence) -> Optional:
    """Appendix D's findLCCA over ``[head] + tail``: the first proper ancestor
    of ``head`` that is a compound state — or the ``<scxml>`` element,
    ``None`` — and contains every state of ``tail``.

    Asked of the source and EVERY target at once: combining pairwise answers
    can only widen the domain.
    """
    ancestor = doc.parent_of(head)
    while ancestor is not None:
        # §scxml-D-findLCCA filters the ancestors with
        # isCompoundStateOrScxmlElement: a `<parallel>` is never a domain.
        if doc.is_compound(ancestor) and all(
            is_descendant(doc, state, ancestor) for state in tail
        ):
            return ancestor
        ancestor = doc.parent_of(ancestor)
    return None


def compute_exit_set(
    doc: Document, transition: EnabledTransition, configuration: Sequence
) -> List:
    """Appendix D's computeExitSet, for one transition: the active proper
    descendants of its domain, in the configuration's own order."""
    # §scxml-D-computeExitSet: the appendix guards the whole computation with
    # `if t.target`, so a transition without one exits nothing at all and can
    # therefore never be preempted.
    if transition.is_targetless:
        return []
    targets = effective_target_states(doc, transition.targets)
    domain = transition_domain(doc, transition.source, targets, transition.is_internal)
    if domain is None:
        # The domain is the `<scxml>` element and every active state is a
        # descendant of it — the sibling regions of an enclosing `<parallel>`
        # included.
        return list(configuration)
    # The domain itself is not exited; everything active below it is.
    return [state for state in configuration if is_descendant(doc, state, domain)]


def compute_states_to_exit(
    doc: Document,
    transitions: Sequence[EnabledTransition],
    configuration: Sequence,
) -> List:
    """Appendix D's computeExitSet over the microstep's transitions, in
    exitOrder — the states exitStates exits."""
    # §scxml-D-computeExitSet takes the microstep's whole transition list and
    # unions the exit sets; §scxml-D-exitStates exits that union in exitOrder —
    # descendants before their ancestors and reverse document order among the
    # rest, which together are exactly reverse document order.
    states_to_exit: List = []
    for transition in transitions:
        for state in compute_exit_set(doc, transition, configuration):
            _add_once(states_to_exit, state)
    states_to_exit.sort(key=doc.document_order, reverse=True)
    return states_to_exit


# ════════════════════════════════════════════════════════════════════════
# The microstep
# ════════════════════════════════════════════════════════════════════════


def microstep(run: Run, transitions: Sequence[EnabledTransition]) -> EntrySet:
    """Appendix D's microstep: exit, run the transitions' content, enter.

    Returns what was entered — the entry set, ``states_to_enter`` in entry
    order.
    """
    # §scxml-D-microstepProcedure: every exit, then every transition's content,
    # then every entry — for the whole set at once, which is what lets the
    # regions of a `<parallel>` each take their own transition in one step.
    exit_states(run, transitions)
    execute_transition_content(run, transitions)
    return enter_states(run, [t.entry_transition() for t in transitions])


def exit_states(run: Run, transitions: Sequence[EnabledTransition]) -> None:
    """Appendix D's exitStates."""
    # §scxml-D-exitStates: the union of the transitions' exit sets, exited in
    # exitOrder. Every history is recorded from the configuration as it stood
    # BEFORE the first exit, which is the snapshot each exit is handed.
    configuration = list(run.configuration())
    for state in compute_states_to_exit(run, transitions, configuration):
        run.exit_state(state, configuration)


def execute_transition_content(
    run: Run, transitions: Sequence[EnabledTransition]
) -> None:
    """Appendix D's executeTransitionContent."""
    # §scxml-D-executeTransitionContent: in the order the transitions were
    # selected, which is not their sources' document order once an ancestor's
    # transition is reached from a later region.
    for transition in transitions:
        run.execute_transition_content(transition)


def enter_states(run: Run, transitions: Sequence[EntryTransition]) -> EntrySet:
    """Appendix D's enterStates.

    Also the whole of the appendix's entry into the initial configuration
    (Appendix D's interpret): hand it the document's initial transition, whose
    source is the ``<scxml>`` element (``source`` ``None``).
    """
    # §scxml-D-interpret enters the initial configuration through this same
    # procedure, with a transition whose source is the <scxml> element.
    entry = compute_entry_set(run, transitions)
    for state in entry.states_to_enter:
        # §scxml-D-enterStates: onentry, then the initial transition's content
        # if and only if this state's initial state is being entered by
        # default, then a history's default content owed to it.
        run.enter_state(state, entry.is_default_entry(state))
        history = entry.default_history_content.get(state)
        if history is not None:
            run.execute_history_default_content(history)
    return entry


# ════════════════════════════════════════════════════════════════════════
# The entry set
# ════════════════════════════════════════════════════════════════════════


def compute_entry_set(doc: Document, transitions: Sequence[EntryTransition]) -> EntrySet:
    """Appendix D's computeEntrySet.

    ``transitions`` are the microstep's, in the order they were selected; a
    transition without targets enters nothing. Returns the entry set,
    ``states_to_enter`` sorted into entry order.
    """
    entry: EntrySet = EntrySet()
    for transition in transitions:
        # §scxml-D-computeEntrySet: first every target with its default
        # descendants, then the ancestors that are entered inside the domain —
        # ancestors outside it were never exited. The order matters for a
        # target SET: the regions of a `<parallel>` that another target already
        # descends into must be seen as taken before the ancestor walk fills the
        # rest with defaults.
        for target in transition.targets:
            add_descendant_states_to_enter(doc, target, entry)
        effective = effective_target_states(doc, transition.targets)
        if not effective:
            continue
        domain = transition_domain(
            doc, transition.source, effective, transition.is_internal
        )
        for state in effective:
            add_ancestor_states_to_enter(doc, StateTarget(state), domain, entry)

    # §scxml-D-enterStates: states are entered in entryOrder — ancestors before
    # descendants, document order between the rest, which is exactly document
    # order, because that is a pre-order walk.
    entry.states_to_enter.sort(key=doc.document_order)
    return entry


def add_descendant_states_to_enter(
    doc: Document, target: EntryTarget, entry: EntrySet
) -> None:
    """Appendix D's addDescendantStatesToEnter.

    Adds ``target`` and every descendant entering it enters: a history's
    recorded or default configuration, a compound state's initial state(s),
    every region of a ``<parallel>`` not already on the set.
    """
    if isinstance(target, HistoryTarget):
        history = target.history
        parent = doc.history_parent(history)
        # §scxml-3.10: a transition to a history behaves as a transition to the
        # configuration it stored, or — before its parent was ever visited — to
        # its default stored configuration.
        recorded = doc.history_value(history)
        if recorded:
            # §scxml-D-addDescendantStatesToEnter: a history that has recorded a
            # configuration enters it, and the ancestors between it and the
            # history's parent — a deep history records atomic states, which
            # need not be children.
            for state in recorded:
                add_descendant_states_to_enter(doc, StateTarget(state), entry)
            for state in recorded:
                add_ancestor_states_to_enter(doc, StateTarget(state), parent, entry)
        else:
            # §scxml-D-addDescendantStatesToEnter: nothing recorded yet, so the
            # default transition is taken, and its content is owed once the
            # parent has been entered — keyed by the parent, a later history of
            # the same parent replacing it.
            entry.default_history_content[parent] = history
            defaults = doc.history_default_targets(history)
            for default in defaults:
                add_descendant_states_to_enter(doc, default, entry)
            for default in defaults:
                add_ancestor_states_to_enter(doc, default, parent, entry)
        return

    state = target.state
    _add_once(entry.states_to_enter, state)
    if doc.is_compound(state):
        # §scxml-D-addDescendantStatesToEnter: a compound state entered as a
        # target is entered by DEFAULT — the one condition under which its
        # initial transition's content runs. Its initial transition names the
        # child or children it enters (§scxml-3.3), possibly several and
        # possibly deep (§scxml-3.6).
        _add_once(entry.states_for_default_entry, state)
        initial = doc.initial_targets(state)
        for child in initial:
            add_descendant_states_to_enter(doc, child, entry)
        for child in initial:
            add_ancestor_states_to_enter(doc, child, state, entry)
    elif doc.is_parallel(state):
        _add_region_defaults(doc, state, entry)


def add_ancestor_states_to_enter(
    doc: Document, target: EntryTarget, ancestor: Optional, entry: EntrySet
) -> None:
    """Appendix D's addAncestorStatesToEnter.

    Adds the proper ancestors of ``target`` up to, not including, ``ancestor``
    (``None``: up to the ``<scxml>`` element), filling in the regions of every
    ``<parallel>`` among them.
    """
    for anc in _proper_ancestors(doc, target, ancestor):
        # §scxml-D-addAncestorStatesToEnter: an ancestor is entered WITHOUT its
        # default initial state — the set already holds the descendant it leads
        # to. A `<parallel>` still gives its other regions their defaults,
        # because all of them are entered.
        _add_once(entry.states_to_enter, anc)
        if doc.is_parallel(anc):
            _add_region_defaults(doc, anc, entry)


def _proper_ancestors(doc: Document, target: EntryTarget, ancestor: Optional) -> List:
    """Appendix D's getProperAncestors: the ancestors of ``target`` in ancestry
    order up to, not including, ``ancestor`` — and nothing at all when
    ``ancestor`` is not above it.

    A ``<history>`` sits in its parent, so the parent is its first proper
    ancestor. ``ancestor`` ``None`` is the ``<scxml>`` element, which is above
    every state and is never entered, so it is not in the list.
    """
    chain: List = []
    if isinstance(target, HistoryTarget):
        current = doc.history_parent(target.history)
    else:
        current = doc.parent_of(target.state)
    while current is not None:
        if current == ancestor:
            return chain
        chain.append(current)
        current = doc.parent_of(current)
    # §scxml-D-getProperAncestors: `ancestor` is the state's parent, the state
    # itself, or one of its descendants — none of which the walk can meet — so
    # the answer is the empty set.
    return chain if ancestor is None else []


def _add_region_defaults(doc: Document, parallel, entry: EntrySet) -> None:
    # §scxml-3.4: every child of an active `<parallel>` is active, so a region
    # nothing on the set descends into is entered by default.
    for child in doc.child_states(parallel):
        taken = any(is_descendant(doc, state, child) for state in entry.states_to_enter)
        if not taken:
            add_descendant_states_to_enter(doc, StateTarget(child), entry)


# ════════════════════════════════════════════════════════════════════════
# History
# ════════════════════════════════════════════════════════════════════════


def recorded_history(
    doc: Document, parent, deep: bool, configuration_before_exit: Sequence
) -> List:
    """Appendix D's exitStates, its history half: what a ``<history>`` of
    ``parent`` records as ``parent`` is exited — for a deep history the active
    atomic states below ``parent``, for a shallow one its active children.

    Read off ``configuration_before_exit``, the configuration as it stood
    before the microstep's first exit: the appendix records every history
    before it exits any state, so every history of one microstep reads the
    same configuration. Returned in that configuration's order.
    """
    if deep:
        # §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`.
        return [
            state
            for state in configuration_before_exit
            if not doc.is_compound(state)
            and not doc.is_parallel(state)
            and is_descendant(doc, state, parent)
        ]
    # §scxml-D-exitStates: `s0.parent == s`.
    return [
        state for state in configuration_before_exit if doc.parent_of(state) == parent
    ]


# ════════════════════════════════════════════════════════════════════════
# Predicates
# ════════════════════════════════════════════════════════════════════════


def is_in_final_state(doc: Document, state, configuration: Sequence) -> bool:
    """Appendix D's isInFinalState: a compound state is in a final state when
    one of its ``<final>`` children is active; a ``<parallel>`` when EVERY one
    of its child states is — asked recursively, so a region that is itself a
    ``<parallel>`` counts only once all of ITS regions do. Nothing else ever
    is."""
    if doc.is_compound(state):
        return any(
            doc.is_final(child) and child in configuration
            for child in doc.child_states(state)
        )
    if doc.is_parallel(state):
        return all(
            is_in_final_state(doc, child, configuration)
            for child in doc.child_states(state)
        )
    return False


def is_descendant(doc: Document, state, ancestor) -> bool:
    """Appendix D's isDescendant: whether ``state`` lies strictly below
    ``ancestor``."""
    current = doc.parent_of(state)
    while current is not None:
        if current == ancestor:
            return True
        current = doc.parent_of(current)
    return False


def _add_once(states: List, state) -> None:
    if state not in states:
        states.append(state)
