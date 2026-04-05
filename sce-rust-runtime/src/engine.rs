// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! [`Engine<P>`]: the execution engine parameterized on a [`StatePolicy`].
//!
//! 1:1 port of C++ `SCE::Static::StaticExecutionEngine<StatePolicy>` from
//! `sce/include/static/StaticExecutionEngine.h`. Method names, behavior, and
//! field layout mirror C++ exactly. When reading alongside the C++ source,
//! every public method here maps to the C++ method of the same name (with
//! `snake_case` conversion).
//!
//! ## Threading model
//!
//! Single-threaded. `Engine<P>` is `Send` but **not** `Sync`. The microstep
//! loop assumes exclusive mutable access. Users needing multi-threaded access
//! wrap in `Arc<Mutex<Engine<P>>>`.
//!
//! ## Safety invariants
//!
//! The engine uses three scoped `unsafe` blocks to split-borrow `self.policy`
//! from the rest of `self` when dispatching policy methods that take both
//! `&mut self` (the policy) and `&mut Engine<Self>` (the engine). Each site
//! is documented with a safety comment. Generated code never writes `unsafe`.
//!
//! ## Phase 1 scope
//!
//! - Lifecycle: `new`, `initialize`, `step`, `tick`, `stop`, `is_running`
//! - State queries: `get_current_state`, `get_active_states`, `is_in_final_state`
//! - Event submission: `raise`, `raise_external`, `process_event`
//! - Scheduler stubs: `schedule_event`, `cancel_event`, `has_ready_events`
//! - Hierarchical transition: `handle_hierarchical_transition` (150 lines ported from C++)
//!
//! Phases 2-4 expand scheduler, HTTP send, invoke support, and the full
//! parallel state processing machinery.

use std::time::{Duration, Instant};

use crate::event::{EventMetadata, EventType, EventWithMetadata};
use crate::helpers::event_queue::EventQueueManager;
use crate::helpers::{hierarchy, state_policy_concepts as concepts};
use crate::http::HttpSendRequest;
use crate::policy::StatePolicy;
use crate::sce_log_debug;

/// Minimal scheduler stub for Phase 1.
///
/// 1:1 API parity with C++ `SCE::PullScheduler<EventType>`, but the Phase 1
/// implementation stores delayed events as immediately-ready — suitable for
/// tests that do not rely on real time. Phase 4 replaces this with a real
/// polling scheduler backed by `std::thread::sleep`-style polling.
///
/// Kept as a concrete (non-trait) type to match C++ `SCE::PullScheduler<Event> scheduler_;`.
#[derive(Debug)]
pub struct PullScheduler<E> {
    /// Pending entries: (event, event_data_json, send_id, ready_at).
    entries: Vec<ScheduledEntry<E>>,
    next_auto_send_id: u64,
}

#[derive(Debug)]
struct ScheduledEntry<E> {
    event: E,
    event_data: String,
    send_id: String,
    ready_at: Instant,
}

impl<E: Clone> PullScheduler<E> {
    /// Construct an empty scheduler.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_auto_send_id: 0,
        }
    }

    /// W3C SCXML 6.2: Schedule an event for delayed delivery.
    ///
    /// If `send_id` is empty, an automatic ID is generated. Returns the ID used
    /// (caller can use it to cancel). Matches C++
    /// `PullScheduler::scheduleEvent(EventType, milliseconds, string, string)`.
    pub fn schedule_event(
        &mut self,
        event: E,
        delay: Duration,
        send_id: &str,
        event_data: &str,
    ) -> String {
        let effective_send_id = if send_id.is_empty() {
            self.next_auto_send_id += 1;
            format!("auto_send_{}", self.next_auto_send_id)
        } else {
            send_id.to_string()
        };
        self.entries.push(ScheduledEntry {
            event,
            event_data: event_data.to_string(),
            send_id: effective_send_id.clone(),
            ready_at: Instant::now() + delay,
        });
        effective_send_id
    }

    /// W3C SCXML 6.2.5: Cancel a scheduled event by send ID. Returns `true` if found.
    pub fn cancel_event(&mut self, send_id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.send_id != send_id);
        self.entries.len() < before
    }

    /// Whether any scheduled events are ready to fire (ready_at <= now).
    pub fn has_ready_events(&self) -> bool {
        let now = Instant::now();
        self.entries.iter().any(|e| e.ready_at <= now)
    }

    /// Pop the next ready event and its data. Returns `None` if nothing is ready.
    ///
    /// Matches C++ `PullScheduler::popReadyEvent(E&, string&) -> bool` (but returns
    /// an `Option` tuple instead of bool+out-params, which is the idiomatic Rust shape).
    pub fn pop_ready_event(&mut self) -> Option<(E, String)> {
        let now = Instant::now();
        let idx = self.entries.iter().position(|e| e.ready_at <= now)?;
        let entry = self.entries.remove(idx);
        Some((entry.event, entry.event_data))
    }
}

impl<E: Clone> Default for PullScheduler<E> {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Engine<P>
// ═══════════════════════════════════════════════════════════════════════════

/// The SCXML execution engine.
///
/// Generic over a [`StatePolicy`] `P` that encodes the state machine structure
/// at compile time. Matches C++ `StaticExecutionEngine<StatePolicy>`.
pub struct Engine<P: StatePolicy> {
    /// Generated per-SM policy struct (datamodel, last-transition flags, etc.).
    pub(crate) policy: P,
    /// Currently active state (or deepest active state for parallel machines).
    pub(crate) current_state: P::State,
    /// W3C SCXML C.1: Internal event queue (high priority, `<raise>` + targetless sends).
    pub(crate) internal_queue: EventQueueManager<EventWithMetadata<P::Event>>,
    /// W3C SCXML C.1: External event queue (low priority, external sends).
    pub(crate) external_queue: EventQueueManager<EventWithMetadata<P::Event>>,
    /// Whether the engine is currently running (set false by `stop()` and final state).
    pub(crate) is_running: bool,
    /// W3C SCXML 6.4: Completion callback invoked when reaching a final state.
    pub(crate) completion_callback: Option<Box<dyn FnMut()>>,
    /// W3C SCXML C.2: HTTP send dispatch callback.
    pub(crate) on_http_send: Option<Box<dyn FnMut(HttpSendRequest)>>,
    /// W3C SCXML 6.2: Delayed event scheduler.
    pub(crate) scheduler: PullScheduler<P::Event>,
    /// W3C SCXML C.2: When true, perform_http_send() simulates a loopback HTTP
    /// server (W3CHttpTestServer equivalent) that echoes events back to the SM.
    pub(crate) http_loopback: bool,
}

impl<P: StatePolicy> Engine<P> {
    // ════════════════════════════════════════
    // Construction
    // ════════════════════════════════════════

    /// Construct a new engine with the given policy instance.
    ///
    /// The initial state is set to `P::initial_state()`. The engine is not yet
    /// running — call [`initialize`](Self::initialize) to enter the initial
    /// configuration and begin processing events.
    pub fn new(policy: P) -> Self {
        Self {
            current_state: P::initial_state(),
            policy,
            internal_queue: EventQueueManager::new(),
            external_queue: EventQueueManager::new(),
            is_running: false,
            completion_callback: None,
            on_http_send: None,
            scheduler: PullScheduler::new(),
            http_loopback: false,
        }
    }

    // ════════════════════════════════════════
    // Internal split-borrow helper
    //
    // The generated `Policy::execute_entry_actions(state, engine)` and similar
    // methods need mutable access to the policy AND mutable access to the
    // engine at the same time. Rust's borrow checker cannot verify this is
    // safe because both handles point into `self`. We use a scoped raw pointer
    // cast to split the borrows; this is safe because the policy field does
    // not alias any other engine field during the scoped call.
    //
    // This pattern is equivalent to C++ `policy_.executeEntryActions(state, *this)`
    // where the compiler has no aliasing restriction at all. In Rust, we
    // document the invariant and scope the unsafe block to three sites.
    // ════════════════════════════════════════

    /// Execute the policy's `execute_entry_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// This function dereferences a raw pointer to `self.policy` while
    /// simultaneously passing `&mut self` to the policy method. This is sound
    /// because:
    /// 1. The policy method only mutates fields within the policy struct via
    ///    the `&mut self` receiver.
    /// 2. The engine fields accessed via the `engine: &mut Engine<P>` parameter
    ///    do NOT overlap with the policy field (`self.policy` is a distinct
    ///    struct field with its own memory).
    /// 3. The pointer is not held beyond the scope of the single call.
    ///
    /// The generated code contract (`StatePolicy::execute_entry_actions`) must
    /// not alias the engine's policy field through `engine.policy` — but this
    /// is a non-issue because the Engine does not expose `policy` publicly.
    pub(crate) fn execute_on_entry(&mut self, state: P::State) {
        // SAFETY: see doc comment above. The policy field and the rest of
        // Engine's fields are disjoint; the split borrow lasts only for the
        // duration of the method call.
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        unsafe {
            (*policy_ptr).execute_entry_actions(state, self);
        }
    }

    /// Execute the policy's `execute_exit_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// See [`Self::execute_on_entry`] for the full safety rationale. The
    /// `pre_transition_active` slice is borrowed from the caller's stack and
    /// does not interact with the split borrow.
    pub(crate) fn execute_on_exit(
        &mut self,
        state: P::State,
        pre_transition_active: &[P::State],
    ) {
        // SAFETY: same as execute_on_entry.
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        unsafe {
            (*policy_ptr).execute_exit_actions(state, self, pre_transition_active);
        }
    }

    /// Execute the policy's `process_transition` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// See [`Self::execute_on_entry`]. The `current_state` parameter is an
    /// owned local variable on the caller's stack; `process_transition` may
    /// mutate it through the `&mut P::State` reference without any aliasing
    /// with engine fields.
    pub(crate) fn process_transition_dispatch(
        &mut self,
        current_state: &mut P::State,
        event: P::Event,
    ) -> bool {
        // SAFETY: same as execute_on_entry.
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        unsafe { (*policy_ptr).process_transition(current_state, event, self) }
    }

    /// Execute the policy's `execute_transition_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// Same as [`Self::execute_on_entry`].
    pub(crate) fn execute_transition_actions_dispatch(&mut self) {
        // SAFETY: same as execute_on_entry.
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        unsafe { (*policy_ptr).execute_transition_actions(self) }
    }

    /// Execute the policy's `initialize_data_model` with split-borrowed `self`.
    pub(crate) fn initialize_data_model_dispatch(&mut self) {
        // SAFETY: same as execute_on_entry.
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        unsafe { (*policy_ptr).initialize_data_model(self) }
    }

    // ════════════════════════════════════════
    // Lifecycle (matches C++ public API)
    // ════════════════════════════════════════

    /// Enter the initial configuration and run the macrostep loop until stable.
    ///
    /// Matches C++ `StaticExecutionEngine::initialize()`. W3C SCXML 5.3
    /// guarantees datamodel initialization happens before any state entry.
    pub fn initialize(&mut self) {
        self.is_running = true;

        // W3C SCXML 5.3: Initialize datamodel before any state entry
        if concepts::has_data_model_init::<P>() {
            self.initialize_data_model_dispatch();
        }

        // W3C SCXML 3.3: Entry chain from root to initial leaf
        let entry_chain = hierarchy::build_entry_chain::<P>(self.current_state);
        for state in entry_chain {
            self.execute_on_entry(state);
        }
        // W3C SCXML 3.3: Resolve current_state to the deepest initial leaf
        self.resolve_current_state_to_leaf();

        // W3C SCXML C.1: Macrostep completion loop — drain internal + eventless
        // until a stable configuration is reached.
        sce_log_debug!("Engine::initialize: entering macrostep completion loop");
        loop {
            self.check_eventless_transitions();
            if !self.internal_queue.has_events() && !self.external_queue.has_events() {
                break;
            }
            self.process_event_queues();
        }
        sce_log_debug!("Engine::initialize: macrostep completion loop finished");

        // W3C SCXML 6.4: Execute pending invokes once stable
        if concepts::has_invoke_support::<P>() {
            // SAFETY: see execute_on_entry.
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            unsafe {
                (*policy_ptr).execute_pending_invokes(self);
            }
            // Process done.invoke events raised by immediately-completed children
            sce_log_debug!("Engine::initialize: processing events raised by completed invokes");
            self.process_event_queues();
            self.check_eventless_transitions();
        }

        // W3C SCXML 6.4: Fire completion callback if we reached a final state during init
        if self.is_in_final_state() && self.completion_callback.is_some() {
            sce_log_debug!("Engine::initialize: reached final state during init, invoking completion callback");
            let active = self.get_active_states();
            let final_state = self.current_state;
            self.execute_on_exit(final_state, &active);
            if let Some(cb) = self.completion_callback.as_mut() {
                cb();
            }
        }
    }

    /// Process one macrostep: drain queues and run eventless transitions.
    ///
    /// Matches C++ `StaticExecutionEngine::step()`. Used by parent SMs to
    /// explicitly drive children after sending them events (W3C SCXML 6.4).
    pub fn step(&mut self) {
        self.process_event_queues();
        self.check_eventless_transitions();

        if self.is_in_final_state() && self.completion_callback.is_some() {
            sce_log_debug!("Engine::step: invoking completion callback");
            if let Some(cb) = self.completion_callback.as_mut() {
                cb();
            }
        }
    }

    /// Poll the scheduler for ready delayed events, then run a macrostep.
    ///
    /// Matches C++ `StaticExecutionEngine::tick()`. Called periodically by
    /// callers that have delayed `<send>` operations.
    pub fn tick(&mut self) {
        if !self.is_running {
            return;
        }
        if self.is_in_final_state() {
            if let Some(cb) = self.completion_callback.as_mut() {
                sce_log_debug!("Engine::tick: final state already reached, invoking completion callback");
                cb();
            }
            return;
        }

        // W3C SCXML 6.2: Pop all ready scheduled events into the external queue
        while let Some((event, data)) = self.scheduler.pop_ready_event() {
            self.raise_external(event, &data, "");
        }

        // W3C SCXML 6.4: Tick child state machines
        if concepts::has_child_tick::<P>() {
            // SAFETY: see execute_on_entry.
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            unsafe {
                (*policy_ptr).tick_children(self);
            }
        }

        // Delegate to step() for queue drain + completion callback
        self.step();

        // W3C SCXML 6.4: Execute pending invokes after macrostep
        if concepts::has_invoke_support::<P>() {
            // SAFETY: see execute_on_entry.
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            unsafe {
                (*policy_ptr).execute_pending_invokes(self);
            }
        }
    }

    /// Stop the engine. Subsequent `tick`/`process_event` calls become no-ops.
    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Whether the engine is running (not stopped or awaiting completion).
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Current active (leaf) state.
    pub fn get_current_state(&self) -> P::State {
        self.current_state
    }

    /// W3C SCXML 3.11: Full list of active states (for history recording + In() predicate).
    ///
    /// Non-parallel machines: returns the hierarchy `[leaf, parent, grandparent, ..., root]`.
    /// Parallel machines: returns the union of all active regions via
    /// [`StatePolicy::get_active_states`](crate::StatePolicy::get_active_states).
    pub fn get_active_states(&self) -> Vec<P::State> {
        if concepts::has_active_states::<P>() {
            return self.policy.get_active_states();
        }
        // Walk from current state up to root
        let mut active = Vec::with_capacity(8);
        let mut current = Some(self.current_state);
        while let Some(state) = current {
            active.push(state);
            current = P::get_parent(state);
        }
        active
    }

    /// W3C SCXML 3.3: Whether the current state is a top-level final state.
    pub fn is_in_final_state(&self) -> bool {
        P::is_final_state(self.current_state)
    }

    // ════════════════════════════════════════
    // Event submission (matches C++ raise / raiseExternal overloads)
    // ════════════════════════════════════════

    /// W3C SCXML C.1: Raise an internal event (high priority).
    ///
    /// Matches C++ `raise(EventWithMetadata)`.
    pub fn raise(&mut self, event: EventWithMetadata<P::Event>) {
        self.internal_queue.raise(event);
    }

    /// W3C SCXML C.1 / 6.2: Raise an external event with optional data and origin.
    ///
    /// Matches C++ `raiseExternal(Event, const string&, const string&)`.
    pub fn raise_external(&mut self, event: P::Event, event_data: &str, origin: &str) {
        let meta = EventWithMetadata::with_fields(
            event,
            event_data.to_string(),
            origin.to_string(),
            String::new(), // send_id
            EventType::External,
            crate::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE.to_string(),
            String::new(), // invoke_id
            String::new(), // target
        );
        self.external_queue.raise(meta);

        // W3C SCXML 5.10.1: Mark next event as external for _event.type
        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// W3C SCXML 6.4.6: Raise an external event by name (for child autoforward).
    ///
    /// Matches C++ `raiseExternal(const string&, const string&)`. If the name does
    /// not match any known event, the call is silently ignored (the child may
    /// simply not have that event declared).
    pub fn raise_external_by_name(&mut self, event_name: &str, event_data: &str) {
        if let Some(event) = P::get_event_from_name(event_name) {
            self.raise_external(event, event_data, "");
        } else {
            sce_log_debug!(
                "Engine::raise_external_by_name: event '{}' not in enum, ignoring",
                event_name
            );
        }
    }

    /// W3C SCXML 6.4.1: Raise an external event with full metadata (for child-to-parent).
    ///
    /// Matches C++ `raiseExternal(const EventWithMetadata&)`. Preserves `invokeid`
    /// for parent finalize handlers.
    pub fn raise_external_with_meta(&mut self, event: EventWithMetadata<P::Event>) {
        sce_log_debug!(
            "Engine::raise_external_with_meta: enqueuing external event with metadata"
        );

        // W3C SCXML 6.4.6: Autoforward
        if concepts::has_autoforward::<P>() {
            let name = P::get_event_name(event.event).to_string();
            // SAFETY: see execute_on_entry.
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            unsafe {
                (*policy_ptr).forward_to_autoforward_children(&name, self);
            }
        }

        self.external_queue.raise(event);

        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// W3C SCXML 3.12: Process an external event (convenience API, runs one macrostep).
    ///
    /// Matches C++ `processEvent(Event)`.
    pub fn process_event(&mut self, event: P::Event) {
        if !self.is_running {
            return;
        }
        self.raise_external(event, "", "");
        self.step();
    }

    /// W3C SCXML 5.10: Process an external event with metadata.
    ///
    /// Matches C++ `processEvent(Event, const EventMetadata&)`.
    pub fn process_event_with_meta(&mut self, event: P::Event, metadata: EventMetadata) {
        if !self.is_running {
            return;
        }
        let meta = EventWithMetadata {
            event,
            metadata,
            target: String::new(),
        };
        self.external_queue.raise(meta);
        self.step();
    }

    // ════════════════════════════════════════
    // Scheduler passthrough
    // ════════════════════════════════════════

    /// Schedule an event for delayed delivery. Returns the send ID.
    pub fn schedule_event(
        &mut self,
        event: P::Event,
        delay: Duration,
        send_id: &str,
        event_data: &str,
    ) -> String {
        self.scheduler.schedule_event(event, delay, send_id, event_data)
    }

    /// Cancel a previously scheduled event by send ID.
    pub fn cancel_event(&mut self, send_id: &str) -> bool {
        self.scheduler.cancel_event(send_id)
    }

    /// Whether the scheduler has events ready to fire.
    pub fn has_ready_events(&self) -> bool {
        self.scheduler.has_ready_events()
    }

    // ════════════════════════════════════════
    // Callbacks
    // ════════════════════════════════════════

    /// W3C SCXML 6.4: Register a callback invoked when the engine reaches a final state.
    pub fn set_completion_callback<F: FnMut() + 'static>(&mut self, callback: F) {
        self.completion_callback = Some(Box::new(callback));
    }

    /// W3C SCXML C.2: Register an HTTP send dispatcher callback.
    pub fn set_http_send_callback<F: FnMut(HttpSendRequest) + 'static>(&mut self, callback: F) {
        self.on_http_send = Some(Box::new(callback));
    }

    /// W3C SCXML C.2: Enable HTTP loopback mode for W3C conformance tests.
    ///
    /// When enabled, `perform_http_send` simulates the W3C HTTP test server
    /// (`W3CHttpTestServer`): parses the request, extracts the event name from
    /// `_scxmleventname` param (or defaults to `HTTP.POST`), and enqueues the
    /// response event back into this engine's external queue.
    pub fn enable_http_loopback(&mut self) {
        self.http_loopback = true;
    }

    /// W3C SCXML C.2: Dispatch a BasicHTTP send through the registered callback.
    ///
    /// If HTTP loopback is enabled, also simulates the W3C HTTP test server by
    /// injecting the response event back into the external queue.
    pub fn perform_http_send(
        &mut self,
        target: String,
        event_name: String,
        content: String,
        params: std::collections::HashMap<String, Vec<String>>,
        send_id: String,
    ) {
        if let Some(cb) = self.on_http_send.as_mut() {
            cb(HttpSendRequest {
                target: target.clone(),
                event_name: event_name.clone(),
                content: content.clone(),
                params: params.clone(),
                send_id: send_id.clone(),
            });
        }

        if self.http_loopback {
            self.http_loopback_inject(event_name, content, params);
        }
    }

    /// W3C SCXML C.2: Simulate W3CHttpTestServer loopback.
    ///
    /// Mirrors the C++ `W3CHttpTestServer::handlePost` logic:
    /// 1. Check `_scxmleventname` param → use as event name
    /// 2. Fall back to `event_name` from the send request
    /// 3. Default to `HTTP.POST` if neither is set
    /// 4. Build event data from all params as a Lua table literal
    /// 5. Enqueue as external event
    fn http_loopback_inject(
        &mut self,
        event_name: String,
        content: String,
        params: std::collections::HashMap<String, Vec<String>>,
    ) {
        // W3C SCXML C.2: Determine response event name
        let response_event_name = if let Some(names) = params.get("_scxmleventname") {
            if let Some(first) = names.first() {
                if !first.is_empty() {
                    first.clone()
                } else if !event_name.is_empty() {
                    event_name.clone()
                } else {
                    "HTTP.POST".to_string()
                }
            } else if !event_name.is_empty() {
                event_name.clone()
            } else {
                "HTTP.POST".to_string()
            }
        } else if !event_name.is_empty() {
            // W3C SCXML C.2: event attribute becomes _scxmleventname in POST body
            event_name.clone()
        } else {
            "HTTP.POST".to_string()
        };

        // W3C SCXML C.2: Build event data as Lua table from POST params
        let data = self.build_http_loopback_data(&event_name, &content, &params);

        // Resolve event name to enum and enqueue
        if let Some(evt) = P::get_event_from_name(&response_event_name) {
            let mut meta = EventWithMetadata::new(evt);
            meta.metadata = EventMetadata::external(String::new(), String::new());
            meta.metadata.data = data;
            self.external_queue.raise(meta);
        }
    }

    /// Build Lua table literal for HTTP loopback event data.
    ///
    /// Mirrors W3CHttpTestServer: all form params become table entries.
    /// If `event_name` is set, it's included as `_scxmleventname`.
    /// If only content (no params), data is the raw content string.
    fn build_http_loopback_data(
        &self,
        event_name: &str,
        content: &str,
        params: &std::collections::HashMap<String, Vec<String>>,
    ) -> String {
        // If no params and no event_name, data is the content body
        if params.is_empty() && event_name.is_empty() {
            return content.to_string();
        }

        let mut parts: Vec<String> = Vec::new();

        // Include _scxmleventname from event attribute (W3C SCXML C.2: test 534)
        if !event_name.is_empty() {
            let already_in_params = params.contains_key("_scxmleventname");
            if !already_in_params {
                parts.push(format!("[\"_scxmleventname\"]=\"{}\"", event_name));
            }
        }

        // Include all params (W3C SCXML C.2: numeric values stay numeric)
        for (key, values) in params {
            if let Some(first) = values.first() {
                if let Ok(i) = first.parse::<i64>() {
                    parts.push(format!("[\"{}\"]={}", key, i));
                } else if let Ok(f) = first.parse::<f64>() {
                    parts.push(format!("[\"{}\"]={}", key, f));
                } else {
                    parts.push(format!("[\"{}\"]=\"{}\"", key, first));
                }
            }
        }

        if parts.is_empty() {
            content.to_string()
        } else {
            format!("{{{}}}", parts.join(","))
        }
    }

    // ════════════════════════════════════════
    // Convenience: runUntilCompletion
    // ════════════════════════════════════════

    /// Run the state machine to completion or timeout (W3C SCXML 6.2).
    ///
    /// Matches C++ `runUntilCompletion(timeout, pollInterval)`. Polls the scheduler
    /// and calls `tick()` in a loop until either the final state is reached or
    /// `timeout` elapses. Returns `true` on completion, `false` on timeout.
    pub fn run_until_completion(&mut self, timeout: Duration, poll_interval: Duration) -> bool {
        // W3C SCXML: if already stopped but reached final state during initialize(), return true
        if !self.is_running {
            return self.is_in_final_state();
        }

        let start = Instant::now();
        while !self.is_in_final_state() {
            if start.elapsed() > timeout {
                return false;
            }
            std::thread::sleep(poll_interval);
            self.tick();
        }
        true
    }

    // ════════════════════════════════════════
    // Internal: microstep + macrostep implementation
    // ════════════════════════════════════════

    /// W3C SCXML D.1: Process both internal and external queues.
    pub(crate) fn process_event_queues(&mut self) {
        sce_log_debug!("Engine::process_event_queues: starting internal queue drain");

        // W3C SCXML C.1: Internal queue first
        while let Some(event_with_meta) = self.internal_queue.pop() {
            // W3C SCXML 5.4.1: Stop if top-level final state reached
            if P::is_final_state(self.current_state) && P::get_parent(self.current_state).is_none() {
                sce_log_debug!(
                    "Engine::process_event_queues: top-level final state reached, stopping"
                );
                return;
            }
            // W3C SCXML 5.10: Populate policy metadata from event (ports C++ populatePolicyFromMetadata)
            self.policy.populate_event_metadata(&event_with_meta.metadata);
            self.execute_transition(event_with_meta.event);
            self.policy.clear_event_metadata();
        }

        // W3C SCXML C.1: External queue second
        while let Some(event_with_meta) = self.external_queue.pop() {
            // W3C SCXML 6.5: Execute finalize before parent's own transition matching
            if concepts::has_finalize::<P>() {
                // SAFETY: see execute_on_entry.
                let policy_ptr: *mut P = &mut self.policy as *mut P;
                unsafe {
                    (*policy_ptr).execute_finalize_for_child_event(&event_with_meta, self);
                }
            }
            // W3C SCXML 5.10: Populate policy metadata from event (ports C++ populatePolicyFromMetadata)
            self.policy.populate_event_metadata(&event_with_meta.metadata);
            self.execute_transition(event_with_meta.event);
            self.policy.clear_event_metadata();
        }
    }

    /// W3C SCXML 3.13: Check and execute eventless transitions until stable.
    ///
    /// Uses bounded iteration to prevent infinite loops from cyclic eventless chains.
    /// Ported from C++ `EventProcessingAlgorithms.h:98-136`.
    pub(crate) fn check_eventless_transitions(&mut self) {
        const MAX_ITERATIONS: usize = 100;
        let null_event = P::null_event();

        for iteration in 0..MAX_ITERATIONS {
            let old_state = self.current_state;
            let pre_transition_states = self.get_active_states();
            let mut new_state = self.current_state;

            let took_transition = self.process_transition_dispatch(&mut new_state, null_event);
            if !took_transition {
                break;
            }

            self.current_state = new_state;
            let needs_hierarchical =
                (old_state != new_state) || (!self.policy.last_transition_is_targetless());

            if !needs_hierarchical {
                // Targetless transition -- execute actions only
                self.execute_transition_actions_dispatch();
                continue;
            }

            // Hierarchical exit/entry
            // For parallel state machines, `process_transition` already performed a full
            // microstep (exit/transition-actions/entry) via `execute_microstep` in the
            // policy. Calling `handle_hierarchical_transition` again would double-run
            // onexit/onentry (see `execute_transition` for the full explanation).
            if !P::HAS_PARALLEL_STATES {
                self.handle_hierarchical_transition(old_state, new_state, &pre_transition_states);
            } else {
                self.resolve_current_state_to_leaf();
            }

            // Check for final state
            if self.is_in_final_state() {
                break;
            }

            if iteration == MAX_ITERATIONS - 1 {
                sce_log_debug!(
                    "Engine::check_eventless_transitions: max iterations reached ({})",
                    MAX_ITERATIONS
                );
            }
        }
    }

    /// W3C SCXML 3.12/3.13: Dispatch a single transition.
    ///
    /// Calls `process_transition` on the policy; if it returns `true`, performs
    /// the hierarchical exit/entry dance via `handle_hierarchical_transition`.
    pub(crate) fn execute_transition(&mut self, event: P::Event) -> bool {
        let old_state = self.current_state;
        let pre_transition_states = self.get_active_states();
        let mut new_state = self.current_state;

        let took_transition = self.process_transition_dispatch(&mut new_state, event);
        if !took_transition {
            return false;
        }

        self.current_state = new_state;
        let is_self_transition = old_state == new_state;
        let needs_hierarchical = (old_state != new_state)
            || (is_self_transition && !self.policy.last_transition_is_targetless());

        if !needs_hierarchical {
            // W3C SCXML 3.4: targetless transition — execute actions only
            self.execute_transition_actions_dispatch();
            return false;
        }

        // W3C SCXML 3.12: Hierarchical exit/entry
        //
        // For parallel state machines the generated `process_transition` already called
        // `execute_microstep` internally (it handles exit actions, transition actions,
        // entry actions, and history recording per Appendix D.2). Calling
        // `handle_hierarchical_transition` again would double-run onexit/onentry actions
        // and, worse, exit states from `pre_transition_states` that were already restored
        // (test 504: Var1/Var2/Var3 increment too many times).
        if !P::HAS_PARALLEL_STATES {
            self.handle_hierarchical_transition(old_state, new_state, &pre_transition_states);
        } else {
            // W3C SCXML 3.3: Still resolve the current_state leaf (execute_microstep
            // sets current_state = target or parallel parent; the macrostep loop needs
            // the deepest active atomic state).
            self.resolve_current_state_to_leaf();
        }
        self.check_eventless_transitions();
        true
    }

    /// W3C SCXML 3.12/3.13: Execute hierarchical exit/entry between two states.
    ///
    /// 1:1 port of C++ `StaticExecutionEngine::handleHierarchicalTransition`
    /// (`StaticExecutionEngine.h:151-299`). Handles:
    /// - Internal vs external transition LCA calculation (W3C 5.9.2)
    /// - Active descendant exit before source exit (W3C 3.13)
    /// - Exit chain to LCA
    /// - Ancestor/self transition target re-entry (W3C 3.10, test 579)
    /// - Transition action execution between exit and entry
    /// - Entry chain from LCA to new state
    /// - No-LCA top-level case
    pub(crate) fn handle_hierarchical_transition(
        &mut self,
        old_state: P::State,
        new_state: P::State,
        pre_transition_states: &[P::State],
    ) {
        sce_log_debug!(
            "Engine::handle_hierarchical_transition: {:?} -> {:?}",
            old_state,
            new_state
        );

        // W3C SCXML 5.9.2: Determine LCA based on transition type
        let lca: Option<P::State> = if self.policy.last_transition_is_internal() {
            let is_self_transition = old_state == new_state;
            let is_proper_descendant =
                !is_self_transition && P::is_descendant_of(new_state, old_state);
            let is_source_compound = P::is_compound_state(old_state);

            if is_proper_descendant && is_source_compound {
                // W3C SCXML 3.13: Internal to proper descendant in compound — source is LCA
                Some(old_state)
            } else {
                // W3C 3.13/5.9.2: Non-compound source or non-descendant — behaves as external
                hierarchy::find_lca::<P>(old_state, new_state)
            }
        } else {
            hierarchy::find_lca::<P>(old_state, new_state)
        };

        if let Some(lca_state) = lca {
            // W3C SCXML 3.13: Exit active descendants of old_state deepest first
            let mut descendants_to_exit: Vec<P::State> = pre_transition_states
                .iter()
                .copied()
                .filter(|&s| s != old_state && P::is_descendant_of(s, old_state))
                .collect();
            // Sort by document order descending (deeper first)
            descendants_to_exit.sort_by(|a, b| P::get_document_order(*b).cmp(&P::get_document_order(*a)));

            for descendant in descendants_to_exit {
                sce_log_debug!("handle_hierarchical_transition: exit descendant {:?}", descendant);
                self.execute_on_exit(descendant, pre_transition_states);
            }

            // W3C SCXML 3.13: Exit from old_state up to (not including) LCA
            let exit_chain = hierarchy::build_exit_chain::<P>(old_state, lca_state);
            for state in exit_chain {
                sce_log_debug!("handle_hierarchical_transition: exit {:?}", state);
                self.execute_on_exit(state, pre_transition_states);
            }

            // W3C SCXML 3.10 (test 579): Ancestor/self transition — exit and re-enter target
            let is_target_active = pre_transition_states.contains(&new_state);
            if new_state == lca_state && is_target_active {
                sce_log_debug!(
                    "handle_hierarchical_transition: ancestor/self transition — exit target {:?}",
                    new_state
                );
                self.execute_on_exit(new_state, pre_transition_states);
            }

            // W3C SCXML 3.13: Execute transition actions between exit and entry
            self.execute_transition_actions_dispatch();

            // W3C SCXML 3.13: Enter from LCA down to new_state
            let entry_chain: Vec<P::State> = if new_state == lca_state {
                // Ancestor/self case — enter full subtree from target
                let full = hierarchy::build_entry_chain::<P>(new_state);
                full.into_iter()
                    .filter(|&s| s == lca_state || P::is_descendant_of(s, lca_state))
                    .collect()
            } else {
                hierarchy::build_entry_chain_from_ancestor::<P>(new_state, lca_state)
            };

            for state in &entry_chain {
                sce_log_debug!("handle_hierarchical_transition: enter {:?}", state);
                self.execute_on_entry(*state);
            }

            if let Some(&last) = entry_chain.last() {
                self.current_state = last;
            }

            // W3C SCXML 3.3: Resolve current_state to the deepest initial leaf.
            // execute_entry_actions for compound states recursively enters initial
            // children, but current_state must track the deepest leaf for
            // eventless transition checks to work correctly.
            self.resolve_current_state_to_leaf();
        } else {
            // No LCA — top-level transition, exit all ancestors of old_state
            sce_log_debug!("handle_hierarchical_transition: no LCA (top-level)");

            let mut current = Some(old_state);
            while let Some(state) = current {
                sce_log_debug!("handle_hierarchical_transition: exit to root: {:?}", state);
                self.execute_on_exit(state, pre_transition_states);
                current = P::get_parent(state);
            }

            self.execute_transition_actions_dispatch();

            let entry_chain = hierarchy::build_entry_chain::<P>(new_state);
            for state in &entry_chain {
                sce_log_debug!("handle_hierarchical_transition: enter from root: {:?}", state);
                self.execute_on_entry(*state);
            }

            if let Some(&last) = entry_chain.last() {
                self.current_state = last;
            }

            self.resolve_current_state_to_leaf();
        }
    }

    /// W3C SCXML 3.3: Walk current_state down through initial children to the leaf.
    ///
    /// For **non-parallel** SMs the generated `execute_entry_actions` does NOT recurse
    /// into compound→initial child (the engine's entry chain already covers ancestors).
    /// This method descends into the compound's initial child, calling `execute_on_entry`
    /// for each level, until it reaches an atomic leaf.
    ///
    /// For **parallel** SMs the generated `execute_entry_actions` already recurses (matching
    /// C++ `executeEntryActions` L319-343), so this is just a pointer walk without entry.
    fn resolve_current_state_to_leaf(&mut self) {
        const MAX_DEPTH: usize = 50;
        for _ in 0..MAX_DEPTH {
            if !P::is_compound_state(self.current_state) {
                break;
            }
            let child = self.policy.get_initial_or_history_child(self.current_state);
            if child == self.current_state {
                break; // No child to descend into
            }
            self.current_state = child;
            if !P::HAS_PARALLEL_STATES {
                // Non-parallel: template doesn't recurse, so we enter here.
                self.execute_on_entry(child);
            }
        }
    }
}
