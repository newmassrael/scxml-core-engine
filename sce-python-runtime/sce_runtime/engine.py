# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCXML execution engine for AOT-generated Python state machines.

Atomic α: atomic states + onentry/onexit + basic guarded transitions.
Atomic β: compound entry chain (W3C SCXML 3.3 / 3.6 / 3.13), early-binding
datamodel init (W3C 5.3), ancestor-chain transition selection
(W3C Appendix D.2), LCCA-based exit/entry boundary (W3C 5.9.2), and
`<raise>` (W3C 4.4) wired through `raise_internal`.
Atomic γ: `<parallel>` regions with active-set tracking, atomic multi-
transition microsteps with W3C SCXML 3.13 conflict resolution, and
`done.state.<parent>` events raised when all regions of a `<parallel>`
reach `<final>`.
"""

from __future__ import annotations

from collections import deque
from typing import Generic, List, Optional, Set, TypeVar

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
        # W3C SCXML 3.3: active configuration tracked as the ordered list
        # of currently-active leaf states. For non-parallel machines this
        # list has at most one entry. For machines containing
        # `<parallel>`, every region contributes its own leaf so the list
        # carries one entry per active region. The list is kept sorted by
        # document order so deterministic iteration matches the W3C
        # algorithm.
        self._active_leaves: List[S] = []
        self._internal_queue: "deque[EventWithMetadata[E]]" = deque()
        self._external_queue: "deque[EventWithMetadata[E]]" = deque()
        self._is_running: bool = False
        self._reached_final: bool = False
        # W3C SCXML 3.7 — set of `<parallel>` states for which a
        # `done.state.<id>` event has already been raised this run, so a
        # second region reaching `<final>` does not re-fire it.
        self._fired_done_state: Set[S] = set()

    # ── Lifecycle ──────────────────────────────────────────────────

    def initialize(self) -> None:
        """Enter the initial configuration and drive the macrostep loop until stable."""
        if self._is_running:
            return
        self._is_running = True
        # W3C SCXML 5.3 early binding: datamodel initialisation runs before
        # any onentry action fires.
        self._policy.initialize_datamodel(self)
        # W3C SCXML 3.3: build the entry path root-first by walking up
        # from the parser-resolved initial leaf. For documents that contain
        # `<parallel>` on the initial path, `_enter_state` automatically
        # branches into every region instead of following the parser's
        # single-leaf resolution.
        initial_leaf = self._policy.initial_state()
        chain = []
        state: Optional[S] = initial_leaf
        while state is not None:
            chain.append(state)
            state = self._policy.get_parent(state)
        chain.reverse()
        # Enter from the top of the document; the recursive entry handles
        # parallel branching and the initial-child chain on its own.
        root = chain[0]
        # Re-target descent to the topmost compound so parallel branching
        # at any depth is honoured, regardless of where the parser's
        # `initial_leaf` lands.
        self._enter_state(root)
        if self._reached_final or not self._is_running:
            return
        self._process_queues()

    def stop(self) -> None:
        self._is_running = False

    # ── Public introspection ───────────────────────────────────────

    @property
    def current_state(self) -> S:
        """W3C SCXML — for non-parallel machines the single active leaf.

        For parallel machines this returns the document-order-earliest
        active leaf; callers that need every region should iterate
        `active_leaves` instead.
        """
        if self._active_leaves:
            return self._active_leaves[0]
        return self._policy.initial_state()

    @property
    def active_leaves(self) -> List[S]:
        """The full ordered list of currently-active leaf states."""
        return list(self._active_leaves)

    @property
    def is_running(self) -> bool:
        return self._is_running

    @property
    def reached_final(self) -> bool:
        return self._reached_final

    def active_configuration(self) -> Set[S]:
        """W3C SCXML 3.3 — every active state (atomic + ancestors)."""
        result: Set[S] = set()
        for leaf in self._active_leaves:
            state: Optional[S] = leaf
            while state is not None and state not in result:
                result.add(state)
                state = self._policy.get_parent(state)
        return result

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
            transitions = self._select_transitions(null_evt)
            if not transitions:
                return
            self._take_transitions(transitions)

    def _dequeue(self) -> Optional[EventWithMetadata[E]]:
        if self._internal_queue:
            return self._internal_queue.popleft()
        if self._external_queue:
            return self._external_queue.popleft()
        return None

    def _dispatch(self, evt: EventWithMetadata[E]) -> None:
        transitions = self._select_transitions(evt.event)
        if not transitions:
            return
        self._take_transitions(transitions)

    # ── Transition selection ──────────────────────────────────────

    def _select_transitions(self, event: E) -> List[TransitionResult[S]]:
        """W3C SCXML 3.13 — pick one transition per active leaf (leaf first,
        then ancestor chain). Then remove conflicting transitions per
        Appendix D.2."""
        candidates: List[TransitionResult[S]] = []
        # Two regions of the same `<parallel>` can both walk up into a
        # shared ancestor and pick the same transition; we deduplicate by
        # (source state, transition_index) so the runtime never executes
        # the same `<transition>` twice in one microstep.
        seen: set = set()
        for leaf in self._active_leaves:
            picked = self._select_from_chain(leaf, event)
            if picked is None:
                continue
            key = (picked.source, picked.transition_index)
            if key in seen:
                continue
            seen.add(key)
            candidates.append(picked)
        if len(candidates) <= 1:
            return candidates
        return self._remove_conflicting_transitions(candidates)

    def _select_from_chain(
        self, leaf: S, event: E
    ) -> Optional[TransitionResult[S]]:
        """W3C SCXML Appendix D.2 — walk from `leaf` upward, return the first
        enabled transition. Stamps the result's `source` if the policy did
        not."""
        state: Optional[S] = leaf
        while state is not None:
            result = self._policy.select_transition(state, event)
            if result is not None:
                if result.source is None:
                    result.source = state
                return result
            state = self._policy.get_parent(state)
        return None

    def _remove_conflicting_transitions(
        self, candidates: List[TransitionResult[S]]
    ) -> List[TransitionResult[S]]:
        """W3C SCXML Appendix D.2 — `removeConflictingTransitions`.

        Two transitions conflict if their exit sets intersect. The one
        with the deeper source (or earlier document order on a tie) wins;
        the other is dropped.
        """
        # Stable sort: deeper-source-first, then document order, so a
        # later iteration consistently sees the winners first.
        candidates = sorted(
            candidates,
            key=lambda t: (
                -self._depth_of(t.source if t.source is not None else t.target),
                self._policy.get_document_order(
                    t.source if t.source is not None else t.target
                ),
            ),
        )
        filtered: List[TransitionResult[S]] = []
        filtered_exits: List[Set[S]] = []
        for cand in candidates:
            cand_exit = self._compute_exit_set(cand)
            conflict = False
            for kept_exit in filtered_exits:
                if cand_exit & kept_exit:
                    conflict = True
                    break
            if not conflict:
                filtered.append(cand)
                filtered_exits.append(cand_exit)
        return filtered

    def _depth_of(self, state: Optional[S]) -> int:
        if state is None:
            return 0
        depth = 0
        s: Optional[S] = state
        while s is not None:
            s = self._policy.get_parent(s)
            depth += 1
        return depth

    # ── Transition execution ──────────────────────────────────────

    def _take_transitions(self, transitions: List[TransitionResult[S]]) -> None:
        """W3C SCXML 3.13 — execute a set of non-conflicting transitions
        atomically. Exit chains are unioned and run deepest-first; entry
        chains run after all exits and transition actions."""
        if not transitions:
            return

        # Combined exit set across all transitions in this microstep.
        combined_exit: Set[S] = set()
        for t in transitions:
            combined_exit |= self._compute_exit_set(t)

        # W3C 3.13: exit in reverse document order (deepest descendants
        # leave first). Inside the same document-order rank, exit order
        # is unspecified — we use document order descending for determinism.
        exit_list = sorted(
            combined_exit,
            key=lambda s: -self._policy.get_document_order(s),
        )
        for s in exit_list:
            self._policy.execute_exit_actions(s, self)

        # Drop exited leaves and any leaves whose proper ancestors were
        # exited (their region's leaf is gone). For parallel exits the
        # ancestor parallel state is in `combined_exit`, which sweeps
        # every region's leaf.
        self._active_leaves = [
            leaf
            for leaf in self._active_leaves
            if leaf not in combined_exit
            and not any(
                self._is_proper_descendant(leaf, exited) for exited in combined_exit
            )
        ]

        # Run transition actions in document order of their source.
        for t in sorted(
            transitions,
            key=lambda t: self._policy.get_document_order(
                t.source if t.source is not None else self._policy.initial_state()
            ),
        ):
            source = t.source if t.source is not None else self._policy.initial_state()
            self._policy.execute_transition_action(source, t.transition_index, self)

        # Enter targets. Targetless transitions already had their action
        # run above and contribute no entries.
        for t in transitions:
            if t.targetless or t.target is None:
                continue
            self._enter_target(t)
            if self._reached_final or not self._is_running:
                return

        # Sort active leaves by document order to keep deterministic
        # iteration in later microsteps.
        self._active_leaves.sort(key=self._policy.get_document_order)

        # W3C SCXML 3.7 — raise `done.state.<parent>` for every parallel
        # whose regions have all reached `<final>`.
        self._check_done_state_events()

    def _enter_target(self, transition: TransitionResult[S]) -> None:
        """Enter the LCA→target path, then descend through the target's
        initial chain (with parallel branching at any `<parallel>`)."""
        target: S = transition.target  # type: ignore[assignment]
        source: S = (
            transition.source if transition.source is not None else target
        )

        # Find the boundary (LCCA). Internal transitions into a descendant
        # of source use `source` itself; otherwise use the standard LCCA.
        if transition.is_internal and self._is_proper_descendant(target, source):
            boundary: Optional[S] = source
        else:
            boundary = self._find_lcca(source, target)

        # Build the entry chain from `target` upward, stopping at `boundary`.
        upward: List[S] = []
        state: Optional[S] = target
        while state is not None and state != boundary:
            upward.append(state)
            state = self._policy.get_parent(state)
        # Enter from boundary-child down to target, then descend.
        for s in reversed(upward[1:]):
            self._policy.execute_entry_actions(s, self)
            if self._policy.is_final_state(s):
                self._active_leaves.append(s)
                self._mark_root_final_if_top_level(s)
                return
        self._enter_state(target)

    def _enter_state(self, state: S) -> None:
        """Recursively enter `state`: run its entry actions, then descend
        through the appropriate child (single initial child for a compound,
        every region for a `<parallel>`)."""
        self._policy.execute_entry_actions(state, self)
        if self._policy.is_final_state(state):
            self._active_leaves.append(state)
            self._mark_root_final_if_top_level(state)
            return
        if self._policy.is_parallel_state(state):
            # W3C SCXML 3.4 — enter every region in document order. The
            # policy provides regions in declaration-time order (which may
            # be alphabetical for some codegen paths); sort here so the
            # entry trace is deterministic against the source document.
            regions = sorted(
                self._policy.get_parallel_regions(state),
                key=self._policy.get_document_order,
            )
            for region in regions:
                self._enter_state(region)
                if self._reached_final or not self._is_running:
                    return
            return
        if self._policy.is_compound_state(state):
            children = self._policy.get_initial_children(state)
            if not children:
                # Defensive: well-formed compound has at least one child.
                self._active_leaves.append(state)
                return
            self._enter_state(children[0])
            return
        # Atomic leaf — record in the active set.
        self._active_leaves.append(state)

    def _mark_root_final_if_top_level(self, final_state: S) -> None:
        """If a `<final>` at the top of the document is entered, the engine
        terminates (W3C SCXML 3.7). `<final>` inside a parallel region only
        marks that region done — termination is governed by
        `_check_done_state_events`."""
        parent = self._policy.get_parent(final_state)
        if parent is None:
            self._reached_final = True
            self._is_running = False

    def _check_done_state_events(self) -> None:
        """W3C SCXML 3.7 — when every region of a `<parallel>` state has
        reached its `<final>`, raise `done.state.<parallel_id>`."""
        active = self.active_configuration()
        # For each active parallel state, check whether every region is
        # represented in the active set by a final leaf.
        for state in list(active):
            if not self._policy.is_parallel_state(state):
                continue
            if state in self._fired_done_state:
                continue
            regions = self._policy.get_parallel_regions(state)
            if not regions:
                continue
            all_final = True
            for region in regions:
                if not self._region_has_final_descendant(region, active):
                    all_final = False
                    break
            if all_final:
                self._fired_done_state.add(state)
                done_event = self._policy.done_state_event(state)
                if done_event is not None:
                    self.raise_internal(done_event)

    def _region_has_final_descendant(
        self, region: S, active: Set[S]
    ) -> bool:
        """True iff `region` itself or one of its descendants is a final
        state currently in `active`."""
        for s in active:
            if s != region and not self._is_proper_descendant(s, region):
                continue
            if self._policy.is_final_state(s):
                return True
        return False

    # ── Hierarchy helpers ─────────────────────────────────────────

    def _compute_exit_set(self, transition: TransitionResult[S]) -> Set[S]:
        """W3C SCXML Appendix D.2 — set of currently-active states the
        transition exits.

        For an external transition: the boundary is LCCA(source, target);
        every active state that is a proper descendant of the boundary
        (including the source's region) is exited.

        For an internal transition where target is a proper descendant of
        source: only the active descendants of `source` itself are exited
        — source survives.

        For a targetless transition: empty set (no states change).
        """
        if transition.targetless or transition.target is None:
            return set()
        source: S = (
            transition.source
            if transition.source is not None
            else self._policy.initial_state()
        )
        target: S = transition.target
        if transition.is_internal and self._is_proper_descendant(target, source):
            boundary: Optional[S] = source
            include_boundary = False
        else:
            boundary = self._find_lcca(source, target)
            include_boundary = False
        result: Set[S] = set()
        for active_state in self.active_configuration():
            if boundary is None:
                # No common ancestor — exit everything except the document
                # root scope (mapped as `boundary == None`).
                result.add(active_state)
                continue
            if active_state == boundary and not include_boundary:
                continue
            if self._is_proper_descendant(active_state, boundary):
                result.add(active_state)
        return result

    def _is_proper_descendant(self, descendant: S, ancestor: S) -> bool:
        s = self._policy.get_parent(descendant)
        while s is not None:
            if s == ancestor:
                return True
            s = self._policy.get_parent(s)
        return False

    def _find_lcca(self, source: S, target: S) -> Optional[S]:
        """W3C SCXML 5.9.2 — Lowest Common Compound Ancestor: lowest state
        that is a proper ancestor of both `source` and `target`. Returns
        None when they share no common ancestor (transition crosses the
        document root)."""
        source_ancestors: List[S] = []
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
