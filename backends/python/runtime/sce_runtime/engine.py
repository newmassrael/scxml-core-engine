# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCXML execution engine for AOT-generated Python state machines.

Atomic α: atomic states + onentry/onexit + basic guarded transitions.
Atomic β: compound entry chain (W3C SCXML 3.3 / 3.6 / 3.13), early-binding
datamodel init (W3C 5.3), ancestor-chain transition selection
(W3C Appendix D.2), LCCA-based exit/entry boundary (W3C 5.9.2), and
`<raise>` (W3C 4.4) wired through `raise_internal`.
Atomic γ-1: `<parallel>` regions with active-set tracking, atomic multi-
transition microsteps with W3C SCXML 3.13 conflict resolution, and
`done.state.<parent>` events raised when all regions of a `<parallel>`
reach `<final>`.
Atomic γ-2: `<history>` (shallow + deep) — snapshot pre-exit
configuration into `_history[history_id]`, replay it on the next entry
through that history state, falling back to the document's default
target (and its default `<transition>` actions) when no snapshot exists.
Atomic γ-3a: `<log>`, `<if>`/`<elseif>`/`<else>`, and `<foreach>` lower
into Python control flow inside the per-state action handlers.
Atomic γ-3b: `<send>` (immediate + delayed) and `<cancel>` ride a pull
scheduler with cancel-by-sendid (W3C SCXML 6.2 + 6.2.2). Virtual time
advances via `Engine.advance_time(ms)`; callers needing wall-clock
behaviour drive this from their own dispatch loop.
Atomic γ-5: `binding="late"` — when the policy reports late binding,
state-local `<datamodel>` blocks are initialised on the first entry of
each owning state instead of up-front at engine.initialize.
"""

from __future__ import annotations

import uuid
from collections import deque
from typing import Any, Callable, Dict, Generic, List, Optional, Set, TypeVar

from .event import EventMetadata, EventWithMetadata
from .http import HttpSendRequest, HttpSendResponse
from .invoke import Invoke, PendingInvoke, create_done_invoke_event_name
from .policy import StatePolicy, TransitionResult
from .scheduler import Scheduler

S = TypeVar("S")
E = TypeVar("E")

# W3C SCXML C.1 / C.2 — canonical Event I/O Processor URIs surfaced on
# `_event.origintype` for events delivered via the corresponding processor.
SCXML_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
BASIC_HTTP_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"


class Engine(Generic[S, E]):
    """Generic SCXML engine bound to a concrete StatePolicy.

    Single-threaded; callers needing concurrency must guard with a lock.
    """

    def __init__(self, policy: StatePolicy[S, E], script_engine: "IScriptEngine") -> None:
        self._policy = policy
        # W3C SCXML 5.10 — each engine owns one script-engine session.
        # The id is allocated up front so the policy can reference it
        # immediately (helpers like `_assign` / `_guard` thread it back
        # into the IScriptEngine without an Optional check). The
        # session itself is created here so generated policies can
        # declare variables during `initialize_datamodel` without a
        # later `create_session` step. Mirrors the Rust / Go / Kotlin /
        # C++ pattern where Engine construction implicitly owns one
        # script-engine session.
        self._session_id: str = uuid.uuid4().hex
        # Path B+ RFC Q1=(d) Python=duck-typed ref; Q5=(a) instance member;
        # Q2=(a) mandatory-when-needed: every Engine now ships with its own
        # engine instance, mirroring Kotlin's constructor-injected pattern.
        self._script_engine = script_engine
        self._script_engine.create_session(self._session_id)
        # Back-ref so the policy's helpers (which run as instance
        # methods on the generated module) can find the engine's
        # session without an extra parameter on every call site.
        # The `set_current_event` hook reads `_engine_ref` to reach
        # the script engine since its protocol signature predates
        # the IScriptEngine migration; the back-ref creates a
        # legitimate cycle that Python's GC collects on engine
        # destruction.
        policy._session_id = self._session_id
        policy._engine_ref = self
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
        # W3C SCXML 3.11 — per-history-id snapshot of the active
        # configuration taken when the owning compound exits. Keyed by
        # the history element's string id so the engine can replay it on
        # the next entry through that history state without needing a
        # State enum member for the history element itself.
        self._history: Dict[str, List[S]] = {}
        # W3C SCXML 6.2 — delayed-event scheduler + virtual clock.
        # Callers advance time via `advance_time(ms)`; the scheduler is
        # pull-based so the engine remains single-threaded.
        self._scheduler: Scheduler[E] = Scheduler()
        self._now_ms: int = 0
        # Auto-generated sendid counter for `<send>` actions without an
        # explicit `id` attribute (W3C SCXML 6.2 — implementations may
        # generate any unique id; we use a deterministic counter so
        # traces remain reproducible).
        self._auto_send_seq: int = 0
        # W3C SCXML 5.3 — states whose local `<datamodel>` has already
        # been initialised. Only consulted under `binding="late"`; under
        # the default early binding the policy initialises every state's
        # data up front and this set stays empty.
        self._initialized_states_data: Set[S] = set()
        # W3C SCXML 6.4 — pending `<invoke>` queue: filled at onentry by
        # `_policy.defer_invokes_on_entry`, drained after the current
        # macrostep settles by `_start_pending_invokes`. Defer-execute
        # is the spec-mandated ordering (entry actions of a compound
        # observe a stable configuration BEFORE any child runs).
        self._pending_invokes: List[PendingInvoke] = []
        # W3C SCXML 6.4 — active invoke instances keyed on `invoke_id`.
        # Populated by `_policy.execute_pending_invokes`; the engine
        # drives lifecycle (tick + drain + cancel) but never instantiates
        # the concrete `Invoke` itself.
        self._active_invokes: Dict[str, Invoke[E]] = {}
        # W3C SCXML 6.4 — per-invoke "done.invoke.<id> already raised"
        # flag. Persistent across `_drive_active_children` calls so the
        # event is raised exactly once even when the child becomes done
        # synchronously during a `forward_event` (parent send to
        # `target="#_<id>"`): the flag is also set by the init-time
        # emission path in `execute_pending_invokes`, and cleared when
        # the invoke is canceled or the engine stops. Mirrors the Rust
        # codegen's `pending_done_invoke_<id>` per-field bool in
        # `tools/codegen/templates/rust/invoke_methods.rs.jinja2`.
        self._done_invoke_emitted: Dict[str, bool] = {}
        # W3C SCXML C.2 — BasicHTTP Event I/O Processor dispatcher. The
        # engine never links against an HTTP library; callers register a
        # callback via `set_http_send_callback` that receives an
        # `HttpSendRequest` and returns an optional `HttpSendResponse`.
        # Used by `perform_http_send`; `None` means fire-and-forget for
        # any HTTP send (matches Rust's "no callback configured" path).
        self._http_send_callback: Optional[
            Callable[[HttpSendRequest], Optional[HttpSendResponse]]
        ] = None
        # W3C SCXML 5.5 + 6.3.1 — donedata stashed when a top-level
        # `<final>` is entered. The invoking parent's `ScxmlInvoke`
        # reads this via `getattr(child, "done_data", None)` so it can
        # lift the value onto `done.invoke.<id>._event.data`. None when
        # the document never set donedata or the final wasn't reached.
        self.done_data: Any = None

    # ── Lifecycle ──────────────────────────────────────────────────

    def initialize(self) -> None:
        """Enter the initial configuration and drive the macrostep loop until stable."""
        if self._is_running:
            return
        self._is_running = True
        # W3C SCXML 5.10 — bind `_sessionid` / `_name` / `_ioprocessors`
        # into the script-engine session before any datamodel `<data>`
        # is registered. The SCXML Event I/O Processor is always
        # present (C.1: "every SCXML Processor MUST support the SCXML
        # Event I/O Processor"); BasicHTTP is added only when the
        # generated policy reports a `<send type=BasicHTTP>` need.
        # Mirrors `tools/codegen/templates/rust/scriptengine_helpers.rs.jinja2`
        # which registers `vec!["scxml".to_string()]` unconditionally.
        io_processors: List[str] = ["scxml"]
        if self._policy.needs_http_send():
            io_processors.append(BASIC_HTTP_EVENT_PROCESSOR_URI)
        self._script_engine.setup_system_variables(
            self._session_id,
            self._policy.machine_name(),
            io_processors,
        )
        # W3C SCXML 5.9.2 — register the `In(state)` predicate against
        # this engine's active configuration. The Lua engine surfaces
        # `In` as a global function in the session scope.
        self._script_engine.set_state_query_callback(
            self._session_id,
            lambda state_id: self._policy.is_state_active(state_id, self),
        )
        # W3C SCXML 5.3 early binding: datamodel initialisation runs before
        # any onentry action fires.
        self._policy.initialize_datamodel(self)
        # W3C SCXML 3.3: enter the parser-resolved initial leaf by
        # walking its ancestor chain root-first. At each compound on
        # the path the child on the way to the leaf wins over the
        # compound's default initial child (test388 — root
        # `<scxml initial="s012">` lands at s012, not s01's default
        # s011); at each parallel ancestor the path-side region
        # follows the leaf path while the OTHER regions enter via
        # their default `get_initial_children` (test413). Mirrors
        # Rust's `build_entry_chain` + per-state generated parallel
        # recursion at `backends/rust/runtime/src/engine.rs`.
        initial_leaf = self._policy.initial_state()
        self._enter_initial_path(initial_leaf)
        if self._reached_final or not self._is_running:
            return
        self._process_queues()
        # W3C SCXML 6.4 — once the initialise macrostep is stable, any
        # `<invoke>` deferred during onentry runs. Newly raised events
        # (e.g. `done.invoke.<id>` from a child that completed during
        # its own initialise) re-enter the macrostep loop so the parent
        # observes them before returning.
        self._start_pending_invokes()

    def stop(self) -> None:
        self._is_running = False
        # W3C SCXML 6.4 — engine shutdown cancels every active invoke
        # so child schedulers stop driving and external resources are
        # released. Mirrors Go's `Engine.Stop` cancelling all children.
        for invoke in list(self._active_invokes.values()):
            invoke.cancel()
        self._active_invokes.clear()
        self._done_invoke_emitted.clear()
        # Release the script-engine session so its Lua runtime memory
        # is collectable. Matches Rust's `Drop` impl on Engine.
        if self._session_id:
            self._script_engine.destroy_session(self._session_id)
            self._session_id = ""

    def __del__(self) -> None:
        try:
            if getattr(self, "_session_id", ""):
                self._script_engine.destroy_session(self._session_id)
                self._session_id = ""
        except Exception:
            pass

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

    @property
    def policy(self) -> StatePolicy[S, E]:
        """W3C SCXML 6.4 — used by `Invoke` impls to resolve a child
        engine's name→Event lookup. Stays read-only from outside the
        engine; the runtime never mutates the policy itself."""
        return self._policy

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
        """W3C SCXML 4.4 `<raise>` — enqueue an internal event (drained
        before externals). When no metadata is supplied the event type
        defaults to `"internal"` so guards reading `_event.type` see
        the correct W3C 5.10 classification (`<raise>` events are
        internal-origin, distinct from `external` and `platform`)."""
        self._internal_queue.append(
            EventWithMetadata(
                event=event,
                metadata=metadata or EventMetadata(event_type="internal"),
            )
        )

    # ── BasicHTTP Event I/O Processor (W3C SCXML C.2) ─────────────

    def set_http_send_callback(
        self,
        callback: Optional[
            Callable[[HttpSendRequest], Optional[HttpSendResponse]]
        ],
    ) -> None:
        """W3C SCXML C.2 — register the dispatcher used by generated
        `<send type="BasicHTTPEventProcessor">` action arms. The
        callback receives the resolved request payload (target URL,
        event name, content, namelist/`<param>` map, sendid) and
        returns the inbound response (`event_name` + `event_data`) the
        engine should inject back onto the external queue. Returning
        `None` is the fire-and-forget path. Passing `None` here clears
        the callback so subsequent sends become no-ops. Mirrors
        `sce_rust_runtime::Engine::set_http_send_callback`."""
        self._http_send_callback = callback

    def perform_http_send(
        self,
        target: str,
        event_name: str,
        content: str,
        params: Dict[str, List[str]],
        send_id: str,
    ) -> None:
        """W3C SCXML C.2 — dispatch a BasicHTTP send through the
        registered callback. Builds the `HttpSendRequest`, invokes the
        callback (no-op when no callback is configured), and if the
        response carries an `event_name` that resolves on the running
        policy's enum, injects the event onto the external queue with
        `event_data` bound as `_event.data` (W3C 5.10.3). Mirrors
        `sce_rust_runtime::Engine::perform_http_send` semantics."""
        if self._http_send_callback is None:
            return
        request = HttpSendRequest(
            target=target,
            event_name=event_name,
            content=content,
            params=params,
            send_id=send_id,
        )
        response = self._http_send_callback(request)
        if response is None or not response.event_name:
            return
        self.send_external_by_name(
            response.event_name,
            data=response.event_data,
            sendid=send_id,
        )

    # ── <send> / <cancel> / scheduler API (W3C SCXML 6.2) ─────────

    def send_external_by_name(
        self,
        event_name: str,
        data: Any = "",
        sendid: str = "",
        invoke_id: str = "",
        origin: str = "",
        origin_type: str = "",
    ) -> None:
        """W3C SCXML 5.10 + 6.4 — enqueue an external event addressed by
        its wire name (`done.invoke.<id>`, `error.execution`, or any
        child-raised `<send target="#_parent">` name). Unknown names
        drop silently; matches W3C 5.10.1 ("if no transition is enabled
        the event is lost"). Used by the runtime to lift child-raised
        events onto the parent's external queue with the originating
        invoke's metadata intact."""
        if not self._is_running:
            return
        event = self._policy.get_event_from_name(event_name)
        if event is None:
            # W3C SCXML 3.13 / 6.3.1 — fall back through dot-token
            # prefixes so a wire name like `done.invoke._invoke_0`
            # surfaces on a document that only declares the generic
            # `done.invoke` descriptor. Token-prefix matching at the
            # transition layer then catches both forms uniformly.
            parts = event_name.split(".") if event_name else []
            while event is None and len(parts) > 1:
                parts.pop()
                event = self._policy.get_event_from_name(".".join(parts))
            if event is None:
                return
        metadata = EventMetadata(
            send_id=sendid,
            event_type="external",
            data=data,
            invoke_id=invoke_id,
            origin=origin,
            origin_type=origin_type,
        )
        self._external_queue.append(EventWithMetadata(event=event, metadata=metadata))

    def send_external(self, event: E, sendid: str = "", data: Any = "") -> None:
        """W3C SCXML 6.2 — enqueue an external event with no delay.
        Drained by the macrostep loop AFTER any pending internal events.
        Unlike `raise_internal`, this does NOT immediately drive the
        loop; it queues so that the current handler can finish first.
        `data` carries the marshalled `<param>`/`<content>`/namelist
        payload (W3C SCXML 5.10) — a `dict` for `<param>`/namelist
        builds, the literal/expr result for `<content>`, or `""` when
        the send had no payload.

        W3C SCXML C.1: this path is the SCXML Event I/O Processor's
        own external delivery, so `_event.origintype` defaults to the
        SCXML processor URI (test253, test352)."""
        metadata = EventMetadata(
            send_id=sendid,
            event_type="external",
            data=data,
            origin_type=SCXML_EVENT_PROCESSOR_URI,
        )
        self._external_queue.append(EventWithMetadata(event=event, metadata=metadata))

    def schedule_send(
        self, event: E, delay_ms: int, sendid: str = "", data: Any = ""
    ) -> None:
        """W3C SCXML 6.2 — schedule `event` for delivery `delay_ms` after
        the current virtual time. Callers later move time forward via
        `advance_time(...)`; the scheduler is then drained into the
        external queue. `data` is preserved across the scheduler delay
        and surfaces on `_event.data` when the event is delivered."""
        if delay_ms <= 0:
            self.send_external(event, sendid, data)
            return
        self._scheduler.schedule(self._now_ms + delay_ms, sendid, event, data)

    def cancel_send(self, sendid: str) -> None:
        """W3C SCXML 6.2.2 — drop a previously scheduled `<send>` by id.
        No-op on empty sendid (matches W3C: cancel requires an id)."""
        self._scheduler.cancel(sendid)

    def advance_time(self, ms: int) -> None:
        """Move virtual time forward by `ms` milliseconds and drain every
        scheduled event whose deadline has now passed. The drained
        events go onto the external queue and the macrostep loop runs
        until stable. Active invoke children are also ticked: their
        schedulers share the parent's virtual-time progression so a
        child `<send delay>` fires at the same absolute time on both
        sides."""
        if ms < 0:
            raise ValueError("advance_time requires a non-negative delta")
        self._now_ms += ms
        # W3C SCXML 6.4 — propagate the time delta to every active
        # child so its scheduler stays in lock-step with the parent's
        # virtual clock. Done BEFORE draining the parent scheduler so
        # any child-raised `<send target="#_parent">` lands in the
        # parent's external queue on the same tick the parent processes
        # its own schedules.
        self._drive_active_children(ms)
        for entry in self._scheduler.drain_due(self._now_ms):
            # W3C SCXML C.1: scheduler drain is the SCXML processor's
            # delayed-delivery path — origintype mirrors send_external.
            metadata = EventMetadata(
                send_id=entry.sendid,
                event_type="external",
                data=entry.data,
                origin_type=SCXML_EVENT_PROCESSOR_URI,
            )
            self._external_queue.append(
                EventWithMetadata(event=entry.event, metadata=metadata)
            )
        # W3C SCXML 6.2 — `_drive_active_children` and the scheduler
        # drain both append onto `_external_queue`; the parent's
        # macrostep must run whenever either produced an event so the
        # parent observes `done.invoke.<id>` and any child-raised
        # `<send target="#_parent">` on the same tick (test347, test236).
        if (self._external_queue or self._internal_queue) and self._is_running and not self._reached_final:
            self._process_queues()
        # W3C SCXML 6.4 — pending invokes deferred during the macrostep
        # land after the queue is drained; their initialise-time
        # outputs feed back into the macrostep on the next iteration.
        self._start_pending_invokes()

    @property
    def now_ms(self) -> int:
        """Current virtual time in milliseconds since engine construction."""
        return self._now_ms

    def _next_auto_sendid(self) -> str:
        """Generate a deterministic sendid for `<send>` actions that did
        not specify an explicit `id`."""
        self._auto_send_seq += 1
        return f"_auto_send_{self._auto_send_seq}"

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
        # W3C SCXML 5.10 — bind `_event` into the datamodel before the
        # microstep so transition guards and action expressions can
        # read `_event.name`, `_event.data`, etc. Eventless transitions
        # do not update `_event` (handled separately in
        # `_drain_eventless`), matching W3C 5.10.2 which only refreshes
        # `_event` when the processor "selects an event for
        # processing".
        self._policy.set_current_event(evt.event, evt.metadata)
        # W3C SCXML 6.5 — `<finalize>` runs before transition selection
        # for events originating from invoked children, so the
        # finalize body can write child-derived values back into the
        # parent datamodel that subsequent guards then read.
        if evt.metadata.invoke_id:
            self._policy.execute_finalize_for_child_event(evt, self)
        # W3C SCXML 6.4.6 — autoforward external events into every
        # active child marked `autoforward="true"`. Done before
        # transition selection so the child observes the event in the
        # same iteration the parent does.
        self._route_to_child(evt)
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
            result = self._policy.select_transition(state, event, self)
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

        # W3C SCXML 3.11 — snapshot history for every exiting compound
        # BEFORE running onexit actions. The active configuration at this
        # point is still the pre-exit one, which is exactly what shallow /
        # deep history records.
        self._snapshot_history(combined_exit)

        # W3C 3.13: exit in reverse document order (deepest descendants
        # leave first). Inside the same document-order rank, exit order
        # is unspecified — we use document order descending for determinism.
        exit_list = sorted(
            combined_exit,
            key=lambda s: -self._policy.get_document_order(s),
        )
        for s in exit_list:
            self._policy.execute_exit_actions(s, self)
            # W3C SCXML 6.4 — cancel any active invokes owned by the
            # exiting state (and drop any still-pending ones queued by
            # an earlier macrostep iteration that hadn't started yet).
            # The policy delegate knows which state owns which invoke.
            self._policy.cancel_invokes_for_state(s, self)
            # W3C SCXML 3.9 (test409): the just-exited state must drop
            # out of the active configuration BEFORE the next state's
            # onexit runs, so an outer `<onexit>` reading `In(child)`
            # observes the child as already inactive. Mirrors the per-
            # state active-set mutation Rust / C++ do as part of their
            # `execute_exit_actions` loop.
            self._active_leaves = [
                leaf
                for leaf in self._active_leaves
                if leaf != s and not self._is_proper_descendant(leaf, s)
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
        initial chain (with parallel branching at any `<parallel>`).

        When `transition.history_id` is set and a snapshot exists, the
        snapshot replaces `target` so the engine restores the
        pre-exit configuration (W3C SCXML 3.11). When no snapshot
        exists, the engine falls back to the parser-resolved default
        target and runs the history element's default-transition
        actions (also W3C 3.11).
        """
        target: S = transition.target  # type: ignore[assignment]
        source: S = (
            transition.source if transition.source is not None else target
        )

        # W3C SCXML 3.11 — replay history if available; otherwise the
        # parser-resolved default target stands. Default-transition
        # actions only run when falling back to the default.
        history_entries: Optional[List[S]] = None
        if transition.history_id is not None:
            saved = self._history.get(transition.history_id)
            if saved:
                history_entries = list(saved)
            else:
                self._policy.execute_history_default_actions(
                    transition.history_id, self
                )

        # Find the boundary (LCCA). Internal transitions into a descendant
        # of source use `source` itself; otherwise use the standard LCCA.
        if transition.is_internal and self._is_proper_descendant(target, source):
            boundary: Optional[S] = source
        else:
            boundary = self._find_lcca(source, target)

        if history_entries:
            # Enter every saved state, sorted by document order so the
            # ancestor compound's onentry runs once even when multiple
            # leaves are restored (each `_enter_target_step` call
            # individually enters the ancestors that aren't already
            # active).
            for saved_state in sorted(
                history_entries, key=self._policy.get_document_order
            ):
                self._enter_target_step(saved_state, boundary)
                if self._reached_final or not self._is_running:
                    return
            return

        self._enter_target_step(target, boundary)

    def _enter_target_step(self, target: S, boundary: Optional[S]) -> None:
        """Enter ancestors of `target` between `boundary` and `target`,
        then descend into `target` via `_enter_state`. Shared by the
        normal-target and history-restore paths so they cannot drift.

        W3C SCXML Appendix D.2 `addDescendantStatesToEnter` — when any
        ancestor on the entry path is a `<parallel>`, every sibling
        region (not the one on the target's path) must also be entered
        via its default initial chain. This covers both:
          - parallel ancestor freshly entered as part of `upward`
            (target descends into a not-yet-active parallel ancestor);
          - parallel ancestor that is the transition boundary itself
            (sibling-to-sibling transition under a parallel: the
            parallel stays active, but its other regions were exited
            and must be re-entered — test403c, test364).
        Mirrors the Rust template's `is_parallel_state(*state)`
        re-entry fan-out at
        `tools/codegen/templates/rust/conflict_resolution.rs.jinja2`."""
        upward: List[S] = []
        state: Optional[S] = target
        while state is not None and state != boundary:
            upward.append(state)
            state = self._policy.get_parent(state)
        already_active = self.active_configuration()
        # Build the path-child map so parallel ancestors can identify
        # the on-path region (which the next loop iteration enters) vs
        # the sibling regions (entered here via default descent).
        path_child: Dict[S, S] = {}
        for i in range(len(upward) - 1):
            path_child[upward[i + 1]] = upward[i]
        # Boundary itself participates in path_child so the fan-out
        # below knows which child of the boundary lies on the target's
        # path when the boundary is a still-active parallel.
        if boundary is not None and upward:
            path_child[boundary] = upward[-1]
        # Enter from boundary-child down to target, then descend.
        for s in reversed(upward[1:]):
            if s in already_active:
                continue
            self._run_entry(s)
            if self._policy.is_final_state(s):
                self._active_leaves.append(s)
                self._mark_root_final_if_top_level(s)
                return
            if self._policy.is_parallel_state(s):
                self._fanout_parallel_siblings(s, path_child, already_active)
                if self._reached_final or not self._is_running:
                    return
        if (
            boundary is not None
            and self._policy.is_parallel_state(boundary)
        ):
            self._fanout_parallel_siblings(boundary, path_child, already_active)
            if self._reached_final or not self._is_running:
                return
        self._enter_state(target)

    def _fanout_parallel_siblings(
        self, parallel_state: S, path_child: Dict[S, S], already_active: Set[S]
    ) -> None:
        """W3C SCXML Appendix D.2 — for each region of `parallel_state`
        not already active and not on the target's path, enter via the
        region's default initial chain."""
        on_path = path_child.get(parallel_state)
        regions = sorted(
            self._policy.get_parallel_regions(parallel_state),
            key=self._policy.get_document_order,
        )
        for region in regions:
            if region == on_path:
                continue
            if region in already_active:
                continue
            self._enter_state(region)
            if self._reached_final or not self._is_running:
                return

    def _run_entry(self, state: S) -> None:
        """W3C SCXML 5.3 + 3.8 — fire late-binding local datamodel init
        (once per state) then run the state's onentry actions. Shared by
        ancestor entry in `_enter_target_step` and target entry in
        `_enter_state` so the two paths cannot drift."""
        if (
            self._policy.is_late_binding()
            and state not in self._initialized_states_data
        ):
            self._policy.init_state_datamodel(state, self)
            self._initialized_states_data.add(state)
        self._policy.execute_entry_actions(state, self)
        # W3C SCXML 6.4 — every `<invoke>` on the entered state defers
        # to the engine's pending list. Actual child instantiation
        # happens after the macrostep settles via
        # `_start_pending_invokes` so onentry observes a stable config
        # before any child runs.
        self._policy.defer_invokes_on_entry(state, self)

    def _enter_initial_path(self, leaf: S) -> None:
        """W3C SCXML 3.3 — enter the explicit document-level initial
        leaf, branching parallel regions but following the path-side
        child at every compound ancestor. Companion to `_enter_state`
        (which descends via default `get_initial_children` at every
        compound); separated because the document's `<scxml initial=>`
        attribute makes the path explicit only at the leaf, while
        every compound between root and leaf still needs path-aware
        descent so the explicit leaf is reached rather than the
        compound's parser-default first child."""
        upward: List[S] = []
        state: Optional[S] = leaf
        while state is not None:
            upward.append(state)
            state = self._policy.get_parent(state)
        path = list(reversed(upward))
        path_child: Dict[S, S] = {
            path[i]: path[i + 1] for i in range(len(path) - 1)
        }
        self._descend_initial_path(path[0], path_child)

    def _descend_initial_path(self, state: S, path_child: Dict[S, S]) -> None:
        is_final = self._policy.is_final_state(state)
        is_parallel = self._policy.is_parallel_state(state)
        is_compound = self._policy.is_compound_state(state)
        if is_final or not (is_parallel or is_compound):
            self._active_leaves.append(state)
        self._run_entry(state)
        if is_final:
            self._mark_root_final_if_top_level(state)
            return
        if is_parallel:
            regions = sorted(
                self._policy.get_parallel_regions(state),
                key=self._policy.get_document_order,
            )
            on_path = path_child.get(state)
            for region in regions:
                if region == on_path:
                    self._descend_initial_path(region, path_child)
                else:
                    # Sibling regions follow their parser-resolved default
                    # entry chain (which `_enter_state` walks).
                    self._enter_state(region)
                if self._reached_final or not self._is_running:
                    return
            return
        if is_compound:
            on_path = path_child.get(state)
            if on_path is not None:
                self._descend_initial_path(on_path, path_child)
                return
            if self._enter_from_history_snapshot(state):
                return
            children = self._policy.get_initial_children(state)
            if not children:
                self._active_leaves.append(state)
                return
            # W3C SCXML 3.3: `initial="s1 s2"` (multi-target initial) pre-
            # resolves to a leaf list that may descend through several
            # ancestors. `_enter_target_step` walks the full ancestor
            # chain from the leaf back up to `state`; the parallel
            # fan-out it performs uses each region's parser-rewritten
            # default initial (test364), which the parser has already
            # set to the sibling target leaf.
            self._enter_target_step(children[0], state)
            return

    def _enter_state(self, state: S) -> None:
        """Recursively enter `state`: run its entry actions, then descend
        through the appropriate child (single initial child for a compound,
        every region for a `<parallel>`).

        W3C SCXML 3.8: a state is part of the active configuration BEFORE
        its `<onentry>` runs — so a guard like `In(s)` evaluated inside
        `s`'s own onentry returns True (test411). Atomic / final leaves
        are appended here ahead of `_run_entry`; compound and parallel
        states acquire their active status automatically once any of
        their descendants land in `_active_leaves` (active_configuration
        walks parents)."""
        is_final = self._policy.is_final_state(state)
        is_parallel = self._policy.is_parallel_state(state)
        is_compound = self._policy.is_compound_state(state)
        if is_final or not (is_parallel or is_compound):
            self._active_leaves.append(state)
        self._run_entry(state)
        if is_final:
            self._mark_root_final_if_top_level(state)
            return
        if is_parallel:
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
        if is_compound:
            if self._enter_from_history_snapshot(state):
                return
            children = self._policy.get_initial_children(state)
            if not children:
                # Defensive: well-formed compound has at least one child.
                self._active_leaves.append(state)
                return
            # W3C SCXML 3.3: multi-target initial — walk through ancestors
            # via `_enter_target_step` so the parallel fan-out hits each
            # region's parser-rewritten default (test364). Single-target
            # case naturally degenerates to a one-leg `_enter_state` call.
            self._enter_target_step(children[0], state)
            return

    def _enter_from_history_snapshot(self, state: S) -> bool:
        """W3C SCXML 3.11 — when `state`'s `<initial>` element targets a
        `<history>` pseudo-state and a snapshot exists in `_history`,
        descend into the snapshot's saved leaves (sorted by document
        order) and return `True`. Returns `False` when there is no
        history-targeting `<initial>` or the snapshot is empty — the
        caller then falls back to the parser-resolved default initial
        children. The history element's default `<transition>` actions
        are emitted by `execute_entry_actions` and guarded by the same
        snapshot-emptiness check, so the entry-action side stays in
        sync with the descent decision."""
        history_id = self._policy.get_initial_history_id(state)
        if history_id is None:
            return False
        snapshot = self._history.get(history_id)
        if not snapshot:
            return False
        for saved in sorted(snapshot, key=self._policy.get_document_order):
            self._enter_target_step(saved, state)
            if self._reached_final or not self._is_running:
                return True
        return True

    def _snapshot_history(self, exiting: Set[S]) -> None:
        """W3C SCXML 3.11 — for every compound about to exit, record its
        history. `shallow` history captures the directly-active child of
        the compound; `deep` history captures the active leaf descendant."""
        if not exiting:
            return
        active = self.active_configuration()
        for compound in exiting:
            history_ids = self._policy.get_history_states_in(compound)
            if not history_ids:
                continue
            for history_id in history_ids:
                kind = self._policy.get_history_type(history_id)
                if kind == "deep":
                    snapshot = [
                        leaf
                        for leaf in self._active_leaves
                        if self._is_proper_descendant(leaf, compound)
                    ]
                else:
                    # shallow: the direct child(ren) of compound that
                    # have an active descendant.
                    snapshot = [
                        state
                        for state in active
                        if self._policy.get_parent(state) == compound
                    ]
                if snapshot:
                    self._history[history_id] = snapshot

    def _mark_root_final_if_top_level(self, final_state: S) -> None:
        """If a `<final>` at the top of the document is entered, the engine
        terminates (W3C SCXML 3.7). `<final>` inside a parallel region only
        marks that region done — termination is governed by
        `_check_done_state_events`.

        W3C SCXML Appendix D.2 `exitInterpreter` — the final state's
        own `<onexit>` actions still execute as the engine winds down
        (test236: a child invoke's `<final><onexit><send target=
        "#_parent">` must reach the parent). The final state stays in
        `_active_leaves` so `current_state` post-termination still
        reports the reached final."""
        parent = self._policy.get_parent(final_state)
        if parent is None:
            self._policy.execute_exit_actions(final_state, self)
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

    # ── Invoke drivers (W3C SCXML 6.4) ────────────────────────────

    def _start_pending_invokes(self) -> None:
        """W3C SCXML 6.4 — instantiate every invoke deferred during the
        current macrostep. The policy hook walks `_pending_invokes`,
        clears it, and inserts the resulting `Invoke` instances into
        `_active_invokes`. Children that complete during their own
        initialise re-enter the macrostep loop via the parent's
        external queue so the parent observes `done.invoke.<id>` and
        any child-raised `<send target="#_parent">` synchronously."""
        if not self._pending_invokes:
            return
        self._policy.execute_pending_invokes(self)
        if (
            (self._external_queue or self._internal_queue)
            and self._is_running
            and not self._reached_final
        ):
            self._process_queues()

    def _drive_active_children(self, ms: int) -> None:
        """W3C SCXML 6.4 — tick every active child, propagating the
        same virtual-time delta the parent received so their schedulers
        stay synchronised, then promote any parent-bound events onto
        the parent's external queue. Children that reach their final
        configuration during this drive raise `done.invoke.<id>` once
        and stay in the active map (marked done) until the owning
        state exits and `cancel_invokes_for_state` removes them."""
        if not self._active_invokes:
            return
        # Iterate over a snapshot so cancellation during the loop
        # (e.g. parent transition triggered by a child-raised event)
        # cannot mutate the dict we are walking.
        for invoke_id, invoke in list(self._active_invokes.items()):
            already_done = invoke.is_done()
            if not already_done and ms > 0:
                # Advance child clock to the parent's new wall time so
                # any child `<send delay>` due at this tick promotes
                # into the child's external queue.
                self._advance_child_time(invoke, ms)
            invoke.tick()
            for event_name, data in invoke.drain_events():
                self.send_external_by_name(
                    event_name,
                    data=data,
                    invoke_id=invoke_id,
                    origin_type=SCXML_EVENT_PROCESSOR_URI,
                )
            # W3C SCXML 6.3.1 — lift the child's terminal donedata onto
            # `done.invoke.<id>._event.data`. Gated on the persistent
            # `_done_invoke_emitted` map (not a local `was_done` snapshot)
            # so the event is raised exactly once even when the child
            # became done synchronously during a parent `<send target=
            # "#_<id>">` (its `forward_event` runs the child's macrostep
            # to completion before this driver is next entered).
            if invoke.is_done() and not self._done_invoke_emitted.get(invoke_id, False):
                self.send_external_by_name(
                    create_done_invoke_event_name(invoke_id),
                    data=invoke.done_data() or "",
                    invoke_id=invoke_id,
                )
                self._done_invoke_emitted[invoke_id] = True

    def _advance_child_time(self, invoke: Invoke[E], ms: int) -> None:
        """Forward a virtual-time delta to a child engine if the
        underlying `Invoke` exposes one. `ScxmlInvoke.child` returns
        the wrapped child engine; HTTP / other invoke kinds don't
        carry their own clock so the call is skipped silently."""
        child = getattr(invoke, "child", None)
        if child is None:
            return
        advance = getattr(child, "advance_time", None)
        if advance is None:
            return
        advance(ms)

    def _route_to_child(self, evt: EventWithMetadata[E]) -> None:
        """W3C SCXML 6.4.6 — autoforward an external event into every
        active child whose `<invoke autoforward="true"/>` requested it.
        Skips internally-raised events (W3C 6.4.6 explicitly forwards
        only externally-triggered ones) and platform `#_…` events."""
        if evt.metadata.event_type != "external":
            return
        if not self._active_invokes:
            return
        name = self._policy.get_event_name(evt.event)
        if not name:
            return
        self._policy.forward_to_autoforward_children(name, evt.metadata.data, self)

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
