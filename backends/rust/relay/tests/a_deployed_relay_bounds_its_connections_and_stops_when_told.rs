// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The relay's server as it is deployed (`serve::Server`). It holds no more connections open than its
// bound and closes the one past it at once; a connection that does not finish its upgrade in time,
// and an open one that asks nothing for longer than its bound, are closed; and a stop closes the
// listener and each connection once it has answered the request it was answering, which the server
// waits for. What it counts is how many, never who or what.

use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sce_rust_relay::serve::{Bounds, Counts, Server, WssLink};
use sce_rust_relay::{
    Authority, Hub, Limits, MemoryStore, Notice, Relay, Request, Response, Session, Signal,
    TokenError, Transport, TransportError,
};
use tokio::net::TcpListener;

const LIST: &str = "home";

/// A relay that takes `delay` over each request, and counts the requests it began.
struct Slow {
    relay: Mutex<Relay<MemoryStore>>,
    delay: Duration,
    began: AtomicU64,
}

impl Hub for Slow {
    fn authenticate(&self, token: &str, now_ms: u64) -> Result<Session, TokenError> {
        self.relay.authenticate(token, now_ms)
    }

    fn handle(&self, token: &str, request: &str, now_ms: u64) -> String {
        self.began.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.delay);
        self.relay.handle(token, request, now_ms)
    }
}

struct Running {
    runtime: tokio::runtime::Runtime,
    server: Server<Slow>,
    hub: Arc<Slow>,
    listening: tokio::task::JoinHandle<()>,
    address: SocketAddr,
    base: String,
    token: String,
}

fn start(bounds: Bounds, delay: Duration) -> Running {
    let authority = Authority::new([4; 32], "a-deployed-relay");
    let hub = Arc::new(Slow {
        relay: Mutex::new(
            Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
        ),
        delay,
        began: AtomicU64::new(0),
    });
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    let server = Server::new(hub.clone(), clock, bounds);
    let listening = runtime.spawn({
        let server = server.clone();
        async move { server.listen(listener, None).await }
    });
    Running {
        runtime,
        server,
        hub,
        listening,
        address,
        base: format!("ws://{address}"),
        token: authority.issue(LIST, 1, u64::MAX),
    }
}

/// The server's counts once `holds` of them, waited for for at most five seconds.
fn once(server: &Server<Slow>, what: &str, holds: impl Fn(&Counts) -> bool) -> Counts {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let counts = server.counts();
        if holds(&counts) {
            return counts;
        }
        assert!(Instant::now() < deadline, "{what}: {counts:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn entries() -> String {
    Request::Entries { from: 0 }.to_json()
}

fn empty_log() -> Result<Response, String> {
    Ok(Response::Entries {
        entries: Vec::new(),
        len: 0,
    })
}

#[test]
fn a_connection_past_the_bound_is_closed_and_one_is_taken_when_a_place_is_free() {
    let running = start(
        Bounds {
            max_connections: 1,
            ..Bounds::default()
        },
        Duration::ZERO,
    );
    let _keep = &running.runtime;
    let none = Signal::new();

    let mut first = WssLink::new(&running.base, &running.token).unwrap();
    let answer = first.exchange(&entries(), None, &none).unwrap();
    assert_eq!(Response::from_json(&answer), empty_log());

    let mut second = WssLink::new(&running.base, &running.token).unwrap();
    let refused = second.exchange(&entries(), None, &none);
    assert!(
        matches!(refused, Err(TransportError::Link(_))),
        "{refused:?}"
    );
    let counts = once(&running.server, "the second was refused", |counts| {
        counts.refused == 1
    });
    assert_eq!((counts.accepted, counts.open), (1, 1));

    drop(first);
    once(&running.server, "the first closed", |counts| {
        counts.open == 0
    });
    let answer = second.exchange(&entries(), None, &none).unwrap();
    assert_eq!(Response::from_json(&answer), empty_log());
    assert_eq!(running.server.counts().requests, 2);
}

#[test]
fn a_connection_that_does_not_finish_its_upgrade_in_time_is_closed() {
    let running = start(
        Bounds {
            handshake: Duration::from_millis(200),
            ..Bounds::default()
        },
        Duration::ZERO,
    );
    let _keep = &running.runtime;

    // It connects and says nothing.
    let mut silent = TcpStream::connect(running.address).unwrap();
    silent
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(
        silent.read(&mut byte).unwrap(),
        0,
        "the server closed it, and said nothing"
    );
    let counts = once(&running.server, "it was not opened", |counts| {
        counts.unopened == 1 && counts.open == 0
    });
    assert_eq!(counts.accepted, 1);
}

#[test]
fn an_open_connection_that_asks_nothing_for_longer_than_its_bound_is_closed() {
    let running = start(
        Bounds {
            idle: Duration::from_secs(1),
            ..Bounds::default()
        },
        Duration::ZERO,
    );
    let _keep = &running.runtime;

    let mut link = WssLink::new(&running.base, &running.token).unwrap();
    let asked = Instant::now();
    link.exchange(&entries(), None, &Signal::new()).unwrap();
    assert_eq!(
        running.server.counts().open,
        1,
        "it stays open after it asked"
    );
    once(&running.server, "the idle one was closed", |counts| {
        counts.open == 0
    });
    assert!(
        asked.elapsed() >= Duration::from_secs(1),
        "not before its bound"
    );
}

/// The link held the connection the relay closed for its idleness: the next exchange reads that
/// the relay closed it, dials again and is answered, and the relay is asked each request once. A
/// device whose last round was longer ago than the relay's bound used to fail its next round as
/// the link's failure, and only the round after it was answered.
#[test]
fn a_link_whose_connection_the_relay_closed_for_its_idleness_dials_again_and_is_answered() {
    let running = start(
        Bounds {
            idle: Duration::from_millis(300),
            ..Bounds::default()
        },
        Duration::ZERO,
    );
    let _keep = &running.runtime;

    let mut link = WssLink::new(&running.base, &running.token).unwrap();
    link.exchange(&entries(), None, &Signal::new()).unwrap();
    assert_eq!(link.take_notices(), vec![Notice::Connected]);
    once(&running.server, "the idle one was closed", |counts| {
        counts.open == 0
    });

    let answered = link.exchange(&entries(), None, &Signal::new());
    assert!(answered.is_ok(), "{answered:?}");
    assert_eq!(
        link.take_notices(),
        vec![
            Notice::Lost("the relay closed the connection while it was idle".to_string()),
            Notice::Connected
        ]
    );
    assert_eq!(
        running.hub.began.load(Ordering::SeqCst),
        2,
        "each request reached the relay once"
    );
}

#[test]
fn a_stop_closes_the_listener_and_each_connection_once_it_has_answered() {
    let running = start(Bounds::default(), Duration::from_millis(500));

    let (base, token) = (running.base.clone(), running.token.clone());
    let asking = std::thread::spawn(move || {
        let mut link = WssLink::new(&base, &token).unwrap();
        link.exchange(&entries(), None, &Signal::new())
    });
    // The relay has begun to answer: the stop comes in the middle of it.
    let deadline = Instant::now() + Duration::from_secs(5);
    while running.hub.began.load(Ordering::SeqCst) == 0 {
        assert!(
            Instant::now() < deadline,
            "the request never reached the relay"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    running.server.stop().raise();
    assert!(running.server.stop().is_raised());

    let answer = asking
        .join()
        .unwrap()
        .expect("the request it began is answered");
    assert_eq!(Response::from_json(&answer), empty_log());

    running.runtime.block_on(running.listening).unwrap();
    let open = running
        .runtime
        .block_on(running.server.drain(Duration::from_secs(5)));
    assert_eq!(open, 0, "every connection closed");
    assert!(
        TcpStream::connect(running.address).is_err(),
        "the listener is closed"
    );
    assert_eq!(running.server.counts().requests, 1);
}
