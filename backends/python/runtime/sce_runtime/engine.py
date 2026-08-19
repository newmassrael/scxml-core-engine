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

from .event import EventMetadata, EventWithMetadata, is_error_event
from .http import HttpSendRequest, HttpSendResponse
from . import io_processors
from .invoke import Invoke, PendingInvoke, create_done_invoke_event_name
from .policy import StatePolicy, TransitionResult
from .scheduler import Scheduler

S = TypeVar("S")
E = TypeVar("E")

# §scxml-C-1 / C.2 — canonical Event I/O Processor URIs surfaced on
# `_event.origintype` for events delivered via the corresponding processor.
# Re-exported from `io_processors`, which needs the same two spellings for the
# `_ioprocessors` entry names; two copies of a URI is one rename away from two
# different URIs.
SCXML_EVENT_PROCESSOR_URI = io_processors.SCXML_EVENT_PROCESSOR_URI
BASIC_HTTP_EVENT_PROCESSOR_URI = io_processors.BASIC_HTTP_EVENT_PROCESSOR_URI

# How many links an `error.*` chain may have before the engine stops feeding
# it — see `Engine.error_cascade_events`.
#
# §scxml-3.12.2 says what to do with an error event nothing matches. It does
# not say what to do when something *does* match it and that handler fails too:
# the failure raises the same error, the same transition answers it, and the
# machine has no way out. Nothing in the specification bounds that, so the
# number is this engine's to choose, and it is the same hundred the Rust, Go
# and C++ engines use for the sibling case of a macrostep that cannot finish.
#
# This engine is where the cost was measured, on 2026-08-19: a two-line
# document turned 37,000 links a second, `initialize()` never returned, the
# configuration never moved and `is_running()` stayed true.
MAX_ERROR_CASCADE_DEPTH = 100


class Engine(Generic[S, E]):
    """Generic SCXML engine bound to a concrete StatePolicy.

    Single-threaded; callers needing concurrency must guard with a lock.
    """

    def __init__(self, policy: StatePolicy[S, E], script_engine: "IScriptEngine") -> None:
        self._policy = policy
        # §scxml-5.10 — each engine owns one script-engine session.
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
        # §scxml-C-2-3: inbound BasicHTTP endpoint serving this machine,
        # declared by the deployment before `initialize()`. The address
        # belongs to whoever runs the listener, so the engine takes it from
        # here rather than guessing one. Empty means no such endpoint is
        # deployed, and no BasicHTTP entry is published in `_ioprocessors`.
        self.basic_http_access_uri: str = ""
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
        # §scxml-3.3: active configuration tracked as the ordered list
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
        # §scxml-3.1.2 — events taken off the external queue that no
        # transition matched, and the most recent of them. See
        # `discarded_external_events`.
        self._discarded_external_events: int = 0
        self._last_discarded_event: Optional[E] = None
        # §scxml-3.12.2 — `error.*` events this engine raised that no
        # transition matched, and the most recent of them. See
        # `unhandled_error_events`.
        self._unhandled_error_events: int = 0
        self._last_unhandled_error: Optional[E] = None
        # §scxml-3.12.2 — the drain is executing a transition an `error.*`
        # event selected, which is the state in which a newly raised error
        # is a link in a chain rather than a first failure. See
        # `error_cascade_events`.
        self._handling_error_event: bool = False
        self._error_cascade_depth: int = 0
        self._error_cascade_events: int = 0
        self._last_error_cascade_event: Optional[E] = None
        # §scxml-3.7 — set of `<parallel>` states for which a
        # `done.state.<id>` event has already been raised this run, so a
        # second region reaching `<final>` does not re-fire it.
        self._fired_done_state: Set[S] = set()
        # §scxml-3.11 — per-history-id snapshot of the active
        # configuration taken when the owning compound exits. Keyed by
        # the history element's string id so the engine can replay it on
        # the next entry through that history state without needing a
        # State enum member for the history element itself.
        self._history: Dict[str, List[S]] = {}
        # §scxml-6.2 — delayed-event scheduler + virtual clock.
        # Callers advance time via `advance_time(ms)`; the scheduler is
        # pull-based so the engine remains single-threaded.
        self._scheduler: Scheduler[E] = Scheduler()
        self._now_ms: int = 0
        # Auto-generated sendid counter for `<send>` actions without an
        # explicit `id` attribute (§scxml-6.2 — implementations may
        # generate any unique id; we use a deterministic counter so
        # traces remain reproducible).
        self._auto_send_seq: int = 0
        # §scxml-5.3 — states whose local `<datamodel>` has already
        # been initialised. Only consulted under `binding="late"`; under
        # the default early binding the policy initialises every state's
        # data up front and this set stays empty.
        self._initialized_states_data: Set[S] = set()
        # §scxml-6.4 — pending `<invoke>` queue: filled at onentry by
        # `_policy.defer_invokes_on_entry`, drained after the current
        # macrostep settles by `_start_pending_invokes`. Defer-execute
        # is the spec-mandated ordering (entry actions of a compound
        # observe a stable configuration BEFORE any child runs).
        self._pending_invokes: List[PendingInvoke] = []
        # §scxml-6.4 — active invoke instances keyed on `invoke_id`.
        # Populated by `_policy.execute_pending_invokes`; the engine
        # drives lifecycle (tick + drain + cancel) but never instantiates
        # the concrete `Invoke` itself.
        self._active_invokes: Dict[str, Invoke[E]] = {}
        # §scxml-6.4 — per-invoke "done.invoke.<id> already raised"
        # flag. Persistent across `_drive_active_children` calls so the
        # event is raised exactly once even when the child becomes done
        # synchronously during a `forward_event` (parent send to
        # `target="#_<id>"`): the flag is also set by the init-time
        # emission path in `execute_pending_invokes`, and cleared when
        # the invoke is canceled or the engine stops. Mirrors the Rust
        # codegen's `pending_done_invoke_<id>` per-field bool in
        # `tools/codegen/templates/rust/invoke_methods.rs.jinja2`.
        self._done_invoke_emitted: Dict[str, bool] = {}
        # §scxml-C-2 — BasicHTTP Event I/O Processor dispatcher. The
        # engine never links against an HTTP library; callers register a
        # callback via `set_http_send_callback` that receives an
        # `HttpSendRequest` and returns an optional `HttpSendResponse`.
        # Used by `perform_http_send`; `None` means fire-and-forget for
        # any HTTP send (matches Rust's "no callback configured" path).
        self._http_send_callback: Optional[
            Callable[[HttpSendRequest], Optional[HttpSendResponse]]
        ] = None
        # §scxml-5.5 + 6.3.1 — donedata stashed when a top-level
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
        # §scxml-C-1-1 / §scxml-C-2-3 — bind `_sessionid` / `_name` /
        # `_ioprocessors` into the script-engine session before any datamodel
        # `<data>` is registered. The entries come from the same helper every
        # other backend uses, so a machine reads the same entry names and the
        # same addresses whichever one runs it. The BasicHTTP entry appears
        # only once `basic_http_access_uri` names a deployed endpoint —
        # advertising the processor because the document happens to `<send>`
        # through it would publish an address nothing answers on.
        self._script_engine.setup_system_variables(
            self._session_id,
            self._policy.machine_name(),
            io_processors.build(self._session_id, self.basic_http_access_uri),
        )
        # §scxml-5.9.2 — register the `In(state)` predicate against
        # this engine's active configuration. The Lua engine surfaces
        # `In` as a global function in the session scope.
        self._script_engine.set_state_query_callback(
            self._session_id,
            lambda state_id: self._policy.is_state_active(state_id, self),
        )
        # §scxml-5.3 early binding: datamodel initialisation runs before
        # any onentry action fires.
        self._policy.initialize_datamodel(self)
        # §scxml-3.3: enter the parser-resolved initial leaf by
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
        # §scxml-D-mainEventLoop — hand over to the outer loop. The macrostep
        # completes on eventless transitions and internal events, then any
        # `<invoke>` deferred during onentry runs, and only then is anything
        # taken off the external queue — so an autoforward child is live for
        # every event `<onentry>` queued on the way in.
        self._run_main_event_loop()

    def stop(self) -> None:
        self._is_running = False
        # §scxml-6.4 — engine shutdown cancels every active invoke
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

    def discarded_external_events(self) -> int:
        """W3C SCXML 3.1.2 — how many events this engine took off the
        external queue and discarded because no transition in any active
        state matched them.

        Discarding is what the clause requires. This is the part the
        clause does not cover: the host that queued the event cannot
        otherwise tell that outcome from a handled one, because a self
        transition, a targetless internal transition and a discard all
        leave the configuration alone. Comparing the count across a drive
        turns "the machine ignored what I sent" into something the
        program can see.

        The C++ Interpreter has answered this all along
        (`processEvent`'s `TransitionResult.success` and
        `getStatistics().failedTransitions`); this is the generated
        engines' side of the same question.

        Counts external-queue events only — an internal `<raise>` that
        matches nothing has both its ends inside the document.
        """
        return self._discarded_external_events

    def last_discarded_event(self) -> Optional[E]:
        """The most recent event `discarded_external_events` counted, or
        `None` while that count is zero.

        A count says something went nowhere; this says which thing did,
        which is the question a host debugging a stalled supervisor
        actually has.
        """
        return self._last_discarded_event

    def unhandled_error_events(self) -> int:
        """W3C SCXML 3.12.2 — how many `error.*` events this engine raised
        that no transition in any active state answered.

        The clause requires the processor to signal its own failures as
        `error.*` events on the internal queue, and says in the same breath
        that "they are ignored if no transition is found that matches them".
        Being ignored is the clause. Being unable to say it happened is not,
        and the difference matters to exactly one party: the host, which did
        not write the document, cannot see the failure anywhere in the
        configuration, and is the only one positioned to do something about
        it. A supervisor driving a machine whose `<assign>` silently fails
        every round reads `is_running() == True` and a plausible state
        forever.

        This is the sibling of `discarded_external_events`, and the two are
        deliberately separate counts rather than one. That one stops at the
        external queue because an author's unmatched `<raise>` has both ends
        inside the document; an error event's sender is the engine, so the
        same reasoning does not reach it. An author's `<raise>` that matches
        nothing is still not counted here.

        An error the document *did* answer is not counted either — the
        document dealt with it, and its handling is visible in the
        configuration the host can already read. What this counts is only the
        silent case.

        The C++ Interpreter has answered this all along, through
        `getLastStateMachineError()` and the message it raises
        `error.execution` with; this is the generated engines' side of it.
        """
        return self._unhandled_error_events

    def last_unhandled_error(self) -> Optional[E]:
        """The most recent `error.*` event `unhandled_error_events` counted,
        or `None` while that count is zero.

        Which error it was narrows a silent failure from "something in this
        machine is broken" to a class: `error.execution` is the document's own
        executable content failing, `error.communication` is a `<send>` or
        `<invoke>` that could not reach its target — two different repairs,
        and a count alone separates neither.
        """
        return self._last_unhandled_error

    def error_cascade_events(self) -> int:
        """How many `error.*` events this engine refused to queue because the
        error handler that raised them had been failing for
        `MAX_ERROR_CASCADE_DEPTH` links running.

        §scxml-3.12.2 says an unmatched error event is ignored, and
        `unhandled_error_events` is that case. This is its opposite and its
        worse half: the document *does* match the error, and the handler fails
        the same way every time. The failure raises `error.execution`, the same
        transition answers it, and the drain never empties. Nothing in the
        clause covers it — it bounds what happens to an error nobody wants, not
        an error everybody wants and nobody can handle.

        Left to run, that is not a hang: it is a core at 100% forever. This
        engine is where it was measured, on 2026-08-19 — 37,000 links a second
        while the configuration never moved and `is_running()` stayed true,
        which is the exact reading an unattended supervisor takes as healthy.

        A document that fails five hundred times cleanly counts zero here: the
        chain is measured from *handler to handler*, not from failure to
        failure, and any other internal event resets it. Nothing is discarded
        that a working document would have processed.
        """
        return self._error_cascade_events

    def last_error_cascade_event(self) -> Optional[E]:
        """The most recent `error.*` event `error_cascade_events` refused, or
        `None` while that count is zero.

        Which error it was names the repair: `error.execution` is a handler
        whose own executable content fails, `error.communication` one that
        answers an unreachable target by talking to it again.
        """
        return self._last_error_cascade_event

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
        self._run_main_event_loop()

    def raise_internal(self, event: E, metadata: Optional[EventMetadata] = None) -> None:
        """W3C SCXML 4.4 `<raise>` — enqueue an internal event (drained
        before externals). When no metadata is supplied the event type
        defaults to `"internal"` so guards reading `_event.type` see
        the correct W3C 5.10 classification (`<raise>` events are
        internal-origin, distinct from `external` and `platform`).

        An `error.*` event raised while an error handler is running is refused
        once the chain reaches `MAX_ERROR_CASCADE_DEPTH` — see
        `error_cascade_events` for why the engine is the one that has to stop
        it. Only the engine's own error events are refused: an author's
        `<raise>` inside an error handler is the document doing its job and
        rides the queue like any other."""
        # §scxml-3.12.2 names the error events this refuses; the clause itself
        # is silent on a handler that fails, which is why the ceiling is a
        # choice this engine documents rather than a rule it implements.
        if self._handling_error_event and is_error_event(
            self._policy.get_event_name(event)
        ):
            self._error_cascade_depth += 1
            if self._error_cascade_depth >= MAX_ERROR_CASCADE_DEPTH:
                # No log line: this runtime has no logging surface at all, and
                # the sibling engines' one-time message is a convenience over
                # the counter, not the signal. `error_cascade_events` is the
                # signal, and it is readable here exactly as it is there.
                self._error_cascade_events += 1
                self._last_error_cascade_event = event
                return
        self._internal_queue.append(
            EventWithMetadata(
                event=event,
                metadata=metadata or EventMetadata(event_type="internal"),
            )
        )

    # ── BasicHTTP Event I/O Processor (§scxml-C-2) ─────────────

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

    # ── <send> / <cancel> / scheduler API (§scxml-6.2) ─────────

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

    def deliver_to_child_session(
        self, child_session_id: str, event_name: str, data: Any = ""
    ) -> bool:
        """W3C SCXML C.1 — deliver an event addressed to a child's published
        location.

        Each invoked child owns a session id, and that id is what the child's
        `_ioprocessors` entry names, so a `<send>` whose target decodes to one
        of them is addressed to that child rather than to this machine. A
        `False` return means the address names no live child of ours and the
        event takes the normal external path — the routing half of C.1 is what
        makes the published location a usable target rather than a string that
        merely compares equal.
        """
        if not child_session_id:
            return False
        for invoke in self._active_invokes.values():
            if invoke.origin() == child_session_id and not invoke.is_done():
                invoke.forward_event(
                    event_name, EventMetadata(event_type="external", data=data)
                )
                return True
        return False

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
        # §scxml-6.2 — dispatch the due events one macrostep apart rather
        # than appending them together. `<cancel>` drops an event that has
        # not been delivered yet, and a step that jumped over several
        # deadlines holds several: appending them all first makes every
        # later one undroppable before the earlier one's transitions have
        # run. On this virtual clock the host's step size alone decided it,
        # so the same document reached a state it forbids at
        # `advance_time(250)` and not at `advance_time(150)` (measured
        # 2026-08-19).
        while True:
            entry = self._scheduler.pop_due(self._now_ms)
            if entry is None:
                break
            # §scxml-C-1: scheduler drain is the SCXML processor's
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
            if not self._is_running or self._reached_final:
                break
            self._run_main_event_loop()
            if not self._is_running or self._reached_final:
                return
        # §scxml-6.2 — `_drive_active_children` and the scheduler
        # drain both append onto `_external_queue`; the parent's
        # macrostep must run whenever either produced an event so the
        # parent observes `done.invoke.<id>` and any child-raised
        # `<send target="#_parent">` on the same tick (test347, test236).
        if self._is_running and not self._reached_final:
            self._run_main_event_loop()

    def time_until_next_scheduled_ms(self) -> Optional[int]:
        """How far virtual time must move for this machine's next
        scheduled event to come due. `0` means one is due now; `None`
        means the scheduler is empty and no clock movement is owed.

        `needs_event_scheduler()` tells a host *which* entry point to
        drive the machine with. This tells it *how far*, and a host that
        cannot ask has only one move left: pick a step size. That guess
        is not free — on this backend the host owns the clock outright,
        so a step coarser than the document's delays does not merely
        arrive late, it steps over deadlines the document distinguishes
        between. The generated W3C wrappers move time in fixed 50 ms
        steps for exactly this reason: nothing told them any better."""
        next_due = self._scheduler.peek_next_due_ms()
        if next_due is None:
            return None
        return max(0, next_due - self._now_ms)

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

    def _run_main_event_loop(self) -> None:
        """The outer loop, and the only place the public entry points express
        macrostep semantics.

        Appendix D names the external queue exactly once per iteration and
        it is *after* ``invoke(inv)``::

            while running:
                while running and not macrostepDone:   # eventless + internal only
                    ... selectEventlessTransitions() / internalQueue.dequeue() ...
                for state in statesToInvoke.sort(entryOrder):
                    for inv in state.invoke.sort(documentOrder):
                        invoke(inv)
                statesToInvoke.clear()
                if not internalQueue.isEmpty(): continue
                externalEvent = externalQueue.dequeue()

        Folding the external drain into the macrostep-completion loop
        instead is a different algorithm, not a shorter one. The invoked
        children do not exist yet while that drain runs, so everything
        ``<onentry>`` queued for this session on the way in is consumed with
        no ``autoforward`` child to receive it — and there is no later point
        at which it is delivered. One external event per iteration for the
        same reason: a state entered by event N's transition must have its
        invokes started before N+1 comes off the queue.
        """
        while True:
            # §scxml-D-mainEventLoop: complete the macrostep on eventless
            # transitions and internal events alone.
            self._process_internal_queue()
            if not self._is_running or self._reached_final:
                return
            # §scxml-6.4: invokes for states entered during this macrostep.
            self._start_pending_invokes()
            # §scxml-D-mainEventLoop: invoking may have raised internal error
            # events (and a child that completed during its own initialise may
            # already have raised `done.invoke`); handle them before touching
            # the external queue.
            if self._internal_queue:
                continue
            if not self._external_queue:
                return
            self._process_next_external_event()

    def _process_internal_queue(self) -> None:
        """Eventless transitions first, then one internal event, until neither
        is available."""
        # §scxml-D-selectEventlessTransitions: eventless transitions are
        # exhausted before an internal event is taken, and the pair repeats
        # until neither is available.
        while self._is_running and not self._reached_final:
            self._drain_eventless()
            if self._reached_final or not self._is_running:
                return
            if not self._internal_queue:
                # The queue emptied, so the chain — refused or merely finished
                # — is over. A machine whose next macrostep starts a new one
                # starts it from zero, and the count of what was refused stays
                # where the host reads it.
                self._error_cascade_depth = 0
                return
            # §scxml-3.12.2 — the processor raises `error.*` into this queue
            # and the clause says they "are ignored if no transition is found
            # that matches them". Ignoring them is the clause; staying silent
            # about it is not. `discarded_external_events` deliberately stops
            # at the external queue because an unmatched `<raise>` has both
            # ends inside the document — but the sender of an error event is
            # this engine, so that reasoning does not reach it. The host never
            # wrote the document, cannot see the failure in the configuration,
            # and is the only party able to act on it.
            #
            # The dispatch runs first and unconditionally: it is what processes
            # every internal event, and folding it into the condition below
            # would skip it for everything that is not an error.
            evt = self._internal_queue.popleft()
            # An error raised from here on is raised *by an error handler*,
            # which is the one situation the engine cannot leave to the
            # document: the handler that failed is the same one that will
            # answer the failure. The flag is what `raise_internal` reads to
            # tell that apart from a first failure, and it is cleared before
            # anything else can run so a chain cannot be attributed to the
            # wrong event.
            is_error = is_error_event(self._policy.get_event_name(evt.event))
            # The chain is not ended by the drain doing something else. An
            # earlier draft reset the depth on every non-error event, which
            # reads as the careful choice and is the opposite: a handler that
            # raises its own event before failing — a document that logs, then
            # fails, which is most of them — leaves the queue alternating
            # `tick, error, tick, error…`, and each `tick` put the ceiling back
            # out of reach. The count needs no such guard, because it only ever
            # rises while an error handler is running.
            self._handling_error_event = is_error
            selected = self._dispatch(evt)
            self._handling_error_event = False
            if not selected and is_error:
                self._unhandled_error_events += 1
                self._last_unhandled_error = evt.event

    def _process_next_external_event(self) -> None:
        """Take exactly one event off the external queue, run the preliminary
        ``<finalize>`` / autoforward step against it, then select transitions.

        Both preliminary steps key off *which queue the event came from*, not
        off the event's name or its ``_event.type`` classification: Appendix D
        applies them to `externalQueue.dequeue()`'s result and to nothing
        else, so expressing that as the caller is what makes it exact.
        """
        # §scxml-D-mainEventLoop — one external event per iteration of the
        # outer loop, taken after the macrostep has completed.
        evt = self._external_queue.popleft()
        # §scxml-6.5 — `<finalize>` runs before transition selection for
        # events originating from invoked children, so the finalize body can
        # write child-derived values back into the parent datamodel that
        # subsequent guards then read.
        if evt.metadata.invoke_id:
            self._policy.set_current_event(evt.event, evt.metadata)
            self._policy.execute_finalize_for_child_event(evt, self)
        # §scxml-6.4.1 — autoforward into every active child marked
        # `autoforward="true"`, before transition selection, so the child
        # observes the event in the same iteration the parent does.
        self._route_to_child(evt)
        # §scxml-3.1.2 — discarding an event no transition matched is the
        # rule; being unable to say so is not part of the rule. The host
        # that queued this event is the one party that cannot see the
        # outcome (a discard leaves the configuration exactly as a self
        # transition does) and the party that got the event wrong.
        # Counted for the external queue only: an internal `<raise>` that
        # matches nothing has both its ends inside the document.
        if not self._dispatch(evt):
            self._discarded_external_events += 1
            self._last_discarded_event = evt.event

    def _drain_eventless(self) -> None:
        """W3C SCXML 3.13: fire all enabled eventless transitions until none remain."""
        null_evt = self._policy.null_event()
        while self._is_running and not self._reached_final:
            transitions = self._select_transitions(null_evt)
            if not transitions:
                return
            self._take_transitions(transitions)

    def _dispatch(self, evt: EventWithMetadata[E]) -> bool:
        # §scxml-5.10 — bind `_event` into the datamodel before the
        # microstep so transition guards and action expressions can
        # read `_event.name`, `_event.data`, etc. Eventless transitions
        # do not update `_event` (handled separately in
        # `_drain_eventless`), matching W3C 5.10.2 which only refreshes
        # `_event` when the processor "selects an event for
        # processing".
        self._policy.set_current_event(evt.event, evt.metadata)
        # §scxml-6.5 / 6.4.1 — the `<finalize>` and autoforward
        # preliminary steps belong to the external dequeue and run in
        # `_process_next_external_event`, which is the only caller that
        # can know the event came off that queue.
        transitions = self._select_transitions(evt.event)
        if not transitions:
            # §scxml-3.1.2 — "If no transition matches in any state, the
            # event is discarded." Reported rather than merely done, so
            # the external dequeue can count it; see
            # `discarded_external_events`.
            return False
        self._take_transitions(transitions)
        return True

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
        """Walk from `leaf` upward, return the first enabled transition.
        Stamps the result's `source` if the policy did not."""
        # §scxml-D-getProperAncestors: the chain walked here is the proper
        # ancestors of `leaf`, innermost first, which is the order the
        # algorithm requires for selecting the transition that wins.
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
        """`removeConflictingTransitions`.

        Two transitions conflict if their exit sets intersect. The one
        with the deeper source (or earlier document order on a tie) wins;
        the other is dropped.
        """
        # §scxml-D-removeConflictingTransitions: intersecting exit sets are
        # the conflict test, and the deeper source wins.
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

        # §scxml-3.11 — snapshot history for every exiting compound
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
            # §scxml-6.4 — cancel any active invokes owned by the
            # exiting state (and drop any still-pending ones queued by
            # an earlier macrostep iteration that hadn't started yet).
            # The policy delegate knows which state owns which invoke.
            self._policy.cancel_invokes_for_state(s, self)
            # §scxml-3.9 (test409): the just-exited state must drop
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

        # §scxml-3.7 — raise `done.state.<parent>` for every parallel
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

        # §scxml-3.11 — replay history if available; otherwise the
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

        `addDescendantStatesToEnter` — when any
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
        # §scxml-D-addDescendantStatesToEnter: a `<parallel>` anywhere on the
        # entry path pulls in every sibling region through its default initial
        # chain, which the fan-out below performs.
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
        """For each region of `parallel_state` not already active and not on
        the target's path, enter via the region's default initial chain."""
        # §scxml-D-addDescendantStatesToEnter: the sibling regions of an
        # entered `<parallel>` are added through their default initial chain.
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
        # §scxml-6.4 — every `<invoke>` on the entered state defers
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
            # §scxml-3.3: `initial="s1 s2"` (multi-target initial) pre-
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
            # §scxml-3.4 — enter every region in document order. The
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
            # §scxml-3.3: multi-target initial — walk through ancestors
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

        `exitInterpreter` — the final state's own `<onexit>` actions still
        execute as the engine winds down (test236: a child invoke's
        `<final><onexit><send target="#_parent">` must reach the parent).
        The final state stays in `_active_leaves` so `current_state`
        post-termination still reports the reached final."""
        # §scxml-D-exitInterpreter: the exit actions of the state that
        # terminated the interpreter still run before the engine stops.
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
        initialise raise `done.invoke.<id>` and any child-side
        `<send target="#_parent">` onto the parent's external queue;
        `_run_main_event_loop` is the caller and picks them up on its
        next iteration."""
        if not self._pending_invokes:
            return
        self._policy.execute_pending_invokes(self)

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
                    origin=invoke.origin(),
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
        """W3C SCXML 6.4.1 — autoforward an external event into every
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
        # §scxml-6.4 requires an exact copy, so the whole metadata goes
        # with the name — the child must see the same `_event.data`,
        # `_event.origin`, `_event.sendid` and `_event.invokeid` the parent saw.
        self._policy.forward_to_autoforward_children(name, evt.metadata, self)

    # ── Hierarchy helpers ─────────────────────────────────────────

    def _compute_exit_set(self, transition: TransitionResult[S]) -> Set[S]:
        """Set of currently-active states the transition exits.

        For an external transition: the boundary is LCCA(source, target);
        every active state that is a proper descendant of the boundary
        (including the source's region) is exited.

        For an internal transition where target is a proper descendant of
        source: only the active descendants of `source` itself are exited
        — source survives.

        For a targetless transition: empty set (no states change).
        """
        # §scxml-D-GlobalVariables: the exit set is computed against the
        # interpreter's current configuration, the global the algorithm names.
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
