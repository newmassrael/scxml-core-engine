// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared schedule off the simulation. Every other test of this world runs the copies in
// one process over a network the test owns, where time is virtual and a run is the same run
// each time; that cannot say whether the copies agree once their messages cross real
// sockets, between processes that are scheduled by an operating system. Here each copy is a
// process of its own (`todo_peer_wss`, the same generated machine served by a Mesh endpoint
// on the WebSocket binding, its route a causal group), a relay node that forwards what one
// member sends to the others listens on a loopback port and the copies dial it, and the test
// drives them with the same events the simulated tests give theirs.
//
// What is asked is what the simulation was asked: that the copies come to hold one list
// whatever order the network delivered in, that every copy forgets a change by itself once
// every copy holds it (docs/adr/0085, section 4), that the agenda is the same on every copy,
// and that what a copy made while every link really closed reaches the others once the links
// are back, and the copies then forget it alike.
//
// By default the four processes run on this machine, on loopback. With
// `SCE_SOCKET_PLACES=relay=<host>,alice=<host>,bob=<host>,carol=<host>` and
// `SCE_SOCKET_SERVER_ADDRESS=<the relay's address>` each runs on the machine named, started
// with ssh from this one, and the copies dial the relay across the network.

mod common;

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use common::calendar::*;
use serde_json::{json, Value};

const BINARY: &str = env!("CARGO_BIN_EXE_todo_peer_wss");

/// How long a copy may take to do what a test waits for: sockets on one machine answer
/// in milliseconds, and a copy that has not by then is not going to.
const PATIENCE: Duration = Duration::from_secs(60);

const MINUTE_MS: i64 = 60_000;

/// The binding a copy knows the relay by.
const PEOPLE: &str = "people";

/// One process: a copy of the list or the relay, the lines it said, and the ends of its
/// pipes.
struct Process {
    name: &'static str,
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    port: Option<u16>,
    linked: Vec<String>,
}

/// The host a process runs on, when the run puts processes on other machines:
/// `SCE_SOCKET_PLACES=relay=pc3,alice=pc2,bob=hkl002,carol=pc2`. Each is reached with ssh,
/// whose standard input and output carry the lines the host drives it with, and runs the
/// binary that is built there (`SCE_SOCKET_REMOTE_BINARY`, the checkout `bx` keeps by
/// default). Without it every process is one of this machine, on loopback.
fn place_of(name: &str) -> Option<String> {
    let places = std::env::var("SCE_SOCKET_PLACES").ok()?;
    let prefix = format!("{name}=");
    places
        .split(',')
        .find_map(|place| place.strip_prefix(&prefix).map(str::to_string))
}

/// The address the copies dial the relay at: its own machine's, when it is on another.
fn relay_address() -> String {
    std::env::var("SCE_SOCKET_SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string())
}

impl Process {
    /// Start `name` with `args`, here or where the run places it.
    fn start(name: &'static str, args: &[String]) -> Process {
        let mut command = match place_of(name) {
            Some(host) => {
                let binary = std::env::var("SCE_SOCKET_REMOTE_BINARY").unwrap_or_else(|_| {
                    "~/remote-build/wire64/target/debug/todo_peer_wss".to_string()
                });
                let mut command = Command::new("ssh");
                command
                    .args(["-o", "BatchMode=yes", &host])
                    .arg(format!("{binary} {}", args.join(" ")));
                command
            }
            None => {
                let mut command = Command::new(BINARY);
                command.args(args);
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
        Process {
            name,
            child,
            stdin,
            lines,
            port: None,
            linked: Vec::new(),
        }
    }

    /// A copy of the list, node `node`, dialing the relay at `url`.
    fn copy(name: &'static str, node: u64, url: &str) -> Process {
        Process::start(name, &[name.to_string(), node.to_string(), url.to_string()])
    }

    /// The relay, listening at `listen`; it says the port it was given.
    fn relay(listen: &str) -> Process {
        let mut relay = Process::start("relay", &["relay".to_string(), listen.to_string()]);
        relay.line_where(|line| line.starts_with("LISTENING "), "its port");
        relay
    }

    /// The next line the process says that `want` takes; the lines before it are noted
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
            if let Some(peer) = line.strip_prefix("LOST ") {
                self.linked.retain(|linked| linked != peer);
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

    /// What the copy's causal layer keeps not yet stable, and holds undelivered.
    fn layer(&mut self) -> (usize, usize) {
        self.command(&json!({"layer": true}));
        let line = self.line_where(|line| line.starts_with("LAYER "), "its layer");
        let mut numbers = line
            .trim_start_matches("LAYER ")
            .split(' ')
            .map(|number| number.parse::<usize>().expect("a count"));
        (
            numbers.next().expect("the log"),
            numbers.next().expect("the held"),
        )
    }

    fn wait_linked(&mut self, peer: &str) {
        if self.linked.iter().any(|linked| linked == peer) {
            return;
        }
        let want = format!("LINK {peer}");
        self.line_where(|line| line == want, &want);
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The relay and the three copies, linked.
struct Room {
    relay: Process,
    alice: Process,
    bob: Process,
    carol: Process,
    now: i64,
}

fn listen_address() -> &'static str {
    if place_of("relay").is_some() {
        "0.0.0.0:0"
    } else {
        "127.0.0.1:0"
    }
}

fn open_room() -> Room {
    let relay = Process::relay(listen_address());
    let url = format!("ws://{}:{}", relay_address(), relay.port.expect("a port"));
    let mut room = Room {
        alice: Process::copy("alice", 1, &url),
        bob: Process::copy("bob", 2, &url),
        carol: Process::copy("carol", 3, &url),
        relay,
        now: instant(2026, 3, 1, 0, 0),
    };
    room.wait_linked();
    room
}

impl Room {
    fn copies(&mut self) -> [&mut Process; 3] {
        [&mut self.alice, &mut self.bob, &mut self.carol]
    }

    /// Every copy's link to the relay is up, and the relay's to every copy.
    fn wait_linked(&mut self) {
        for copy in self.copies() {
            copy.wait_linked(PEOPLE);
        }
        for name in ["alice", "bob", "carol"] {
            self.relay.wait_linked(name);
        }
    }

    fn copy(&mut self, who: &str) -> &mut Process {
        match who {
            "alice" => &mut self.alice,
            "bob" => &mut self.bob,
            "carol" => &mut self.carol,
            other => panic!("{other} is not a copy"),
        }
    }

    /// A person changes a field of a to-do, when the host's clock reads the room's time.
    fn set(&mut self, who: &str, id: u64, field: u64, value: i64) {
        self.now += 1;
        let data = format!(
            r#"{{"id":{id},"field":{field},"value":"{value}","text":"","now":"{}"}}"#,
            self.now
        );
        self.copy(who).raise("todo.edit", &data);
    }

    /// Every to-do repeats daily from a start, an hour long.
    fn schedule(&mut self, who: &str, id: u64) {
        self.set(who, id, START, instant(2026, 3, 1, 9, 0));
        self.set(who, id, LENGTH, HOUR_MS);
        self.set(who, id, REPEAT, DAILY);
    }

    /// A day: the people move and lengthen a few events, concurrently.
    fn an_afternoon(&mut self, day: i64) {
        for (who, id) in [("alice", 1), ("bob", 2), ("carol", 3), ("bob", 1)] {
            let start = instant(2026, 3, 1 + (day % 28), 8 + id as i64, 0);
            self.set(who, id, START, start);
            self.set(who, id, LENGTH, HOUR_MS * (1 + day % 3));
        }
        self.now += MINUTE_MS;
    }

    /// Until the three hold one list, which they do when what each sent has arrived and
    /// been taken in everywhere.
    fn wait_agreed(&mut self) -> Value {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let held: Vec<Value> = self
                .copies()
                .iter_mut()
                .map(|copy| copy.variables()["values"].clone())
                .collect();
            if held.iter().all(|values| sorted(values) == sorted(&held[0]))
                && !held[0].as_array().expect("a list").is_empty()
            {
                return held[0].clone();
            }
            assert!(
                Instant::now() < deadline,
                "the copies did not come to hold one list: {:?}",
                held.iter()
                    .map(|values| values.as_array().map_or(0, Vec::len))
                    .collect::<Vec<_>>()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Until every copy's causal layer keeps nothing and holds nothing: every copy took in
    /// every change, and knows that every other did.
    fn wait_forgotten(&mut self) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let layers: Vec<(usize, usize)> =
                self.copies().iter_mut().map(|copy| copy.layer()).collect();
            if layers.iter().all(|layer| *layer == (0, 0)) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the copies did not forget what they all hold: {layers:?}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
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

/// The values a copy holds, in an order that does not depend on the order they arrived in,
/// which is the network's.
fn sorted(values: &Value) -> Vec<String> {
    let mut each: Vec<String> = values
        .as_array()
        .expect("a list")
        .iter()
        .map(Value::to_string)
        .collect();
    each.sort();
    each
}

/// Three processes over loopback sockets hold one list after a fortnight of concurrent
/// changes, forget every change by themselves once each holds it, and show one agenda.
#[test]
fn three_processes_over_real_sockets_agree_and_forget() {
    let mut room = open_room();
    for id in 1..=3 {
        room.schedule("alice", id);
    }
    for day in 0..14 {
        room.an_afternoon(day);
    }
    let values = room.wait_agreed();
    let agendas = room.agendas(1);
    assert!(!agendas[0].as_array().expect("a list").is_empty());
    assert!(agendas.iter().all(|agenda| *agenda == agendas[0]));

    room.wait_forgotten();
    for copy in room.copies() {
        let variables = copy.variables();
        assert_eq!(
            variables["refusals"],
            json!(0),
            "{} refused something",
            copy.name
        );
    }
    let after = room.wait_agreed();
    assert_eq!(
        sorted(&after),
        sorted(&values),
        "forgetting changed a value"
    );
    assert_eq!(room.agendas(1), agendas, "forgetting changed the agenda");
}

/// The relay's process dies, and with it every link; bob changes his to-dos while no link is
/// up, and his layer keeps what it could not send. A relay is started again on the same port,
/// the copies dial it, the links that came back carry each copy's vector at once, and bob's
/// changes reach the others: the three hold one list and forget it alike.
#[test]
fn what_a_copy_made_while_every_link_was_closed_reaches_the_others() {
    let mut room = open_room();
    for id in 1..=3 {
        room.schedule("alice", id);
    }
    for day in 0..6 {
        room.an_afternoon(day);
    }
    room.wait_agreed();
    room.wait_forgotten();

    let port = room.relay.port.expect("a port");
    let _ = room.relay.child.kill();
    let _ = room.relay.child.wait();
    for copy in room.copies() {
        copy.line_where(|line| line == "LOST people", "its link closing");
    }
    for day in 6..12 {
        let start = instant(2026, 3, 1 + (day % 28), 11, 0);
        room.set("bob", 2, START, start);
        room.now += MINUTE_MS;
    }
    let (kept, _) = room.bob.layer();
    assert!(kept > 0, "bob's layer kept nothing it could not send");

    let listen = if place_of("relay").is_some() {
        format!("0.0.0.0:{port}")
    } else {
        format!("127.0.0.1:{port}")
    };
    room.relay = Process::relay(&listen);
    room.wait_linked();
    let values = room.wait_agreed();
    room.wait_forgotten();
    let bob = room.bob.variables()["values"].clone();
    assert_eq!(sorted(&bob), sorted(&values));
    let agendas = room.agendas(2);
    assert!(agendas.iter().all(|agenda| *agenda == agendas[0]));
}
