// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A server upgraded in place under load loses nothing (docs/adr/0109, D11 and "Conformance";
//! D10's outbox; docs/adr/0112's adapter).
//!
//! The server's sessions run the first version of a generated document
//! (tests/server/documents/server_upgrade_1.scxml), served by the library's adapter of a generated
//! machine; the next build serves the next version (server_upgrade_2.scxml), which renames a
//! state, adds a variable and reads its notes at a later revision of their schema, and declares
//! the migration from the first version's committed shape. The client's binding is durable, so
//! every reply and every reminder it is owed is kept in its outbox in the store.
//!
//! 1. A ledger server (written by hand, of another shape) leaves two sessions in the store.
//! 2. The first build, under load: sessions left at every state (fresh, active, holding a note the
//!    client signed and one it sealed by name, closed, ended), reminders pending, the last poll's
//!    answer lost so its envelopes stay unacknowledged, and requests still arriving when it is
//!    told to stop in order. Nothing is left unsaved.
//! 3. The next build, killed with SIGKILL after its sixth checkpoint while the client's reads load
//!    and migrate the sessions: the store then holds states of both versions' shapes.
//! 4. The next build again. The client polls through the restart with the incarnation and the
//!    acknowledgement it last held and sends the last request of each session again. Then:
//!    * every session migrated: it answers from `serving` (`active` renamed), `fresh` or
//!      `closed`, holds the added variable at the constant the step gives, and is kept under the
//!      next version's shape;
//!    * a session of the ledger's shape is refused, answered `InternalError`, and its checkpoint is
//!      left as it was; a session that ended is gone, and its key starts anew;
//!    * every take the client saw answered is in the session's count, and none twice: the request
//!      sent again is answered from the first answer, with no second effect;
//!    * no envelope is lost: every reply and every reminder a take scheduled is taken in, and none
//!      is taken in twice or under two numbers, the incarnation unchanged across every restart;
//!    * the note signed and the note sealed under the first build verify and open under the next,
//!      at the defaults of the fields its revision added, both as the state kept them and verified
//!      again.
//! 5. The first build started again on the store refuses every session the next one saved, by its
//!    shape, and leaves them as they are: a rollback needs a migration back.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, Instant};

use sce_rust_mesh::checkpoint;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_mesh::generated::rpc_status::RpcStatus;
use sce_rust_runtime::json::{self, Value};
use sce_rust_session_store::{SessionStore, SqliteStore};
use sce_server_suite::draw::seed_for;
use sce_server_suite::ledger;
use sce_server_suite::process::{Reported, ServerProcess};
use sce_server_suite::protocol::{Build, Command, Durable, Halt, Report, ServerOptions, Stage};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::upgrade::{
    key_data, object_data, take_data, Notes, CLOSE, END, NOTE, READ, RECHECK, REMINDED, SEAL, TAKE,
};
use sce_server_suite::wire::{self, Ids, Request, Seen};

/// How many sessions the first build serves: four of each kind.
const SESSIONS: usize = 24;

/// How many takes a session that records any is sent before the stop.
const TAKES: usize = 3;

/// How long the server may take to say it serves, to stop, and to answer.
const PATIENCE: Duration = Duration::from_secs(60);

/// How long the documents' reminders wait after a take, and a margin past it.
const REMINDER_DELAY: Duration = Duration::from_secs(3);

/// The checkpoint of the next build after which it is killed, partway through the upgrade.
const KILLED_AFTER: u64 = 6;

/// What a session of the first build is left as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Asked only to read: it stands in `fresh`.
    Fresh,
    /// Took tags: it stands in `active`, renamed `serving` by the migration.
    Active,
    /// Took tags and verified a note the client signed.
    Signed,
    /// Took tags and opened a note the client sealed.
    Sealed,
    /// Took a tag and closed.
    Closed,
    /// Took a tag, closed and ended: a session that finished.
    Ended,
}

const KINDS: [Kind; 6] = [
    Kind::Fresh,
    Kind::Active,
    Kind::Signed,
    Kind::Sealed,
    Kind::Closed,
    Kind::Ended,
];

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

fn options(scratch: &Scratch, build: Build, halt: Option<Halt>) -> ServerOptions {
    ServerOptions {
        store: scratch.store(),
        owner: "server".to_string(),
        // Longer than the run: a session stays in memory until the stop saves it.
        idle_ms: 600_000,
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt,
        // A window no run fills and a retain no run reaches: what the client is owed is kept
        // until it acknowledges it.
        durable: Some(Durable {
            mailbox: 4_096,
            retain_ms: 600_000,
        }),
        build,
    }
}

fn start(scratch: &Scratch, build: Build, halt: Option<Halt>) -> ServerProcess {
    let process =
        ServerProcess::start(server(), &options(scratch, build, halt)).expect("a server process");
    process
        .await_ready(PATIENCE)
        .unwrap_or_else(|why| panic!("the {} build does not serve: {why}", build.name()));
    process
}

/// A request the client sent, and what it is for.
#[derive(Debug, Clone)]
struct Asked {
    key: String,
    event: &'static str,
    /// The tag of a take.
    tag: String,
    request: Request,
}

/// The durable client: what it sent, what it took in from its outbox, by number, and what it
/// acknowledged.
struct Client {
    ids: Ids,
    epoch: Option<String>,
    ack: u64,
    asked: Vec<Asked>,
    /// Every envelope taken in, in the order taken.
    taken: Vec<Seen>,
    /// The envelope id each outbox number carried.
    numbers: BTreeMap<u64, [u8; 16]>,
    /// Envelopes taken in a second time, and numbers that carried two envelopes.
    twice: Vec<String>,
}

impl Client {
    fn new(seed: u64) -> Client {
        Client {
            ids: Ids::new(seed),
            epoch: None,
            ack: 0,
            asked: Vec::new(),
            taken: Vec::new(),
            numbers: BTreeMap::new(),
            twice: Vec::new(),
        }
    }

    /// Send `event` with `data` for `key` to `server`.
    fn ask(
        &mut self,
        server: &ServerProcess,
        key: &str,
        event: &'static str,
        data: &str,
        tag: &str,
    ) {
        let request = wire::request(&mut self.ids, event, data);
        server.send(&Command::Envelope(request.bytes.clone()));
        self.asked.push(Asked {
            key: key.to_string(),
            event,
            tag: tag.to_string(),
            request,
        });
    }

    /// Send `asked` again, byte for byte: a request the client did not see answered, as a client
    /// whose connection broke sends it.
    fn again(&self, server: &ServerProcess, asked: &Asked) {
        server.send(&Command::Envelope(asked.request.bytes.clone()));
    }

    /// One exchange: acknowledge what it took in, and take in what the outbox holds beyond it.
    /// When `lost`, the answer is lost on its way: nothing is taken in and nothing acknowledged.
    fn poll(&mut self, server: &ServerProcess, lost: bool) {
        server.send(&Command::Poll {
            epoch: self.epoch.clone(),
            ack: self.ack,
        });
        let (epoch, envelopes) = await_report(server, |report| match report {
            Report::Mailbox { epoch, envelopes } => Some((epoch, envelopes)),
            _ => None,
        });
        if lost {
            return;
        }
        match &self.epoch {
            None => self.epoch = Some(epoch),
            Some(held) => assert_eq!(
                held, &epoch,
                "the outbox's incarnation changed: its numbering did not survive"
            ),
        }
        for (number, bytes) in envelopes {
            if number <= self.ack {
                continue;
            }
            let seen = wire::read(bytes).expect("the outbox holds envelopes");
            if let Some(earlier) = self.numbers.insert(number, seen.id) {
                if earlier != seen.id {
                    self.twice
                        .push(format!("number {number} carried two envelopes"));
                }
            }
            if self.taken.iter().any(|taken| taken.id == seen.id) {
                self.twice.push(format!(
                    "{} {} taken in twice",
                    seen.event,
                    wire::hex(&seen.id)
                ));
            }
            self.ack = self.ack.max(number);
            self.taken.push(seen);
        }
    }

    /// The reply taken in to `asked`, if any.
    fn reply(&self, asked: &Asked) -> Option<&Seen> {
        self.taken.iter().find(|seen| {
            seen.pattern == PatternKind::RpcReply && seen.wire_id == Some(asked.request.wire_id)
        })
    }

    /// Poll until every request `asked[from..]` has its reply, at most [`PATIENCE`].
    fn settle(&mut self, server: &ServerProcess, from: usize) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            self.poll(server, false);
            let missing = self.asked[from..]
                .iter()
                .filter(|asked| self.reply(asked).is_none())
                .count();
            if missing == 0 {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "{missing} requests were not answered within {PATIENCE:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Read every key of `keys` and answer what each read answered, once all are answered.
    fn read_all(&mut self, server: &ServerProcess, keys: &[String]) -> BTreeMap<String, Seen> {
        let from = self.asked.len();
        for key in keys {
            self.ask(server, key, READ, &key_data(key), "");
        }
        self.settle(server, from);
        self.asked[from..]
            .iter()
            .map(|asked| {
                (
                    asked.key.clone(),
                    self.reply(asked).expect("answered").clone(),
                )
            })
            .collect()
    }
}

/// The next report of `server` that `pick` takes, waiting at most [`PATIENCE`] for it.
fn await_report<T>(server: &ServerProcess, mut pick: impl FnMut(Report) -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "the server did not report in time");
        if let Some(Reported { report, .. }) = server.report_within(left) {
            if let Some(taken) = pick(report) {
                return taken;
            }
        }
    }
}

/// Stop `server` in order and hold it to having saved every session it held.
fn stop(server: ServerProcess) {
    server.send(&Command::Stop(30_000));
    let not_saved = await_report(&server, |report| match report {
        Report::Stopped { not_saved, .. } => Some(not_saved),
        _ => None,
    });
    server.stop(PATIENCE);
    assert_eq!(not_saved, 0, "the stop left sessions unsaved");
}

/// The data of `seen`, read as JSON.
fn data(seen: &Seen) -> Value {
    json::parse(&seen.data).unwrap_or_else(|_| panic!("{} is no JSON: {}", seen.event, seen.data))
}

fn text(value: &Value, name: &str) -> String {
    match value.member(name) {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("{name} is {other:?} in {value:?}"),
    }
}

fn number(value: &Value, name: &str) -> i64 {
    match value.member(name) {
        Some(Value::Number(digits)) => digits
            .parse()
            .unwrap_or_else(|_| panic!("{name} is {digits}")),
        other => panic!("{name} is {other:?} in {value:?}"),
    }
}

fn flag(value: &Value, name: &str) -> bool {
    match value.member(name) {
        Some(Value::Bool(flag)) => *flag,
        other => panic!("{name} is {other:?} in {value:?}"),
    }
}

/// The shape and the bytes of each checkpoint the store holds of `keys`.
fn stored(scratch: &Scratch, keys: &[String]) -> BTreeMap<String, (String, Vec<u8>)> {
    let store = SqliteStore::open(scratch.store()).expect("the store the servers kept");
    keys.iter()
        .filter_map(|key| {
            let bytes = store.get(key).expect("the store answers")?.state;
            let loaded = checkpoint::load(&store, key)
                .expect("a checkpoint this core reads")
                .expect("the checkpoint just read");
            Some((key.clone(), (loaded.checkpoint.state.shape, bytes)))
        })
        .collect()
}

#[test]
fn a_server_upgraded_in_place_under_load_loses_nothing() {
    let seed = seed_for("a_server_upgraded_in_place_under_load_loses_nothing");
    let scratch = Scratch::new("upgraded-in-place");
    let notes = Notes::read();
    let mut client = Client::new(seed);

    // 1. Sessions of another document's shape: the ledger's.
    let foreign: Vec<String> = (0..2).map(|n| format!("z{n}")).collect();
    let first = start(&scratch, Build::Ledger, None);
    for key in &foreign {
        client.ask(
            &first,
            key,
            ledger::TAKE,
            &ledger::take_data(key, &format!("{key}.ledger"), &[]),
            "",
        );
    }
    client.settle(&first, 0);
    stop(first);

    // 2. The first build, under load.
    let keys: Vec<String> = (0..SESSIONS).map(|n| format!("k{n}")).collect();
    let kind_of = |key: &str| -> Kind {
        let n: usize = key[1..].parse().expect("a numbered key");
        KINDS[n % KINDS.len()]
    };
    let old = start(&scratch, Build::Upgrade1, None);
    let before = client.asked.len();
    for key in &keys {
        match kind_of(key) {
            Kind::Fresh => client.ask(&old, key, READ, &key_data(key), ""),
            kind => {
                let takes = if matches!(kind, Kind::Closed | Kind::Ended) {
                    1
                } else {
                    TAKES
                };
                for t in 0..takes {
                    let tag = format!("{key}.t{t}");
                    client.ask(&old, key, TAKE, &take_data(key, &tag), &tag);
                }
                match kind {
                    Kind::Signed => {
                        client.ask(&old, key, NOTE, &object_data(key, &notes.signed), "")
                    }
                    Kind::Sealed => {
                        client.ask(&old, key, SEAL, &object_data(key, &notes.sealed), "")
                    }
                    Kind::Closed => client.ask(&old, key, CLOSE, &key_data(key), ""),
                    Kind::Ended => {
                        client.ask(&old, key, CLOSE, &key_data(key), "");
                        client.ask(&old, key, END, &key_data(key), "");
                    }
                    _ => {}
                }
            }
        }
    }
    client.settle(&old, before);
    // Writes in flight: a take of every session that records, sent with the stop behind it. Each
    // is taken in and answered before the stop, or refused; either way the next build agrees.
    let in_flight = client.asked.len();
    for key in keys
        .iter()
        .filter(|key| matches!(kind_of(key), Kind::Active | Kind::Signed | Kind::Sealed))
    {
        let tag = format!("{key}.late");
        client.ask(&old, key, TAKE, &take_data(key, &tag), &tag);
    }
    // The answer to this poll is lost: what it held stays unacknowledged in the outbox.
    client.poll(&old, true);
    stop(old);
    let after_first = stored(&scratch, &keys);
    let first_shape = after_first
        .values()
        .next()
        .expect("the first build kept sessions")
        .0
        .clone();
    assert!(
        after_first.values().all(|(shape, _)| *shape == first_shape),
        "the first build kept sessions of two shapes"
    );
    assert!(
        keys.iter()
            .filter(|key| kind_of(key) == Kind::Ended)
            .all(|key| !after_first.contains_key(key)),
        "a session that ended is still kept"
    );
    let foreign_before = stored(&scratch, &foreign);
    assert_eq!(
        foreign_before.len(),
        foreign.len(),
        "the ledger's sessions are kept"
    );

    // 3. The next build, killed partway through the upgrade.
    let killed = start(
        &scratch,
        Build::Upgrade2,
        Some(Halt {
            stage: Stage::CheckpointAfter,
            at: KILLED_AFTER,
        }),
    );
    let live: Vec<String> = keys
        .iter()
        .filter(|key| kind_of(key) != Kind::Ended)
        .cloned()
        .collect();
    for key in &live {
        client.ask(&killed, key, READ, &key_data(key), "");
    }
    await_report(&killed, |report| match report {
        Report::Halted(_) => Some(()),
        _ => None,
    });
    killed.kill();
    let midway = stored(&scratch, &keys);
    let migrated = midway
        .values()
        .filter(|(shape, _)| *shape != first_shape)
        .count();
    assert!(
        migrated > 0 && migrated < midway.len(),
        "the kill did not fall partway through the upgrade: {migrated} of {} sessions migrated",
        midway.len()
    );

    // 4. The next build again, the client polling through the restart and sending the last
    // request of each session again.
    let new = start(&scratch, Build::Upgrade2, None);
    let resent: Vec<Asked> = keys
        .iter()
        .filter_map(|key| {
            client.asked[before..in_flight]
                .iter()
                .rev()
                .find(|asked| &asked.key == key)
                .cloned()
        })
        .collect();
    for asked in &resent {
        client.again(&new, asked);
    }
    for key in &foreign {
        client.ask(&new, key, READ, &key_data(key), "");
    }
    let refused_from = client.asked.len() - foreign.len();
    client.settle(&new, refused_from);
    for asked in &client.asked[refused_from..] {
        let reply = client.reply(asked).expect("answered");
        assert_eq!(
            reply.status,
            Some(RpcStatus::InternalError),
            "a request for {}, a session of the ledger's shape, was answered {reply:?}",
            asked.key
        );
    }
    // Every reminder a take scheduled comes due under the next build.
    std::thread::sleep(REMINDER_DELAY + Duration::from_secs(1));
    let reads = client.read_all(&new, &keys);
    for key in keys
        .iter()
        .filter(|key| matches!(kind_of(key), Kind::Signed | Kind::Sealed))
    {
        if kind_of(key) == Kind::Signed {
            let from = client.asked.len();
            client.ask(&new, key, RECHECK, &key_data(key), "");
            client.settle(&new, from);
        }
    }
    let rechecked = client.read_all(&new, &keys);
    client.poll(&new, false);
    stop(new);

    let after = stored(&scratch, &keys);
    let next_shape = after
        .values()
        .map(|(shape, _)| shape.clone())
        .find(|shape| *shape != first_shape)
        .expect("the next build kept sessions under its shape");
    for (key, (shape, _)) in &after {
        assert_eq!(
            shape, &next_shape,
            "the session {key} was not saved under the next version's shape"
        );
    }
    assert_eq!(
        stored(&scratch, &foreign),
        foreign_before,
        "a refused checkpoint was not left as it was"
    );
    assert!(
        client.twice.is_empty(),
        "envelopes delivered twice: {:?}",
        client.twice
    );

    // What each session of the upgraded document was answered for its takes.
    let mut takes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for asked in client
        .asked
        .iter()
        .filter(|asked| asked.event == TAKE && keys.contains(&asked.key))
    {
        let Some(reply) = client.reply(asked) else {
            continue;
        };
        if !reply.is_ok_reply() {
            continue;
        }
        let answer = data(reply);
        if text(&answer, "state") != "closed" {
            takes
                .entry(asked.key.clone())
                .or_default()
                .push(asked.tag.clone());
        }
    }
    for asked in client.asked.iter().filter(|asked| asked.event == TAKE) {
        let replies: BTreeSet<[u8; 16]> = client
            .taken
            .iter()
            .filter(|seen| seen.wire_id == Some(asked.request.wire_id))
            .map(|seen| seen.id)
            .collect();
        assert!(
            replies.len() <= 1,
            "the take {} of {} was answered by {} envelopes: a second effect",
            asked.tag,
            asked.key,
            replies.len()
        );
    }
    // Every reminder a recorded take scheduled, once.
    let reminded: Vec<String> = client
        .taken
        .iter()
        .filter(|seen| seen.event == REMINDED)
        .map(|seen| text(&data(seen), "tag"))
        .collect();
    let distinct: BTreeSet<&String> = reminded.iter().collect();
    assert_eq!(
        distinct.len(),
        reminded.len(),
        "a reminder was taken in twice: {reminded:?}"
    );
    for (key, tags) in &takes {
        if kind_of(key) == Kind::Ended {
            continue;
        }
        for tag in tags {
            assert!(reminded.contains(tag), "the reminder of {tag} was lost");
        }
    }

    for key in &keys {
        let read = data(&reads[key]);
        let again = data(&rechecked[key]);
        let kind = kind_of(key);
        let state = match kind {
            Kind::Fresh | Kind::Ended => "fresh",
            Kind::Closed => "closed",
            _ => "serving",
        };
        assert_eq!(
            text(&read, "state"),
            state,
            "{key} ({kind:?}) stands elsewhere: {read:?}"
        );
        let expected = if kind == Kind::Ended {
            0
        } else {
            takes.get(key).map_or(0, Vec::len)
        };
        assert_eq!(
            number(&read, "taken"),
            expected as i64,
            "{key} ({kind:?}) holds another count than the takes it answered: {read:?}"
        );
        assert_eq!(
            number(&read, "ceiling"),
            16,
            "{key}: the added variable: {read:?}"
        );
        if kind == Kind::Signed {
            for (when, read) in [("as kept", &read), ("verified again", &again)] {
                assert!(
                    flag(read, "verified"),
                    "{key}: the note {when} does not verify: {read:?}"
                );
                assert_eq!(
                    number(read, "serial"),
                    notes.serial as i64,
                    "{key}: the note {when}: {read:?}"
                );
                assert_eq!(
                    text(read, "owner"),
                    notes.owner,
                    "{key}: the note {when}: {read:?}"
                );
                assert_eq!(
                    number(read, "rank"),
                    -2,
                    "{key}: rank's default {when}: {read:?}"
                );
                assert_eq!(
                    text(read, "label"),
                    "none",
                    "{key}: label's default {when}: {read:?}"
                );
            }
        }
        if kind == Kind::Sealed {
            assert!(
                flag(&read, "opened"),
                "{key}: the sealed note does not open: {read:?}"
            );
            assert_eq!(
                number(&read, "sealed_serial"),
                notes.serial as i64,
                "{key}: {read:?}"
            );
            assert_eq!(
                text(&read, "sealed_label"),
                "none",
                "{key}: label's default: {read:?}"
            );
        }
    }

    // 5. A rollback: the first build refuses what the next one saved, and leaves it.
    let rolled_back = start(&scratch, Build::Upgrade1, None);
    let from = client.asked.len();
    for key in &live {
        client.ask(&rolled_back, key, READ, &key_data(key), "");
    }
    client.settle(&rolled_back, from);
    for asked in &client.asked[from..] {
        let reply = client.reply(asked).expect("answered");
        assert_eq!(
            reply.status,
            Some(RpcStatus::InternalError),
            "the first build served {}, which the next one saved: {reply:?}",
            asked.key
        );
    }
    stop(rolled_back);
    let untouched = stored(&scratch, &live);
    for key in &live {
        assert_eq!(
            untouched.get(key),
            after.get(key),
            "the rollback changed {key}"
        );
    }
}
