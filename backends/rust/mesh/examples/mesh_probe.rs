// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the Rust Mesh core does under the load of a shared-list server: many devices that come and
//! go, writes that must reach the others, and one machine that has to hold them all.
//!
//! ```text
//! mesh_probe bench [--sizes 100,1000,3000] [--events 20]
//! mesh_probe hub --n N [--admit-all]          (started by `bench`; reads commands on stdin)
//! ```
//!
//! `bench` starts the hub as a child process, so that its memory and CPU are its own, and runs the
//! devices from this process. For each size it measures:
//!
//! 1. **Scale.** `N` devices are bound at the hub ahead of time (the peer table is fixed when the
//!    hub's router is built), connect, and send `--events` events each: the hub's memory per bound
//!    peer, per connected peer, and the envelopes a second its one thread takes in.
//! 2. **Fan-out.** One event from the hub to every connected device, as a group write would need:
//!    the hub addresses each device by name; there is no group to address.
//! 3. **Offline.** 100 events sent to a device that is not connected, with no outbound buffer and
//!    with one of ten, and what the device is given when it connects.
//! 4. **A device the hub was not built for.** Refused at the upgrade; and, when the hub lets the
//!    link in, what the core does with what it sends.
//!
//! The hub holds its peer names for the life of the process (`Box::leak`): `PeerConfig` carries
//! `&'static` data, which a name learned at run time cannot be, so a hub that took devices as they
//! came would leak one allocation for each.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc as std_mpsc, Arc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sce_rust_mesh::endpoint::{Endpoint, EngineCall, Environment, MESH_PROCESSOR_TYPE};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::OutboundBuffer;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::wss::{
    accept_authenticated_with, connect, deliver, LinkConfig, LinkEvent, WssTransport,
    DEFAULT_KEEPALIVE,
};
use sce_rust_runtime::HostSendRequest;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// A monotonic clock, and envelope ids that differ between the senders and within one.
struct Clock {
    start: Instant,
    salt: u64,
    next: u64,
}

impl Clock {
    fn new(name: &str) -> Self {
        let salt = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
        });
        Self {
            start: Instant::now(),
            salt,
            next: 0,
        }
    }
}

impl Environment for Clock {
    fn now_ms(&mut self) -> i64 {
        i64::try_from(self.start.elapsed().as_millis()).unwrap_or(i64::MAX)
    }
    fn now_unix_ms(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        self.next += 1;
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&self.salt.to_be_bytes());
        id[8..].copy_from_slice(&self.next.to_be_bytes());
        id
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

/// The configuration of a `wss` peer that answers to itself alone.
fn config(responder: &'static str, buffer: Option<OutboundBuffer>) -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer,
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders: Box::leak(Box::new([responder])),
        deadline_ms: None,
        reply_events: &[],
    }
}

fn leaked(name: String) -> &'static str {
    Box::leak(name.into_boxed_str())
}

fn event(target: &str, name: &str, id: &str) -> HostSendRequest {
    HostSendRequest {
        processor_type: MESH_PROCESSOR_TYPE.to_string(),
        event_name: name.to_string(),
        target: format!("#{target}"),
        send_id: id.to_string(),
        event_data: "{}".to_string(),
        ..HostSendRequest::default()
    }
}

fn status_value(name: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|line| line.starts_with(name))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
}

fn cpu_seconds() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let Some(after) = stat.rfind(')') else {
        return 0.0;
    };
    let fields: Vec<&str> = stat[after + 2..].split_whitespace().collect();
    let ticks = |i: usize| {
        fields
            .get(i)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    (ticks(11) + ticks(12)) / 100.0
}

// ── hub ───────────────────────────────────────────────────────────────────────────────────────

enum Msg {
    Link(LinkEvent),
    Command(String),
}

fn run_hub(args: &[String]) {
    let n: usize = flag(args, "--n")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let admit_all = args.iter().any(|a| a == "--admit-all");
    // A hub that binds a device when its link arrives and unbinds it when the link is lost, and
    // so was built for none of them.
    let dynamic = args.iter().any(|a| a == "--dynamic");
    let link = LinkConfig {
        read_buffer: flag(args, "--read-buffer").and_then(|v| v.parse().ok()),
        write_buffer: flag(args, "--write-buffer").and_then(|v| v.parse().ok()),
        max_message: flag(args, "--max-message").and_then(|v| v.parse().ok()),
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    let listener = runtime
        .block_on(TcpListener::bind("127.0.0.1:0"))
        .expect("a port");
    println!(
        "LISTENING {}",
        listener.local_addr().expect("an address").port()
    );
    std::io::stdout().flush().expect("stdout");

    // dev-0 has no outbound buffer; dev-1 has one of ten; every other device has none.
    let mut router = Router::new("hub", 64, 50).expect("a router");
    let mut bound = HashSet::new();
    for i in 0..n {
        let name = leaked(format!("dev-{i}"));
        let buffer = (i == 1).then_some(OutboundBuffer {
            max_pending: 10,
            max_age_ms: 0,
        });
        router.add_peer(name, config(name, buffer));
        bound.insert(name.to_string());
    }
    let bound = Arc::new(bound);
    let transport = WssTransport::new();
    let mut endpoint = Endpoint::new(router, transport.clone(), Clock::new("hub"));

    let (to_hub, from_world) = std_mpsc::channel::<Msg>();
    let (link_tx, mut link_rx) = mpsc::unbounded_channel();
    let forward = to_hub.clone();
    runtime.spawn(async move {
        while let Some(event) = link_rx.recv().await {
            if forward.send(Msg::Link(event)).is_err() {
                break;
            }
        }
    });
    {
        let transport = transport.clone();
        runtime.spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let (bound, transport, link_tx) =
                    (bound.clone(), transport.clone(), link_tx.clone());
                tokio::spawn(async move {
                    let _ = accept_authenticated_with(
                        stream,
                        move |name, _token| dynamic || admit_all || bound.contains(name),
                        transport,
                        link_tx,
                        DEFAULT_KEEPALIVE,
                        link,
                    )
                    .await;
                });
            }
        });
    }
    let commands = to_hub.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines().map_while(Result::ok) {
            if commands.send(Msg::Command(line)).is_err() {
                break;
            }
        }
    });

    let (mut received, mut errors, mut fanout_ms) = (0u64, 0u64, 0u64);
    let mut first_error = String::new();
    let absorb = |endpoint: &mut Endpoint<WssTransport, Clock>,
                  received: &mut u64,
                  errors: &mut u64,
                  first_error: &mut String| {
        for call in endpoint.take_calls() {
            if matches!(call, EngineCall::Raise(_)) {
                *received += 1;
            }
        }
        for error in endpoint.take_host_errors() {
            *errors += 1;
            if first_error.is_empty() {
                *first_error = format!("{error:?}").replace('\n', " ");
            }
        }
    };
    for message in from_world {
        match message {
            Msg::Link(link) => {
                if dynamic {
                    match &link {
                        LinkEvent::Ready(name) if !endpoint.has_peer(name) => {
                            endpoint.add_peer(name, config(leaked(name.clone()), None));
                        }
                        LinkEvent::Lost(name, _) => {
                            deliver(&mut endpoint, link.clone());
                            endpoint.remove_peer(name);
                            absorb(&mut endpoint, &mut received, &mut errors, &mut first_error);
                            continue;
                        }
                        _ => {}
                    }
                }
                deliver(&mut endpoint, link);
                absorb(&mut endpoint, &mut received, &mut errors, &mut first_error);
            }
            Msg::Command(line) => {
                let words: Vec<&str> = line.split_whitespace().collect();
                match words.as_slice() {
                    ["STATS"] => {
                        println!(
                            "STATS rss_kib={} hwm_kib={} cpu_s={:.3} received={} errors={} fanout_ms={} first_error={}",
                            status_value("VmRSS:").unwrap_or(0),
                            status_value("VmHWM:").unwrap_or(0),
                            cpu_seconds(),
                            received,
                            errors,
                            fanout_ms,
                            if first_error.is_empty() { "-" } else { &first_error },
                        );
                    }
                    ["FANOUT", first, last] => {
                        let (first, last): (usize, usize) =
                            (first.parse().unwrap_or(0), last.parse().unwrap_or(0));
                        let start = Instant::now();
                        for i in first..last {
                            endpoint.send(&event(&format!("dev-{i}"), "note", &format!("n-{i}")));
                        }
                        fanout_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                        absorb(&mut endpoint, &mut received, &mut errors, &mut first_error);
                        println!("FANNED {}", last - first);
                    }
                    ["SENDTO", peer, count] => {
                        for k in 0..count.parse::<usize>().unwrap_or(0) {
                            endpoint.send(&event(peer, "note", &format!("{peer}-{k}")));
                        }
                        absorb(&mut endpoint, &mut received, &mut errors, &mut first_error);
                        println!("SENT {peer}");
                    }
                    _ => println!("UNKNOWN {line}"),
                }
                std::io::stdout().flush().expect("stdout");
            }
        }
    }
}

// ── devices ───────────────────────────────────────────────────────────────────────────────────

struct Tally {
    ready: AtomicUsize,
    refused: AtomicUsize,
    notes: AtomicUsize,
}

/// One device: dial the hub as `name`, send `events` events once ready, count the notes it is given.
async fn device(base: String, name: String, events: usize, tally: Arc<Tally>) {
    let mut router = Router::new(&name, 64, 50).expect("a router");
    router.add_peer("hub", config("hub", None));
    let transport = WssTransport::new();
    let mut endpoint = Endpoint::new(router, transport.clone(), Clock::new(&name));
    let (tx, mut rx) = mpsc::unbounded_channel();
    let dial = {
        let name = name.clone();
        tokio::spawn(
            async move { connect(&base, &name, "hub", transport, tx, DEFAULT_KEEPALIVE).await },
        )
    };
    while let Some(link) = rx.recv().await {
        let ready = matches!(link, LinkEvent::Ready(_));
        let lost = matches!(link, LinkEvent::Lost(..));
        deliver(&mut endpoint, link);
        if ready {
            tally.ready.fetch_add(1, Ordering::Relaxed);
            for k in 0..events {
                endpoint.send(&event("hub", "ping", &format!("{name}-{k}")));
            }
        }
        for call in endpoint.take_calls() {
            if matches!(call, EngineCall::Raise(_)) {
                tally.notes.fetch_add(1, Ordering::Relaxed);
            }
        }
        if lost {
            break;
        }
    }
    if !matches!(dial.await, Ok(Ok(()))) {
        tally.refused.fetch_add(1, Ordering::Relaxed);
    }
}

// ── bench ─────────────────────────────────────────────────────────────────────────────────────

struct Hub {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
    base: String,
}

impl Hub {
    fn start(n: usize, admit_all: bool, extra: &[&str]) -> Self {
        let exe = std::env::current_exe().expect("this program");
        let mut command = Command::new(exe);
        command.arg("hub").arg("--n").arg(n.to_string());
        if admit_all {
            command.arg("--admit-all");
        }
        command.args(extra);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the hub starts");
        let input = child.stdin.take().expect("a pipe");
        let mut output = BufReader::new(child.stdout.take().expect("a pipe"));
        let mut line = String::new();
        output.read_line(&mut line).expect("the hub says where");
        let port = line
            .trim()
            .strip_prefix("LISTENING ")
            .expect("a port line")
            .to_string();
        Self {
            child,
            input,
            output,
            base: format!("ws://127.0.0.1:{port}"),
        }
    }

    fn ask(&mut self, command: &str) -> String {
        writeln!(self.input, "{command}").expect("the hub reads");
        self.input.flush().expect("flush");
        let mut line = String::new();
        self.output.read_line(&mut line).expect("the hub answers");
        line.trim().to_string()
    }

    fn stats(&mut self) -> Stats {
        let line = self.ask("STATS");
        let value = |key: &str| -> f64 {
            line.split_whitespace()
                .find_map(|w| w.strip_prefix(key))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0)
        };
        Stats {
            rss_kib: value("rss_kib="),
            cpu_s: value("cpu_s="),
            received: value("received=") as u64,
            errors: value("errors=") as u64,
            fanout_ms: value("fanout_ms=") as u64,
            first_error: line
                .split_once("first_error=")
                .map_or("-".to_string(), |(_, e)| e.to_string()),
        }
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Stats {
    rss_kib: f64,
    cpu_s: f64,
    received: u64,
    errors: u64,
    fanout_ms: u64,
    first_error: String,
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

async fn until(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if done() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    done()
}

async fn bench_size(n: usize, events: usize) {
    println!("--- {n} devices bound at the hub, {events} events each");
    let mut hub = Hub::start(n, false, &[]);
    let before = hub.stats();
    println!(
        "bound, none connected: hub {:.1} MiB ({:.0} bytes per bound peer over the empty hub)",
        before.rss_kib / 1024.0,
        0.0
    );
    let baseline = before.rss_kib;
    let tally = Arc::new(Tally {
        ready: AtomicUsize::new(0),
        refused: AtomicUsize::new(0),
        notes: AtomicUsize::new(0),
    });
    let connected = n.saturating_sub(2);
    let start = Instant::now();
    for i in 2..n {
        tokio::spawn(device(
            hub.base.clone(),
            format!("dev-{i}"),
            events,
            tally.clone(),
        ));
    }
    let want = (connected * events) as u64;
    let mut seen = 0;
    let ok = until(Duration::from_secs(120), || {
        seen = hub.stats().received;
        seen >= want
    })
    .await;
    let took = start.elapsed().as_secs_f64();
    let after = hub.stats();
    println!(
        "scale: {} of {} envelopes in {:.2}s = {:.0}/s ({}), {} devices ready, {} refused; hub {:.1} MiB, {:.0} KiB per connected peer, cpu {:.2}s",
        after.received,
        want,
        took,
        after.received as f64 / took,
        if ok { "complete" } else { "INCOMPLETE" },
        tally.ready.load(Ordering::Relaxed),
        tally.refused.load(Ordering::Relaxed),
        after.rss_kib / 1024.0,
        (after.rss_kib - baseline) / connected.max(1) as f64,
        after.cpu_s,
    );

    // Fan-out: one event to every connected device.
    let before_cpu = hub.stats().cpu_s;
    let fanned = hub.ask(&format!("FANOUT 2 {n}"));
    let stats = hub.stats();
    let got = until(Duration::from_secs(60), || {
        tally.notes.load(Ordering::Relaxed) >= connected
    })
    .await;
    println!(
        "fan-out: {fanned}: the hub made {} sends in {} ms ({:.0} us each, {:.3} s cpu), {} of {} devices were given it ({})",
        connected,
        stats.fanout_ms,
        stats.fanout_ms as f64 * 1000.0 / connected.max(1) as f64,
        stats.cpu_s - before_cpu,
        tally.notes.load(Ordering::Relaxed),
        connected,
        if got { "all" } else { "NOT ALL" },
    );

    // Offline: dev-0 has no buffer, dev-1 a buffer of ten; neither is connected.
    let errors_before = hub.stats().errors;
    hub.ask("SENDTO dev-0 100");
    let after_zero = hub.stats();
    hub.ask("SENDTO dev-1 100");
    let after_one = hub.stats();
    println!(
        "offline: 100 sends to an unconnected device raised {} errors with no buffer and {} with a buffer of ten; first: {}",
        after_zero.errors - errors_before,
        after_one.errors - after_zero.errors,
        after_zero.first_error,
    );
    let late = Arc::new(Tally {
        ready: AtomicUsize::new(0),
        refused: AtomicUsize::new(0),
        notes: AtomicUsize::new(0),
    });
    let late_zero = Arc::new(Tally {
        ready: AtomicUsize::new(0),
        refused: AtomicUsize::new(0),
        notes: AtomicUsize::new(0),
    });
    tokio::spawn(device(
        hub.base.clone(),
        "dev-0".to_string(),
        0,
        late_zero.clone(),
    ));
    tokio::spawn(device(
        hub.base.clone(),
        "dev-1".to_string(),
        0,
        late.clone(),
    ));
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!(
        "offline: when they connect, the device with no buffer is given {} of the 100 and the one with a buffer of ten is given {}",
        late_zero.notes.load(Ordering::Relaxed),
        late.notes.load(Ordering::Relaxed),
    );

    // A device the hub was not built for.
    let stranger = Arc::new(Tally {
        ready: AtomicUsize::new(0),
        refused: AtomicUsize::new(0),
        notes: AtomicUsize::new(0),
    });
    tokio::spawn(device(
        hub.base.clone(),
        "stranger-1".to_string(),
        5,
        stranger.clone(),
    ));
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!(
        "stranger: a device the hub has no peer for: {} (ready {}, refused at the upgrade {})",
        if stranger.ready.load(Ordering::Relaxed) == 0 {
            "never connected"
        } else {
            "connected"
        },
        stranger.ready.load(Ordering::Relaxed),
        stranger.refused.load(Ordering::Relaxed),
    );
    drop(hub);

    let mut hub = Hub::start(n.min(10), true, &[]);
    let stranger = Arc::new(Tally {
        ready: AtomicUsize::new(0),
        refused: AtomicUsize::new(0),
        notes: AtomicUsize::new(0),
    });
    tokio::spawn(device(
        hub.base.clone(),
        "stranger-2".to_string(),
        5,
        stranger.clone(),
    ));
    tokio::time::sleep(Duration::from_secs(1)).await;
    let stats = hub.stats();
    println!(
        "stranger: when the hub lets the link in anyway: connected {}, the hub was given {} of its 5 events and raised {} errors; first: {}",
        stranger.ready.load(Ordering::Relaxed) > 0,
        stats.received,
        stats.errors,
        stats.first_error,
    );
}

/// What one connected device costs a hub that was built for none of them, with the link sizes the
/// library chooses and with small ones: the hub binds each device as its link arrives.
async fn bench_memory(n: usize) {
    for (label, extra) in [
        ("library buffers (128 KiB read, 128 KiB write)", vec![]),
        (
            "4 KiB read, 4 KiB write",
            vec!["--read-buffer", "4096", "--write-buffer", "4096"],
        ),
        (
            "1 KiB read, 1 KiB write",
            vec!["--read-buffer", "1024", "--write-buffer", "1024"],
        ),
    ] {
        let mut args = vec!["--dynamic"];
        args.extend(extra);
        let mut hub = Hub::start(0, false, &args);
        let baseline = hub.stats().rss_kib;
        let tally = Arc::new(Tally {
            ready: AtomicUsize::new(0),
            refused: AtomicUsize::new(0),
            notes: AtomicUsize::new(0),
        });
        for i in 0..n {
            tokio::spawn(device(
                hub.base.clone(),
                format!("dyn-{i}"),
                1,
                tally.clone(),
            ));
        }
        let want = n as u64;
        let ok = until(Duration::from_secs(120), || hub.stats().received >= want).await;
        let stats = hub.stats();
        println!(
            "memory [{label}]: {n} devices the hub was not built for: {} of {n} events ({}), {} errors, hub {:.1} MiB = {:.0} KiB per connected device; first error: {}",
            stats.received,
            if ok { "complete" } else { "INCOMPLETE" },
            stats.errors,
            stats.rss_kib / 1024.0,
            (stats.rss_kib - baseline) / n.max(1) as f64,
            stats.first_error,
        );
    }
}

fn run_bench(args: &[String]) {
    let sizes: Vec<usize> = flag(args, "--sizes")
        .unwrap_or_else(|| "100,1000,3000".to_string())
        .split(',')
        .map(|s| s.trim().parse().expect("--sizes wants numbers"))
        .collect();
    let events = flag(args, "--events")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()
        .expect("a runtime");
    for n in sizes {
        runtime.block_on(bench_size(n, events));
        runtime.block_on(bench_memory(n));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("hub") => run_hub(&args),
        Some("bench") => run_bench(&args),
        _ => {
            eprintln!("usage: mesh_probe bench|hub (see the file's header)");
            std::process::exit(2);
        }
    }
}
