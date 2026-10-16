// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared schedule off the simulation. Every other test of this world runs the copies in
// one process over a network the test owns, where time is virtual and a run is the same run
// each time; that cannot say whether the copies agree once their messages cross real
// sockets, between processes that are scheduled by an operating system. Here each copy is a
// process of its own (`todo_peer_wss`, the same generated machine served by a Mesh endpoint
// on the WebSocket binding), the hub listens on a loopback port and the people dial it, and
// the test drives them with the same events the simulated tests give theirs.
//
// What is asked is what the simulation was asked: that the copies come to hold one list and
// one digest whatever order the network delivered in, that the instant they settle among
// themselves (`todo.settle`) is the one every copy then forgets up to, that the agenda is the
// same on every copy, and that a copy that is killed and started again is repaired by the
// digest walk, over a link that really closed, and settles with the others after it.
//
// By default the three processes run on this machine, on loopback. With
// `SCE_SOCKET_PLACES=server=<host>,alice=<host>,bob=<host>` and
// `SCE_SOCKET_SERVER_ADDRESS=<the hub's address>` each runs on the machine named, started
// with ssh from this one, and the people dial the hub across the network.

mod common;

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use common::calendar::*;
use common::whole;
use serde_json::{json, Value};

const BINARY: &str = env!("CARGO_BIN_EXE_todo_peer_wss");

/// How long a copy may take to do what a test waits for: sockets on one machine answer
/// in milliseconds, and a copy that has not by then is not going to.
const PATIENCE: Duration = Duration::from_secs(60);

const MINUTE_MS: i64 = 60_000;

/// One copy of the list: a process, the lines it said, and the ends of its pipes.
struct Copy {
    name: &'static str,
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    port: Option<u16>,
    linked: Vec<String>,
}

/// The host a copy runs on, when the run puts copies on other machines:
/// `SCE_SOCKET_PLACES=server=pc3,alice=pc2,bob=hkl002`. Each is reached with ssh, whose
/// standard input and output carry the lines the host drives it with, and runs the binary
/// that is built there (`SCE_SOCKET_REMOTE_BINARY`, the checkout `bx` keeps by default).
/// Without it every copy is a process of this machine, on loopback.
fn place_of(name: &str) -> Option<String> {
    let places = std::env::var("SCE_SOCKET_PLACES").ok()?;
    let prefix = format!("{name}=");
    places
        .split(',')
        .find_map(|place| place.strip_prefix(&prefix).map(str::to_string))
}

/// The address the people dial the hub at: its own machine's, when it is on another.
fn server_address() -> String {
    std::env::var("SCE_SOCKET_SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string())
}

impl Copy {
    fn start(name: &'static str, node: u64, address: &str) -> Copy {
        let mut command = match place_of(name) {
            Some(host) => {
                let binary = std::env::var("SCE_SOCKET_REMOTE_BINARY").unwrap_or_else(|_| {
                    "~/remote-build/wire64/target/debug/todo_peer_wss".to_string()
                });
                let mut command = Command::new("ssh");
                command
                    .args(["-o", "BatchMode=yes", &host])
                    .arg(format!("{binary} {name} {node} {address}"));
                command
            }
            None => {
                let mut command = Command::new(BINARY);
                command.args([name, &node.to_string(), address]);
                command
            }
        };
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("start {name}: {e}"));
        let stdin = child.stdin.take().expect("a piped stdin");
        let stdout = child.stdout.take().expect("a piped stdout");
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stdout)
                .lines()
                .map_while(Result::ok)
            {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Copy {
            name,
            child,
            stdin,
            lines,
            port: None,
            linked: Vec::new(),
        }
    }

    /// The next line the copy says that `want` takes; the lines before it are noted
    /// (a link that came up) or dropped.
    fn line_where(&mut self, want: impl Fn(&str) -> bool, what: &str) -> String {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let left = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_else(|| panic!("{} did not say {what} in time", self.name));
            let line = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("{} did not say {what} in time", self.name));
            if let Some(peer) = line.strip_prefix("LINK ") {
                self.linked.push(peer.to_string());
            }
            if let Some(port) = line.strip_prefix("LISTENING ") {
                self.port = port.parse().ok();
            }
            if want(&line) {
                return line;
            }
        }
    }

    fn command(&mut self, command: &Value) {
        writeln!(self.stdin, "{command}").unwrap_or_else(|e| panic!("{} is gone: {e}", self.name));
        self.stdin.flush().expect("a flushed pipe");
    }

    /// The host gives the machine an event.
    fn raise(&mut self, event: &str, data: &str) {
        self.command(&json!({"raise": event, "data": data}));
    }

    /// The copy's variables, as it saves them.
    fn variables(&mut self) -> Value {
        self.command(&json!({"saved": true}));
        let line = self.line_where(|line| line.starts_with("SAVED "), "its saved state");
        let saved: Value =
            serde_json::from_str(line.trim_start_matches("SAVED ")).expect("a saved state is JSON");
        saved["variables"].clone()
    }

    fn wait_linked(&mut self, peer: &str) {
        if self.linked.iter().any(|linked| linked == peer) {
            return;
        }
        let want = format!("LINK {peer}");
        self.line_where(|line| line == want, &want);
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The hub and both people, linked.
struct Room {
    server: Copy,
    alice: Copy,
    bob: Copy,
    now: i64,
}

fn open_room() -> Room {
    let listen = if place_of("server").is_some() {
        "0.0.0.0:0"
    } else {
        "127.0.0.1:0"
    };
    let mut server = Copy::start("server", 100, listen);
    server.line_where(|line| line.starts_with("LISTENING "), "its port");
    let url = format!("ws://{}:{}", server_address(), server.port.expect("a port"));
    let mut alice = Copy::start("alice", 1, &url);
    let mut bob = Copy::start("bob", 2, &url);
    alice.wait_linked("server");
    bob.wait_linked("server");
    server.wait_linked("alice");
    server.wait_linked("bob");
    Room {
        server,
        alice,
        bob,
        now: instant(2026, 3, 1, 0, 0),
    }
}

impl Room {
    fn copies(&mut self) -> [&mut Copy; 3] {
        [&mut self.alice, &mut self.bob, &mut self.server]
    }

    /// A person changes a field of a to-do, when the host's clock reads the room's time.
    fn set(&mut self, who: &str, id: u64, field: u64, value: i64) {
        self.now += 1;
        let data = format!(
            r#"{{"id":{id},"field":{field},"value":"{value}","text":"","now":"{}"}}"#,
            self.now
        );
        match who {
            "alice" => self.alice.raise("todo.edit", &data),
            "bob" => self.bob.raise("todo.edit", &data),
            other => panic!("{other} is not a person"),
        }
    }

    /// Every to-do repeats daily from a start, an hour long.
    fn schedule(&mut self, who: &str, id: u64) {
        self.set(who, id, START, instant(2026, 3, 1, 9, 0));
        self.set(who, id, LENGTH, HOUR_MS);
        self.set(who, id, REPEAT, DAILY);
    }

    /// A day: both people move and lengthen a few events, concurrently.
    fn an_afternoon(&mut self, day: i64) {
        for (who, id) in [("alice", 1), ("bob", 2), ("alice", 3), ("bob", 1)] {
            let start = instant(2026, 3, 1 + (day % 28), 8 + id as i64, 0);
            self.set(who, id, START, start);
            self.set(who, id, LENGTH, HOUR_MS * (1 + day % 3));
        }
        self.now += MINUTE_MS;
    }

    /// Until the three hold one list and one digest, which they do when what each sent
    /// has arrived and been taken in everywhere.
    fn wait_agreed(&mut self) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let held: Vec<(Value, Value)> = self
                .copies()
                .iter_mut()
                .map(|copy| {
                    let variables = copy.variables();
                    (variables["values"].clone(), variables["digest"].clone())
                })
                .collect();
            let same_digest = held.iter().all(|(_, digest)| *digest == held[0].1);
            // The values arrive in the order each copy took them in, which is the network's.
            let sorted = |values: &Value| {
                let mut each: Vec<String> = values
                    .as_array()
                    .expect("a list")
                    .iter()
                    .map(Value::to_string)
                    .collect();
                each.sort();
                each
            };
            let same_values = held
                .iter()
                .all(|(values, _)| sorted(values) == sorted(&held[0].0));
            if same_digest && same_values && !held[0].0.as_array().expect("a list").is_empty() {
                return held[0].0.clone();
            }
            assert!(
                Instant::now() < deadline,
                "the copies did not come to hold one list: digests {:?}",
                held.iter()
                    .map(|(_, digest)| digest.to_string().len())
                    .collect::<Vec<_>>()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Until every copy's floor is `before`.
    fn wait_floor(&mut self, before: i64) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let floors: Vec<i128> = self
                .copies()
                .iter_mut()
                .map(|copy| whole(&copy.variables()["floor"]))
                .collect();
            if floors.iter().all(|floor| *floor == i128::from(before)) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the copies did not settle {before}: floors {floors:?}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn settle(&mut self, before: i64) {
        self.server.raise(
            "todo.settle",
            &format!(r#"{{"before":"{before}","members":2}}"#),
        );
        self.wait_floor(before);
    }

    /// What each copy says is scheduled for a to-do in a window.
    fn agendas(&mut self, id: u64) -> Vec<Value> {
        let data = format!(
            r#"{{"id":{id},"windowStart":"{}","windowEnd":"{}","skip":0}}"#,
            instant(2026, 3, 1, 0, 0),
            instant(2026, 4, 1, 0, 0)
        );
        self.copies()
            .iter_mut()
            .map(|copy| {
                copy.raise("todo.agenda", &data);
                copy.variables()["agenda"].clone()
            })
            .collect()
    }
}

fn held(copy: &mut Copy, list: &str) -> usize {
    copy.variables()[list].as_array().expect("a list").len()
}

/// Three processes on loopback sockets hold one list after a fortnight of concurrent
/// changes, settle an instant by themselves, forget up to it, and show one agenda.
#[test]
fn three_processes_over_real_sockets_agree_and_settle() {
    let mut room = open_room();
    for id in 1..=3 {
        room.schedule("alice", id);
    }
    for day in 0..14 {
        room.an_afternoon(day);
    }
    let values = room.wait_agreed();
    let adds = held(&mut room.server, "adds");
    let agendas = room.agendas(1);
    assert!(!agendas[0].as_array().expect("a list").is_empty());
    assert!(agendas.iter().all(|agenda| *agenda == agendas[0]));

    let before = room.now;
    room.now += MINUTE_MS;
    room.settle(before);

    for copy in room.copies() {
        let variables = copy.variables();
        assert_eq!(variables["stale"], json!(false), "{} is stale", copy.name);
        assert_eq!(
            whole(&variables["late"]),
            0,
            "{} refused a write",
            copy.name
        );
        assert_eq!(whole(&variables["refusals"]), 0);
    }
    assert!(
        held(&mut room.server, "adds") < adds,
        "the hub still holds the {adds} adds it held before it settled"
    );
    let after = room.wait_agreed();
    let sorted = |values: &Value| {
        let mut each: Vec<String> = values
            .as_array()
            .expect("a list")
            .iter()
            .map(Value::to_string)
            .collect();
        each.sort();
        each
    };
    assert_eq!(sorted(&after), sorted(&values), "settling changed a value");
    let agendas_after = room.agendas(1);
    assert_eq!(agendas_after, agendas, "settling changed the agenda");
}

/// A copy that is killed has its link really closed; the hub keeps changing; the copy is
/// started again from nothing, dials the hub, and is walked to what the others hold, and
/// the three then settle an instant together.
#[test]
fn a_killed_copy_is_repaired_over_real_sockets_and_then_settles() {
    let mut room = open_room();
    for id in 1..=3 {
        room.schedule("alice", id);
    }
    for day in 0..6 {
        room.an_afternoon(day);
    }
    room.wait_agreed();

    // Alice's process dies with her link; bob goes on.
    let url = format!(
        "ws://{}:{}",
        server_address(),
        room.server.port.expect("a port")
    );
    let _ = room.alice.child.kill();
    let _ = room.alice.child.wait();
    for day in 6..12 {
        let start = instant(2026, 3, 1 + (day % 28), 11, 0);
        room.set("bob", 2, START, start);
        room.now += MINUTE_MS;
    }

    // Started again, she holds nothing, and says her link is back.
    room.alice = Copy::start("alice", 1, &url);
    room.alice.wait_linked("server");
    room.server.wait_linked("alice");
    room.alice.raise("todo.resync", "");
    room.bob.raise("todo.resync", "");
    room.wait_agreed();

    let before = room.now;
    room.now += MINUTE_MS;
    room.settle(before);
    for copy in room.copies() {
        assert_eq!(
            copy.variables()["stale"],
            json!(false),
            "{} is stale",
            copy.name
        );
    }
    let agendas = room.agendas(2);
    assert!(agendas.iter().all(|agenda| *agenda == agendas[0]));
}
