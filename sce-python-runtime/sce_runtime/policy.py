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
    """

    target: Optional[S]
    transition_index: int
    is_internal: bool = False
    targetless: bool = False
    source: Optional[S] = None


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

    def get_initial_children(self, state: S) -> List[S]:
        """W3C SCXML 3.6 — for a compound `state`, the targets named by its
        `<initial>` element (or the first child in document order). Empty
        list when `state` is atomic. β returns at most one entry (no
        parallel regions); γ may return multiple for parallel entry."""
        return []

    def initialize_datamodel(self, engine: "Engine[S, E]") -> None:
        """W3C SCXML 5.3 — early-binding datamodel initialisation. Called
        exactly once by `Engine.initialize` before any onentry action
        fires. Default no-op for documents without a `<datamodel>`."""

    def get_document_order(self, state: S) -> int:
        """W3C SCXML Appendix D — used for deterministic ordering."""
        return 0

    def needs_script_engine(self) -> bool:
        """Whether the policy uses scripts (informational)."""
        return False
