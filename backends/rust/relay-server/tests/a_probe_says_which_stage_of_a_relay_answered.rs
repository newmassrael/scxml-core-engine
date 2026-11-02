// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A probe makes the exchange a device makes with a relay and says what it came to (`sce-relay
// probe`). Against a relay running in this process, over `ws://` and over `wss://`: with a token for
// the probe list it gives the relay that list's first entry, which the relay keeps once and finds
// held after, and reads it back; with a token for another list it only reads. Each stage that fails
// is named by its own outcome and its own exit status: a relay nobody answers for, a certificate the
// probe does not believe, a token the relay did not issue, a relay that takes the connection and
// says nothing, an answer that is not the protocol's, a store that did not take the record, and an
// allowance spent. The line it prints never holds the token.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sce_rust_relay::serve::{serve, serve_tls};
use sce_rust_relay::tls::{self, Roots};
use sce_rust_relay::{Authority, Limits, MemoryStore, Record, Relay, Store};
use sce_rust_relay_server::probe::{
    self, own_token, Asked, Outcome, Report, Stage, Target, PROBE_LIST,
};
use tokio::net::TcpListener;

const BIN: &str = env!("CARGO_BIN_EXE_sce-relay");
const AUDIENCE: &str = "relay.test";

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// A directory of the test's own, taken away when it ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sce-relay-probe-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A file only its owner may read, as a token or a key is kept.
    fn secret(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A store that takes no record: a disk that is full, or gone.
struct Refusing;

impl Store for Refusing {
    fn load(&mut self) -> std::io::Result<Vec<(String, Record)>> {
        Ok(Vec::new())
    }

    fn append(&mut self, _list: &str, _record: &Record) -> std::io::Result<()> {
        Err(std::io::Error::other("no space left on the device"))
    }
}

/// A relay of `authority` over `store`, answering on a port of this host, `wss://` when it is given
/// a certificate and its key (each DER).
struct Running<S: Store + Send + 'static> {
    runtime: tokio::runtime::Runtime,
    relay: Arc<Mutex<Relay<S>>>,
    base: String,
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn start<S: Store + Send + 'static>(
    store: S,
    authority: &Authority,
    limits: Limits,
    certified: Option<(Vec<u8>, Vec<u8>)>,
) -> Running<S> {
    let relay = Arc::new(Mutex::new(
        Relay::open(store, authority.clone(), limits).unwrap(),
    ));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(now_ms);
    let runtime = runtime();
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    let base = match certified {
        Some((certificate, key)) => {
            let config = tls::server_config(vec![certificate], key).unwrap();
            runtime.spawn(serve_tls(listener, config, relay.clone(), clock, 64 * 1024));
            format!("wss://{address}")
        }
        None => {
            runtime.spawn(serve(listener, relay.clone(), clock, 64 * 1024));
            format!("ws://{address}")
        }
    };
    Running {
        runtime,
        relay,
        base,
    }
}

impl<S: Store + Send + 'static> Running<S> {
    fn probe(&self, roots: &Roots, token: &str) -> Report {
        let target = Target::new(&self.base, roots, token).unwrap();
        self.runtime
            .block_on(probe::probe(&target, Duration::from_secs(10)))
    }

    fn records(&self, list: &str) -> usize {
        self.relay.lock().unwrap().records(list)
    }
}

/// A probe of `base` from a runtime of the probe's own, within `within`.
fn probe_at(base: &str, token: &str, within: Duration) -> Report {
    let target = Target::new(base, &Roots::Public, token).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(probe::probe(&target, within))
}

fn stages(report: &Report) -> Vec<Stage> {
    report.timings.iter().map(|(stage, _)| *stage).collect()
}

/// A certificate for `127.0.0.1` that no public authority issued, and its key, each DER.
fn certificate() -> (Vec<u8>, Vec<u8>) {
    let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
    (issued.cert.der().to_vec(), issued.key_pair.serialize_der())
}

/// An address at which nothing listens.
fn nothing_listens() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("ws://{address}")
}

/// An address whose server reads a request and answers it with `answer`, once.
fn answering(answer: &'static str) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut seen = Vec::new();
        let mut buffer = [0u8; 1024];
        while !seen.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => seen.extend_from_slice(&buffer[..read]),
            }
        }
        let _ = stream.write_all(answer.as_bytes());
    });
    format!("ws://{address}")
}

#[test]
fn a_probe_with_its_own_token_has_the_relay_keep_its_list_once_and_reads_it_back() {
    let authority = Authority::new([7; 32], AUDIENCE);
    let running = start(MemoryStore::new(), &authority, Limits::default(), None);
    let token = own_token(&authority, now_ms());

    let first = running.probe(&Roots::Public, &token);
    assert_eq!(first.outcome, Outcome::Ok, "{}", first.line());
    assert_eq!(first.asked, Asked::WriteRead);
    assert_eq!(first.created, Some(true), "the first probe makes the list");
    assert_eq!(
        stages(&first),
        vec![
            Stage::Resolve,
            Stage::Connect,
            Stage::Upgrade,
            Stage::Exchange
        ],
        "no TLS over ws://"
    );
    assert_eq!(running.records(PROBE_LIST), 1);

    let again = running.probe(&Roots::Public, &token);
    assert_eq!(again.outcome, Outcome::Ok, "{}", again.line());
    assert_eq!(
        again.created,
        Some(false),
        "every later probe finds it held"
    );
    assert_eq!(running.records(PROBE_LIST), 1, "and the list never grows");
    assert!(again.line().contains(" list=held "), "{}", again.line());
    assert!(!again.line().contains(&token));
}

#[test]
fn a_probe_with_a_token_for_another_list_reads_it_and_writes_nothing() {
    let authority = Authority::new([7; 32], AUDIENCE);
    let running = start(MemoryStore::new(), &authority, Limits::default(), None);
    let token = authority.issue("home", 3, now_ms() + 60_000);

    let report = running.probe(&Roots::Public, &token);
    assert_eq!(report.outcome, Outcome::Ok, "{}", report.line());
    assert_eq!(report.asked, Asked::Read);
    assert_eq!(report.created, None);
    assert_eq!(running.records("home"), 0);
    assert_eq!(running.records(PROBE_LIST), 0);
}

#[test]
fn a_probe_over_tls_believes_the_authority_it_is_given_and_no_other() {
    let authority = Authority::new([7; 32], AUDIENCE);
    let (certificate, key) = certificate();
    let running = start(
        MemoryStore::new(),
        &authority,
        Limits::default(),
        Some((certificate.clone(), key)),
    );
    let token = own_token(&authority, now_ms());

    let believed = running.probe(&Roots::Only(vec![certificate]), &token);
    assert_eq!(believed.outcome, Outcome::Ok, "{}", believed.line());
    assert_eq!(
        stages(&believed),
        vec![
            Stage::Resolve,
            Stage::Connect,
            Stage::Tls,
            Stage::Upgrade,
            Stage::Exchange
        ]
    );

    let public = running.probe(&Roots::Public, &token);
    assert_eq!(public.outcome, Outcome::TlsRefused, "{}", public.line());
    assert_eq!(public.stage, Stage::Tls);
    assert!(
        public.reason.as_deref().unwrap().contains("UnknownIssuer"),
        "{}",
        public.line()
    );
    assert_eq!(public.outcome.status(), 79);
}

#[test]
fn a_token_the_relay_did_not_issue_is_refused_at_the_upgrade() {
    let authority = Authority::new([7; 32], AUDIENCE);
    let running = start(MemoryStore::new(), &authority, Limits::default(), None);
    let stranger = Authority::new([8; 32], AUDIENCE);

    let report = running.probe(&Roots::Public, &own_token(&stranger, now_ms()));
    assert_eq!(report.outcome, Outcome::TokenRefused, "{}", report.line());
    assert_eq!(report.stage, Stage::Upgrade);
    assert_eq!(report.outcome.status(), 77);
    assert_eq!(running.records(PROBE_LIST), 0);
}

#[test]
fn a_relay_whose_store_takes_nothing_is_told_from_one_that_is_down() {
    let authority = Authority::new([7; 32], AUDIENCE);
    let running = start(Refusing, &authority, Limits::default(), None);

    let report = running.probe(&Roots::Public, &own_token(&authority, now_ms()));
    assert_eq!(report.outcome, Outcome::StoreFailed, "{}", report.line());
    assert_eq!(report.stage, Stage::Exchange);
    assert_eq!(report.outcome.status(), 74);
}

#[test]
fn a_relay_that_asks_the_probe_to_wait_is_busy_and_not_broken() {
    let authority = Authority::new([7; 32], AUDIENCE);
    // An allowance the probe's write spends whole, which never refills: its read is refused.
    let limits = Limits {
        burst: 1,
        refill_per_second: 0,
        ..Limits::default()
    };
    let running = start(MemoryStore::new(), &authority, limits, None);

    let report = running.probe(&Roots::Public, &own_token(&authority, now_ms()));
    assert_eq!(report.outcome, Outcome::Busy, "{}", report.line());
    assert_eq!(report.stage, Stage::Exchange);
    assert_eq!(report.outcome.status(), 75);
}

#[test]
fn a_relay_nobody_answers_for_is_unreachable() {
    let report = probe_at(&nothing_listens(), "token", Duration::from_secs(10));
    assert_eq!(report.outcome, Outcome::Unreachable, "{}", report.line());
    assert_eq!(report.stage, Stage::Connect);
    assert_eq!(stages(&report), vec![Stage::Resolve]);
    assert_eq!(report.outcome.status(), 69);
}

#[test]
fn a_relay_that_takes_the_connection_and_says_nothing_is_busy_when_the_time_is_up() {
    // A listener the kernel takes connections for and that never answers one.
    let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("ws://{}", silent.local_addr().unwrap());

    let report = probe_at(&base, "token", Duration::from_secs(1));
    assert_eq!(report.outcome, Outcome::Busy, "{}", report.line());
    assert_eq!(report.stage, Stage::Upgrade);
    assert!(
        report.total >= Duration::from_secs(1),
        "it waited the time it was given: {:?}",
        report.total
    );
    drop(silent);
}

#[test]
fn an_answer_that_is_not_an_upgrade_is_a_protocol_error() {
    let base = answering("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
    let report = probe_at(&base, "token", Duration::from_secs(10));
    assert_eq!(report.outcome, Outcome::ProtocolError, "{}", report.line());
    assert_eq!(report.stage, Stage::Upgrade);
    assert!(
        report.reason.as_deref().unwrap().contains("200"),
        "{}",
        report.line()
    );
    assert_eq!(report.outcome.status(), 76);
}

/// `sce-relay probe` with `args`, run to its end.
fn sce_relay_probe(args: &[&str]) -> Output {
    Command::new(BIN)
        .arg("probe")
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("CREDENTIALS_DIRECTORY")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// The one line a probe printed, when it printed exactly one.
fn line_of(output: &Output) -> String {
    let out = text(&output.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1, "{out:?}; {}", text(&output.stderr));
    lines[0].to_string()
}

#[test]
fn the_program_prints_one_line_and_exits_with_the_status_of_its_outcome() {
    let scratch = Scratch::new("program");
    let authority = Authority::new([9; 32], AUDIENCE);
    let key = scratch.secret("authority.key", &authority.to_key_text());
    let key = key.to_str().unwrap();
    let running = start(MemoryStore::new(), &authority, Limits::default(), None);

    // From the authority's key: its own list, kept once.
    let first = sce_relay_probe(&[&running.base, "--key", key]);
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    let line = line_of(&first);
    assert!(
        line.starts_with("outcome=ok relay=ws://")
            && line.contains(" asked=write_read list=created "),
        "{line}"
    );
    assert!(line.contains(" total_ms="), "{line}");
    let again = sce_relay_probe(&[&running.base, "--key", key]);
    assert_eq!(again.status.code(), Some(0), "{}", text(&again.stderr));
    assert!(line_of(&again).contains(" list=held "));

    // From a token in a file: refused, and never printed.
    let stranger = own_token(&Authority::new([1; 32], AUDIENCE), now_ms());
    let file = scratch.secret("stranger.token", &format!("{stranger}\n"));
    let refused = sce_relay_probe(&[&running.base, "--token-file", file.to_str().unwrap()]);
    assert_eq!(refused.status.code(), Some(77), "{}", text(&refused.stderr));
    let line = line_of(&refused);
    assert!(
        line.starts_with("outcome=token_refused ") && line.contains(" stage=upgrade "),
        "{line}"
    );
    assert!(!text(&refused.stdout).contains(&stranger));
    assert!(!text(&refused.stderr).contains(&stranger));

    // A token file others may read is refused before anything is dialled.
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    let exposed = sce_relay_probe(&[&running.base, "--token-file", file.to_str().unwrap()]);
    assert_eq!(exposed.status.code(), Some(78), "{}", text(&exposed.stderr));
    assert!(exposed.stdout.is_empty());

    // Nobody at the address.
    let unreachable = sce_relay_probe(&[&nothing_listens(), "--key", key]);
    assert_eq!(unreachable.status.code(), Some(69));
    assert!(line_of(&unreachable).starts_with("outcome=unreachable "));
}

#[test]
fn the_program_reaches_a_relay_under_a_private_authority_only_when_told_to_believe_it() {
    let scratch = Scratch::new("tls");
    let authority = Authority::new([9; 32], AUDIENCE);
    let key = scratch.secret("authority.key", &authority.to_key_text());
    let key = key.to_str().unwrap();
    let (certificate, private) = certificate();
    let ca = scratch.path("ca.der");
    fs::write(&ca, &certificate).unwrap();
    let running = start(
        MemoryStore::new(),
        &authority,
        Limits::default(),
        Some((certificate, private)),
    );

    let believed = sce_relay_probe(&[&running.base, "--key", key, "--ca", ca.to_str().unwrap()]);
    assert_eq!(
        believed.status.code(),
        Some(0),
        "{}",
        text(&believed.stderr)
    );
    assert!(line_of(&believed).contains(" tls_ms="));

    let public = sce_relay_probe(&[&running.base, "--key", key]);
    assert_eq!(public.status.code(), Some(79), "{}", text(&public.stderr));
    assert!(line_of(&public).starts_with("outcome=tls_refused "));

    let not_a_certificate = scratch.path("empty.pem");
    fs::write(&not_a_certificate, "").unwrap();
    let unusable = sce_relay_probe(&[
        &running.base,
        "--key",
        key,
        "--ca",
        not_a_certificate.to_str().unwrap(),
    ]);
    assert_eq!(
        unusable.status.code(),
        Some(78),
        "{}",
        text(&unusable.stderr)
    );
}

#[test]
fn the_program_refuses_a_probe_it_cannot_make_before_it_dials() {
    let scratch = Scratch::new("usage");
    let key = scratch.secret(
        "authority.key",
        &Authority::new([9; 32], AUDIENCE).to_key_text(),
    );
    let key = key.to_str().unwrap();
    let token = scratch.secret("probe.token", "token\n");
    let token = token.to_str().unwrap();
    let usage: [&[&str]; 4] = [
        // No token and no key.
        &["ws://127.0.0.1:1"],
        // Both.
        &["ws://127.0.0.1:1", "--key", key, "--token-file", token],
        &["ws://127.0.0.1:1", "--key", key, "--timeout", "0"],
        &["https://127.0.0.1:1", "--key", key],
    ];
    for args in usage {
        let output = sce_relay_probe(args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            text(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{args:?}");
    }
    let absent = Path::new(key).with_extension("absent");
    let missing = sce_relay_probe(&["ws://127.0.0.1:1", "--key", absent.to_str().unwrap()]);
    assert_eq!(missing.status.code(), Some(78), "{}", text(&missing.stderr));
}
