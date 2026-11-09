// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The relay as the program a service manager runs (`sce-relay`). A key is made for its owner alone and
// never printed, and a token issued from it names one device of one list; a relay that cannot start
// says what is wrong and exits with a status that says whether starting it again could help; and a
// relay that started says it is ready (`sd_notify`), answers a device that presents a token, stops when
// it is sent SIGTERM (saying so, and exiting 0), and holds after a restart what it took before it; a
// relay that takes creations with no token removes, as it starts, the lists created so that lapsed
// while it was stopped, and says how many and never which. No line it writes holds the key or a
// token.
//
// The relay's readiness is read from an abstract socket, which only Linux has: the deployment target.

#![cfg(target_os = "linux")]

use std::fs;
use std::io::{BufRead, BufReader};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sce_rust_relay::serve::WssLink;
use sce_rust_relay::{
    Authority, FileStore, Link, Openly, Record, Request, Response, Session, Store, TokenError,
};
use sce_rust_trust::membership::{Body, Log, Profile};
use sce_rust_trust::Identity;

const BIN: &str = env!("CARGO_BIN_EXE_sce-relay");
const LIST: &str = "home";
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
            "sce-relay-svc-{tag}-{}-{}",
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
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// `sce-relay` with `args`, run to its end, with no journal and no service manager.
fn sce_relay(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("NOTIFY_SOCKET")
        .env_remove("CREDENTIALS_DIRECTORY")
        .output()
        .unwrap()
}

fn status_of(output: &Output) -> Option<i32> {
    output.status.code()
}

/// Make the authority's key in `scratch`.
fn keygen(scratch: &Scratch) -> PathBuf {
    let key = scratch.path("authority.key");
    let made = sce_relay(&[
        "keygen",
        "--out",
        key.to_str().unwrap(),
        "--audience",
        AUDIENCE,
    ]);
    assert_eq!(status_of(&made), Some(0), "{}", text(&made.stderr));
    key
}

/// The secret of the key at `key`, as its file writes it: what no line may hold.
fn secret_of(key: &Path) -> String {
    fs::read_to_string(key)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("secret "))
        .unwrap()
        .to_string()
}

/// A token for node 1 of the list, issued by `sce-relay token issue`.
fn token(key: &Path) -> String {
    let issued = sce_relay(&[
        "token",
        "issue",
        "--key",
        key.to_str().unwrap(),
        "--list",
        LIST,
        "--node",
        "1",
    ]);
    assert_eq!(status_of(&issued), Some(0), "{}", text(&issued.stderr));
    text(&issued.stdout).trim().to_string()
}

/// A configuration in `scratch` that listens on `listen`, keeps its store beside it and reads the
/// key at `key`.
fn configuration(scratch: &Scratch, key: &Path, listen: &str) -> PathBuf {
    let path = scratch.path("relay.toml");
    fs::write(
        &path,
        format!(
            "[listen]\nplain = [\"{listen}\"]\n\n[store]\ndirectory = \"store\"\n\n\
             [authority]\nkey_file = \"{}\"\n\n[connections]\nshutdown_grace_seconds = 5\n",
            key.display()
        ),
    )
    .unwrap();
    path
}

/// The first entry of the list's log, by its administrator.
fn genesis() -> String {
    genesis_of(LIST)
}

/// The first entry of the log of `list`, by its administrator.
fn genesis_of(list: &str) -> String {
    let administrator = Identity::from_seed([9; 32]);
    Log::genesis_draft(
        list,
        Body::Genesis {
            administrators: vec![administrator.public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator)
    .finish()
    .to_json()
}

/// A relay running as a service would run it, with a service manager's socket to tell.
struct Running {
    child: Child,
    lines: Receiver<String>,
    seen: Vec<String>,
    manager: UnixDatagram,
    address: String,
}

impl Running {
    fn start(configuration: &Path) -> Running {
        let name = format!(
            "sce-relay-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let manager =
            UnixDatagram::bind_addr(&SocketAddr::from_abstract_name(name.as_bytes()).unwrap())
                .unwrap();
        manager
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        let mut child = Command::new(BIN)
            .args(["serve", "--config", configuration.to_str().unwrap()])
            .env("NOTIFY_SOCKET", format!("@{name}"))
            .env_remove("JOURNAL_STREAM")
            .env_remove("CREDENTIALS_DIRECTORY")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { break };
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        let mut running = Running {
            child,
            lines,
            seen: Vec::new(),
            manager,
            address: String::new(),
        };
        let ready = running.told();
        assert!(
            ready.lines().any(|line| line == "READY=1"),
            "{ready:?}; {:?}",
            running.seen
        );
        let listening = running.line_with("event=listening");
        running.address = listening
            .split_whitespace()
            .find_map(|word| word.strip_prefix("address="))
            .unwrap()
            .to_string();
        running
    }

    /// What the relay told its service manager next.
    fn told(&self) -> String {
        let mut message = [0u8; 512];
        let length = self
            .manager
            .recv(&mut message)
            .expect("the relay told its service manager nothing");
        text(&message[..length])
    }

    /// The next line on stderr that holds `word`.
    fn line_with(&mut self, word: &str) -> String {
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(60))
                .unwrap_or_else(|_| panic!("no line with {word}; seen {:?}", self.seen));
            self.seen.push(line.clone());
            if line.contains(word) {
                return line;
            }
        }
    }

    fn link(&self, token: &str) -> WssLink {
        WssLink::new(&format!("ws://{}", self.address), token).unwrap()
    }

    /// Send SIGTERM, as a service manager stops a service, and wait for the relay to exit: its
    /// status and every line it wrote.
    fn terminate(&mut self) -> (ExitStatus, Vec<String>) {
        let sent = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(sent.success());
        let stopping = self.told();
        assert!(
            stopping.lines().any(|line| line == "STOPPING=1"),
            "{stopping:?}"
        );
        let status = self.child.wait().unwrap();
        while let Ok(line) = self.lines.recv_timeout(Duration::from_secs(10)) {
            self.seen.push(line);
        }
        (status, std::mem::take(&mut self.seen))
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // A test that failed leaves no relay running.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_key_is_made_for_its_owner_alone_and_a_token_from_it_names_one_device() {
    let scratch = Scratch::new("key");
    let key = scratch.path("authority.key");
    let made = sce_relay(&[
        "keygen",
        "--out",
        key.to_str().unwrap(),
        "--audience",
        AUDIENCE,
    ]);
    assert_eq!(status_of(&made), Some(0), "{}", text(&made.stderr));
    assert!(made.stdout.is_empty(), "a key is never printed");
    let secret = secret_of(&key);
    assert!(!text(&made.stderr).contains(&secret));
    assert!(text(&made.stderr).contains("event=key_made"));
    let mode = fs::metadata(&key).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let authority = Authority::from_key_text(&fs::read_to_string(&key).unwrap()).unwrap();
    assert_eq!(authority.audience(), AUDIENCE);

    // A key is never made over another.
    let again = sce_relay(&[
        "keygen",
        "--out",
        key.to_str().unwrap(),
        "--audience",
        AUDIENCE,
    ]);
    assert_eq!(status_of(&again), Some(73), "{}", text(&again.stderr));
    assert_eq!(secret_of(&key), secret);
    let unnamed = sce_relay(&[
        "keygen",
        "--out",
        scratch.path("other.key").to_str().unwrap(),
        "--audience",
        " spaced",
    ]);
    assert_eq!(status_of(&unnamed), Some(2), "{}", text(&unnamed.stderr));
    assert!(!scratch.path("other.key").exists());

    // A token names node 7 of the list, for the time it was issued for and no longer.
    let issued = sce_relay(&[
        "token",
        "issue",
        "--key",
        key.to_str().unwrap(),
        "--list",
        LIST,
        "--node",
        "7",
        "--valid-for",
        "600",
    ]);
    // Issued before now: good now, and gone 600 seconds from now.
    let after = now_ms();
    assert_eq!(status_of(&issued), Some(0), "{}", text(&issued.stderr));
    let token = text(&issued.stdout).trim().to_string();
    assert!(!text(&issued.stderr).contains(&token));
    assert_eq!(
        authority.verify(&token, after),
        Ok(Session {
            list: LIST.to_string(),
            node: 7
        })
    );
    assert_eq!(
        authority.verify(&token, after + 600_000),
        Err(TokenError::Expired)
    );

    // A key others may read issues nothing.
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    let exposed = sce_relay(&[
        "token",
        "issue",
        "--key",
        key.to_str().unwrap(),
        "--list",
        LIST,
        "--node",
        "7",
    ]);
    assert_eq!(status_of(&exposed), Some(78), "{}", text(&exposed.stderr));
    assert!(exposed.stdout.is_empty());
    assert!(
        text(&exposed.stderr).contains("chmod 600"),
        "{}",
        text(&exposed.stderr)
    );
}

#[test]
fn a_relay_that_cannot_start_says_what_is_wrong_and_exits_with_its_reason() {
    let scratch = Scratch::new("refuse");
    let key = keygen(&scratch);
    let configuration = configuration(&scratch, &key, "127.0.0.1:0");
    let check =
        |configuration: &Path| sce_relay(&["check", "--config", configuration.to_str().unwrap()]);

    let checked = check(&configuration);
    assert_eq!(status_of(&checked), Some(0), "{}", text(&checked.stderr));
    assert!(text(&checked.stderr).contains("event=checked"));

    // No configuration file.
    let absent = check(&scratch.path("absent.toml"));
    assert_eq!(status_of(&absent), Some(78), "{}", text(&absent.stderr));
    assert!(text(&absent.stderr).contains("absent.toml"));

    // A setting the relay does not read.
    let mistyped = scratch.path("mistyped.toml");
    fs::write(
        &mistyped,
        fs::read_to_string(&configuration)
            .unwrap()
            .replace("key_file", "keyfile"),
    )
    .unwrap();
    let refused = check(&mistyped);
    assert_eq!(status_of(&refused), Some(78), "{}", text(&refused.stderr));
    assert!(
        text(&refused.stderr).contains("[authority] keyfile is not a setting"),
        "{}",
        text(&refused.stderr)
    );

    // A key that is not there, and one others may read.
    let elsewhere = scratch.path("elsewhere.toml");
    fs::write(
        &elsewhere,
        fs::read_to_string(&configuration)
            .unwrap()
            .replace("authority.key", "missing.key"),
    )
    .unwrap();
    let missing = check(&elsewhere);
    assert_eq!(status_of(&missing), Some(78), "{}", text(&missing.stderr));
    assert!(
        text(&missing.stderr).contains("missing.key"),
        "{}",
        text(&missing.stderr)
    );
    fs::set_permissions(&key, fs::Permissions::from_mode(0o640)).unwrap();
    let exposed = check(&configuration);
    assert_eq!(status_of(&exposed), Some(78), "{}", text(&exposed.stderr));
    assert!(!text(&exposed.stderr).contains(&secret_of(&key)));
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();

    // A store that holds what no member signed.
    let slot = scratch.path("store").join("slot-0");
    fs::create_dir_all(&slot).unwrap();
    fs::write(
        slot.join("686f6d65.jsonl"),
        "{\"list\":\"home\",\"record\":{\"k\":\"op\",\"e\":\"record_list.change\",\"d\":\"{}\"}}\n",
    )
    .unwrap();
    let forged = check(&configuration);
    assert_eq!(status_of(&forged), Some(65), "{}", text(&forged.stderr));
    assert!(
        text(&forged.stderr).contains("does not verify"),
        "{}",
        text(&forged.stderr)
    );

    // A command line the program does not take.
    let usage = sce_relay(&["serve", "--limit", "bursts=1"]);
    assert_eq!(status_of(&usage), Some(2), "{}", text(&usage.stderr));
}

#[test]
fn a_relay_that_takes_creations_removes_the_lists_that_lapsed_while_it_was_stopped() {
    let scratch = Scratch::new("lapse");
    let key = keygen(&scratch);
    let configuration = configuration(&scratch, &key, "127.0.0.1:0");
    let mut written = fs::read_to_string(&configuration).unwrap();
    written.push_str("\n[creation]\npolicy = \"open\"\nlapse_days = 90\n");
    fs::write(&configuration, written).unwrap();
    // A store that holds a list created openly whose last login was at the epoch, long ago, and
    // one created with a token, which never lapses.
    let slot = scratch.path("store").join("slot-0");
    let mut store = FileStore::open(&slot).unwrap();
    store
        .mark(
            LIST,
            &Openly {
                created_at_ms: 0,
                touched_at_ms: 0,
            },
        )
        .unwrap();
    store.append(LIST, &Record::Entry(genesis())).unwrap();
    store
        .append("kept", &Record::Entry(genesis_of("kept")))
        .unwrap();
    drop(store);

    let mut running = Running::start(&configuration);
    let ready = running.line_with("event=ready");
    assert!(ready.contains("creation=open"), "{ready}");
    assert!(ready.contains("lapse_days=90"), "{ready}");
    assert!(
        running
            .seen
            .iter()
            .any(|line| line.contains("event=lapsed removed=1 kept=0")),
        "{:?}",
        running.seen
    );
    let (status, lines) = running.terminate();
    assert_eq!(status.code(), Some(0), "{lines:?}");
    let mut after = FileStore::open(&slot).unwrap();
    let held: Vec<String> = after
        .load()
        .unwrap()
        .into_iter()
        .map(|(list, _)| list)
        .collect();
    assert_eq!(
        held,
        vec!["kept".to_string()],
        "the list that lapsed is gone"
    );
    assert!(after.marks().unwrap().is_empty());
}

#[test]
fn a_relay_answers_until_it_is_told_to_stop_and_holds_what_it_took_after_a_restart() {
    let scratch = Scratch::new("serve");
    let key = keygen(&scratch);
    let configuration = configuration(&scratch, &key, "127.0.0.1:0");
    let token = token(&key);
    let secret = secret_of(&key);

    let mut running = Running::start(&configuration);
    let mut link = running.link(&token);
    let appended = link.call(&Request::Append { entry: genesis() }).unwrap();
    assert_eq!(appended, Response::Appended { new: true, len: 1 });

    // A second relay on the same address cannot take it, and says it may later.
    let taken = sce_relay(&[
        "serve",
        "--config",
        configuration.to_str().unwrap(),
        "--listen",
        &running.address,
    ]);
    assert_eq!(status_of(&taken), Some(69), "{}", text(&taken.stderr));
    assert!(text(&taken.stderr).contains("cannot listen on"));

    // Stopped with a device still connected: it closes the connection and exits 0.
    let (status, lines) = running.terminate();
    assert_eq!(status.code(), Some(0), "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("event=stopping signal=SIGTERM")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("event=stopped")),
        "{lines:?}"
    );
    for line in &lines {
        assert!(!line.contains(&secret) && !line.contains(&token), "{line}");
    }
    drop(link);

    // Started again on the same store, it holds the log it was given.
    let mut again = Running::start(&configuration);
    let mut link = again.link(&token);
    let entries = link.call(&Request::Entries { from: 0 }).unwrap();
    assert!(
        matches!(entries, Response::Entries { len: 1, .. }),
        "{entries:?}"
    );
    let (status, lines) = again.terminate();
    assert_eq!(status.code(), Some(0), "{lines:?}");
}
