// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The wait budget a deploy declares (`session_runtime.rx_busy_poll_us`, SCE
//! Protocol-Synthesis RFC §synth-5-Q), consumed by the code SCE owns for it:
//! the generated `rx_busy_poll()` carries the figure and
//! `sce_link_runtime::busy_poll::read_within` spends it reading the socket
//! with `read(2)`.
//!
//! The sockets are plain `std` ones with no reactor anywhere, which is the
//! point: a wait that ends inside the budget is answered by the descriptor
//! itself, where a `try_read` on a reactor's readiness would not be.

use sce_deploy_conformance::{own_thread_node_session_runtime, pool_node_session_runtime};
use sce_link_runtime::busy_poll::read_within;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// A connected pair: the reading end is non-blocking, as the function requires.
fn pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let writer = TcpStream::connect(listener.local_addr().expect("addr")).expect("connect");
    let (reader, _) = listener.accept().expect("accept");
    reader.set_nonblocking(true).expect("non-blocking");
    (reader, writer)
}

#[test]
fn the_generated_budget_is_the_one_the_deploy_declared() {
    assert_eq!(
        own_thread_node_session_runtime::rx_busy_poll(),
        Duration::ZERO
    );
    assert_eq!(
        pool_node_session_runtime::rx_busy_poll(),
        Duration::from_micros(10)
    );
}

#[test]
fn bytes_already_there_are_read_at_once() {
    let (reader, mut writer) = pair();
    writer.write_all(b"hello").expect("write");
    let mut buf = [0u8; 16];
    // The kernel may need a moment to make the loopback write readable.
    let read = read_within(&reader, &mut buf, Duration::from_secs(5)).expect("read");
    assert_eq!(read, Some(5));
    assert_eq!(&buf[..5], b"hello");
}

#[test]
fn nothing_within_the_budget_answers_none_after_spending_it() {
    let (reader, _writer) = pair();
    let mut buf = [0u8; 16];
    let budget = Duration::from_millis(5);
    let started = Instant::now();
    let read = read_within(&reader, &mut buf, budget).expect("read");
    assert_eq!(read, None);
    assert!(
        started.elapsed() >= budget,
        "the whole budget is spent before the caller is told to wait"
    );
}

#[test]
fn a_zero_budget_is_one_attempt() {
    let (reader, _writer) = pair();
    let mut buf = [0u8; 16];
    assert_eq!(
        read_within(&reader, &mut buf, Duration::ZERO).expect("read"),
        None
    );
}

#[test]
fn bytes_that_arrive_inside_the_budget_are_read_with_no_reactor() {
    let (reader, mut writer) = pair();
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2));
        writer.write_all(b"late").expect("write");
        writer
    });
    let mut buf = [0u8; 16];
    let read = read_within(&reader, &mut buf, Duration::from_secs(5)).expect("read");
    assert_eq!(read, Some(4));
    assert_eq!(&buf[..4], b"late");
    drop(sender.join().expect("the sender finishes"));
}

#[test]
fn the_peers_end_of_stream_is_a_read_of_zero_and_not_a_wait() {
    let (reader, writer) = pair();
    drop(writer);
    let mut buf = [0u8; 16];
    let read = read_within(&reader, &mut buf, Duration::from_secs(5)).expect("read");
    assert_eq!(read, Some(0));
}

#[test]
fn the_generated_budget_drives_the_read() {
    let (reader, _writer) = pair();
    let mut buf = [0u8; 16];
    let started = Instant::now();
    let read =
        read_within(&reader, &mut buf, pool_node_session_runtime::rx_busy_poll()).expect("read");
    assert_eq!(read, None);
    assert!(started.elapsed() >= pool_node_session_runtime::rx_busy_poll());
}
