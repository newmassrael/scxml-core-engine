# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCXML execution engine for AOT-generated Python state machines.

Atomic α scope (locked): atomic states + basic transitions + onentry/onexit.
Compound / parallel / history / invoke / scheduler land in β / γ.
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
        self._policy.execute_entry_actions(self._current_state)
        if self._policy.is_final_state(self._current_state):
            self._reached_final = True
            self._is_running = False
            return
        self._drain_eventless()

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
        """W3C SCXML Appendix D.2 macrostep loop: internal first, then external,
        with eventless transitions drained between each event."""
        while self._is_running:
            self._drain_eventless()
            if self._reached_final or not self._is_running:
                return
            evt = self._dequeue()
            if evt is None:
                return
            self._dispatch(evt)
            if self._reached_final:
                return

    def _drain_eventless(self) -> None:
        """Take all eventless (null-event) transitions that are enabled.

        Atomic α: each pass tries one null transition from current state; loop
        until none enabled. β will extend to ancestor-chain selection.
        """
        null_evt = self._policy.null_event()
        while self._is_running and not self._reached_final:
            result = self._policy.select_transition(self._current_state, null_evt)
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
        result = self._policy.select_transition(self._current_state, evt.event)
        if result is None:
            return
        self._take_transition(result)

    def _take_transition(self, result: TransitionResult[S]) -> None:
        """W3C SCXML 3.13 — execute exit, action, entry in order."""
        source = self._current_state
        if result.targetless or result.target is None:
            self._policy.execute_transition_action(source, result.transition_index)
            return
        target = result.target
        if not result.is_internal:
            self._policy.execute_exit_actions(source)
        self._policy.execute_transition_action(source, result.transition_index)
        self._current_state = target
        self._policy.execute_entry_actions(target)
        if self._policy.is_final_state(target):
            self._reached_final = True
            self._is_running = False
