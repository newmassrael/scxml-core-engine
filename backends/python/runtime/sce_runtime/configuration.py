# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""W3C SCXML 3.11 — is a set of states a CONFIGURATION of the
document, and is ``current`` its current state?

``Engine.active_configuration`` publishes the configuration a machine is in and
``Engine.enter_at`` is the door that takes one back. This module owns the
question between them, and it is the only place that asks it.

Why this rejects instead of raising: every other input to this engine is
authored — the generator wrote the policy, and a hierarchy that does not walk is
a generator defect. A restored configuration is the single exception. It arrives
from OUTSIDE the process, read back by a host from wherever it persisted it,
recorded against a document revision that may since have moved. A host holding a
stale record has to be able to handle that answer, so this returns a reason and
the engine hands it on.

The Python twin of the Rust runtime's ``helpers::configuration::validate``, of
the C++ ``SCE::Core::validateConfiguration`` and of the Go
``sce.ValidateConfiguration``, asking the same questions of the same static
hierarchy, so a configuration one engine accepts is one the others accept.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Generic, List, Optional, Sequence, TypeVar

from .policy import StatePolicy

S = TypeVar("S")
E = TypeVar("E")


class ConfigurationRejection(Enum):
    """Why a configuration was refused. ``NONE`` is the accepting answer.

    An enumerated reason rather than a bool: a host handing back a journal it
    wrote itself needs to know WHICH rule its record broke, and "invalid" sends
    it looking at the wrong half.
    """

    NONE = "accepted"
    EMPTY = "the configuration is empty; a machine is never in nothing"
    DUPLICATE = "a state appears twice"
    ANCESTOR_MISSING = (
        "a state is present whose parent is not, so the set is not ancestor-closed"
    )
    ROOT_COUNT = "a configuration closes on exactly one root (W3C SCXML 3.11)"
    COMPOUND_CHILD_COUNT = (
        "a compound state holds exactly one active child (W3C SCXML 3.11)"
    )
    PARALLEL_REGION_MISSING = (
        "a <parallel> holds every region and one is missing (W3C SCXML 3.11)"
    )
    PARALLEL_CHILD_COUNT = (
        "a <parallel> holds every region and nothing else (W3C SCXML 3.11)"
    )
    ATOMIC_HAS_CHILDREN = "an atomic state has a child in the set"
    CURRENT_NOT_ACTIVE = "the current state is not in the configuration"
    CURRENT_NOT_ATOMIC = (
        "the current state must be the atomic state the engine descended to"
    )

    def __str__(self) -> str:
        """The human-readable reason, for the message a refusal carries."""
        return self.value


def validate_configuration(
    policy: StatePolicy[S, E],
    configuration: Sequence[S],
    current: S,
) -> ConfigurationRejection:
    """Whether ``configuration`` is a configuration of ``policy``'s document,
    with ``current`` as its current state.

    Pure: reads the policy's static hierarchy and nothing else. Cost is
    quadratic in the set length, which is a handful of states, and this runs
    once per restore — the shape is chosen for being obviously right rather than
    for being fast, exactly as its Rust, C++ and Go twins are.

    What cannot be wrong, and why it is not checked: a member is a value of the
    generated state enum, one member per state of this document, so "no such
    state" needs no rejection variant.

    What is checked:

    - the set is not empty and names nothing twice;
    - it is ancestor-closed, and closes on exactly one root;
    - W3C SCXML 3.11: a compound member holds exactly ONE active child — this is
      what refuses two siblings of one region;
    - W3C SCXML 3.11: a ``<parallel>`` member holds ALL of its regions, because
      they are simultaneously active when the parent element is active;
    - an atomic member holds no children;
    - the claimed current state is an atomic member of the set.
    """
    if not configuration:
        return ConfigurationRejection.EMPTY

    for index, state in enumerate(configuration):
        if state in configuration[:index]:
            return ConfigurationRejection.DUPLICATE

    members = list(configuration)

    roots = 0
    for state in members:
        parent = policy.get_parent(state)
        if parent is None:
            roots += 1
        elif parent not in members:
            return ConfigurationRejection.ANCESTOR_MISSING
    if roots != 1:
        return ConfigurationRejection.ROOT_COUNT

    for state in members:
        verdict = _child_arity(policy, members, state)
        if verdict is not ConfigurationRejection.NONE:
            return verdict

    if current not in members:
        return ConfigurationRejection.CURRENT_NOT_ACTIVE
    if policy.is_compound_state(current) or policy.is_parallel_state(current):
        return ConfigurationRejection.CURRENT_NOT_ATOMIC

    return ConfigurationRejection.NONE


def _child_arity(
    policy: StatePolicy[S, E], among: Sequence[S], state: S
) -> ConfigurationRejection:
    """Whether ``state``, in the set ``among``, holds the number of children a
    configuration gives it: one for a compound state (W3C SCXML 3.11), every
    region and nothing else for a ``<parallel>`` (3.4), none for an atomic one."""
    children = sum(1 for candidate in among if policy.get_parent(candidate) == state)

    if policy.is_parallel_state(state):
        regions = policy.get_child_states(state)
        for region in regions:
            if region not in among:
                return ConfigurationRejection.PARALLEL_REGION_MISSING
        if children != len(regions):
            return ConfigurationRejection.PARALLEL_CHILD_COUNT
        return ConfigurationRejection.NONE

    if policy.is_compound_state(state):
        if children != 1:
            return ConfigurationRejection.COMPOUND_CHILD_COUNT
    elif children != 0:
        return ConfigurationRejection.ATOMIC_HAS_CHILDREN
    return ConfigurationRejection.NONE


class HistoryRejectionKind(Enum):
    """Why a recorded ``<history>`` value was refused (W3C SCXML 3.10)."""

    ACCEPTED = "accepted"
    # The value holds no state. A history records at least one.
    EMPTY = "empty"
    # A state appears more than once.
    DUPLICATE = "duplicate"
    # A state is not below the history's parent.
    NOT_BELOW = "not_below"
    # A shallow history records the children of its parent, and a state is deeper.
    NOT_A_CHILD = "not_a_child"
    # A deep history records atomic states, and one has children.
    NOT_ATOMIC = "not_atomic"
    # What the value holds is part of no configuration of the document: a compound
    # state with two active children, a region missing.
    ARITY = "arity"


@dataclass(frozen=True)
class HistoryRejection(Generic[S]):
    """The verdict on a recorded ``<history>`` value: its kind, the state it is
    about and the history's parent where the kind names them, and the
    configuration rule broken for ``ARITY``."""

    kind: HistoryRejectionKind
    state: Optional[S] = None
    parent: Optional[S] = None
    arity: ConfigurationRejection = ConfigurationRejection.NONE


def validate_history(
    policy: StatePolicy[S, E], parent: S, deep: bool, recorded: Sequence[S]
) -> HistoryRejection[S]:
    """Whether ``recorded`` is a value the ``<history>`` of ``parent`` could have
    recorded: its active children for a shallow one, the active atomic states below
    it for a deep one (``deep``) (W3C SCXML 3.10).

    What a history recorded is part of a configuration, so a value read back from a
    saved state is held to that, and the restored machine can only be taken through
    the history to a configuration its document could have been in. It is read as a
    configuration of the subtree under ``parent``: the recorded states and every
    state between each of them and ``parent`` satisfy the child arity a whole
    configuration does. A shallow history records only the children, so the arity is
    judged on ``parent`` alone.

    The Python twin of ``sce.ValidateHistory`` (Go)."""
    if not recorded:
        return HistoryRejection(HistoryRejectionKind.EMPTY)
    for at, state in enumerate(recorded):
        if state in recorded[:at]:
            return HistoryRejection(HistoryRejectionKind.DUPLICATE, state=state)

    # The recorded states, and every state between each of them and parent.
    subtree: List[S] = [parent]
    for state in recorded:
        chain: List[S] = []
        at_state: Optional[S] = state
        while at_state != parent:
            chain.append(at_state)
            at_state = policy.get_parent(at_state)
            if at_state is None:
                return HistoryRejection(
                    HistoryRejectionKind.NOT_BELOW, state=state, parent=parent
                )
        # A state equal to parent is below nothing.
        if not chain:
            return HistoryRejection(
                HistoryRejectionKind.NOT_BELOW, state=state, parent=parent
            )
        for between in chain:
            if between not in subtree:
                subtree.append(between)

    if deep:
        for state in recorded:
            if policy.get_child_states(state):
                return HistoryRejection(HistoryRejectionKind.NOT_ATOMIC, state=state)
        for state in subtree:
            verdict = _child_arity(policy, subtree, state)
            if verdict is not ConfigurationRejection.NONE:
                return HistoryRejection(HistoryRejectionKind.ARITY, arity=verdict)
        return HistoryRejection(HistoryRejectionKind.ACCEPTED)
    for state in recorded:
        if policy.get_parent(state) != parent:
            return HistoryRejection(
                HistoryRejectionKind.NOT_A_CHILD, state=state, parent=parent
            )
    verdict = _child_arity(policy, subtree, parent)
    if verdict is not ConfigurationRejection.NONE:
        return HistoryRejection(HistoryRejectionKind.ARITY, arity=verdict)
    return HistoryRejection(HistoryRejectionKind.ACCEPTED)
