# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""StatePolicy protocol — contract that generated state machine modules implement.

Mirrors `sce.StatePolicy[S, E]` in Go and the `StatePolicy` trait in Rust.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import TYPE_CHECKING, Generic, List, Optional, TypeVar

S = TypeVar("S")
E = TypeVar("E")

if TYPE_CHECKING:
    from .engine import Engine


@dataclass
class TransitionResult(Generic[S]):
    """Outcome of select_transition: either a transition to take or None.

    The runtime invokes `apply_action` (or the policy's own helper) on the
    transition's action payload; the runtime does not inspect the payload
    shape directly. For atomic-state SMs without parallel/history, `target`
    is the destination leaf. `is_internal` distinguishes W3C 3.13 internal
    transitions from external ones. `targetless` is True for transitions
    with no `target` attribute (action-only transitions that do not change
    the configuration). `source` is the state the transition was matched on
    — required for compound bubbling because the source may be an ancestor
    of `Engine.current_state` (W3C SCXML Appendix D.2).

    `history_id` is the string id of a `<history>` element when the
    original `<transition>` targeted history (W3C SCXML 3.11). The parser
    pre-resolves `target` to the history's default-leaf so the field is
    populated only as a signal: if the runtime engine has a snapshot for
    `history_id`, it enters the snapshot in place of `target`; otherwise
    it enters `target` and runs the default-transition actions.
    """

    target: Optional[S]
    transition_index: int
    is_internal: bool = False
    targetless: bool = False
    source: Optional[S] = None
    history_id: Optional[str] = None


class StatePolicy(ABC, Generic[S, E]):
    """W3C SCXML state machine policy.

    Generated `*_sm.py` modules subclass this and provide concrete State/Event
    enum types. The Engine drives all algorithm logic against these hooks.
    """

    @abstractmethod
    def initial_state(self) -> S:
        """W3C SCXML 3.3 — the initial leaf state at engine startup."""

    @abstractmethod
    def is_final_state(self, state: S) -> bool:
        """W3C SCXML 3.7 — whether `state` is a `<final>` element."""

    @abstractmethod
    def get_parent(self, state: S) -> Optional[S]:
        """W3C SCXML 3.3 — parent state in the document hierarchy, or None for root children."""

    @abstractmethod
    def get_state_name(self, state: S) -> str:
        """Human-readable name of `state`."""

    @abstractmethod
    def get_event_name(self, event: E) -> str:
        """Human-readable name of `event`."""

    @abstractmethod
    def null_event(self) -> E:
        """W3C SCXML 3.13 — sentinel for eventless transition dispatch."""

    @abstractmethod
    def select_transition(self, state: S, event: E) -> Optional[TransitionResult[S]]:
        """W3C SCXML 3.13 — pick the enabled transition for (state, event), if any.

        Returns None if no transition is enabled. The runtime invokes this
        once per state in the ancestor chain (leaf first, then upward) so the
        generated implementation should match only transitions declared on
        the supplied `state` itself — not on its ancestors.
        """

    @abstractmethod
    def execute_entry_actions(self, state: S, engine: "Engine[S, E]") -> None:
        """W3C SCXML 3.8 — run onentry actions for `state`. Actions that raise
        internal events do so via `engine.raise_internal(...)`."""

    @abstractmethod
    def execute_exit_actions(self, state: S, engine: "Engine[S, E]") -> None:
        """W3C SCXML 3.9 — run onexit actions for `state`."""

    @abstractmethod
    def execute_transition_action(
        self, state: S, transition_index: int, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 3.13 — run the action payload of a transition."""

    # ── Optional hooks ─────────────────────────────────────────────

    def is_compound_state(self, state: S) -> bool:
        """W3C SCXML 3.3 — true if `state` has child states."""
        return False

    def is_parallel_state(self, state: S) -> bool:
        """W3C SCXML 3.4 — true if `state` is a `<parallel>` element."""
        return False

    def get_parallel_regions(self, state: S) -> List[S]:
        """W3C SCXML 3.4 — child regions of a `<parallel>` state in document
        order. Empty list when `state` is not parallel."""
        return []

    def done_state_event(self, parallel_state: S) -> Optional[E]:
        """W3C SCXML 3.7 — the `done.state.<id>` event raised when every
        region of `parallel_state` has reached `<final>`. Returns `None`
        when the document declares no transitions waiting on this event
        (in which case the codegen omits the corresponding `Event` enum
        member)."""
        return None

    def get_initial_children(self, state: S) -> List[S]:
        """W3C SCXML 3.6 — for a compound `state`, the targets named by its
        `<initial>` element (or the first child in document order). Empty
        list when `state` is atomic. β returns at most one entry; γ keeps
        single-child semantics for ordinary compounds (parallel branching
        is handled by `get_parallel_regions`)."""
        return []

    def get_history_states_in(self, compound: S) -> List[str]:
        """W3C SCXML 3.11 — string ids of every `<history>` element whose
        `parent` is `compound`. Returned in document order so the engine
        snapshots them deterministically on compound exit. Empty for
        states with no nested history."""
        return []

    def get_history_type(self, history_id: str) -> str:
        """W3C SCXML 3.11 — `"shallow"` (records the directly-active child)
        or `"deep"` (records the leaf descendant). Empty/unknown for ids
        that are not history states."""
        return "shallow"

    def execute_history_default_actions(
        self, history_id: str, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 3.11 — run the action body of the history's default
        `<transition>` when no snapshot is available and the engine falls
        back to the default target. Default no-op."""

    def initialize_datamodel(self, engine: "Engine[S, E]") -> None:
        """W3C SCXML 5.3 — root datamodel + (early-binding) all state-local
        datamodels. Called exactly once by `Engine.initialize` before any
        onentry action fires. Late-binding documents init state-local data
        on first entry of each owning state via `init_state_datamodel`
        instead."""

    def init_state_datamodel(self, state: S, engine: "Engine[S, E]") -> None:
        """W3C SCXML 5.3 — state-local `<datamodel>` initialisation. Under
        late binding the engine invokes this exactly once per state on
        its first entry. Under early binding `initialize_datamodel` has
        already run every state's init at startup so the engine does not
        call this hook."""

    def is_late_binding(self) -> bool:
        """W3C SCXML 5.3 — `binding="late"` on the document root. False
        for the default (`binding="early"`)."""
        return False

    def get_document_order(self, state: S) -> int:
        """W3C SCXML Appendix D — used for deterministic ordering."""
        return 0

    def needs_script_engine(self) -> bool:
        """Whether the policy uses scripts (informational)."""
        return False

    def set_current_event(self, event: E, metadata) -> None:
        """W3C SCXML 5.10 — bind `_event` into the datamodel for the
        duration of the current microstep. The engine calls this once
        per externally-triggered or internally-raised event, before
        transition selection runs, so guards (`<transition cond="...">`)
        and action expressions (`<assign expr="_event.data.foo">`) can
        read the event's name / type / send id / payload. Default
        no-op so generated policies opt in; concrete `*_sm.py`
        emits a binding into `self._ns["_event"]`."""

    # ── Invoke hooks (W3C SCXML 6.4) ─────────────────────────────

    def get_event_from_name(self, event_name: str) -> Optional[E]:
        """W3C SCXML 5.10 — resolve a wire-format event name (`done.foo`,
        `error.execution`, …) to the policy's concrete Event enum
        member. Used by the runtime to lift child-raised and external
        events onto the parent's queue. Default `None` (no lookup
        table); generated policies for any SM with `<invoke>` or
        external sends override against the module-level
        `_EVENT_BY_NAME` dictionary."""
        return None

    def defer_invokes_on_entry(
        self, state: S, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 6.4 — queue a `PendingInvoke` on `engine` for every
        `<invoke>` declared on `state`. The engine drains the queue
        after the current macrostep settles so the child observes a
        stable parent configuration before it starts. Default no-op."""

    def cancel_invokes_for_state(
        self, state: S, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 6.4 — invoked just after `execute_exit_actions`
        for a state that owned `<invoke>` elements. The hook cancels
        any active children and drops their entries from
        `engine._active_invokes`; still-pending entries that have not
        started yet are pruned from `engine._pending_invokes`. Default
        no-op (states without invokes never reach this hook
        non-trivially)."""

    def execute_pending_invokes(self, engine: "Engine[S, E]") -> None:
        """W3C SCXML 6.4 — drain `engine._pending_invokes` by
        instantiating each `<invoke>`'s child policy + engine, wrapping
        the pair in an `Invoke`, calling `Invoke.start(engine)`, and
        installing the result in `engine._active_invokes`. Generated
        code emits one branch per invoke id. Default no-op."""

    def forward_to_autoforward_children(
        self, event_name: str, data, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 6.4.6 — for every active child whose `<invoke>`
        declared `autoforward="true"`, deliver `event_name` into the
        child via `Invoke.forward_event`. Default no-op."""

    def execute_finalize_for_child_event(
        self, event_with_meta, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 6.5 — run the `<finalize>` block associated with
        the invoke that produced `event_with_meta`. The hook executes
        in the parent's datamodel so the finalize body can write
        child-derived values back. Default no-op."""
