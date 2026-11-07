// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md §2 to
//! §5, §12): its entities, its ports and timers, the round scheduler on the
//! logical clock, and the test case verdict.
//!
//! A component's machine is generated from its `test-component` and reaches
//! this run only through its [`ComponentContext`] (§12.3): the snapshot its
//! branches' guards read, the statement each taken branch runs first, the
//! statements of the profile, and the doors of a send and of a PTC. The run
//! knows nothing of the engine or of the record types, so it is written once
//! for every suite and tested on its own.
//!
//! # Ownership
//!
//! The state of a run is shared between the run and a handle per component,
//! as the C++ runtime's context holds a reference to its run: the machine
//! keeps its [`ComponentContext`] from `start` on, and so may the host
//! processors it registers on its engine for the doors of a send and of a PTC,
//! which the engine holds as `Send + 'static` closures. The state is therefore
//! an `Arc<Mutex<_>>`, although a run is driven on one thread.
//!
//! The run never holds the lock across a call into a machine: it takes the
//! machine out of its entity, lets the lock go, calls it, and puts it back. So
//! every call the machine makes into its context finds the lock free. A call
//! that finds it held is a call made while the run, or the same component, is
//! already inside the state, which on one thread can only be a re-entrant call
//! (a snapshot reader that calls its context, say), and it panics, naming the
//! component, rather than deadlocking.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use super::module_parameters::ModuleParameters;
use super::port::{Duration, Instant, Message, Port};
use super::timer::Timer;
use super::verdict::{worse, Verdict};
use crate::saved_state::StateRefusal;

/// The name of the MTC in a configuration; no PTC takes it (§11.6).
pub const MTC_NAME: &str = "mtc";

/// A component port named in a configuration: the component's name (`mtc` or a
/// PTC's id) and the port's id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Endpoint {
    /// The component's name.
    pub component: String,
    /// The port's id.
    pub port: String,
}

impl Endpoint {
    /// The port `port` of the component `component`.
    pub fn new(component: impl Into<String>, port: impl Into<String>) -> Self {
        Endpoint {
            component: component.into(),
            port: port.into(),
        }
    }

    fn is(&self, component: &str, port: &str) -> bool {
        self.component == component && self.port == port
    }
}

/// A `test-configuration` as the run reads it (§11.6).
///
/// Generated from the configuration document. Each port takes part in one
/// connection or one mapping (§2.2), which the build has checked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Configuration {
    /// The MTC's component type.
    pub mtc_type: String,
    /// Each PTC's name and its component type.
    pub ptc_types: BTreeMap<String, String>,
    /// The port ids each component type declares.
    pub ports_of_type: BTreeMap<String, Vec<String>>,
    /// The timer ids each component type declares.
    pub timers_of_type: BTreeMap<String, Vec<String>>,
    /// The `<sce:connect>` pairs (-4 §9.10).
    pub connections: Vec<(Endpoint, Endpoint)>,
    /// The `<sce:map>` entries: a component port and its system port (-4 §9.32).
    pub mappings: Vec<(Endpoint, String)>,
}

/// What the test log records of a run (§12.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    /// The MTC was created and started.
    Start,
    /// A PTC was created and started.
    Create,
    /// A component sent on a port.
    Send,
    /// A receive branch removed the head of a port.
    Receive,
    /// A timeout branch took a timer.
    Timeout,
    /// A PTC terminated (P10).
    Done,
    /// A PTC was removed.
    Kill,
    /// A component set its local verdict.
    Verdict,
    /// A `<log>`.
    Log,
    /// A dynamic error ended the run (P13).
    Error,
}

/// One line of the test log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRecord {
    /// `NOW` when it was recorded.
    pub at: Instant,
    /// The component it is of.
    pub component: String,
    /// What it records.
    pub kind: LogKind,
    /// The port and event, the timer, the verdict and reason, the text or the
    /// error, by kind; empty for a kind that has none.
    pub detail: String,
}

/// The outcome of one test case: its verdict, the instant it ended at, and its
/// log.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestCaseResult {
    /// The test case verdict (P14).
    pub verdict: Verdict,
    /// `NOW` when the run ended.
    pub ended_at: Instant,
    /// The test log.
    pub log: Vec<LogRecord>,
}

/// The SUT's side of the mapped ports (§12.2).
///
/// Shaped after TRI (ES 201 873-5): `send` is `triSend`, and the inputs it
/// returns are what `triEnqueueMsg` would enqueue. The adapter owns the codec
/// and the transport; the run sees values only (ADR 0091 C2). The instants are
/// the logical clock's, so a recorded trace replays to the same run (§3.1).
///
/// `Send` because the run's state, which holds it, is shared with the
/// components' host processors (see the module's documentation).
pub trait SystemAdapter: Send {
    /// A component sent `message` on a port mapped to `system_port`.
    fn send(&mut self, system_port: &str, message: &Message, now: Instant);

    /// The instant of the earliest input not yet taken, if any.
    fn next_input(&self) -> Option<Instant>;

    /// Every input due at or before `now`, in arrival order, each with the
    /// system port it arrives on.
    fn take_inputs_due(&mut self, now: Instant) -> Vec<(String, Message)>;
}

/// A component's machine, as the run drives it (§12.3).
///
/// Generated from a `test-component`: the backend's statechart machine, with
/// each branch a transition on the round's snapshot event.
pub trait ComponentMachine: Send {
    /// Enter the initial configuration and complete the first macrostep. The
    /// machine keeps `context`, and every call it makes, now and on each later
    /// delivery, goes to it.
    fn start(&mut self, context: ComponentContext);

    /// Deliver the round's snapshot event: the machine takes the first branch
    /// whose guard holds on the snapshot (P3) and completes its macrostep (P4),
    /// or takes none.
    fn deliver_snapshot(&mut self);

    /// The machine has entered a top-level `<final>` (P10).
    fn terminated(&self) -> bool;
}

/// Builds the machine of a component type, or none for a type it does not make,
/// which ends the run with `error` where a component of that type is created.
pub type MachineFactory = Arc<dyn Fn(&str) -> Option<Box<dyn ComponentMachine>> + Send + Sync>;

/// What one component holds in the run.
#[derive(Default)]
struct ComponentState {
    ports: BTreeMap<String, Port>,
    timers: BTreeMap<String, Timer>,
    done_snapshot: BTreeSet<String>,
    verdict: Verdict,
    branch_taken: bool,
}

struct Entity {
    /// Which creation this is: a PTC removed and created again in one round is
    /// a new component, which took no snapshot in it.
    serial: u64,
    state: ComponentState,
    /// The machine, or none while the run has it out to call it.
    machine: Option<Box<dyn ComponentMachine>>,
    terminated: bool,
}

/// The machines a step of the run took out of the state, dropped by the caller
/// once the lock is let go: dropping a machine is a call into it too.
type Released = Vec<Box<dyn ComponentMachine>>;

/// The state of one run, shared by the run and every component's context.
struct RunState {
    configuration: Configuration,
    factory: MachineFactory,
    adapter: Box<dyn SystemAdapter>,
    guard: Option<Duration>,
    module_parameters: ModuleParameters,

    now: Instant,
    created: u64,
    entities: BTreeMap<String, Entity>,
    order: Vec<String>,
    done: BTreeSet<String>,
    removed_verdict: Verdict,
    mtc_terminated: bool,
    error: bool,
    result: TestCaseResult,
}

type SharedRun = Arc<Mutex<RunState>>;

/// The state, for `component`; a call that finds it held is re-entrant.
fn lock<'r>(run: &'r Mutex<RunState>, component: &str) -> MutexGuard<'r, RunState> {
    match run.try_lock() {
        Ok(state) => state,
        Err(TryLockError::WouldBlock) => {
            panic!("re-entrant access to the test run from {component}")
        }
        Err(TryLockError::Poisoned(_)) => panic!(
            "the test run was left mid-step by an earlier panic, so {component} cannot go on"
        ),
    }
}

impl RunState {
    // ── The round (§3.2, §12.4) ──────────────────────────────────────

    /// Step 2, the update: inputs and timer expiries due at `NOW`. Then step 3,
    /// the snapshot: every waiting component at once. Returns the components
    /// that took it, in creation order, each with its serial.
    fn update_and_snapshot(&mut self) -> Vec<(String, u64)> {
        let now = self.now;
        for (system_port, message) in self.adapter.take_inputs_due(now) {
            self.deliver_input(&system_port, message);
        }
        for entity in self.entities.values_mut() {
            for timer in entity.state.timers.values_mut() {
                timer.update(now);
            }
        }
        let mut waiting = Vec::new();
        for name in &self.order {
            let Some(entity) = self.entities.get_mut(name) else {
                continue;
            };
            if entity.terminated {
                continue;
            }
            for port in entity.state.ports.values_mut() {
                port.take_snapshot();
            }
            for timer in entity.state.timers.values_mut() {
                timer.take_snapshot();
            }
            entity.state.done_snapshot = self.done.clone();
            waiting.push((name.clone(), entity.serial));
        }
        waiting
    }

    /// The machine of a component that took this round's snapshot, out of its
    /// entity to be delivered; none for one removed or terminated earlier in
    /// the round, which does not select (§3.2 step 5).
    fn take_for_delivery(&mut self, name: &str, serial: u64) -> Option<Box<dyn ComponentMachine>> {
        let entity = self.entities.get_mut(name)?;
        if entity.serial != serial || entity.terminated {
            return None;
        }
        entity.state.branch_taken = false;
        entity.machine.take()
    }

    fn branch_taken(&self, name: &str, serial: u64) -> bool {
        self.entities
            .get(name)
            .is_some_and(|entity| entity.serial == serial && entity.state.branch_taken)
    }

    /// No branch fired: time advances to the next instant something can happen
    /// at (§3.3), or the test case can never end (P13).
    fn advance(&mut self) {
        let mut next = self.adapter.next_input();
        for entity in self.entities.values() {
            if entity.terminated {
                continue;
            }
            for timer in entity.state.timers.values() {
                if let Some(expiry) = timer.expiry() {
                    next = Some(next.map_or(expiry, |earliest| earliest.min(expiry)));
                }
            }
        }
        if let Some(guard) = self.guard {
            next = Some(next.map_or(guard, |earliest| earliest.min(guard)));
        }
        let Some(next) = next else {
            self.fail(
                MTC_NAME,
                "no branch can fire and no instant is pending: the test case can never end"
                    .to_string(),
            );
            return;
        };
        self.now = self.now.max(next);
        if self.guard.is_some_and(|guard| self.now >= guard) {
            self.fail(
                MTC_NAME,
                "the guard of the test case was reached".to_string(),
            );
        }
    }

    /// A component's machine has just run and is put back. If it terminated,
    /// P10 applies.
    ///
    /// The entity is still there: a component whose machine is running is not
    /// removed (`RunState::kill`), and nothing else removes one mid-round.
    fn settle(
        &mut self,
        name: &str,
        serial: u64,
        machine: Box<dyn ComponentMachine>,
        terminated: bool,
    ) {
        let entity = match self.entities.get_mut(name) {
            Some(entity) if entity.serial == serial => entity,
            _ => panic!("the component {name} was removed while its machine ran"),
        };
        entity.machine = Some(machine);
        if entity.terminated || !terminated {
            return;
        }
        entity.terminated = true;
        if name == MTC_NAME {
            self.mtc_terminated = true;
            return;
        }
        self.done.insert(name.to_string());
        self.record(name, LogKind::Done, String::new());
    }

    fn ended(&self) -> bool {
        self.mtc_terminated || self.error
    }

    /// The MTC terminated or the run ended in error: every PTC still present is
    /// removed, and the test case verdict is the worst local verdict (P10,
    /// P14). The MTC's machine is handed back too, so that no machine outlives
    /// its run holding a context of it.
    fn finish(&mut self) -> Released {
        let ptcs: Vec<String> = self
            .entities
            .keys()
            .filter(|name| name.as_str() != MTC_NAME)
            .cloned()
            .collect();
        let mut released: Released = ptcs
            .iter()
            .filter_map(|name| self.remove_entity(name))
            .collect();
        let mut verdict = self.removed_verdict;
        if let Some(mtc) = self.entities.get_mut(MTC_NAME) {
            verdict = worse(verdict, mtc.state.verdict);
            released.extend(mtc.machine.take());
        }
        if self.error {
            verdict = Verdict::Error;
        }
        self.result.verdict = verdict;
        self.result.ended_at = self.now;
        released
    }

    // ── Entities (§2.1, P9) ──────────────────────────────────────────

    /// Enters a new component of the type `component_type` into the table,
    /// with its ports and timers, and logs its creation. Returns its serial.
    fn enter(&mut self, name: &str, component_type: &str) -> u64 {
        self.created += 1;
        let mut state = ComponentState::default();
        if let Some(ports) = self.configuration.ports_of_type.get(component_type) {
            for port in ports {
                state.ports.entry(port.clone()).or_default();
            }
        }
        if let Some(timers) = self.configuration.timers_of_type.get(component_type) {
            for timer in timers {
                state.timers.entry(timer.clone()).or_default();
            }
        }
        self.entities.insert(
            name.to_string(),
            Entity {
                serial: self.created,
                state,
                machine: None,
                terminated: false,
            },
        );
        self.order.push(name.to_string());
        let kind = if name == MTC_NAME {
            LogKind::Start
        } else {
            LogKind::Create
        };
        self.record(name, kind, component_type.to_string());
        self.created
    }

    /// The type of the PTC `ptc` that `creator` creates, or none, with the
    /// dynamic error, when it cannot be created.
    fn ptc_to_create(&mut self, creator: &str, ptc: &str) -> Option<String> {
        let Some(component_type) = self.configuration.ptc_types.get(ptc).cloned() else {
            self.fail(
                creator,
                format!("creates '{ptc}', which the configuration does not declare"),
            );
            return None;
        };
        if self.entities.contains_key(ptc) {
            self.fail(
                creator,
                format!("creates '{ptc}', which is already present"),
            );
            return None;
        }
        self.done.remove(ptc);
        Some(component_type)
    }

    /// The kill of the PTC `ptc` by `killer` (P9). A component whose machine
    /// is running, up the call stack of this one, is not removed under it: that
    /// is a dynamic error.
    fn kill(&mut self, killer: &str, ptc: &str) -> Option<Box<dyn ComponentMachine>> {
        if self
            .entities
            .get(ptc)
            .is_some_and(|entity| entity.machine.is_none())
        {
            self.fail(killer, format!("kills '{ptc}', whose machine is running"));
            return None;
        }
        self.remove_entity(ptc)
    }

    /// Removes a PTC: its local verdict still counts (-4 §9.29a.2), and its
    /// ports are gone, so a send to one of them is an error. Hands back its
    /// machine, to be dropped outside the lock.
    fn remove_entity(&mut self, name: &str) -> Option<Box<dyn ComponentMachine>> {
        let entity = self.entities.remove(name)?;
        self.removed_verdict = worse(self.removed_verdict, entity.state.verdict);
        self.record(name, LogKind::Kill, String::new());
        self.order.retain(|present| present != name);
        self.done.remove(name);
        entity.machine
    }

    // ── Ports (§2.2, P6) ─────────────────────────────────────────────

    fn send(&mut self, sender: &str, port: &str, message: Message) {
        self.record(sender, LogKind::Send, format!("{port} {}", message.event));
        let peer = self.configuration.connections.iter().find_map(|(a, b)| {
            if a.is(sender, port) {
                Some(b.clone())
            } else if b.is(sender, port) {
                Some(a.clone())
            } else {
                None
            }
        });
        if let Some(to) = peer {
            if let Some(entity) = self.entities.get_mut(&to.component) {
                entity
                    .state
                    .ports
                    .entry(to.port)
                    .or_default()
                    .enqueue(message);
                return;
            }
            self.fail(
                sender,
                format!(
                    "sends on '{port}', whose peer '{}' is not present",
                    to.component
                ),
            );
            return;
        }
        let mapped = self
            .configuration
            .mappings
            .iter()
            .find(|(endpoint, _)| endpoint.is(sender, port))
            .map(|(_, system_port)| system_port.clone());
        if let Some(system_port) = mapped {
            let now = self.now;
            self.adapter.send(&system_port, &message, now);
            return;
        }
        self.fail(
            sender,
            format!("sends on '{port}', which is neither connected nor mapped"),
        );
    }

    fn deliver_input(&mut self, system_port: &str, message: Message) {
        let endpoint = self
            .configuration
            .mappings
            .iter()
            .find(|(_, mapped)| mapped == system_port)
            .map(|(endpoint, _)| endpoint.clone());
        let Some(endpoint) = endpoint else {
            self.fail(
                MTC_NAME,
                format!("an input on system port '{system_port}', which no port maps"),
            );
            return;
        };
        if let Some(entity) = self.entities.get_mut(&endpoint.component) {
            entity
                .state
                .ports
                .entry(endpoint.port)
                .or_default()
                .enqueue(message);
            return;
        }
        self.fail(
            &endpoint.component,
            format!("an input on system port '{system_port}' found no component"),
        );
    }

    // ── A component's own state, through its context ─────────────────

    fn component(&self, name: &str, serial: u64) -> &ComponentState {
        match self.entities.get(name) {
            Some(entity) if entity.serial == serial => &entity.state,
            _ => panic!("the component {name} was removed, and its context outlived it"),
        }
    }

    fn component_mut(&mut self, name: &str, serial: u64) -> &mut ComponentState {
        match self.entities.get_mut(name) {
            Some(entity) if entity.serial == serial => &mut entity.state,
            _ => panic!("the component {name} was removed, and its context outlived it"),
        }
    }

    fn receive(&mut self, name: &str, serial: u64, port: &str) -> Message {
        let component = self.component_mut(name, serial);
        component.branch_taken = true;
        let Some(queue) = component.ports.get_mut(port) else {
            panic!("{name} receives on '{port}', which its component type does not declare");
        };
        let message = queue.remove_head();
        self.record(name, LogKind::Receive, format!("{port} {}", message.event));
        message
    }

    fn timeout(&mut self, name: &str, serial: u64, timer: &str) {
        let component = self.component_mut(name, serial);
        component.branch_taken = true;
        let Some(binding) = component.timers.get_mut(timer) else {
            panic!("{name} times out on '{timer}', which its component type does not declare");
        };
        binding.timeout_taken();
        self.record(name, LogKind::Timeout, timer.to_string());
    }

    fn setverdict(&mut self, name: &str, serial: u64, verdict: Verdict, reason: &str) {
        let component = self.component_mut(name, serial);
        component.verdict = worse(component.verdict, verdict);
        let detail = if reason.is_empty() {
            verdict.name().to_string()
        } else {
            format!("{} {reason}", verdict.name())
        };
        self.record(name, LogKind::Verdict, detail);
    }

    // ── The verdict and the log (§5, §12.5) ──────────────────────────

    /// A dynamic error (P13): the run ends with `error`.
    fn fail(&mut self, component: &str, why: String) {
        self.error = true;
        self.record(component, LogKind::Error, why);
    }

    fn record(&mut self, component: &str, kind: LogKind, detail: String) {
        self.result.log.push(LogRecord {
            at: self.now,
            component: component.to_string(),
            kind,
            detail,
        });
    }
}

/// Creates the component `name` of the type `component_type` and starts its
/// machine, with the lock let go while the factory and the machine run.
fn create_entity(run: &SharedRun, name: &str, component_type: &str) {
    let factory = Arc::clone(&lock(run, name).factory);
    let Some(mut machine) = factory(component_type) else {
        lock(run, name).fail(
            name,
            format!(
                "is of the component type '{component_type}', which the factory of machines does not make"
            ),
        );
        return;
    };
    let serial = lock(run, name).enter(name, component_type);
    machine.start(ComponentContext {
        run: Arc::clone(run),
        name: name.to_string(),
        serial,
    });
    let terminated = machine.terminated();
    lock(run, name).settle(name, serial, machine, terminated);
}

/// One round (§3.2, §12.4).
fn round(run: &SharedRun) {
    let waiting = lock(run, MTC_NAME).update_and_snapshot();
    // Steps 4 and 5: one component at a time, in creation order, each selects
    // on its own snapshot and runs the macrostep of the branch it took. A
    // component removed or terminated earlier in the round does not select,
    // and nothing does once the MTC has terminated (§3.2).
    let mut any_taken = false;
    for (name, serial) in waiting {
        let machine = {
            let mut state = lock(run, &name);
            if state.ended() {
                break;
            }
            state.take_for_delivery(&name, serial)
        };
        let Some(mut machine) = machine else {
            continue;
        };
        machine.deliver_snapshot();
        let terminated = machine.terminated();
        let mut state = lock(run, &name);
        any_taken = any_taken || state.branch_taken(&name, serial);
        state.settle(&name, serial, machine, terminated);
    }
    let mut state = lock(run, MTC_NAME);
    if state.ended() || any_taken {
        return;
    }
    state.advance();
}

fn ended(run: &SharedRun) -> bool {
    lock(run, MTC_NAME).ended()
}

/// What a component's machine reads and does through the run (§12.3).
///
/// A handle: the machine keeps it from `start` on, and a clone of it may be
/// captured by the host processors the machine registers for the doors of a
/// send and of a PTC. Each call takes the run's state for its own length and
/// lets it go before it returns, so no call holds it across a call into a
/// machine. A call made while the state is held, from inside a snapshot reader
/// say, is re-entrant and panics, naming the component.
#[derive(Clone)]
pub struct ComponentContext {
    run: SharedRun,
    name: String,
    serial: u64,
}

impl fmt::Debug for ComponentContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentContext")
            .field("name", &self.name)
            .field("serial", &self.serial)
            .finish_non_exhaustive()
    }
}

impl ComponentContext {
    fn lock(&self) -> MutexGuard<'_, RunState> {
        lock(&self.run, &self.name)
    }

    /// The component's name: `mtc`, or the PTC's id.
    pub fn name(&self) -> &str {
        &self.name
    }

    // ── The snapshot, read by the branches' guards (P2) ──────────────

    /// Hands `read` the message the snapshot saw at the head of `port`, or
    /// none, and returns what it returns. `read` runs while the run's state is
    /// held, so it must not call this context: such a call panics.
    pub fn with_snapshot<R>(&self, port: &str, read: impl FnOnce(Option<&Message>) -> R) -> R {
        let state = self.lock();
        let head = state
            .component(&self.name, self.serial)
            .ports
            .get(port)
            .and_then(Port::snapshot);
        read(head)
    }

    /// The snapshot saw a message at the head of `port`: the guard of a
    /// receive any branch.
    pub fn snapshot_present(&self, port: &str) -> bool {
        self.with_snapshot(port, |head| head.is_some())
    }

    /// The snapshot saw `timer` timed out.
    pub fn timed_out(&self, timer: &str) -> bool {
        self.lock()
            .component(&self.name, self.serial)
            .timers
            .get(timer)
            .is_some_and(Timer::snapshot_timed_out)
    }

    /// The snapshot saw the PTC `ptc` in `DONE`.
    pub fn done(&self, ptc: &str) -> bool {
        self.lock()
            .component(&self.name, self.serial)
            .done_snapshot
            .contains(ptc)
    }

    // ── Taking a branch (P4 step 1), the first statement of each ─────

    /// A receive branch: remove the head of `port`, which its content reads as
    /// `_event.data`.
    ///
    /// # Panics
    ///
    /// When the component type declares no port `port`, or its queue is empty:
    /// a receive branch is taken only on a snapshot that saw a message there.
    pub fn receive(&self, port: &str) -> Message {
        self.lock().receive(&self.name, self.serial, port)
    }

    /// A timeout branch: `timer` becomes idle.
    ///
    /// # Panics
    ///
    /// When the component type declares no timer `timer`.
    pub fn timeout(&self, timer: &str) {
        self.lock().timeout(&self.name, self.serial, timer);
    }

    /// A done branch on the PTC `_ptc`: nothing is removed (-4 §9.16); the
    /// component took a branch in this round.
    pub fn done_taken(&self, _ptc: &str) {
        self.lock()
            .component_mut(&self.name, self.serial)
            .branch_taken = true;
    }

    // ── Statements of the profile ────────────────────────────────────

    /// `<sce:setverdict>` (P12): the local verdict becomes the worse of the
    /// two, and the verdict and its reason are logged.
    pub fn setverdict(&self, verdict: Verdict, reason: &str) {
        self.lock()
            .setverdict(&self.name, self.serial, verdict, reason);
    }

    /// `<sce:start-timer>` (P7), for `duration` microseconds from `NOW`.
    pub fn start_timer(&self, timer: &str, duration: Duration) {
        let mut state = self.lock();
        let now = state.now;
        state
            .component_mut(&self.name, self.serial)
            .timers
            .entry(timer.to_string())
            .or_default()
            .start(now, duration);
    }

    /// `<sce:stop-timer>` (P7).
    pub fn stop_timer(&self, timer: &str) {
        self.lock()
            .component_mut(&self.name, self.serial)
            .timers
            .entry(timer.to_string())
            .or_default()
            .stop();
    }

    /// A `<log>` (P8): recorded in the test log with `NOW` and the component.
    pub fn log(&self, text: &str) {
        self.lock()
            .record(&self.name, LogKind::Log, text.to_string());
    }

    // ── Doors (§12.3) ────────────────────────────────────────────────

    /// A send on `port` (P6): to the queue of the connected port, or to the
    /// system adapter when the port is mapped.
    pub fn send(&self, port: &str, message: Message) {
        self.lock().send(&self.name, port, message);
    }

    /// The start of the invocation that creates the PTC `ptc` (P9): the PTC
    /// is created and its machine started before this returns.
    pub fn create(&self, ptc: &str) {
        let component_type = self.lock().ptc_to_create(&self.name, ptc);
        if let Some(component_type) = component_type {
            create_entity(&self.run, ptc, &component_type);
        }
    }

    /// The cancel of that invocation: the PTC is removed (P9).
    pub fn kill(&self, ptc: &str) {
        let machine = self.lock().kill(&self.name, ptc);
        drop(machine);
    }

    // ── Module parameters (§11.5, §11.8) ─────────────────────────────

    /// Hands `read` the module parameters of the run, which it reads into the
    /// variables the component declares as module parameters, before the
    /// component's machine starts. A value the run was not given
    /// ([`ModuleParameters::of`]), or one `read` refuses as not of the type the
    /// component declares, is a dynamic error (P13): the run ends with `error`
    /// and false is returned, so the machine is not started on a value made up
    /// in its place. `read` runs with the run's state let go.
    pub fn read_module_parameters(
        &self,
        read: impl FnOnce(&ModuleParameters) -> Result<(), StateRefusal>,
    ) -> bool {
        let given = self.lock().module_parameters.clone();
        match read(&given) {
            Ok(()) => true,
            Err(refused) => {
                self.lock().fail(&self.name, refused.reason().to_string());
                false
            }
        }
    }

    /// `NOW`.
    pub fn now(&self) -> Instant {
        self.lock().now
    }
}

/// Whether a receive branch with a template matches the snapshot of `context`
/// (P2, docs/SCE_TEST_PROFILE.md §12.3).
///
/// The snapshot saw a message at the head of `port`, under `event`, the event
/// name of the template's record, whose value is a `V`, and `matches`, the
/// template's `match` bound to its arguments, holds of that value. A message of
/// another record, or one whose value is of another type, does not match.
/// `matches` runs while the run's state is held, as a reader of
/// [`ComponentContext::with_snapshot`] does.
pub fn receives<V: 'static>(
    context: &ComponentContext,
    port: &str,
    event: &str,
    matches: impl FnOnce(&V) -> bool,
) -> bool {
    context.with_snapshot(port, |head| {
        head.filter(|message| message.event == event)
            .and_then(|message| message.value_as::<V>())
            .is_some_and(matches)
    })
}

/// One run of one test case (§2.3, §3, §4.9 to §4.11, §5).
///
/// The components run on the calling thread, in the order they were created;
/// §3.2 shows that the order within a round does not change the result.
pub struct TestCaseRun {
    run: SharedRun,
}

impl TestCaseRun {
    /// A run of the test case whose configuration is `configuration`, whose
    /// machines `factory` builds, against `adapter`.
    ///
    /// `guard` is the test case's guard duration (P11), if it has one, and
    /// `module_parameters` the values of the module parameters the components
    /// read (§11.8): the suite's, with any the caller overrides.
    pub fn new(
        configuration: Configuration,
        factory: MachineFactory,
        adapter: Box<dyn SystemAdapter>,
        guard: Option<Duration>,
        module_parameters: ModuleParameters,
    ) -> Self {
        TestCaseRun {
            run: Arc::new(Mutex::new(RunState {
                configuration,
                factory,
                adapter,
                guard,
                module_parameters,
                now: 0,
                created: 0,
                entities: BTreeMap::new(),
                order: Vec::new(),
                done: BTreeSet::new(),
                removed_verdict: Verdict::None,
                mtc_terminated: false,
                error: false,
                result: TestCaseResult::default(),
            })),
        }
    }

    /// Runs the test case to its end and returns its verdict and log.
    pub fn run(self) -> TestCaseResult {
        let mtc_type = lock(&self.run, MTC_NAME).configuration.mtc_type.clone();
        create_entity(&self.run, MTC_NAME, &mtc_type);
        while !ended(&self.run) {
            round(&self.run);
        }
        let (result, released) = {
            let mut state = lock(&self.run, MTC_NAME);
            let released = state.finish();
            (std::mem::take(&mut state.result), released)
        };
        drop(released);
        result
    }
}
