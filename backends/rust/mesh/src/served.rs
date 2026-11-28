// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A generated machine served as the session of a keyed pool (docs/adr/0112).
//!
//! [`Pool`](crate::pool::Pool) runs each session through a
//! [`SessionMachine`] a [`SessionFactory`] builds. For a machine the
//! generator wrote, nothing of that depends on the document: [`Served`] is the
//! factory and [`ServedMachine`] the machine, over any policy whose machine
//! can be saved ([`PersistentPolicy`]). A deployment gives them what it varies
//! through a [`SessionHost`]: the policy of each session (the values its
//! deployment supplies, docs/adr/0109, D12) and the host operations it
//! registers.
//!
//! What it does, in this order, is what lets a session's checkpoint
//! (SCE_MESH.md section 10.11) and its machine's restore (docs/adr/0109, D11)
//! meet:
//!
//! 1. The engine measures time on a clock the adapter owns
//!    ([`SceClock::Manual`]), read as milliseconds since the Unix epoch and
//!    moved to the pool's wall clock at every run, so a delayed send the
//!    machine arms is saved as the wall-clock moment it comes due and a
//!    restore arms it against the same clock.
//! 2. A start builds the engine, sets the clock, registers the session's
//!    endpoint (`endpoint::register`) and then the host's operations, and
//!    only then initializes, so that what the initial configuration sends or
//!    starts reaches both.
//! 3. A restore carries the saved state into this version of the document
//!    (`PersistentPolicy::restore_with`, which applies the migration the
//!    document declares), and registers the endpoint and the host's operations
//!    before the machine takes any event: an invocation the saved state held
//!    running starts again at the top of the first macrostep it drives. A state
//!    no step carries is refused with its reason, and the pool answers each
//!    request for the key as one it could not serve.
//! 4. A run hands the endpoint's calls to the engine (`endpoint::apply_to`),
//!    moves its clock to the wall clock and ticks it until nothing more is due
//!    at that instant.
//! 5. A save is the generated `save_at` at the engine's own instant, which its last run moved
//!    to the wall clock, so a delayed send is saved as the moment it comes due.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::marker::PhantomData;

use sce_rust_runtime::saved_state::{PersistentPolicy, SavedState};
use sce_rust_runtime::{Engine, SceClock};

use crate::endpoint::{self, EngineCall, Environment, Transport};
use crate::pool::{SessionEndpoint, SessionFactory, SessionMachine};

/// The most ticks one run takes at one instant. A tick takes every event that
/// is due when it begins; a send armed with no delay by that tick is due at
/// the same instant, and is taken by the next. A document that keeps arming
/// one is left to its next run, which the pool makes at the earliest due the
/// session's checkpoint names, rather than run without end.
pub const TICKS_PER_RUN: usize = 64;

/// What a deployment gives the machine of each session.
///
/// Both calls are made for a start and for a restore alike, so a session is
/// built around the same values and serves the same operations whether it was
/// started in this process or restored from the store.
pub trait SessionHost<P: PersistentPolicy>: Send + Sync {
    /// The policy the machine of `key` is built around: the generated
    /// `<Machine>Policy::new()`, or one given the values its deployment
    /// supplies. Refused, with the reason, when it cannot be built; the pool
    /// serves no session for the key then.
    fn policy(&self, key: &str) -> Result<P, String>;

    /// Register on `engine`, the machine of `key`, the host operations the
    /// deployment serves it (`Engine::register_invoker`, an event processor):
    /// called after its Mesh endpoint is registered and before it takes its
    /// first event.
    fn register(&self, key: &str, engine: &mut Engine<P>);
}

/// The [`SessionFactory`] of a generated machine whose policy is `P`, built
/// for each session as `host` says.
pub struct Served<P, H> {
    host: H,
    machine: PhantomData<fn() -> P>,
}

impl<P, H> Served<P, H> {
    /// The factory of sessions of `P`'s machine, each given what `host` gives.
    pub fn new(host: H) -> Self {
        Self {
            host,
            machine: PhantomData,
        }
    }

    /// What the factory gives each session.
    pub fn host(&self) -> &H {
        &self.host
    }
}

/// One session's generated machine.
pub struct ServedMachine<P: PersistentPolicy> {
    engine: Engine<P>,
}

impl<P: PersistentPolicy> ServedMachine<P> {
    /// The engine, for a host that reads what the machine holds.
    pub fn engine(&self) -> &Engine<P> {
        &self.engine
    }
}

impl<P, H, T, E> SessionFactory<T, E> for Served<P, H>
where
    P: PersistentPolicy,
    Engine<P>: Send,
    H: SessionHost<P>,
    T: Transport + 'static,
    E: Environment + 'static,
{
    type Machine = ServedMachine<P>;

    fn start(
        &self,
        key: &str,
        endpoint: &SessionEndpoint<T, E>,
        wall_now_ms: u64,
    ) -> Result<ServedMachine<P>, String> {
        let mut engine = Engine::new(self.host.policy(key)?);
        engine.set_clock(SceClock::Manual(wall_now_ms));
        endpoint::register(&mut engine, endpoint);
        self.host.register(key, &mut engine);
        engine
            .initialize()
            .map_err(|failure| format!("the machine of {key} did not start: {failure:?}"))?;
        let mut machine = ServedMachine { engine };
        machine.settle();
        Ok(machine)
    }

    fn restore(
        &self,
        key: &str,
        state: &SavedState,
        endpoint: &SessionEndpoint<T, E>,
        wall_now_ms: u64,
    ) -> Result<ServedMachine<P>, String> {
        let policy = self.host.policy(key)?;
        let mut engine = P::restore_with(policy, state, SceClock::Manual(wall_now_ms), wall_now_ms)
            .map_err(|refusal| refusal.reason().to_string())?;
        endpoint::register(&mut engine, endpoint);
        self.host.register(key, &mut engine);
        Ok(ServedMachine { engine })
    }
}

impl<P: PersistentPolicy> ServedMachine<P> {
    /// Tick while something is due at the engine's instant, at most
    /// [`TICKS_PER_RUN`] times.
    fn settle(&mut self) {
        for _ in 0..TICKS_PER_RUN {
            if self.engine.time_until_next_scheduled_ms() != Some(0) {
                return;
            }
            self.engine.tick();
        }
    }
}

impl<P: PersistentPolicy> SessionMachine for ServedMachine<P>
where
    Engine<P>: Send,
{
    fn run(&mut self, calls: Vec<EngineCall>, wall_now_ms: u64) -> bool {
        let told = !calls.is_empty();
        endpoint::apply_to(&mut self.engine, calls);
        let now = self.engine.now_ms();
        let elapsed = wall_now_ms.saturating_sub(now);
        let came_due = self
            .engine
            .time_until_next_scheduled_ms()
            .is_some_and(|after| after <= elapsed);
        // The clock moves to the wall clock and the engine ticks once at it;
        // a wall clock read behind the engine's leaves the engine where it is.
        if elapsed > 0 {
            self.engine.advance_time_ms(elapsed);
        } else {
            self.engine.tick();
        }
        self.settle();
        told || came_due
    }

    /// The generated `save_at` at the engine's own instant, not at `wall_now_ms`: the engine's
    /// clock is the wall clock as of its last run (the module's step 1), so each delayed send is
    /// saved as the wall-clock moment it comes due. Saved at a later wall clock, every send
    /// would be written as due that much later, and moved again at each stop.
    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        P::save_at(&self.engine, self.engine.now_ms())
            .map_err(|refusal| refusal.reason().to_string())
    }

    fn finished(&self) -> bool {
        self.engine.is_in_final_state()
    }

    /// The facts the engine's macrosteps said since the last take (docs/adr/0114, D1), which the
    /// pool appends with the state that said them, each with whom it was said for as the pool
    /// records a person (docs/adr/0116, D7).
    fn take_journal(&mut self) -> Vec<sce_rust_runtime::journal::JournalEntry> {
        self.engine
            .take_journal()
            .into_iter()
            .map(sce_rust_runtime::journal::JournalEntry::into_record)
            .collect()
    }
}
