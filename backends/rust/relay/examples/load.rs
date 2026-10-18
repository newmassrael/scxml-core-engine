// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Load for the reference relay: how many signed writes a relay takes in a second, and what each one
//! costs it, over a real WebSocket, as the cores it may use grow.
//!
//! ```text
//! load server --listen 127.0.0.1:7100 [--store DIR] [--workers N] [--seed N]
//! load client --url ws://127.0.0.1:7100 [--lists L] [--devices D] [--writes W] [--threads T] [--seed N]
//! load bench  --cores 1,2,4,8 [--lists L] [--devices D] [--writes W] [--threads T] [--store DIR]
//! ```
//!
//! `bench` starts the server as a child process pinned to the first `n` cores with `taskset`, runs
//! the client against it from this process, reads the child's peak resident memory, and prints one
//! line for each `n`. Run it under `taskset` on other cores so the clients do not compete with the
//! server. The relay's per-device request allowance is lifted here: a load that comes from a few
//! devices would otherwise measure the allowance and not the relay.
//!
//! A write is made the way a host makes it (`Trust::seal`, which signs it as the device), so the
//! relay does the work it does for a member: it checks the token, the signature, the device's chain
//! against the log, and keeps the record.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_relay::serve::{serve, WssLink};
use sce_rust_relay::{
    Authority, Code, Courier, FileStore, Limits, Link, MemoryStore, Relay, Request, Response, Store,
};
use sce_rust_trust::membership::{Body, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::json;
use tokio::net::TcpListener;

const AUDIENCE: &str = "load";

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn number(args: &[String], name: &str, default: usize) -> usize {
    flag(args, name).map_or(default, |v| {
        v.parse()
            .unwrap_or_else(|_| panic!("{name} wants a number, got {v:?}"))
    })
}

fn authority(seed: u8) -> Authority {
    Authority::new([seed; 32], AUDIENCE)
}

fn administrator() -> Identity {
    Identity::from_seed([0xFF; 32])
}

fn device_key(node: u64) -> Identity {
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&node.to_le_bytes());
    Identity::from_seed(seed)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

// ── server ────────────────────────────────────────────────────────────────────────────────────

fn run_server(args: &[String]) {
    let listen = flag(args, "--listen").unwrap_or_else(|| "127.0.0.1:7100".to_string());
    let workers = number(args, "--workers", 0);
    let seed = number(args, "--seed", 4) as u8;
    match flag(args, "--store") {
        Some(dir) => serve_on(
            FileStore::open(&dir).expect("the store directory opens"),
            &listen,
            workers,
            seed,
        ),
        None => serve_on(MemoryStore::new(), &listen, workers, seed),
    }
}

fn serve_on<St: Store + Send + 'static>(store: St, listen: &str, workers: usize, seed: u8) {
    let limits = Limits {
        burst: u32::MAX / 4,
        refill_per_second: u32::MAX / 4,
        max_records_per_list: 100_000_000,
        ..Limits::default()
    };
    let relay = Arc::new(Mutex::new(
        Relay::open(store, authority(seed), limits).expect("the relay opens"),
    ));
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    if workers > 0 {
        builder.worker_threads(workers);
    }
    let runtime = builder.enable_all().build().expect("a runtime");
    let listener = runtime
        .block_on(TcpListener::bind(listen))
        .expect("the address binds");
    println!("listening {}", listener.local_addr().expect("an address"));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(now_ms);
    let _ = runtime.block_on(serve(listener, relay, clock, 1024 * 1024));
}

// ── client ────────────────────────────────────────────────────────────────────────────────────

struct Device {
    node: u64,
    trust: Trust,
    link: WssLink,
    next: u64,
}

/// The lists and their devices, with the log of each list given to the relay by its first device.
fn set_up(url: &str, seed: u8, lists: usize, devices: usize) -> Vec<Device> {
    let authority = authority(seed);
    let mut all = Vec::new();
    for l in 0..lists {
        let list = format!("load-{l}");
        let base = Trust::open(&list);
        let genesis = Log::genesis_draft(
            &list,
            Body::Genesis {
                administrators: vec![administrator().public()],
                quorum: 1,
                profile: Profile::Untrusted,
            },
        )
        .sign(&administrator())
        .finish();
        base.append(&genesis).expect("the genesis is taken");
        for node in 1..=devices as u64 {
            let grant = base
                .draft(Body::Grant {
                    node,
                    role: 2,
                    key: device_key(node).public(),
                    final_seq: None,
                })
                .sign(&administrator())
                .finish();
            base.append(&grant).expect("the grant is taken");
        }
        let entries = base.entries();
        for node in 1..=devices as u64 {
            let trust = Trust::open(&list);
            for entry in &entries {
                trust.append(entry).expect("the log is taken");
            }
            trust.set_identity(node, device_key(node));
            let token = authority.issue(&list, node, u64::MAX);
            let link = WssLink::new(url, &token).expect("a link");
            if node == 1 {
                let mut courier =
                    Courier::new(trust.clone(), WssLink::new(url, &token).expect("a link"));
                let round = courier.sync().expect("the log reaches the relay");
                assert!(round.refused.is_empty(), "{:?}", round.refused);
            }
            all.push(Device {
                node,
                trust,
                link,
                next: 0,
            });
        }
    }
    all
}

#[derive(Default)]
struct Outcome {
    latencies_us: Vec<u64>,
    errors: usize,
    first_error: Option<String>,
}

fn drive(mut devices: Vec<Device>, writes: usize) -> Outcome {
    let mut outcome = Outcome::default();
    for _ in 0..writes {
        for device in &mut devices {
            let k = device.next;
            device.next += 1;
            let write = json!({
                "id": (k % 64).to_string(),
                "field": "3",
                "value": k.to_string(),
                "text": "",
                "wallTime": (1_000 + k).to_string(),
                "counter": "0",
                "nodeId": device.node.to_string(),
            })
            .to_string();
            let sealed = match device.trust.seal("#server", "todo.write", &write) {
                Ok(Some(sealed)) => sealed,
                other => {
                    outcome.errors += 1;
                    outcome
                        .first_error
                        .get_or_insert_with(|| format!("seal: {other:?}"));
                    continue;
                }
            };
            let start = Instant::now();
            let answer = device.link.call(&Request::Submit {
                event: "todo.write".to_string(),
                data: sealed,
            });
            let took = start.elapsed().as_micros() as u64;
            match answer {
                Ok(Response::Accepted { new: true, .. }) => outcome.latencies_us.push(took),
                Ok(Response::Refused { code, reason }) => {
                    outcome.errors += 1;
                    outcome
                        .first_error
                        .get_or_insert_with(|| format!("refused {}: {reason}", code_name(code)));
                }
                other => {
                    outcome.errors += 1;
                    outcome
                        .first_error
                        .get_or_insert_with(|| format!("{other:?}"));
                }
            }
        }
    }
    outcome
}

fn code_name(code: Code) -> &'static str {
    match code {
        Code::Unauthorized => "unauthorized",
        Code::Forbidden => "forbidden",
        Code::Behind => "behind",
        Code::Invalid => "invalid",
        Code::TooLarge => "too-large",
        Code::Unavailable => "unavailable",
        _ => "other",
    }
}

struct Report {
    accepted: usize,
    errors: usize,
    first_error: Option<String>,
    seconds: f64,
    p50_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    /// CPU the server used while the writes ran, in seconds; over `seconds` it is the cores busy.
    server_cpu_s: Option<f64>,
}

fn run_load(
    url: &str,
    seed: u8,
    lists: usize,
    devices: usize,
    writes: usize,
    threads: usize,
    server_pid: Option<u32>,
) -> Report {
    let all = set_up(url, seed, lists, devices);
    let mut shares: Vec<Vec<Device>> = (0..threads).map(|_| Vec::new()).collect();
    for (index, device) in all.into_iter().enumerate() {
        shares[index % threads].push(device);
    }
    let cpu_before = server_pid.and_then(cpu_seconds);
    let start = Instant::now();
    let handles: Vec<_> = shares
        .into_iter()
        .filter(|share| !share.is_empty())
        .map(|share| std::thread::spawn(move || drive(share, writes)))
        .collect();
    let mut latencies = Vec::new();
    let mut errors = 0;
    let mut first_error = None;
    for handle in handles {
        let outcome = handle.join().expect("a driver ran");
        latencies.extend(outcome.latencies_us);
        errors += outcome.errors;
        first_error = first_error.or(outcome.first_error);
    }
    let seconds = start.elapsed().as_secs_f64();
    let server_cpu_s = server_pid
        .and_then(cpu_seconds)
        .zip(cpu_before)
        .map(|(after, before)| after - before);
    latencies.sort_unstable();
    let at = |q: f64| {
        latencies
            .get(((latencies.len() as f64 * q) as usize).min(latencies.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0) as f64
            / 1000.0
    };
    Report {
        accepted: latencies.len(),
        errors,
        first_error,
        seconds,
        p50_ms: at(0.50),
        p99_ms: at(0.99),
        max_ms: latencies.last().copied().unwrap_or(0) as f64 / 1000.0,
        server_cpu_s,
    }
}

/// The CPU time process `pid` has used (user and system), in seconds, from the kernel's account of
/// it. The clock tick is the 100 Hz every Linux this runs on reports.
fn cpu_seconds(pid: u32) -> Option<f64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields: Vec<&str> = stat[stat.rfind(')')? + 2..].split_whitespace().collect();
    let ticks = |index: usize| fields.get(index)?.parse::<f64>().ok();
    Some((ticks(11)? + ticks(12)?) / 100.0)
}

fn print_report(label: &str, report: &Report, extra: &str) {
    println!(
        "{label}: {} writes in {:.2}s = {:.0}/s, p50 {:.2} ms, p99 {:.2} ms, max {:.2} ms, errors {}{extra}",
        report.accepted,
        report.seconds,
        report.accepted as f64 / report.seconds,
        report.p50_ms,
        report.p99_ms,
        report.max_ms,
        report.errors,
    );
    if let Some(cpu) = report.server_cpu_s {
        println!(
            "{label}: server used {:.2} cores while the writes ran",
            cpu / report.seconds
        );
    }
    if let Some(error) = &report.first_error {
        println!("{label}: first error: {error}");
    }
}

fn run_client(args: &[String]) {
    let url = flag(args, "--url").unwrap_or_else(|| "ws://127.0.0.1:7100".to_string());
    let report = run_load(
        &url,
        number(args, "--seed", 4) as u8,
        number(args, "--lists", 1),
        number(args, "--devices", 8),
        number(args, "--writes", 100),
        number(args, "--threads", 4),
        None,
    );
    print_report("client", &report, "");
}

// ── bench ─────────────────────────────────────────────────────────────────────────────────────

/// The peak resident memory of process `pid` in KiB, from the kernel's own account of it.
fn peak_rss_kib(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
}

fn run_bench(args: &[String]) {
    let cores: Vec<usize> = flag(args, "--cores")
        .unwrap_or_else(|| "1,2,4,8".to_string())
        .split(',')
        .map(|c| c.trim().parse().expect("--cores wants numbers"))
        .collect();
    let lists = number(args, "--lists", 8);
    let devices = number(args, "--devices", 8);
    let writes = number(args, "--writes", 200);
    let threads = number(args, "--threads", 16);
    let store = flag(args, "--store");
    let exe = std::env::current_exe().expect("this program's path");
    println!(
        "bench: {lists} lists x {devices} devices x {writes} writes, {threads} client threads, store {}",
        store.as_deref().unwrap_or("memory")
    );
    for (round, n) in cores.iter().enumerate() {
        let mut command = Command::new("taskset");
        command
            .arg("-c")
            .arg(format!("0-{}", n - 1))
            .arg(&exe)
            .arg("server")
            .arg("--listen")
            .arg("127.0.0.1:0")
            .arg("--workers")
            .arg(n.to_string());
        if let Some(dir) = &store {
            let dir = format!("{dir}/cores-{n}-{round}");
            std::fs::create_dir_all(&dir).expect("a store directory");
            command.arg("--store").arg(dir);
        }
        let mut child = command
            .stdout(Stdio::piped())
            .spawn()
            .expect("taskset starts the server");
        let mut lines = BufReader::new(child.stdout.take().expect("a pipe"));
        let mut line = String::new();
        lines.read_line(&mut line).expect("the server says where");
        let address = line
            .trim()
            .strip_prefix("listening ")
            .unwrap_or_else(|| panic!("the server said {line:?}"))
            .to_string();
        let report = run_load(
            &format!("ws://{address}"),
            4,
            lists,
            devices,
            writes,
            threads,
            Some(child.id()),
        );
        let rss = peak_rss_kib(child.id());
        let _ = child.kill();
        let _ = child.wait();
        print_report(
            &format!("cores {n}"),
            &report,
            &format!(
                ", server peak {} MiB",
                rss.map_or("?".to_string(), |k| (k / 1024).to_string())
            ),
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("server") => run_server(&args),
        Some("client") => run_client(&args),
        Some("bench") => run_bench(&args),
        _ => {
            eprintln!("usage: load server|client|bench (see the file's header)");
            std::process::exit(2);
        }
    }
}
