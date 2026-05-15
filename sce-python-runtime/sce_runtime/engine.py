# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCXML execution engine for AOT-generated Python state machines.

Atomic α: atomic states + onentry/onexit + basic guarded transitions.
Atomic β: compound entry chain (W3C SCXML 3.3 / 3.6 / 3.13), early-binding
datamodel init (W3C 5.3), ancestor-chain transition selection
(W3C Appendix D.2), LCCA-based exit/entry boundary (W3C 5.9.2), and
`<raise>` (W3C 4.4) wired through `raise_internal`. Parallel / history /
invoke / scheduler land in γ.
"""

from __future__ import annotations

from collections import deque
from typing import Generic, Optional, TypeVar

from .event import EventMetadata, EventWithMetadata
from .policy import StatePolicy, TransitionResult

S = TypeVar("S")
E = TypeVar("E")


class Engine(Generic[S, E]):
    """Generic SCXML engine bound to a concrete StatePolicy.

    Single-threaded; callers needing concurrency must guard with a lock.
    """

    def __init__(self, policy: StatePolicy[S, E]) -> None:
        self._policy = policy
        # W3C SCXML 3.3: the active leaf. β keeps a single leaf at a time
        # (no parallel regions); γ will replace this with an active set.
        self._current_state: S = policy.initial_state()
        self._internal_queue: "deque[EventWithMetadata[E]]" = deque()
        self._external_queue: "deque[EventWithMetadata[E]]" = deque()
        self._is_running: bool = False
        self._reached_final: bool = False

    # ── Lifecycle ──────────────────────────────────────────────────

    def initialize(self) -> None:
        """Enter the initial configuration and drive the macrostep loop until stable."""
        if self._is_running:
            return
        self._is_running = True
        # W3C SCXML 5.3 early binding: datamodel initialisation runs before
        # any onentry action fires.
        self._policy.initialize_datamodel(self)
        # W3C SCXML 3.6: `initial_state()` returns the resolved leaf
        # (sce-build's parser walks the initial chain down to atomic at
        # parse time). Re-build the entry chain root-first by walking up
        # the leaf's ancestors so every enclosing compound's onentry
        # actions fire in document order.
        leaf = self._policy.initial_state()
        chain = []
        state: Optional[S] = leaf
        while state is not None:
            chain.append(state)
            state = self._policy.get_parent(state)
        for s in reversed(chain):
            self._policy.execute_entry_actions(s, self)
            if self._policy.is_final_state(s):
                self._current_state = s
                self._reached_final = True
                self._is_running = False
                return
        self._current_state = leaf
        if self._reached_final or not self._is_running:
            return
        self._process_queues()

    def stop(self) -> None:
        self._is_running = False

    # ── Public introspection ───────────────────────────────────────

    @property
    def current_state(self) -> S:
        return self._current_state

    @property
    def is_running(self) -> bool:
        return self._is_running

    @property
    def reached_final(self) -> bool:
        return self._reached_final

    # ── Event injection ────────────────────────────────────────────

    def send_event(self, event: E, metadata: Optional[EventMetadata] = None) -> None:
        """W3C SCXML 5.10.1 — enqueue an external event and process to stability."""
        if not self._is_running:
            return
        self._external_queue.append(
            EventWithMetadata(event=event, metadata=metadata or EventMetadata())
        )
        self._process_queues()

    def raise_internal(self, event: E, metadata: Optional[EventMetadata] = None) -> None:
        """W3C SCXML 4.4 `<raise>` — enqueue an internal event (drained before externals)."""
        self._internal_queue.append(
            EventWithMetadata(event=event, metadata=metadata or EventMetadata())
        )

    # ── Microstep / macrostep core ────────────────────────────────

    def _process_queues(self) -> None:
        """W3C SCXML Appendix D.2 macrostep loop: drain eventless first,
        then consume one queued event (internal-first, then external),
        repeat until stable or final reached."""
        while self._is_running and not self._reached_final:
            self._drain_eventless()
            if self._reached_final or not self._is_running:
                return
            evt = self._dequeue()
            if evt is None:
                return
            self._dispatch(evt)

    def _drain_eventless(self) -> None:
        """W3C SCXML 3.13: fire all enabled eventless transitions until none remain."""
        null_evt = self._policy.null_event()
        while self._is_running and not self._reached_final:
            result = self._select_with_ancestors(null_evt)
            if result is None:
                return
            self._take_transition(result)

    def _dequeue(self) -> Optional[EventWithMetadata[E]]:
        if self._internal_queue:
            return self._internal_queue.popleft()
        if self._external_queue:
            return self._external_queue.popleft()
        return None

    def _dispatch(self, evt: EventWithMetadata[E]) -> None:
        result = self._select_with_ancestors(evt.event)
        if result is None:
            return
        self._take_transition(result)

    def _select_with_ancestors(self, event: E) -> Optional[TransitionResult[S]]:
        """W3C SCXML Appendix D.2 — walk from the active leaf up the ancestor
        chain, returning the first enabled transition. The TransitionResult's
        `source` is stamped with the matching state so `_take_transition`
        knows which ancestor scope to exit from."""
        state: Optional[S] = self._current_state
        while state is not None:
            result = self._policy.select_transition(state, event)
            if result is not None:
                if result.source is None:
                    result.source = state
                return result
            state = self._policy.get_parent(state)
        return None

    def _take_transition(self, result: TransitionResult[S]) -> None:
        """W3C SCXML 3.13 + 5.9.2 — exit the active descendants up to the
        LCCA, run the transition action, then enter the path from the
        LCCA's child down to (and through) the target's compound chain."""
        source: S = result.source if result.source is not None else self._current_state

        if result.targetless or result.target is None:
            self._policy.execute_transition_action(source, result.transition_index, self)
            return

        target: S = result.target

        # W3C SCXML 5.9.2: pick the boundary (LCCA). For internal
        # transitions where target is a proper descendant of source, the
        # boundary is source itself — neither source nor any of its
        # surviving ancestors are exited or re-entered.
        if result.is_internal and self._is_proper_descendant(target, source):
            boundary: Optional[S] = source
        else:
            boundary = self._find_lcca(source, target)

        # Exit chain: leaf upward, stopping at (but not exiting) `boundary`.
        state: Optional[S] = self._current_state
        while state is not None and state != boundary:
            self._policy.execute_exit_actions(state, self)
            state = self._policy.get_parent(state)

        # W3C 3.13: transition action runs between exits and entries.
        self._policy.execute_transition_action(source, result.transition_index, self)

        # Entry chain: collect from target upward (stopping at boundary),
        # then fire in document order (root-most first).
        upward = []
        state = target
        while state is not None and state != boundary:
            upward.append(state)
            state = self._policy.get_parent(state)
        for s in reversed(upward[1:]):  # ancestors of target, root-most first
            self._policy.execute_entry_actions(s, self)
            if self._policy.is_final_state(s):
                self._current_state = s
                self._reached_final = True
                self._is_running = False
                return

        # Finally enter `target` and descend through its `<initial>` chain.
        self._descend_initial(target)

    # ── Compound entry / hierarchy helpers ────────────────────────

    def _descend_initial(self, target: S) -> None:
        """W3C SCXML 3.3 / 3.6 — fire entry on `target`, then descend
        through its initial children until a leaf is reached. β has at
        most one initial child per compound (no parallel regions)."""
        state: Optional[S] = target
        while state is not None:
            self._policy.execute_entry_actions(state, self)
            if self._policy.is_final_state(state):
                self._current_state = state
                self._reached_final = True
                self._is_running = False
                return
            self._current_state = state
            if self._policy.is_compound_state(state):
                children = self._policy.get_initial_children(state)
                if not children:
                    return  # well-formed compound has children; defensive.
                state = children[0]
                continue
            return  # atomic leaf reached.

    def _is_proper_descendant(self, descendant: S, ancestor: S) -> bool:
        """True iff `descendant` is a strict descendant of `ancestor`."""
        s = self._policy.get_parent(descendant)
        while s is not None:
            if s == ancestor:
                return True
            s = self._policy.get_parent(s)
        return False

    def _find_lcca(self, source: S, target: S) -> Optional[S]:
        """W3C SCXML 5.9.2 — Lowest Common Compound Ancestor: the lowest
        state that is a proper ancestor of both `source` and `target`.
        Returns None if they share no common ancestor (i.e., the
        transition scope is the document root)."""
        source_ancestors = []
        s = self._policy.get_parent(source)
        while s is not None:
            source_ancestors.append(s)
            s = self._policy.get_parent(s)
        s = self._policy.get_parent(target)
        while s is not None:
            if s in source_ancestors:
                return s
            s = self._policy.get_parent(s)
        return None
