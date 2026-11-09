// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Devices of the standard shared record list (sce:std/shared/record_list.scxml) that each hold
// the list through the peer core of ADR 0079, and the relay between them, over a network that
// runs on the world's clock. What the tests that ask whether every device holds one list have
// in common.
//
// A device is a machine and a core. The machine is a world actor, a copy of the record list,
// and knows nothing of the core: what it sends is kept by its host and given to the core, which
// signs it and keeps it for the relay, and what a round of the core brings back is raised into
// it. The core is one of two:
//
// - a Rust device's is `sce_rust_peer::Peer` in this process, driven by the world through the
//   texts a host that carries them itself gives a core (`poll_request`, `respond`, `fail`): the
//   core does no I/O, which is what lets the world's clock and network carry its rounds;
// - a Kotlin device's is the same core in the JVM of its machine's step server, reached through
//   the C ABI with the Kotlin binding (backends/kotlin/peer), and a round is the library's own
//   driver, `sce_peer_run`, over a transport whose every exchange the step server hands to the
//   world (`peer_*` of `sce_rust_world::external`). Nothing of the relay's protocol is written in
//   Kotlin, and the world reads the outcome the ABI gives, as an application's lane would.
//
// Between every device and the relay is a path the world draws from the seed: how long a
// request and a response take, how far that spreads (far enough to bring one device's request
// to the relay after another's that was sent later), how many of them are lost, and how long
// the device's transport waits for an answer. A lost request is the connection breaking, a lost
// or late response the deadline passing, and a device can be suspended with a request out: its
// job's time ends, its run is cancelled, and the relay still takes what reached it. The relay is
// the reference relay, with its own allowance (`Limits`), and it can be made to answer less than
// it holds or to give a write it made up ([`Dishonesty`]).
//
// With no JVM, a test that asks for a Kotlin device gets a Rust one in its place and says so;
// with `SCE_REQUIRE_TOOLS` set it fails instead, as every test that needs a toolchain does.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path as FilePath, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_peer::{
    CourierError, Device as GroupDevice, GroupStep, Kept, Outcome, Peer, Profile, RoundKind,
    TransportError, WRITER,
};
use sce_rust_relay::{Authority, Code, Limits, MemoryStore, Record, Relay, Request, Response};
use sce_rust_trust::Identity;
use sce_rust_world::external::{ExternalFailure, ExternalMachine};
use sce_rust_world::{Outcome as Rest, Rng, World};
use serde_json::{json, Value};

use super::kotlin::step_server_for;
use super::{
    record_copy, record_edit_at, record_variables_of, records_of, Field, RecordList, DUE, TITLE,
};

/// The list every device holds.
pub const LIST: &str = "home";

/// The nodes of the three devices: alice's holds the administrator's key and opens the list.
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CAROL: u64 = 3;

/// The row a relay that forges moves a write to; no device writes it.
pub const FORGED_ROW: u64 = 77;

/// The machine a Kotlin device's step server hosts.
const MACHINE: &str = "record_list";

/// What a change is called, which a relay that forges makes one of: one change is one signed
/// operation (docs/adr/0085, section 1).
const CHANGE: &str = "record_list.change";

/// What a device's host answers its machine's send with: the send is the core's from then on,
/// and no peer of the Mesh is sent anything.
const GIVEN: &str = "given to the core of the device";

/// The status `sce_peer_run` stops with when its cancellation was raised
/// (`SCE_PEER_ERR_CANCELLED`, `include/sce_peer.h`).
const RUN_CANCELLED: i64 = 24;

/// How many passes of rounds the devices get to come to rest.
const PASSES: usize = 80;

/// The world's time that passes between two passes of rounds: a device's job runs again.
const BETWEEN_PASSES_MS: i64 = 1_000;

/// What the machine of a device sent that its host has not given the core yet.
type Sends = Arc<Mutex<Vec<(String, String)>>>;

/// The key of the person of `node`.
pub fn key_of(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// The administrator's key, which alice's device holds.
pub fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// A relay with an allowance no test reaches: what the faults of the network and of the relay
/// are asked under, so that no refusal for the rate is among what they cause.
pub fn ample() -> Limits {
    Limits {
        burst: 1_000_000,
        refill_per_second: 1_000_000,
        ..Limits::default()
    }
}

/// The peer core's shared library (`sce-rust-peer-capi`), which a Kotlin device loads. A test
/// binary links Rust libraries and builds no shared one, so it is built here, once for the
/// binary, into the target directory the test runs from, as the ABI's own lane tests build it.
pub fn peer_library() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let exe = std::env::current_exe().expect("the test knows where it runs from");
            let profile = exe
                .parent()
                .and_then(FilePath::parent)
                .expect("a test runs from deps/ of a profile's directory")
                .to_path_buf();
            let manifest =
                FilePath::new(env!("CARGO_MANIFEST_DIR")).join("../peer-capi/Cargo.toml");
            let mut build =
                Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
            build
                .args(["build", "--lib", "--locked", "--manifest-path"])
                .arg(&manifest);
            if profile.file_name().is_some_and(|name| name == "release") {
                build.arg("--release");
            }
            let output = build.output().expect("cargo runs");
            assert!(
                output.status.success(),
                "the peer core's shared library did not build:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            [
                "libsce_peer_capi.so",
                "libsce_peer_capi.dylib",
                "sce_peer_capi.dll",
            ]
            .iter()
            .map(|name| profile.join(name))
            .find(|path| path.exists())
            .unwrap_or_else(|| {
                panic!(
                    "no shared library of the peer core in {}",
                    profile.display()
                )
            })
        })
        .clone()
}

/// A step server hosting the record list, for a Kotlin device named `name`; `None` after saying
/// why this run has none (the convention of `kotlin::step_server`).
///
/// # Panics
///
/// When `SCE_REQUIRE_TOOLS` is set and the JVM or the build is missing, or the server does not
/// start.
pub fn kotlin_device(name: &str) -> Option<ExternalMachine> {
    let command = step_server_for(MACHINE)?;
    let machine = ExternalMachine::spawn(&command.args(["--session-id", name]))
        .unwrap_or_else(|failure| panic!("the Kotlin step server for {name}: {failure}"));
    assert_eq!(
        machine.machine(),
        MACHINE,
        "the step server hosts another machine"
    );
    Some(machine)
}

/// What the network between one device and the relay does, each way.
#[derive(Debug, Clone, Copy)]
pub struct Path {
    /// The least time a request, or a response, takes.
    pub latency_ms: i64,
    /// What is added to it, drawn below this from the seed.
    pub jitter_ms: i64,
    /// Percent of requests, and of responses, lost: a lost request is the connection breaking
    /// before the relay had it, a lost response the deadline passing after it did.
    pub loss_pct: u64,
    /// How long the device's transport waits for the answer to a request it sent.
    pub timeout_ms: i64,
}

impl Default for Path {
    fn default() -> Self {
        Self {
            latency_ms: 5,
            jitter_ms: 0,
            loss_pct: 0,
            timeout_ms: 2_000,
        }
    }
}

impl Path {
    fn delay(&self, rng: &mut Rng) -> i64 {
        let spread = u64::try_from(self.jitter_ms).unwrap_or(0);
        self.latency_ms + i64::try_from(rng.below(spread + 1)).unwrap_or(0)
    }
}

/// What the relay does that it should not.
#[derive(Debug, Clone, Default)]
pub struct Dishonesty {
    /// `(node, row)`: every operation of the row is left out of what the relay gives the device
    /// of the node, in a feed and in a range, and the feed's cursor goes past it all the same.
    pub withhold: Vec<(u64, u64)>,
    /// The nodes whose devices the relay gives, after each write, a copy it made up: the same
    /// write moved to [`FORGED_ROW`], with the signature the true one carries.
    pub forge_toward: Vec<u64>,
}

/// How the list's content is keyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keying {
    /// By nothing: no epoch is begun, and a device is admitted with a grant.
    Clear,
    /// By the list's MLS group, of which every device holds a device: a device is admitted
    /// from the key package it left with the relay ([`Devices::admit_through_the_group`]).
    Group,
}

/// An exchange a device's core begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Begin {
    Round(RoundKind),
    /// The publication of a key package of the group's device.
    KeyPackage,
}

impl Begin {
    fn word(self) -> &'static str {
        match self {
            Begin::Round(RoundKind::Sync) => "sync",
            Begin::Round(RoundKind::Resync) => "resync",
            Begin::Round(RoundKind::Reread) => "reread",
            Begin::KeyPackage => "key_package",
        }
    }
}

/// What an exchange of a device's core came to, read alike from the Rust core's outcome and
/// from the text the C ABI gives (docs/SCE_SHARED_LIST_PEER_ABI.md section 5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    /// Why it failed (`link`, `relay` or `unexpected`) and the reason; `None` when it ended.
    pub failed: Option<(String, String)>,
    /// When the relay refused it for the device's allowance, how long until it may ask again.
    pub retry_after_ms: Option<u64>,
    /// The events for the machine, in order.
    pub events: Vec<(String, String)>,
    /// What the core or the relay refused, and why.
    pub refused: Vec<String>,
    /// How many records the relay was given that it lacked.
    pub pushed: usize,
    pub stalled: bool,
    pub rebuild: bool,
    /// What the group's device did with what was read: `joined`, `applied`, `removed`,
    /// `refused`.
    pub steps: Vec<String>,
    /// Whether the run stopped because its cancellation was raised.
    pub cancelled: bool,
}

impl Ran {
    fn of_core(outcome: Result<Outcome, CourierError>) -> Self {
        match outcome {
            Ok(outcome) => Ran {
                events: outcome.events(),
                refused: outcome.refused(),
                pushed: outcome.round.pushed
                    + outcome.reread.as_ref().map_or(0, |again| again.pushed),
                stalled: outcome.stalled(),
                rebuild: outcome.rebuild(),
                steps: outcome.steps.iter().map(step_word).collect(),
                ..Ran::default()
            },
            // The words a transport's failure is given to the core in, which `sce_peer_run`
            // gives a run whose cancellation was raised.
            Err(CourierError::Link(reason)) => Ran {
                cancelled: reason == TransportError::Cancelled.to_string(),
                failed: Some(("link".to_string(), reason)),
                ..Ran::default()
            },
            Err(CourierError::Relay { code, reason }) => Ran {
                retry_after_ms: match code {
                    Code::RateLimited { retry_after_ms } => Some(retry_after_ms),
                    _ => None,
                },
                failed: Some(("relay".to_string(), reason)),
                ..Ran::default()
            },
            Err(CourierError::Unexpected(reason)) => Ran {
                failed: Some(("unexpected".to_string(), reason)),
                ..Ran::default()
            },
        }
    }

    fn of_text(status: i64, text: &str) -> Self {
        let value: Value = serde_json::from_str(text).unwrap_or_else(|error| {
            panic!("an outcome the ABI gave is not JSON ({error}): {text}")
        });
        let word = |name: &str| value[name].as_str().map(str::to_string);
        let words = |name: &str| -> Vec<String> {
            value[name]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut ran = match value["outcome"].as_str() {
            Some("ended") => Ran {
                events: value["events"]
                    .as_array()
                    .unwrap_or_else(|| panic!("an ended outcome tells its events: {text}"))
                    .iter()
                    .map(|pair| {
                        let part = |at: usize| {
                            pair[at]
                                .as_str()
                                .unwrap_or_else(|| panic!("an event is two texts: {pair}"))
                                .to_string()
                        };
                        (part(0), part(1))
                    })
                    .collect(),
                refused: words("refused"),
                pushed: value["pushed"]
                    .as_u64()
                    .and_then(|pushed| usize::try_from(pushed).ok())
                    .unwrap_or_else(|| panic!("an ended outcome says what it pushed: {text}")),
                stalled: value["stalled"].as_bool() == Some(true),
                rebuild: value["rebuild"].as_bool() == Some(true),
                steps: value["steps"]
                    .as_array()
                    .map(|steps| {
                        steps
                            .iter()
                            .filter_map(|step| step["step"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                ..Ran::default()
            },
            Some("failed") => Ran {
                failed: Some((
                    word("failure").unwrap_or_default(),
                    word("reason").unwrap_or_default(),
                )),
                retry_after_ms: word("retry_after_ms").and_then(|after| after.parse().ok()),
                ..Ran::default()
            },
            _ => panic!("an outcome is ended or failed: {text}"),
        };
        ran.cancelled = status == RUN_CANCELLED;
        ran
    }

    /// Whether the exchange moved anything: it failed, gave the relay something, told the
    /// machine something, or stopped before the end of what the relay holds.
    fn moved(&self) -> bool {
        self.failed.is_some() || self.pushed > 0 || !self.events.is_empty() || self.stalled
    }
}

fn step_word(step: &GroupStep) -> String {
    match step {
        GroupStep::Joined(_) => "joined",
        GroupStep::Applied(_) => "applied",
        GroupStep::Removed => "removed",
        GroupStep::Refused(_) => "refused",
    }
    .to_string()
}

/// What became of one exchange, as a transport says it.
enum Carried {
    Response(String),
    Failed(TransportError),
}

impl Carried {
    /// As the step server's `peer_carried` says it.
    fn request(&self) -> Value {
        match self {
            Carried::Response(text) => json!({ "op": "peer_carried", "response": text }),
            Carried::Failed(TransportError::Link(reason)) => {
                json!({ "op": "peer_carried", "failed": "link", "reason": reason })
            }
            Carried::Failed(TransportError::Deadline) => {
                json!({ "op": "peer_carried", "failed": "deadline" })
            }
            Carried::Failed(TransportError::Cancelled) => {
                json!({ "op": "peer_carried", "failed": "cancelled" })
            }
            Carried::Failed(TransportError::Unauthorized(_)) => {
                unreachable!("the world's relay takes every device's token")
            }
            // The world's network has no endpoint that refuses a connection: the relay's refusal
            // for the rate is a response the world carries.
            Carried::Failed(TransportError::RateLimited { .. }) => {
                unreachable!("the world's relay refuses no connection for the rate")
            }
        }
    }
}

/// What a device's exchange asks next, or what it came to.
enum Next {
    Ask(String),
    /// The run waits this long on the world's clock before it asks again: the relay refused a
    /// request for the rate, and a Kotlin device's run waits on its host's clock
    /// (`sce_peer_link_set_pause`), which is the world's.
    Pause(u64),
    Ran(Ran),
}

/// The core of a device.
enum Core {
    /// In this process.
    Rust(Box<Peer>),
    /// In the step server of the device's machine, through the C ABI.
    Kotlin(ExternalMachine),
}

/// What stands behind a device's machine: its host keeps each send for the core, and no peer
/// of the Mesh is sent anything, since the relay carries what the core signs.
struct ToTheCore(Sends);

impl Sealer for ToTheCore {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        self.0
            .lock()
            .expect("the sends are never left mid-update")
            .push((event.to_string(), data.to_string()));
        Err(format!("{event}: {GIVEN}"))
    }
}

/// One device of the list.
pub struct Device {
    pub name: &'static str,
    pub node: u64,
    core: Core,
    sent: Sends,
    path: Path,
    /// What the core did with each send it was given, in order.
    pub kept: Vec<Kept>,
    /// Every refusal its exchanges came to.
    pub refused: Vec<String>,
    /// What its group's device did, in order.
    pub steps: Vec<String>,
    /// Its next round is suspended once this many requests of it went out.
    suspend_after: Option<usize>,
}

impl Device {
    fn rust(&mut self) -> &mut Peer {
        match &mut self.core {
            Core::Rust(peer) => peer,
            Core::Kotlin(_) => panic!("{} is not a Rust device", self.name),
        }
    }

    fn outbound(&mut self, event: &str, data: &str) -> Kept {
        let name = self.name;
        match &mut self.core {
            Core::Rust(peer) => peer.outbound(event, data),
            Core::Kotlin(machine) => {
                let reply = ask(
                    name,
                    machine,
                    &json!({ "op": "peer_outbound", "event": event, "data": data }),
                );
                let reason = reply["reason"].as_str().map(str::to_string);
                match reply["kept"].as_str() {
                    Some("taken") => Kept::Taken,
                    Some("held") => Kept::Held,
                    Some("not_signed") => Kept::NotSigned,
                    Some("refused") => Kept::Refused(reason.unwrap_or_default()),
                    Some("waiting") => Kept::Waiting(reason.unwrap_or_default()),
                    _ => panic!("{name}: the core kept a send as nothing the ABI says: {reply}"),
                }
            }
        }
    }

    fn begin(&mut self, begin: Begin) -> Next {
        let name = self.name;
        match &mut self.core {
            Core::Rust(peer) => {
                let begun = match begin {
                    Begin::Round(kind) => peer.start(kind),
                    Begin::KeyPackage => peer.publish_key_package(),
                };
                begun.unwrap_or_else(|error| panic!("{name}: no exchange begins: {error:?}"));
                next_of_core(peer)
            }
            Core::Kotlin(machine) => next_of_text(
                name,
                &ask(
                    name,
                    machine,
                    &json!({ "op": "peer_round", "begin": begin.word() }),
                ),
            ),
        }
    }

    fn carry(&mut self, carried: Carried) -> Next {
        let name = self.name;
        match &mut self.core {
            Core::Rust(peer) => {
                let given = match carried {
                    Carried::Response(text) => peer.respond(&text),
                    Carried::Failed(error) => peer.fail(&error.to_string()),
                };
                given.unwrap_or_else(|error| {
                    panic!("{name}: the core waits for no answer: {error:?}")
                });
                next_of_core(peer)
            }
            Core::Kotlin(machine) => next_of_text(name, &ask(name, machine, &carried.request())),
        }
    }

    /// The wait the device's run asked for is over on the world's clock: its run goes on.
    fn resume(&mut self) -> Next {
        let name = self.name;
        match &mut self.core {
            Core::Rust(_) => unreachable!("{name}: a core the world drives by hand never pauses"),
            Core::Kotlin(machine) => {
                next_of_text(name, &ask(name, machine, &json!({ "op": "peer_paused" })))
            }
        }
    }

    /// The epochs whose keys the device holds.
    fn keys(&mut self) -> Vec<u64> {
        let name = self.name;
        match &mut self.core {
            Core::Rust(peer) => peer.standing().keys.into_iter().map(u64::from).collect(),
            Core::Kotlin(machine) => {
                let reply = ask(name, machine, &json!({ "op": "peer_standing" }));
                let text = reply["standing"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{name}: no standing: {reply}"));
                let standing: Value = serde_json::from_str(text)
                    .unwrap_or_else(|error| panic!("{name}: a standing is JSON ({error}): {text}"));
                standing["keys"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{name}: a standing names its keys: {text}"))
                    .iter()
                    .map(|epoch| epoch.as_u64().expect("an epoch is a number"))
                    .collect()
            }
        }
    }
}

/// A request of the device's step server, and its reply, or the end of the run.
fn ask(name: &str, machine: &ExternalMachine, request: &Value) -> Value {
    machine
        .ask(request)
        .unwrap_or_else(|failure: ExternalFailure| panic!("{name}: {failure}"))
}

fn next_of_core(peer: &mut Peer) -> Next {
    match peer.poll_request() {
        Some(request) => Next::Ask(request),
        None => Next::Ran(Ran::of_core(
            peer.take_outcome()
                .expect("an exchange that asks nothing more has ended"),
        )),
    }
}

fn next_of_text(name: &str, reply: &Value) -> Next {
    if let Some(request) = reply["exchange"].as_str() {
        return Next::Ask(request.to_string());
    }
    if let Some(wait_ms) = reply["pause"].as_u64() {
        return Next::Pause(wait_ms);
    }
    let status = reply["ran"]
        .as_i64()
        .unwrap_or_else(|| panic!("{name}: a round ends with the status of its run: {reply}"));
    let outcome = reply["outcome"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: a round ends with its outcome: {reply}"));
    Next::Ran(Ran::of_text(status, outcome))
}

/// One thing due on the network.
enum Due {
    /// A request reaches the relay.
    AtRelay {
        device: usize,
        flight: u64,
        request: String,
        sent_ms: i64,
    },
    /// What a device's transport gives back for the request of `flight`.
    Back {
        device: usize,
        flight: u64,
        carried: Carried,
    },
    /// The wait of `flight` a device's run asked for is over.
    Resume { device: usize, flight: u64 },
}

struct Pending {
    at_ms: i64,
    order: u64,
    due: Due,
}

/// What a set of exchanges run at once has on the network, and what they came to.
#[derive(Default)]
struct Flights {
    queue: Vec<Pending>,
    /// What orders two things due at one time, and names each request that goes out.
    order: u64,
    /// The flight each device waits on, and how many requests of its exchange went out.
    waiting: BTreeMap<usize, (u64, usize)>,
    ran: BTreeMap<&'static str, Ran>,
    /// The latest time a request that reached the relay was sent at.
    latest_sent: Option<i64>,
}

impl Flights {
    fn push(&mut self, at_ms: i64, due: Due) {
        self.order += 1;
        self.queue.push(Pending {
            at_ms,
            order: self.order,
            due,
        });
    }

    /// The next thing due, the earliest first and, at one time, the first put.
    fn take_earliest(&mut self) -> Option<Pending> {
        let at = self
            .queue
            .iter()
            .enumerate()
            .min_by_key(|(_, pending)| (pending.at_ms, pending.order))
            .map(|(at, _)| at)?;
        Some(self.queue.swap_remove(at))
    }

    /// Whether the device waits on the request of `flight`.
    fn awaits(&self, device: usize, flight: u64) -> bool {
        self.waiting.get(&device).map(|(waited, _)| *waited) == Some(flight)
    }
}

/// The devices of one list, the relay, and the network between them.
pub struct Devices {
    pub world: World,
    relay: Relay<MemoryStore>,
    authority: Authority,
    rng: Rng,
    devices: Vec<Device>,
    pub dishonesty: Dishonesty,
    /// What every device wrote to each field, by (row, field).
    written: BTreeMap<(u64, u64), Vec<i64>>,
    /// How many requests reached the relay after one sent later than they were.
    pub reordered: usize,
    /// How many exchanges the network failed.
    pub failed_exchanges: usize,
    /// How many rounds the relay refused for a device's allowance.
    pub limited: usize,
    /// How many times each device's run waited on the world's clock for the relay's allowance and
    /// asked again (a Kotlin device's; one the world drives by hand ends its round instead).
    pub paused: BTreeMap<&'static str, usize>,
    /// How long, on the world's clock, each device's run waited in all.
    pub paused_ms: BTreeMap<&'static str, u64>,
    /// How many writes the relay made up and gave each node.
    pub forged: BTreeMap<u64, usize>,
}

impl Devices {
    /// Alice's device (Rust, node 1, holding the administrator's key) opens the list; bob's
    /// (node 2) and carol's (node 3) are invited with the hash of its genesis. Bob's device is
    /// Kotlin when `bob` is a step server, Rust otherwise; carol's is Rust. Under
    /// [`Keying::Clear`] alice grants both writing here; under [`Keying::Group`] every device
    /// holds a device of the group, and they are admitted by
    /// [`Self::admit_through_the_group`]. The relay keeps `limits`, and every path is the
    /// default one.
    ///
    /// The order is a host's: alice's device gives the relay the list it opened before the
    /// others are told of it, since an invitation is sent once the list is there to be read
    /// ([`Self::open_at_the_relay`]); and under [`Keying::Clear`] every invited device reads the
    /// list, its grant with it, before its person writes, since a core signs nothing for a
    /// device the log it holds does not let write.
    pub fn new(seed: u64, bob: Option<&ExternalMachine>, keying: Keying, limits: Limits) -> Self {
        let grouped = keying == Keying::Group;
        let authority = Authority::new([3; 32], "the-relay");
        let relay = Relay::open(MemoryStore::new(), authority.clone(), limits)
            .expect("a relay in memory opens");
        let mut alice = Peer::open(LIST);
        if grouped {
            alice = alice.with_group(GroupDevice::new(LIST, ALICE, key_of(ALICE)), ALICE);
        }
        alice.set_identity(ALICE, key_of(ALICE));
        alice.set_administrator(administrator());
        let genesis = alice
            .open_list(Profile::Untrusted)
            .expect("alice's device opens the list");
        if !grouped {
            for node in [BOB, CAROL] {
                alice
                    .grant(node, WRITER, key_of(node).public())
                    .expect("alice's device grants writing");
            }
        }
        let invited = |node: u64| {
            let mut peer = Peer::invited(LIST, genesis);
            if grouped {
                peer = peer.with_group(GroupDevice::new(LIST, node, key_of(node)), node);
            }
            peer.set_identity(node, key_of(node));
            Core::Rust(Box::new(peer))
        };
        let bob_core = match bob {
            Some(machine) => {
                let made = json!({
                    "op": "peer_make",
                    "library": peer_library().display().to_string(),
                    "list": LIST,
                    "node": BOB,
                    "seed": hex(&[BOB as u8; 32]),
                    "genesis": hex(&genesis),
                    "administrator": null,
                    "group": grouped,
                });
                ask("bob", machine, &made);
                Core::Kotlin(machine.clone())
            }
            None => invited(BOB),
        };
        let cores = [
            ("alice", ALICE, Core::Rust(Box::new(alice))),
            ("bob", BOB, bob_core),
            ("carol", CAROL, invited(CAROL)),
        ];

        let mut world = World::new(seed);
        let mut devices = Vec::new();
        for (name, node, core) in cores {
            match &core {
                Core::Rust(_) => world.add(name, RecordList::new(), record_copy(name)),
                Core::Kotlin(machine) => world.add_external(name, machine, record_copy(name)),
            }
            let sent = Sends::default();
            world.set_sealer(name, Box::new(ToTheCore(sent.clone())));
            // Joined with `journal` false (docs/adr/0083, docs/adr/0093): this host gives its
            // core what the machine sends to `#people` (`Peer::outbound`), which a copy that
            // keeps a journal does not send; a host that gives its core each offered change
            // and its copy the verdict comes with the core's `Peer::made`.
            world.raise(
                name,
                "record_list.join",
                &format!(r#"{{"node":"{node}","journal":false}}"#),
            );
            devices.push(Device {
                name,
                node,
                core,
                sent,
                path: Path::default(),
                kept: Vec::new(),
                refused: Vec::new(),
                steps: Vec::new(),
                suspend_after: None,
            });
        }
        let mut devices = Self {
            world,
            relay,
            authority,
            // Not the world's own sequence: the network to the relay draws apart from it.
            rng: Rng::new(!seed),
            devices,
            dishonesty: Dishonesty::default(),
            written: BTreeMap::new(),
            reordered: 0,
            failed_exchanges: 0,
            limited: 0,
            paused: BTreeMap::new(),
            paused_ms: BTreeMap::new(),
            forged: BTreeMap::new(),
        };
        devices.quiet();
        devices.open_at_the_relay();
        if !grouped {
            devices.settle();
        }
        devices
    }

    /// Alice's device gives the relay the list it opened: rounds of hers alone, each after the
    /// wait the relay asks for when it refuses one for her allowance, until one ends. Until
    /// then the relay holds no grant of an invited device, and refuses it anything it reads.
    ///
    /// # Panics
    ///
    /// When a round of hers fails for anything but her allowance, or none ends in [`PASSES`].
    fn open_at_the_relay(&mut self) {
        for _ in 0..PASSES {
            let ran = self.rounds(&["alice"], Begin::Round(RoundKind::Sync));
            let came = &ran["alice"];
            match (&came.failed, came.retry_after_ms) {
                (None, _) => return,
                (Some(_), Some(after)) => self
                    .world
                    .advance_ms(i64::try_from(after).unwrap_or(i64::MAX / 2) + BETWEEN_PASSES_MS),
                (Some(failure), None) => {
                    panic!("alice's device did not give the relay the list: {failure:?}")
                }
            }
        }
        panic!("alice's device did not give the relay the list in {PASSES} rounds");
    }

    /// The names of the devices, in the order of their nodes.
    pub fn names(&self) -> Vec<&'static str> {
        self.devices.iter().map(|device| device.name).collect()
    }

    /// Whether the device `name` reaches its core through the C ABI.
    pub fn is_kotlin(&self, name: &str) -> bool {
        matches!(self.device(name).core, Core::Kotlin(_))
    }

    fn index(&self, name: &str) -> usize {
        self.devices
            .iter()
            .position(|device| device.name == name)
            .unwrap_or_else(|| panic!("no device is named {name}"))
    }

    fn device(&self, name: &str) -> &Device {
        &self.devices[self.index(name)]
    }

    /// The device `name`, for what it kept and refused.
    pub fn device_mut(&mut self, name: &str) -> &mut Device {
        let at = self.index(name);
        &mut self.devices[at]
    }

    /// The path between the device `name` and the relay is `path` from now on.
    pub fn set_path(&mut self, name: &str, path: Path) {
        self.device_mut(name).path = path;
    }

    /// Every path the default one again: a network that loses and delays nothing past a
    /// deadline.
    pub fn heal(&mut self) {
        for device in &mut self.devices {
            device.path = Path::default();
        }
    }

    /// The next round of the device `name` is suspended once `requests` of it went out: its
    /// job's time ends with the last of them on its way, which still reaches the relay.
    pub fn suspend(&mut self, name: &str, requests: usize) {
        self.device_mut(name).suspend_after = Some(requests);
    }

    /// The person of the device `name` changes a field `DUE` of row `row` to `value`, now.
    pub fn set(&mut self, name: &str, row: u64, value: i64) {
        let now = self.world.now_ms();
        record_edit_at(&mut self.world, name, row, DUE, value, "", now);
        self.written.entry((row, DUE)).or_default().push(value);
    }

    /// The person of the device `name` gives row `row` a title, now.
    pub fn title(&mut self, name: &str, row: u64, title: &str) {
        let now = self.world.now_ms();
        record_edit_at(&mut self.world, name, row, TITLE, 0, title, now);
        self.written.entry((row, TITLE)).or_default().push(0);
    }

    /// The machines run until nothing is left to happen, and what they sent is the cores'.
    fn quiet(&mut self) {
        assert_eq!(self.world.run_until_quiet(600_000), Rest::Quiet);
        let errors = self.world.host_errors();
        assert!(
            errors.iter().all(|error| error.contains(GIVEN)),
            "a host met something it could not act on: {errors:?}"
        );
    }

    /// What every machine sent since it was last asked, given to its device's core.
    pub fn give_sends(&mut self) {
        self.quiet();
        for device in &mut self.devices {
            let sent = std::mem::take(&mut *device.sent.lock().expect("the sends are whole"));
            for (event, data) in sent {
                let kept = device.outbound(&event, &data);
                device.kept.push(kept);
            }
        }
    }

    fn token(&self, node: u64) -> String {
        self.authority.issue(LIST, node, u64::MAX)
    }

    fn advance_to(&mut self, at_ms: i64) {
        let delta = at_ms - self.world.now_ms();
        if delta > 0 {
            self.world.advance_ms(delta);
        }
    }

    /// An exchange of `begin` of each device of `names`, all run at once over the network, so
    /// that their requests reach the relay in the order the network brings them; what each
    /// brought is raised into its machine as its run ends. What each came to, by name.
    pub fn rounds(&mut self, names: &[&str], begin: Begin) -> BTreeMap<&'static str, Ran> {
        let mut flights = Flights::default();
        for name in names {
            let device = self.index(name);
            let next = self.devices[device].begin(begin);
            self.go_on(device, next, &mut flights);
        }
        while let Some(Pending { at_ms, due, .. }) = flights.take_earliest() {
            self.advance_to(at_ms);
            match due {
                Due::AtRelay {
                    device,
                    flight,
                    request,
                    sent_ms,
                } => {
                    if flights.latest_sent.is_some_and(|latest| sent_ms < latest) {
                        self.reordered += 1;
                    }
                    flights.latest_sent = Some(
                        flights
                            .latest_sent
                            .map_or(sent_ms, |latest| latest.max(sent_ms)),
                    );
                    let node = self.devices[device].node;
                    let now = u64::try_from(self.world.now_ms()).unwrap_or(0);
                    let token = self.token(node);
                    let answered = self.relay.handle(&token, &request, now);
                    let response = self.dishonest(node, answered);
                    let path = self.devices[device].path;
                    // A response lost on its way never comes, and the device's deadline,
                    // which went out with the request, passes; one that comes after the
                    // deadline finds the device given up.
                    if self.rng.below(100) >= path.loss_pct {
                        flights.push(
                            at_ms + path.delay(&mut self.rng),
                            Due::Back {
                                device,
                                flight,
                                carried: Carried::Response(response),
                            },
                        );
                    }
                }
                Due::Back {
                    device,
                    flight,
                    carried,
                } => {
                    // An answer the device gave up waiting for, or that came after its
                    // job's time ended, reaches no one.
                    if !flights.awaits(device, flight) {
                        continue;
                    }
                    if matches!(carried, Carried::Failed(_)) {
                        self.failed_exchanges += 1;
                    }
                    let next = self.devices[device].carry(carried);
                    self.go_on(device, next, &mut flights);
                }
                Due::Resume { device, flight } => {
                    if !flights.awaits(device, flight) {
                        continue;
                    }
                    let next = self.devices[device].resume();
                    self.go_on(device, next, &mut flights);
                }
            }
        }
        for name in names {
            assert!(
                flights.ran.contains_key(*name),
                "{name}'s exchange never came to an end"
            );
        }
        self.quiet();
        flights.ran
    }

    /// What a device's exchange does next: its request is put on its path to the relay, or what
    /// it came to is kept and told to its machine.
    fn go_on(&mut self, device: usize, next: Next, flights: &mut Flights) {
        let name = self.devices[device].name;
        let request = match next {
            Next::Ran(came) => {
                flights.waiting.remove(&device);
                if came.retry_after_ms.is_some() {
                    self.limited += 1;
                }
                for (event, data) in &came.events {
                    self.world.raise(name, event, data);
                }
                let held = &mut self.devices[device];
                held.refused.extend(came.refused.iter().cloned());
                held.steps.extend(came.steps.iter().cloned());
                flights.ran.insert(name, came);
                return;
            }
            Next::Pause(wait_ms) => {
                // The run waits on the world's clock: nothing is on the network meanwhile, and the
                // run goes on once the world's time reaches the end of the wait.
                *self.paused.entry(name).or_default() += 1;
                *self.paused_ms.entry(name).or_default() += wait_ms;
                flights.order += 1;
                let flight = flights.order;
                let asked = flights.waiting.get(&device).map_or(0, |(_, asked)| *asked);
                flights.waiting.insert(device, (flight, asked));
                let until = self
                    .world
                    .now_ms()
                    .saturating_add(i64::try_from(wait_ms).unwrap_or(i64::MAX / 2));
                flights.push(until, Due::Resume { device, flight });
                return;
            }
            Next::Ask(request) => request,
        };
        let now = self.world.now_ms();
        flights.order += 1;
        let flight = flights.order;
        let asked = flights.waiting.get(&device).map_or(0, |(_, asked)| *asked) + 1;
        flights.waiting.insert(device, (flight, asked));
        let path = self.devices[device].path;
        // The device's transport waits this long for the answer, and gives up after.
        flights.push(
            now + path.timeout_ms,
            Due::Back {
                device,
                flight,
                carried: Carried::Failed(TransportError::Deadline),
            },
        );
        if self.devices[device].suspend_after == Some(asked) {
            // The job's time ends with this request on its way: the run is cancelled now,
            // and the request still reaches the relay.
            self.devices[device].suspend_after = None;
            flights.push(
                now,
                Due::Back {
                    device,
                    flight,
                    carried: Carried::Failed(TransportError::Cancelled),
                },
            );
        } else if self.rng.below(100) < path.loss_pct {
            // Lost on its way: the connection breaks before the relay has it.
            flights.push(
                now + path.latency_ms,
                Due::Back {
                    device,
                    flight,
                    carried: Carried::Failed(TransportError::Link(
                        "the connection broke before the relay had the request".to_string(),
                    )),
                },
            );
            return;
        }
        flights.push(
            now + path.delay(&mut self.rng),
            Due::AtRelay {
                device,
                flight,
                request,
                sent_ms: now,
            },
        );
    }

    /// The relay's response to the device of `node`, as a dishonest relay gives it.
    fn dishonest(&mut self, node: u64, response: String) -> String {
        let withheld: Vec<u64> = self
            .dishonesty
            .withhold
            .iter()
            .filter(|(toward, _)| *toward == node)
            .map(|(_, row)| *row)
            .collect();
        let forge = self.dishonesty.forge_toward.contains(&node);
        if withheld.is_empty() && !forge {
            return response;
        }
        let kept = |data: &str| !row_of(data).is_some_and(|row| withheld.contains(&row));
        match Response::from_json(&response) {
            Ok(Response::Feed {
                records,
                next,
                more,
            }) => {
                let mut given = Vec::new();
                for record in records {
                    if let Record::Operation { event, data } = &record {
                        if !kept(data) {
                            continue;
                        }
                        if forge && event == CHANGE {
                            let made_up = Record::Operation {
                                event: event.clone(),
                                data: moved_to(data, FORGED_ROW),
                            };
                            given.push(record);
                            given.push(made_up);
                            *self.forged.entry(node).or_default() += 1;
                            continue;
                        }
                    }
                    given.push(record);
                }
                Response::Feed {
                    records: given,
                    next,
                    more,
                }
                .to_json()
            }
            Ok(Response::Operations { operations }) => Response::Operations {
                operations: operations
                    .into_iter()
                    .filter(|(_, data)| kept(data))
                    .collect(),
            }
            .to_json(),
            _ => response,
        }
    }

    /// The administrator's core: alice's.
    pub fn administrator(&mut self) -> &mut Peer {
        self.device_mut("alice").rust()
    }

    /// Bob and carol admitted to the list and to its group, each from the key package its
    /// device left with the relay (which holds the list since [`Self::new`]), by alice's device
    /// in one act each, each act given to the relay by a round of hers before the next, and
    /// every device brought to rest: what an administrator's device does to admit a device to a
    /// list keyed by its group. Bob joins at the epoch that admits him and applies the one that
    /// admits carol; carol joins at that one.
    pub fn admit_through_the_group(&mut self) {
        let left = self.rounds(&["bob", "carol"], Begin::KeyPackage);
        for (name, ran) in &left {
            assert_eq!(ran.failed, None, "{name} left a key package");
        }
        let read = self.rounds(&["alice"], Begin::Round(RoundKind::Sync));
        assert_eq!(read["alice"].failed, None, "alice read the key packages");
        for node in [BOB, CAROL] {
            self.administrator()
                .add_member(node, WRITER, key_of(node).public())
                .unwrap_or_else(|error| panic!("alice's device admits node {node}: {error:?}"));
            let given = self.rounds(&["alice"], Begin::Round(RoundKind::Sync));
            assert_eq!(
                given["alice"].failed, None,
                "the act on node {node} reached the relay"
            );
        }
        self.settle();
    }

    /// The epochs whose keys the device `name` holds.
    pub fn keys(&mut self, name: &str) -> Vec<u64> {
        self.device_mut(name).keys()
    }

    /// Passes of a round of every device, all run at once, until a pass in which none failed
    /// and none moved anything; between passes the world's time moves on, past any wait the
    /// relay asked for.
    ///
    /// # Panics
    ///
    /// When the devices do not come to rest in [`PASSES`] passes.
    pub fn settle(&mut self) {
        for _ in 0..PASSES {
            if !self.pass() {
                return;
            }
        }
        panic!("the devices did not come to rest in {PASSES} passes");
    }

    /// One pass: what the machines sent given to the cores, a round of every device, and the
    /// time to the next. Whether it moved anything.
    pub fn pass(&mut self) -> bool {
        self.give_sends();
        let names = self.names();
        let ran = self.rounds(&names, Begin::Round(RoundKind::Sync));
        let mut wait = BETWEEN_PASSES_MS;
        let mut moved = false;
        for (name, came) in &ran {
            assert!(
                !came.rebuild,
                "{name}'s machine was told to be built again: {came:?}"
            );
            moved |= came.moved();
            if let Some(after) = came.retry_after_ms {
                wait = wait.max(i64::try_from(after).unwrap_or(i64::MAX / 2) + BETWEEN_PASSES_MS);
            }
        }
        self.world.advance_ms(wait);
        self.quiet();
        moved
    }

    /// The fields the device `name`'s copy holds, by (row, field).
    pub fn records(&mut self, name: &str) -> BTreeMap<(u64, u64), Field> {
        records_of(&mut self.world, name)
    }

    /// Every copy holds one list, with one set of live writes; which. The list keeps its effect
    /// and nothing of its delivery (docs/adr/0085, section 2): what two copies hold alike is their
    /// values and the tags of their live writes, whatever order the changes reached each in.
    pub fn agreed(&mut self) -> BTreeMap<(u64, u64), Field> {
        let names = self.names();
        let first = records_of(&mut self.world, names[0]);
        let adds = live_writes(&mut self.world, names[0]);
        for name in &names[1..] {
            assert_eq!(
                records_of(&mut self.world, name),
                first,
                "{} and {name} hold different lists",
                names[0]
            );
            assert_eq!(
                live_writes(&mut self.world, name),
                adds,
                "{} and {name} hold different live writes",
                names[0]
            );
        }
        first
    }

    /// Every copy holds one list, and it is what the people wrote: a field for each field
    /// written, and in each only values written to it.
    pub fn assert_holds_what_was_written(&mut self) {
        let list = self.agreed();
        let held: BTreeSet<(u64, u64)> = list.keys().copied().collect();
        let wrote: BTreeSet<(u64, u64)> = self.written.keys().copied().collect();
        assert_eq!(held, wrote, "the fields held are the fields written");
        for (key, field) in &list {
            for live in &field.live {
                assert!(
                    self.written[key].contains(&live.value),
                    "{key:?} holds {} and nobody wrote it: {:?}",
                    live.value,
                    self.written[key]
                );
            }
        }
    }

    /// Every refusal any device's exchanges came to, by device.
    pub fn refused(&self) -> BTreeMap<&'static str, Vec<String>> {
        self.devices
            .iter()
            .filter(|device| !device.refused.is_empty())
            .map(|device| (device.name, device.refused.clone()))
            .collect()
    }

    /// What the relay holds of the list, as it gives it to alice.
    pub fn feed(&mut self) -> String {
        let now = u64::try_from(self.world.now_ms()).unwrap_or(0);
        let request = Request::Feed {
            cursor: 0,
            limit: 500,
        }
        .to_json();
        let token = self.token(ALICE);
        self.relay.handle(&token, &request, now)
    }
}

/// The tags of the live writes the copy `name` holds (`adds`), in one order: what two copies that
/// took the same changes in, in any order, hold alike.
fn live_writes(world: &mut World, name: &str) -> Vec<String> {
    let mut adds: Vec<String> = record_variables_of(world, name)["adds"]
        .as_array()
        .expect("a copy's live writes are a list")
        .iter()
        .map(Value::to_string)
        .collect();
    adds.sort();
    adds
}

/// The row a change's first operation names, when the data is a change. The devices here make
/// changes of one write each, so a change names one row.
fn row_of(data: &str) -> Option<u64> {
    serde_json::from_str::<Value>(data)
        .ok()
        .and_then(|value| value["ops"][0]["id"].as_u64())
}

/// The change's data with the row of its first operation changed to `row`, and everything else,
/// its signature among it, as it was.
fn moved_to(data: &str, row: u64) -> String {
    let mut value: Value =
        serde_json::from_str(data).expect("an operation the relay holds is JSON");
    value["ops"][0]["id"] = json!(row);
    value.to_string()
}
