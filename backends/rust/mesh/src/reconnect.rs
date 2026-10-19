// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Zenoh binding that opens itself again when its session to the router ends (SCE_MESH.md
//! section 20.3).
//!
//! [`open_with`](crate::inbox::open_with) makes a binding on one client and is over when that
//! client's session is. [`open_reconnecting`] makes the binding a host keeps: a supervisor thread
//! of this module's own owns the current session, and when the client says it ended
//! ([`ZenohClient::on_session_end`]) it waits out a backoff, asks the [`ClientFactory`] for a new
//! client and sets the machine up on it again, through the same code as the plain binding (the end
//! notice, the inbox, presence with history, the token, in that order).
//!
//! What the host reads is one channel through every session. When a session ends, the binding has
//! already told the core every peer it had ready is lost; the presence history of the next session
//! announces whoever is still there as ready again. A peer that left while the session was down
//! cannot be missed, because everyone was lost first and only the ones still there come back.
//!
//! The first connection is made on the caller's thread, so a router address that does not answer
//! is an error of [`open_reconnecting`] and not a silent retry; the later ones are the
//! supervisor's. A send while there is no session is a retryable failure and never waits for the
//! reconnection.
//!
//! This module needs no feature and no dependency beyond the standard library and the core.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::endpoint::{Transport, TransportFailure};
use crate::inbox::{
    check_machine_name, open_into, ClientError, ZenohClient, ZenohError, ZenohEvents,
};
use crate::link::LinkEvent;

/// How a new client is made each time a session has to be opened: the first, and every one after
/// an end. The factory knows the router; a harness or a host makes one that dials it.
pub trait ClientFactory: Send + 'static {
    /// The client this factory makes.
    type Client: ZenohClient;

    /// Make a client with a session of its own. An `Err` is the router not answering, and is
    /// retried after a backoff, except for the first connection, which is the caller's error.
    fn connect(&mut self) -> Result<Self::Client, ClientError>;
}

/// How long the supervisor waits before each attempt to open a session again: `initial` before the
/// first, multiplied by `factor` after each failed one, never more than `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// The wait before the first attempt after a session ended.
    pub initial: Duration,
    /// The longest wait between two attempts, before jitter.
    pub max: Duration,
    /// What each failed attempt multiplies the wait by; below 1 is read as 1.
    pub factor: u32,
    /// Whether each wait is spread by up to 20 percent either way, so machines that lost their
    /// session to one router together do not all dial it in the same instant.
    pub jitter: bool,
}

impl Default for Backoff {
    fn default() -> Self {
        Backoff {
            initial: Duration::from_millis(250),
            max: Duration::from_secs(30),
            factor: 2,
            jitter: true,
        }
    }
}

impl Backoff {
    /// The wait before attempt `attempt` (the first is 0), without jitter: `initial` times
    /// `factor` to the power `attempt`, capped at `max`.
    pub fn base_delay(&self, attempt: u32) -> Duration {
        let factor = self.factor.max(1);
        let mut delay = self.initial.min(self.max);
        for _ in 0..attempt {
            if delay >= self.max || delay.is_zero() || factor == 1 {
                break;
            }
            delay = delay
                .checked_mul(factor)
                .map_or(self.max, |grown| grown.min(self.max));
        }
        delay
    }

    /// The wait before attempt `attempt`: [`base_delay`](Self::base_delay) exactly when `jitter`
    /// is off, and otherwise that, scaled by a draw between 80 and 120 percent. The draw comes
    /// from the standard library's randomly keyed hasher, so no dependency is needed for it.
    pub fn delay(&self, attempt: u32) -> Duration {
        let base = self.base_delay(attempt);
        if !self.jitter {
            return base;
        }
        let mut hasher = RandomState::new().build_hasher();
        attempt.hash(&mut hasher);
        Instant::now().hash(&mut hasher);
        // A draw in 0..=4000, read as -2000..=2000 ten-thousandths: minus to plus 20 percent.
        let spread = (hasher.finish() % 4001) as i64 - 2000;
        let scaled = base.as_nanos() * u128::from((10_000 + spread) as u64) / 10_000;
        Duration::from_nanos(u64::try_from(scaled).unwrap_or(u64::MAX))
    }
}

/// What the supervisor is told.
enum Signal {
    /// The current session ended; the binding has already told the core every peer is lost.
    Ended,
    /// The binding was dropped.
    Stop,
}

/// The transport of the session there is at the moment, or nothing while there is none.
type Slot = Arc<Mutex<Option<Box<dyn Transport>>>>;

fn locked(slot: &Slot) -> MutexGuard<'_, Option<Box<dyn Transport>>> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the supervisor thread owns: how to make a client, where the events go, and the signals it
/// waits on.
struct Supervisor<F: ClientFactory> {
    factory: F,
    machine: String,
    backoff: Backoff,
    /// The channel the host reads, through every session.
    events: mpsc::Sender<LinkEvent>,
    /// What a session's end notice sends on, cloned into each one.
    signals_to_self: mpsc::Sender<Signal>,
    signals: mpsc::Receiver<Signal>,
    slot: Slot,
}

impl<F: ClientFactory> Supervisor<F> {
    /// Make a client and set the machine up on it, the same way the plain binding is, and make it
    /// the one sends go to.
    fn connect_and_install(&mut self) -> Result<(), ZenohError> {
        let client = self.factory.connect()?;
        let ended = self.signals_to_self.clone();
        let transport = open_into(client, &self.machine, self.events.clone(), move |_| {
            let _ = ended.send(Signal::Ended);
        })?;
        *locked(&self.slot) = Some(Box::new(transport));
        Ok(())
    }

    /// Wait for the session to end, wait out the backoff, open another, and again, until stopped.
    fn run(mut self) {
        let mut failures = 0u32;
        'sessions: loop {
            match self.signals.recv() {
                Ok(Signal::Ended) => {}
                Ok(Signal::Stop) | Err(_) => break,
            }
            // The dead session's client is dropped outside the lock, so a send is never kept
            // waiting for it to close.
            let dead = locked(&self.slot).take();
            drop(dead);

            loop {
                match self.signals.recv_timeout(self.backoff.delay(failures)) {
                    Err(RecvTimeoutError::Timeout) => {}
                    Ok(Signal::Stop) | Err(RecvTimeoutError::Disconnected) => break 'sessions,
                    // No session is held while this loop waits, so an end notice here is a
                    // client saying it twice; the wait is not changed by it.
                    Ok(Signal::Ended) => continue,
                }
                match self.connect_and_install() {
                    Ok(()) => {
                        failures = 0;
                        break;
                    }
                    Err(_) => failures = failures.saturating_add(1),
                }
            }
        }
        let last = locked(&self.slot).take();
        drop(last);
    }
}

/// Stops the supervisor when dropped, and waits for it: by then the current client is dropped, so
/// presence has ended.
struct SupervisorGuard {
    signals: mpsc::Sender<Signal>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for SupervisorGuard {
    fn drop(&mut self) {
        let _ = self.signals.send(Signal::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The core's side of a reconnecting binding: bytes published on the current session, or a
/// retryable failure while there is none. Dropping it stops the supervisor and drops the client,
/// which ends this machine's presence.
pub struct ReconnectingTransport {
    slot: Slot,
    _supervisor: SupervisorGuard,
}

impl Transport for ReconnectingTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        // The lock is held for the put and for nothing that waits on a reconnection: the
        // supervisor opens a session with the lock free and takes it only to swap one in or out.
        match locked(&self.slot).as_mut() {
            Some(transport) => transport.transmit(peer, bytes),
            None => Err(TransportFailure {
                retryable: true,
                message: Some("the session to the router is being reopened".to_string()),
            }),
        }
    }
}

/// An open reconnecting binding: the transport to give the
/// [`Endpoint`](crate::endpoint::Endpoint), and the events to feed it, the same type the plain
/// binding returns so a host pumps either the same way.
pub struct ReconnectingBinding {
    pub transport: ReconnectingTransport,
    pub events: ZenohEvents,
}

/// Open the binding for `machine` on a client from `factory`, and open it again on a new one each
/// time its session ends, waiting `backoff` between attempts.
///
/// The name is checked and the first client made and set up before this returns, on the caller's
/// thread, so an address that does not answer is this function's error. Everything after that is a
/// supervisor thread's. Dropping the transport stops it and waits for it to finish, which waits
/// for an attempt already under way: a factory's `connect` that can block for long should bound it.
pub fn open_reconnecting<F: ClientFactory>(
    factory: F,
    machine: &str,
    backoff: Backoff,
) -> Result<ReconnectingBinding, ZenohError> {
    check_machine_name(machine)?;
    let (events_tx, events_rx) = mpsc::channel::<LinkEvent>();
    let (signals_to_self, signals) = mpsc::channel::<Signal>();
    let slot: Slot = Arc::new(Mutex::new(None));
    let mut supervisor = Supervisor {
        factory,
        machine: machine.to_string(),
        backoff,
        events: events_tx,
        signals_to_self: signals_to_self.clone(),
        signals,
        slot: slot.clone(),
    };
    supervisor.connect_and_install()?;

    let thread = std::thread::Builder::new()
        .name("mesh-reconnect".to_string())
        .spawn(move || supervisor.run())
        .map_err(|e| ZenohError::Session(e.to_string()))?;
    Ok(ReconnectingBinding {
        transport: ReconnectingTransport {
            slot,
            _supervisor: SupervisorGuard {
                signals: signals_to_self,
                thread: Some(thread),
            },
        },
        events: ZenohEvents::unending(events_rx),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbox::{Held, PresenceKind, PutError};
    use std::collections::VecDeque;

    type PresenceCallback = Box<dyn FnMut(&str, PresenceKind) + Send>;
    type EndCallback = Box<dyn FnOnce(String) + Send>;

    /// What one fake client was asked, and the callbacks it was given.
    #[derive(Default)]
    struct Handles {
        calls: Vec<&'static str>,
        presence: Option<PresenceCallback>,
        end: Option<EndCallback>,
        puts: Vec<(String, Vec<u8>)>,
        closed: bool,
    }

    struct FakeClient(Arc<Mutex<Handles>>);

    impl FakeClient {
        fn record(&self, call: &'static str) {
            self.0.lock().unwrap().calls.push(call);
        }
    }

    impl Drop for FakeClient {
        fn drop(&mut self) {
            self.0.lock().unwrap().closed = true;
        }
    }

    impl ZenohClient for FakeClient {
        fn subscribe<F>(&mut self, _filter: &str, _on_sample: F) -> Result<Held, ClientError>
        where
            F: FnMut(&str, &[u8]) + Send + 'static,
        {
            self.record("inbox");
            Ok(Box::new(()))
        }

        fn subscribe_presence<F>(
            &mut self,
            _filter: &str,
            _history: bool,
            on_change: F,
        ) -> Result<Held, ClientError>
        where
            F: FnMut(&str, PresenceKind) + Send + 'static,
        {
            self.record("presence");
            self.0.lock().unwrap().presence = Some(Box::new(on_change));
            Ok(Box::new(()))
        }

        fn declare_presence(&mut self, _key: &str) -> Result<Held, ClientError> {
            self.record("token");
            Ok(Box::new(()))
        }

        fn put(&mut self, key: &str, payload: &[u8]) -> Result<(), PutError> {
            self.0
                .lock()
                .unwrap()
                .puts
                .push((key.to_string(), payload.to_vec()));
            Ok(())
        }

        fn on_session_end<F>(&mut self, on_end: F) -> Result<(), ClientError>
        where
            F: FnOnce(String) + Send + 'static,
        {
            self.record("end");
            self.0.lock().unwrap().end = Some(Box::new(on_end));
            Ok(())
        }
    }

    /// What a scripted `connect` does.
    #[derive(Clone, Copy)]
    enum Step {
        Up,
        Down,
    }

    /// What the factory made and when it was asked, read by the test.
    #[derive(Default)]
    struct World {
        clients: Mutex<Vec<Arc<Mutex<Handles>>>>,
        attempts: Mutex<Vec<Instant>>,
    }

    impl World {
        fn client(&self, index: usize) -> Arc<Mutex<Handles>> {
            self.clients.lock().unwrap()[index].clone()
        }

        fn made(&self) -> usize {
            self.clients.lock().unwrap().len()
        }

        fn attempted(&self) -> usize {
            self.attempts.lock().unwrap().len()
        }
    }

    struct ScriptedFactory {
        script: VecDeque<Step>,
        otherwise: Step,
        world: Arc<World>,
    }

    impl ClientFactory for ScriptedFactory {
        type Client = FakeClient;

        fn connect(&mut self) -> Result<FakeClient, ClientError> {
            self.world.attempts.lock().unwrap().push(Instant::now());
            match self.script.pop_front().unwrap_or(self.otherwise) {
                Step::Down => Err(ClientError::new("router down")),
                Step::Up => {
                    let handles = Arc::new(Mutex::new(Handles::default()));
                    self.world.clients.lock().unwrap().push(handles.clone());
                    Ok(FakeClient(handles))
                }
            }
        }
    }

    fn scripted(steps: &[Step], otherwise: Step) -> (ScriptedFactory, Arc<World>) {
        let world = Arc::new(World::default());
        let factory = ScriptedFactory {
            script: steps.iter().copied().collect(),
            otherwise,
            world: world.clone(),
        };
        (factory, world)
    }

    fn exact(initial_ms: u64, max_ms: u64, factor: u32) -> Backoff {
        Backoff {
            initial: Duration::from_millis(initial_ms),
            max: Duration::from_millis(max_ms),
            factor,
            jitter: false,
        }
    }

    fn open(factory: ScriptedFactory, backoff: Backoff) -> ReconnectingBinding {
        match open_reconnecting(factory, "me", backoff) {
            Ok(binding) => binding,
            Err(why) => panic!("the binding did not open: {why}"),
        }
    }

    /// Wait until `condition` holds, for up to `within`; whether it did.
    fn wait_until(within: Duration, condition: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + within;
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        condition()
    }

    fn presence(client: &Arc<Mutex<Handles>>, machine: &str, kind: PresenceKind) {
        let mut handles = client.lock().unwrap();
        let on_change = handles.presence.as_mut().expect("presence was subscribed");
        on_change(&format!("sce-mesh/1/up/{machine}"), kind);
    }

    fn end(client: &Arc<Mutex<Handles>>) {
        let on_end = client.lock().unwrap().end.take().expect("asked for");
        on_end("link reset".to_string());
    }

    fn next(events: &ZenohEvents) -> LinkEvent {
        events
            .next_within(Duration::from_secs(10))
            .expect("an event arrives")
    }

    fn lost(peer: &str) -> LinkEvent {
        LinkEvent::Lost(
            peer.to_string(),
            "the session to the router ended".to_string(),
        )
    }

    const SETUP_ORDER: [&str; 4] = ["end", "inbox", "presence", "token"];

    #[test]
    fn the_wait_doubles_to_its_cap_and_stays_there() {
        let backoff = Backoff {
            jitter: false,
            ..Backoff::default()
        };
        let waits: Vec<Duration> = (0..9).map(|n| backoff.delay(n)).collect();
        let millis = [250, 500, 1000, 2000, 4000, 8000, 16000, 30000, 30000];
        assert_eq!(waits, millis.map(Duration::from_millis));
        assert_eq!(backoff.delay(u32::MAX), Duration::from_secs(30));
    }

    #[test]
    fn a_wait_that_cannot_grow_does_not_loop() {
        for backoff in [exact(0, 100, 2), exact(40, 100, 0), exact(40, 100, 1)] {
            let wait = backoff.base_delay(u32::MAX);
            assert!(wait <= Duration::from_millis(100));
        }
        assert_eq!(exact(0, 100, 2).delay(5), Duration::ZERO);
        assert_eq!(exact(40, 100, 1).delay(5), Duration::from_millis(40));
        assert_eq!(
            exact(500, 100, 2).delay(0),
            Duration::from_millis(100),
            "an initial wait above the cap is the cap"
        );
    }

    #[test]
    fn the_default_backoff_is_jittered_and_starts_at_a_quarter_second() {
        let backoff = Backoff::default();
        assert!(backoff.jitter);
        assert_eq!(backoff.initial, Duration::from_millis(250));
        assert_eq!(backoff.max, Duration::from_secs(30));
        assert_eq!(backoff.factor, 2);
    }

    #[test]
    fn jitter_stays_within_a_fifth_of_the_base_and_varies() {
        let backoff = Backoff {
            jitter: true,
            ..exact(1000, 60_000, 2)
        };
        let mut seen = std::collections::BTreeSet::new();
        for attempt in 0..6 {
            let base = backoff.base_delay(attempt);
            for _ in 0..50 {
                let wait = backoff.delay(attempt);
                assert!(
                    wait.as_nanos() * 10 >= base.as_nanos() * 8,
                    "{wait:?} under 80% of {base:?}"
                );
                assert!(
                    wait.as_nanos() * 10 <= base.as_nanos() * 12,
                    "{wait:?} over 120% of {base:?}"
                );
                seen.insert(wait);
            }
        }
        assert!(seen.len() > 10, "the draws did not vary: {}", seen.len());
    }

    #[test]
    fn the_first_connect_error_is_the_callers_and_nothing_is_retried() {
        let (factory, world) = scripted(&[Step::Down], Step::Up);
        match open_reconnecting(factory, "me", exact(1, 1, 1)) {
            Err(ZenohError::Session(why)) => assert_eq!(why, "router down"),
            Err(other) => panic!("a first connect error gave {other}"),
            Ok(_) => panic!("a first connect error opened a binding"),
        }
        assert_eq!(world.attempted(), 1);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(world.attempted(), 1, "no supervisor was left behind");
    }

    #[test]
    fn a_name_that_is_not_a_key_chunk_costs_no_connection() {
        let (factory, world) = scripted(&[], Step::Up);
        match open_reconnecting(factory, "a/b", exact(1, 1, 1)) {
            Err(ZenohError::NotAKeyChunk(name)) => assert_eq!(name, "a/b"),
            Err(other) => panic!("a bad name gave {other}"),
            Ok(_) => panic!("a bad name opened a binding"),
        }
        assert_eq!(world.attempted(), 0);
    }

    #[test]
    fn after_the_end_everyone_is_lost_first_and_the_history_announces_who_is_still_there() {
        let (factory, world) = scripted(&[Step::Up, Step::Down, Step::Down, Step::Up], Step::Up);
        let binding = open(factory, exact(20, 400, 2));
        let first = world.client(0);
        assert_eq!(first.lock().unwrap().calls, SETUP_ORDER);

        presence(&first, "alice", PresenceKind::Put);
        presence(&first, "bob", PresenceKind::Put);
        assert_eq!(next(&binding.events), LinkEvent::Ready("alice".to_string()));
        assert_eq!(next(&binding.events), LinkEvent::Ready("bob".to_string()));

        let ended_at = Instant::now();
        end(&first);
        assert_eq!(next(&binding.events), lost("alice"));
        assert_eq!(next(&binding.events), lost("bob"));

        assert!(
            wait_until(Duration::from_secs(10), || {
                world.attempted() == 4
                    && world.made() == 2
                    && world.client(1).lock().unwrap().calls.len() == 4
            }),
            "the second session was opened after two refusals"
        );
        let attempts = world.attempts.lock().unwrap().clone();
        assert!(attempts[1] - ended_at >= Duration::from_millis(20));
        assert!(attempts[2] - attempts[1] >= Duration::from_millis(40));
        assert!(attempts[3] - attempts[2] >= Duration::from_millis(80));
        assert_eq!(
            world.client(1).lock().unwrap().calls,
            SETUP_ORDER,
            "the new session is set up in the order the first one was"
        );
        assert!(
            world.client(0).lock().unwrap().closed,
            "the dead client was dropped"
        );

        // Alice is still there and bob is not: only alice is announced, after both were lost.
        presence(&world.client(1), "alice", PresenceKind::Put);
        assert_eq!(next(&binding.events), LinkEvent::Ready("alice".to_string()));
        assert_eq!(binding.events.try_next(), None, "bob did not come back");
        assert_eq!(binding.events.session_ended(), None);
    }

    #[test]
    fn a_send_during_the_gap_is_retryable_and_does_not_wait() {
        let (factory, world) = scripted(&[Step::Up], Step::Down);
        let ReconnectingBinding {
            mut transport,
            events: _events,
        } = open(factory, exact(60_000, 60_000, 1));
        let first = world.client(0);

        transport
            .transmit("alice", b"before")
            .expect("a session carries it");
        assert_eq!(
            first.lock().unwrap().puts,
            [("sce-mesh/1/to/alice/me".to_string(), b"before".to_vec())]
        );

        end(&first);
        assert!(
            wait_until(Duration::from_secs(10), || first.lock().unwrap().closed),
            "the supervisor dropped the dead client"
        );
        let asked = Instant::now();
        let failure = transport
            .transmit("alice", b"during")
            .expect_err("no session is open");
        assert!(asked.elapsed() < Duration::from_secs(5), "the send waited");
        assert_eq!(
            failure,
            TransportFailure {
                retryable: true,
                message: Some("the session to the router is being reopened".to_string())
            }
        );
        assert_eq!(first.lock().unwrap().puts.len(), 1, "nothing was put");
    }

    #[test]
    fn dropping_during_a_backoff_returns_at_once() {
        let (factory, world) = scripted(&[Step::Up], Step::Down);
        let ReconnectingBinding { transport, events } = open(factory, exact(60_000, 60_000, 1));
        end(&world.client(0));
        assert!(wait_until(Duration::from_secs(10), || world
            .client(0)
            .lock()
            .unwrap()
            .closed));

        let dropped_at = Instant::now();
        drop(transport);
        drop(events);
        assert!(
            dropped_at.elapsed() < Duration::from_secs(5),
            "the supervisor slept its backoff out"
        );
        assert_eq!(world.attempted(), 1, "no attempt was made after the drop");
    }

    #[test]
    fn dropping_the_transport_closes_the_current_client_and_stops_reconnecting() {
        let (factory, world) = scripted(&[], Step::Up);
        let ReconnectingBinding { transport, events } = open(factory, exact(1, 1, 1));
        let first = world.client(0);
        assert!(!first.lock().unwrap().closed);

        drop(transport);
        assert!(
            first.lock().unwrap().closed,
            "the client is closed when the drop returns"
        );
        drop(events);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(world.attempted(), 1);
    }

    #[test]
    fn the_wait_starts_over_once_a_session_is_made() {
        // Three waits of 10, 80 and 640 ms; had the count not started over, the wait after the
        // second end would be 5.12 seconds.
        let (factory, world) = scripted(&[Step::Up, Step::Down, Step::Down, Step::Up], Step::Up);
        let _binding = open(factory, exact(10, 60_000, 8));
        end(&world.client(0));
        assert!(wait_until(Duration::from_secs(10), || world.made() == 2
            && world.client(1).lock().unwrap().calls.len() == 4));

        end(&world.client(1));
        assert!(
            wait_until(Duration::from_secs(3), || world.made() == 3),
            "the third session was opened after the initial wait"
        );
    }
}
