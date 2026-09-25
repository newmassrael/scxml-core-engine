# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""StatePolicy protocol — contract that generated state machine modules implement.

Mirrors `sce.StatePolicy[S, E]` in Go and the `StatePolicy` trait in Rust.

The division of labour is Appendix D's. The runtime owns the algorithm — which
transitions an event selects, which survive preemption, what a microstep exits
and enters and in which order — once, in `microstep.py`. A policy answers what
only the document knows: its structure as written (child states, initial
targets, `<history>` elements), which of ONE state's transitions an event
enables, and the executable content of a state or a transition. Nothing here
resolves a target, walks a hierarchy or decides what to enter.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, Dict, Generic, Optional, Sequence, TypeVar

from .microstep import EnabledTransition, EntryTarget

S = TypeVar("S")
E = TypeVar("E")

if TYPE_CHECKING:
    from .engine import Engine


class StatePolicy(ABC, Generic[S, E]):
    """W3C SCXML state machine policy.

    Generated `*_sm.py` modules subclass this and provide concrete State/Event
    enum types. The Engine drives all algorithm logic against these hooks.

    A `<history>` is identified by its string id: it is not a state, never in
    a configuration, and needs no State enum member.
    """

    # §scxml-5.10 — script-engine session id, populated by `Engine`
    # at construction. Concrete policies thread it through helper
    # methods (`_assign`, `_guard`, `_exec`, …) so each one can call
    # `IScriptEngine.evaluate_expression(self._session_id, …)`. Empty
    # string before the engine binds (e.g. during static analysis).
    _session_id: str = ""

    # Back-reference to the owning `Engine`, set in `Engine.__init__`
    # so `set_current_event` (whose protocol signature pre-dates the
    # IScriptEngine migration and takes no `engine` parameter) can
    # still reach the script engine session.
    _engine_ref: Optional[Any] = None

    # §scxml-6.4.1 — staging dict for `<invoke>` param/namelist
    # values that must seed the child's datamodel before its first
    # macrostep. The parent's `execute_pending_invokes` template
    # assigns a fresh dict on the child policy (so the class-level
    # default stays `None` and instances never share state); the
    # child's `initialize_datamodel` applies the overlay as the
    # final step so the staged values win over any default-init for
    # the same `<data>` id. Mirrors the Rust child policy's
    # `set_param_in_script_engine` ordering — there the pre-init runs
    # via `ensure_script_engine` BEFORE the data-init macros, here
    # the overlay runs AFTER because Python's Engine constructs the
    # session up front and data init is part of `initialize_datamodel`.
    _pre_init_vars: Optional[Dict[str, Any]] = None

    @abstractmethod
    def initial_state(self) -> S:
        """W3C SCXML 3.3 — a leaf the document's initial transition reaches.

        Read only as `Engine.current_state` before `initialize` has entered
        anything. What `initialize` enters is `get_document_initial_targets`,
        through the runtime's entry procedures."""

    @abstractmethod
    def get_document_initial_targets(self) -> Sequence[EntryTarget]:
        """W3C SCXML 3.2 — the target set of the document's initial
        transition, as written: one entry per token of `<scxml initial>`, or
        the first child state in document order when the attribute is absent.
        What Appendix D's interpret enters from the `<scxml>` element."""
        # §scxml-D-interpret: the initial transition of the <scxml> element.

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
    def get_state_from_name(self, name: str) -> Optional[S]:
        """Reverse lookup: the state `name` names, or `None` when this document
        declares no such state.

        Required (no default) for the reason `get_state_name` is: the mapping is
        structural, and a default could only ever answer `None`. A policy that
        had not emitted the table would then report every recorded configuration
        as unknown — which a caller reads as "this run has no history" rather
        than as a policy that is incomplete.

        This is what lets a configuration cross a process. A host can only
        record state NAMES — a journal, a wire, a file — while `Engine.enter_at`
        takes states. Without the reverse, a recorded configuration cannot be
        turned back into the argument that door asks for and resuming degrades
        to replaying from the initial state. A consumer-side table would age
        silently the moment the document gained a state; only the generator
        writes one that ages with the document.

        The round trip is an identity: `get_state_from_name(get_state_name(s))`
        is `s` for every state of the document, and a name the document does not
        carry is `None` rather than a guess.
        """

    @abstractmethod
    def get_event_name(self, event: E) -> str:
        """Human-readable name of `event`."""

    @abstractmethod
    def null_event(self) -> E:
        """W3C SCXML 3.13 — sentinel for eventless transition dispatch."""

    @abstractmethod
    def first_enabled_transition(
        self, state: S, event: E, engine: "Engine[S, E]"
    ) -> Optional[EnabledTransition[S]]:
        """W3C SCXML 3.13 — the first transition declared on `state` itself,
        in document order, that `event` enables and whose guard holds; for
        `null_event()`, the first eventless one whose guard holds. `None` when
        there is none.

        Only `state`'s own transitions: Appendix D's selectTransitions walks the
        ancestors, and the runtime does that walk. The targets are the
        `target` attribute as written — a `<history>` stays a `HistoryTarget`,
        which the runtime dereferences when it computes the entry set.

        `engine` is threaded through so guard evaluation can raise
        `error.execution` on the internal queue when a `<transition cond>`
        expression fails (W3C SCXML 3.13: "if a `cond` evaluates to an
        error the SCXML Processor MUST place error.execution on the
        internal queue and treat the cond as having the value 'false'").
        """
        # §scxml-D-selectTransitions: the per-state half of the selection.

    @abstractmethod
    def execute_entry_actions(
        self, state: S, engine: "Engine[S, E]", is_default_entry: bool
    ) -> None:
        """W3C SCXML 3.8 — what Appendix D's enterStates does for `state` once
        it is in the configuration: its `<onentry>` blocks, then — only when
        `is_default_entry` — its `<initial>` transition's executable content,
        then, for a `<final>`, the `done.state` events its entry raises.
        Actions that raise internal events do so via
        `engine.raise_internal(...)`."""
        # §scxml-D-enterStates: one state's share of the entry.

    @abstractmethod
    def execute_exit_actions(self, state: S, engine: "Engine[S, E]") -> None:
        """W3C SCXML 3.9 — run onexit actions for `state`."""

    @abstractmethod
    def execute_transition_content(
        self, state: S, transition_index: int, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 3.13 — run the executable content of `state`'s transition
        `transition_index` (Appendix D's executeTransitionContent)."""
        # §scxml-D-executeTransitionContent: one selected transition's content.

    @abstractmethod
    def get_document_order(self, state: S) -> int:
        """W3C SCXML Appendix D — the state's position in document order,
        which is also entry order and, reversed, exit order."""

    # ── Structure, as written ──────────────────────────────────────

    def is_compound_state(self, state: S) -> bool:
        """W3C SCXML 3.3 — true if `state` is a `<state>` with child states.
        A `<parallel>` answers False: this is Appendix D's
        `isCompoundState`."""
        return False

    def is_parallel_state(self, state: S) -> bool:
        """W3C SCXML 3.4 — true if `state` is a `<parallel>` element."""
        return False

    def get_child_states(self, state: S) -> Sequence[S]:
        """Appendix D's getChildStates — `state`'s `<state>`, `<parallel>` and
        `<final>` children in document order; for a `<parallel>`, its regions.
        Empty for an atomic state and a `<final>`."""
        # §scxml-D-getChildStates: the default is a state with no children.
        return ()

    def get_initial_targets(self, state: S) -> Sequence[EntryTarget]:
        """W3C SCXML 3.6 — a compound state's initial transition target, as
        written: the tokens of its `initial` attribute or of its `<initial>`
        element's transition, a `<history>` among them staying a
        `HistoryTarget`, or the first child state when the document names
        none. Empty for a state that is not compound."""
        return ()

    def get_history_states_in(self, state: S) -> Sequence[str]:
        """W3C SCXML 3.10 — string ids of every `<history>` element declared in
        `state`, in document order. Empty for a state with none."""
        return ()

    def get_history_type(self, history_id: str) -> str:
        """W3C SCXML 3.10 — `"deep"` (records the active atomic descendants)
        or `"shallow"` (records the active children)."""
        return "shallow"

    def get_history_parent(self, history_id: str) -> S:
        """W3C SCXML 3.10 — the state `history_id` is declared in.

        No neutral answer exists, so a policy whose document declares a
        `<history>` must override it; one that declares none is never asked."""
        raise NotImplementedError(
            f"{type(self).__name__} names a <history> {history_id!r} it does not declare"
        )

    def get_history_default_targets(self, history_id: str) -> Sequence[EntryTarget]:
        """W3C SCXML 3.10.2 — the history's default transition target, as
        written: its default stored state configuration."""
        return ()

    def execute_history_default_content(
        self, history_id: str, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 3.10.2 — run the executable content of the history's
        default `<transition>`. Appendix D's enterStates runs it after the
        history's parent is entered, when the history was taken with nothing
        recorded. Default no-op."""
        # §scxml-D-enterStates: owed once the history's parent is entered.

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

    def needs_script_engine(self) -> bool:
        """Whether the policy uses scripts (informational)."""
        return False

    def machine_name(self) -> str:
        """W3C SCXML 5.10 — `_name` system variable. Generated `*_sm.py`
        overrides to return the source document's `name` attribute (or
        the module's machine name when the attribute was absent)."""
        return ""

    def is_state_active(self, state_id: str, engine: "Engine[S, E]") -> bool:
        """W3C SCXML 5.9.2 — backing predicate for the script engine's
        `In(state)` callback. Generated `*_sm.py` resolves `state_id`
        against its State enum and walks the engine's active
        configuration. Empty / unknown ids return False."""
        for active in engine.active_configuration():
            if self.get_state_name(active) == state_id:
                return True
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

    # ── Invoke hooks (§scxml-6.4) ─────────────────────────────

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
        self, event_name: str, metadata, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 6.4.1 — for every active child whose `<invoke>`
        declared `autoforward="true"`, deliver `event_name` into the
        child via `Invoke.forward_event`. Default no-op.

        `metadata` is the source event's whole `EventMetadata`: §6.4
        requires the child receive an exact copy, so the name alone (or
        the name plus payload) is not enough."""

    def execute_finalize_for_child_event(
        self, event_with_meta, engine: "Engine[S, E]"
    ) -> None:
        """W3C SCXML 6.5 — run the `<finalize>` block associated with
        the invoke that produced `event_with_meta`. The hook executes
        in the parent's datamodel so the finalize body can write
        child-derived values back. Default no-op."""
