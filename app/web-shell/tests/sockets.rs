// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The server over real sockets: what it answers, and that no client can make
//! it stop answering others.
//!
//! The case that earned this file: a request that announces a large body and
//! never sends it. The first server this shell used kept a thread reading after
//! it had replied, so a few such requests (made before any token was looked at)
//! left nothing to serve the next caller. Each case here is a client that stops
//! partway, and the assertion is always the same: a request made afterwards is
//! still answered.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use sce_app_core::{NoRenderer, WorkStore};
use sce_web_shell::server::{serve, Limits};
use sce_web_shell::{Shell, MAX_BODY_BYTES};
use serde_json::json;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// How long a test waits for something that should be immediate.
const PROMPT: Duration = Duration::from_secs(3);

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("sockets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A server on a free loopback port, stopped when this is dropped.
struct Running {
    port: u16,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
    _scratch: Scratch,
}

impl Running {
    fn start(name: &str, limits: Limits) -> Running {
        let scratch = Scratch::new(name);
        let shell = Arc::new(Shell::new(
            WorkStore::at(scratch.0.join("works")),
            Box::new(NoRenderer),
            TOKEN.to_string(),
            None,
        ));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                serve(shell, listener, limits, async {
                    let _ = stopped.await;
                })
                .await;
            });
        });
        Running {
            port,
            stop: Some(stop),
            thread: Some(thread),
            _scratch: scratch,
        }
    }

    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.set_read_timeout(Some(PROMPT)).unwrap();
        stream
    }

    /// One complete request and everything the server says back.
    fn exchange(&self, authorization: Option<&str>, body: &str) -> String {
        let mut stream = self.connect();
        let auth = authorization.map_or(String::new(), |a| format!("Authorization: {a}\r\n"));
        write!(
            stream,
            "POST /api/call HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        read_all(&mut stream)
    }

    fn describe(&self) -> String {
        self.exchange(
            Some(&format!("Bearer {TOKEN}")),
            &json!({"name": "describe"}).to_string(),
        )
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Read until the server closes the connection, or fail when it does not.
fn read_all(stream: &mut TcpStream) -> String {
    let mut answer = Vec::new();
    match stream.read_to_end(&mut answer) {
        Ok(_) => {}
        // A reset after the answer is the server closing on a client that was
        // still sending; what arrived before it is the answer.
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset && !answer.is_empty() => {}
        Err(e) => panic!("the server did not finish the answer: {e} (read so far: {answer:?})"),
    }
    String::from_utf8_lossy(&answer).into_owned()
}

fn quick() -> Limits {
    Limits {
        header_read: Duration::from_millis(400),
        body_read: Duration::from_millis(400),
        connections: 64,
    }
}

#[test]
fn a_call_with_the_token_is_answered_with_its_security_headers() {
    let server = Running::start("answer", Limits::default());
    let answer = server.describe();
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    let lower = answer.to_ascii_lowercase();
    assert!(lower.contains("cache-control: no-store"), "{answer}");
    assert!(
        lower.contains("x-content-type-options: nosniff"),
        "{answer}"
    );
    assert!(
        lower.contains("content-security-policy: default-src 'self'"),
        "{answer}"
    );
    assert!(answer.contains("command_set_version"), "{answer}");
}

#[test]
fn a_call_without_the_token_is_refused() {
    let server = Running::start("no-token", Limits::default());
    let answer = server.exchange(None, &json!({"name": "describe"}).to_string());
    assert!(answer.starts_with("HTTP/1.1 401"), "{answer}");
}

#[test]
fn a_body_announced_larger_than_any_source_is_refused_unread() {
    // The body timeout is long on purpose: a 413 that arrives anyway was not
    // waiting for a body.
    let long = Limits {
        body_read: Duration::from_secs(60),
        ..quick()
    };
    let server = Running::start("announced", long);
    let mut stream = server.connect();
    write!(
        stream,
        "POST /api/call HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\n\r\n",
        MAX_BODY_BYTES + 1
    )
    .unwrap();
    let started = Instant::now();
    let answer = read_all(&mut stream);
    assert!(answer.starts_with("HTTP/1.1 413"), "{answer}");
    assert!(started.elapsed() < PROMPT);
    assert!(server.describe().starts_with("HTTP/1.1 200"));
}

#[test]
fn a_call_without_the_token_is_refused_before_its_body_is_waited_for() {
    let long = Limits {
        body_read: Duration::from_secs(60),
        ..quick()
    };
    let server = Running::start("refuse-first", long);
    let mut stream = server.connect();
    // Announces a body and sends none.
    write!(
        stream,
        "POST /api/call HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1000\r\n\r\n"
    )
    .unwrap();
    let started = Instant::now();
    let answer = read_all(&mut stream);
    assert!(answer.starts_with("HTTP/1.1 401"), "{answer}");
    assert!(started.elapsed() < PROMPT);
}

#[test]
fn a_body_that_stops_arriving_is_given_up_on_and_does_not_hold_the_server() {
    let server = Running::start("stalled-body", quick());
    // Eight clients, more than the first server had threads, each announcing a
    // body, sending a few bytes of it and going quiet.
    let mut stalled: Vec<TcpStream> = (0..8)
        .map(|_| {
            let mut stream = server.connect();
            write!(
                stream,
                "POST /api/call HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 100000\r\n\r\n{{\"name\""
            )
            .unwrap();
            stream
        })
        .collect();

    // Meanwhile, an honest caller is served without waiting for them.
    let started = Instant::now();
    assert!(server.describe().starts_with("HTTP/1.1 200"));
    assert!(started.elapsed() < PROMPT);

    // And each stalled one is told, once its time is up.
    for stream in &mut stalled {
        let answer = read_all(stream);
        assert!(answer.starts_with("HTTP/1.1 408"), "{answer}");
    }
}

#[test]
fn headers_that_stop_arriving_are_given_up_on_and_do_not_hold_the_server() {
    let server = Running::start("stalled-headers", quick());
    let mut stalled: Vec<TcpStream> = (0..8)
        .map(|_| {
            let mut stream = server.connect();
            write!(stream, "POST /api/call HTTP/1.1\r\nHost: local").unwrap();
            stream
        })
        .collect();
    assert!(server.describe().starts_with("HTTP/1.1 200"));
    // The server hangs up on each; whatever it says first, the connection ends.
    for stream in &mut stalled {
        let mut sink = Vec::new();
        let ended = stream.read_to_end(&mut sink);
        assert!(
            ended.is_ok() || ended.is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset),
            "the connection was left open"
        );
    }
}

#[test]
fn past_the_connection_limit_a_client_is_closed_unanswered_and_later_ones_are_served() {
    let one = Limits {
        connections: 1,
        ..quick()
    };
    let server = Running::start("limit", one);

    // This one takes the only place and does not finish its headers.
    let mut holder = server.connect();
    write!(holder, "POST /api/call HTTP/1.1\r\nHost: loc").unwrap();
    thread::sleep(Duration::from_millis(100));

    let mut turned_away = server.connect();
    write!(turned_away, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut sink = Vec::new();
    let result = turned_away.read_to_end(&mut sink);
    assert!(
        sink.is_empty(),
        "a server out of room answered: {sink:?} ({result:?})"
    );

    // The holder times out and the place is free again.
    let _ = holder.read_to_end(&mut sink);
    assert!(server.describe().starts_with("HTTP/1.1 200"));
}

#[test]
fn a_connection_serves_one_request_and_closes() {
    let server = Running::start("one-request", Limits::default());
    let mut stream = server.connect();
    let call = json!({"name": "describe"}).to_string();
    for _ in 0..2 {
        // The second write goes to a connection the server has closed, or is
        // closing; the client sees the end, not a second answer.
        let _ = write!(
            stream,
            "POST /api/call HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\n\r\n{call}",
            call.len()
        );
    }
    let answer = read_all(&mut stream);
    assert_eq!(answer.matches("HTTP/1.1 ").count(), 1, "{answer}");
}
