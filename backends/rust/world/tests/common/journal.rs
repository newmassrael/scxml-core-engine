// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host of a copy of the standard shared list that keeps a journal, on a copy with no core
// (docs/adr/0093, sections 1 and 3; docs/adr/0085, sections 3.2, 4.3 and 4.4).
//
// A copy joined with `journal` takes a change of its own in only once its layer has answered
// for it. It offers the change to its host on the route `#journal` as `record_list.made`; the
// host gives it to the copy's Mesh core on `#people`, which tags it and answers with a verdict;
// the host commits the change with its dot to the journal, tells the core the record is
// committed, which releases the change to the group, and gives the copy the verdict back, all
// in the one call. The copy then reports the outcome, `record_list.closed`, which the host
// journals too. A host stands in front of the copy's sends for that ([`World::set_host`]).
//
// The journal holds the copy's own changes only (docs/adr/0093, section 1.1); the other
// copies' changes are the layer's, which keeps them until the stable cut. The Mesh core gives
// no read of its log, so the host reads them as the layer delivers them to the copy, through
// an admission that admits everything, and holds them until it folds, as the log holds them
// until the cut. At a fold the host takes the copy's state as the snapshot (the live values
// with their dots and the clock) and drops what it held before; a fold is made where every
// layer is at rest, so the stable cut covers every change the copy took in and the copy's
// state is the state at the cut (docs/adr/0085, section 4.3). A copy is built again from
// what was made, never from the commands: the snapshot as `record_list.restore`, then every
// change after it as `record_list.recall`, its own and the others', in the order the copy
// took them in, which is a causal order (docs/adr/0085, section 4.4).
//
// The same host serves a copy on a route with no causal layer, the run against a rule with
// no causal layer that docs/adr/0085 section 9 asks for: nothing answers an offer there, so
// the host accepts every change itself, as a host with a layer that accepts everything does
// (docs/adr/0093, section 3), and the change has left before the host journals it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::causal::{Dot, Refusal, Start};
use sce_rust_mesh::endpoint::{Admission, CausalVerdict};
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_runtime::{Engine, HostSendRequest, HostSendResponse};
use sce_rust_world::{Fate, Host, MeshEndpoint, Outcome, World};
use serde_json::{json, Value};

use super::{
    assert_layers_rest, at_rest, group_world_of, links_of, record_copy, record_node_of,
    record_variables_of, records_of, replica_of, whole, Field, RecordList, COPIES, LAYER_ROOM,
    PEOPLE, TRIO, VECTOR_EVERY_MS,
};

/// The route a copy that keeps a journal offers its changes and reports its outcomes on.
pub const JOURNAL: &str = "#journal";

/// How far apart every two copies are, in the world's milliseconds.
pub const LATENCY_MS: i64 = 20;

/// The most operations one `record_list.restore` carries: `shared_parts` as the list is
/// built, so a stable state of more values is given in several.
pub const PARTS: usize = 64;

/// A change of the copy's own as the host journalled it: the change as it went to the layer,
/// the host's name for the command that ended it, the dot the layer tagged it with (none on a
/// route with no causal layer), and its place in the order the copy took changes in.
#[derive(Debug, Clone)]
pub struct Journalled {
    pub change: Value,
    pub command: Value,
    pub dot: Option<Dot>,
    pub at: u64,
}

/// Another copy's change as the layer delivered it, and its place in the same order.
#[derive(Debug, Clone)]
pub struct Delivered {
    pub change: Value,
    pub at: u64,
}

/// The state a host folded its journal into: the live values the copy held, each with its
/// own dot, and its clock.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub values: Vec<Value>,
    pub wall_time: Value,
    pub counter: Value,
}

/// What one copy's host keeps.
#[derive(Debug, Default)]
pub struct Book {
    /// The copy's own accepted changes since the last fold.
    pub journal: Vec<Journalled>,
    /// The other copies' changes the layer delivered since the last fold.
    pub history: Vec<Delivered>,
    /// The snapshot of the last fold, when there was one.
    pub snapshot: Option<Snapshot>,
    /// Every outcome the copy reported, in order (`record_list.closed`).
    pub closed: Vec<Value>,
    /// How many offers the layer accepted.
    pub accepted: usize,
    /// Every refusal the layer gave an offer.
    pub refused: Vec<Refusal>,
    /// The stamp of every change the layer delivered to the copy, as `(by, wallTime,
    /// counter)`, kept across folds: what was given, and how often.
    pub given: Vec<(i128, i128, i128)>,
    /// The next place in the order the copy took changes in.
    next: u64,
}

impl Book {
    fn place(&mut self) -> u64 {
        let at = self.next;
        self.next += 1;
        at
    }
}

/// A book as the host, the admission and a test share it.
pub type SharedBook = Arc<Mutex<Book>>;

pub fn lock(book: &SharedBook) -> MutexGuard<'_, Book> {
    book.lock().expect("a host's book is never left mid-update")
}

/// The stamp a change states, `(by, wallTime, counter)`: the dot of every write in it.
pub fn stamp_of(change: &Value) -> (i128, i128, i128) {
    (
        whole(&change["by"]),
        whole(&change["wallTime"]),
        whole(&change["counter"]),
    )
}

/// The host in front of a copy's sends: it serves `#journal` and hands every other send on.
pub struct JournalHost {
    book: SharedBook,
    /// Whether the copy's route is causal, so that an offer is answered by its layer.
    causal: bool,
}

impl JournalHost {
    /// Offer the change `report` names to the layer, journal it by the verdict and give the
    /// copy the verdict (docs/adr/0093, section 3.1).
    fn offer(
        &self,
        made: &HostSendRequest,
        report: Value,
        endpoint: &mut MeshEndpoint,
    ) -> Vec<HostSendResponse> {
        let mut change = report.clone();
        let command = change
            .as_object_mut()
            .and_then(|fields| fields.remove("command"))
            .expect("a report names the command that ended its change");
        endpoint.send(&HostSendRequest {
            processor_type: made.processor_type.clone(),
            target: format!("#{PEOPLE}"),
            event_name: "record_list.change".to_string(),
            event_data: change.to_string(),
            ..HostSendRequest::default()
        });
        let verdicts = endpoint.take_verdicts();
        let (outcome, reason) = match (self.causal, verdicts.as_slice()) {
            (false, []) => {
                self.journal(change, command, None);
                ("accepted", Value::Null)
            }
            (true, [CausalVerdict::Accepted { peer, dot }]) => {
                // Write-ahead: the record is committed before the layer lets the change go.
                self.journal(change, command, Some(*dot));
                endpoint.committed(peer, *dot);
                ("accepted", Value::Null)
            }
            (true, [CausalVerdict::Refused { refusal, .. }]) => {
                lock(&self.book).refused.push(*refusal);
                let reason = match refusal {
                    Refusal::NoRoomForHistory => json!("noRoomForHistory"),
                    Refusal::NumbersExhausted => Value::Null,
                };
                ("refused", reason)
            }
            (causal, answered) => {
                panic!("an offer on a route causal {causal} was answered {answered:?}")
            }
        };
        let verdict = json!({
            "wallTime": report["wallTime"],
            "counter": report["counter"],
            "outcome": outcome,
            "reason": reason,
        });
        vec![HostSendResponse {
            event_name: "record_list.verdict".to_string(),
            event_data: verdict.to_string(),
            ..HostSendResponse::default()
        }]
    }

    fn journal(&self, change: Value, command: Value, dot: Option<Dot>) {
        let mut book = lock(&self.book);
        let at = book.place();
        book.accepted += 1;
        book.journal.push(Journalled {
            change,
            command,
            dot,
            at,
        });
    }
}

impl Host for JournalHost {
    fn send(
        &mut self,
        request: HostSendRequest,
        endpoint: &mut MeshEndpoint,
    ) -> Vec<HostSendResponse> {
        if request.target != JOURNAL {
            endpoint.send(&request);
            return Vec::new();
        }
        let report: Value = serde_json::from_str(&request.event_data)
            .expect("what a copy reports to its journal is JSON");
        match request.event_name.as_str() {
            "record_list.made" => self.offer(&request, report, endpoint),
            "record_list.closed" => {
                lock(&self.book).closed.push(report);
                Vec::new()
            }
            other => panic!("a copy reports no {other} to its journal"),
        }
    }
}

/// What the layer delivers to the copy, read by its host as it is delivered: every change
/// is admitted as it came, and kept in the book's history.
pub struct HistoryReader {
    book: SharedBook,
}

impl Admission for HistoryReader {
    fn admit(&mut self, _peer: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        if event != "record_list.change" {
            return Ok(None);
        }
        let change: Value = serde_json::from_str(data)
            .map_err(|error| format!("a change that is not JSON: {error}"))?;
        let mut book = lock(&self.book);
        let at = book.place();
        book.given.push(stamp_of(&change));
        book.history.push(Delivered { change, at });
        Ok(None)
    }
}

/// The route a world's copies are bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// `delivery: causal`, each layer with room for `room` changes not yet stable.
    Causal { room: usize },
    /// A plain binding, with no causal layer below the list.
    Plain,
}

/// A change one copy put on the network: who sent it, the envelope, and the change.
#[derive(Debug, Clone)]
pub struct Made {
    pub from: String,
    pub bytes: Vec<u8>,
    pub change: Value,
}

/// What a copy holds that a rebuild has to give back: its registers, its adds as
/// `(element, wallTime, counter, nodeId)` in order, and its clock as `(wallTime, counter)`.
/// The clock's node is the copy's own, which a state handed over does not carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub records: BTreeMap<(u64, u64), Field>,
    pub adds: Vec<(i128, i128, i128, i128)>,
    pub clock: (i128, i128),
}

/// Three copies of the standard list, alice (node 1), bob (node 2) and carol (node 3), each
/// joined with `journal` and served by a [`JournalHost`], on `route`.
pub struct Hosted {
    pub world: World,
    pub causal: bool,
    books: Vec<(&'static str, SharedBook)>,
    commands: u64,
}

impl Hosted {
    /// The world, at rest after every copy has joined.
    pub fn new(seed: u64, route: Route) -> Self {
        let mut world = match route {
            Route::Causal { room } => {
                let mut world =
                    group_world_of(seed, LATENCY_MS, &COPIES, TRIO, |world, who, router| {
                        world.add(who, RecordList::new(), router);
                    });
                // Before any copy has joined, each layer is started again with the room
                // this world gives it, as its first incarnation.
                if room != LAYER_ROOM {
                    for who in COPIES {
                        world.start_causal(
                            who,
                            PEOPLE,
                            Start::NewIncarnation(replica_of(who, 0)),
                            room,
                            VECTOR_EVERY_MS,
                        );
                    }
                }
                world
            }
            Route::Plain => {
                let mut world = World::new(seed);
                for who in COPIES {
                    world.add(who, RecordList::new(), record_copy(who));
                }
                for (a, b) in links_of(&COPIES) {
                    world.link(a, b, LATENCY_MS);
                }
                world.group(PEOPLE, &COPIES);
                world
            }
        };
        let causal = matches!(route, Route::Causal { .. });
        let mut books = Vec::new();
        for who in COPIES {
            let book = SharedBook::default();
            world.set_host::<RecordList>(
                who,
                Box::new(JournalHost {
                    book: Arc::clone(&book),
                    causal,
                }),
            );
            world.set_admission(
                who,
                Box::new(HistoryReader {
                    book: Arc::clone(&book),
                }),
            );
            world.raise(
                who,
                "record_list.join",
                &format!(r#"{{"node":"{}","journal":true}}"#, record_node_of(who)),
            );
            books.push((who, book));
        }
        at_rest(&mut world);
        Self {
            world,
            causal,
            books,
            commands: 0,
        }
    }

    /// The book of `who`'s host.
    pub fn book(&self, who: &str) -> SharedBook {
        let (_, book) = self
            .books
            .iter()
            .find(|(name, _)| *name == who)
            .unwrap_or_else(|| panic!("{who} is not a copy of this world"));
        Arc::clone(book)
    }

    fn command(&mut self) -> u64 {
        self.commands += 1;
        self.commands
    }

    /// A person writes `value` to `register`, a command of a change that goes on when `more`.
    pub fn edit(&mut self, who: &str, register: (u64, u64), value: i64, more: bool) {
        let command = self.command();
        let now = self.world.now_ms();
        self.world.raise(
            who,
            "record_list.edit",
            &format!(
                r#"{{"id":{},"field":{},"value":"{value}","text":"","now":"{now}","more":{more},"command":"{command}"}}"#,
                register.0, register.1
            ),
        );
    }

    /// A person empties `register`, a command of a change that goes on when `more`.
    pub fn clear(&mut self, who: &str, register: (u64, u64), more: bool) {
        let command = self.command();
        let now = self.world.now_ms();
        self.world.raise(
            who,
            "record_list.clear",
            &format!(
                r#"{{"id":{},"field":{},"now":"{now}","more":{more},"command":"{command}"}}"#,
                register.0, register.1
            ),
        );
    }

    /// Let a millisecond pass, in which every copy takes what it was given.
    pub fn step(&mut self) {
        self.world.advance_ms(1);
    }

    /// Run until nothing is left to happen; on a causal route every layer is then at rest.
    pub fn quiet(&mut self, context: &str) {
        assert_eq!(
            self.world.run_until_quiet(600_000),
            Outcome::Quiet,
            "{context}: the world comes to rest"
        );
        if self.causal {
            assert_layers_rest(&mut self.world, &COPIES, context);
        }
    }

    /// Every link loses everything it carries, both ways, or carries again (`held` false):
    /// while it does, a change reaches a copy only when it is given to it by hand.
    pub fn hold(&mut self, held: bool) {
        for (a, b) in links_of(&COPIES) {
            self.world.degrade(a, b, if held { 100 } else { 0 }, 0);
        }
    }

    /// `who` makes one change by `commands`, which is taken in at its verdict and put on
    /// the network; the change as the network carried it.
    pub fn make(&mut self, who: &str, commands: impl FnOnce(&mut Self)) -> Made {
        let mark = self.world.trace().len();
        commands(self);
        self.step();
        self.sent_since(mark, who)
    }

    fn sent_since(&self, mark: usize, who: &str) -> Made {
        for line in self.world.trace().into_iter().skip(mark) {
            if line.from != who {
                continue;
            }
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&line.bytes)) else {
                continue;
            };
            if envelope.pattern == PatternKind::CausalVector
                || envelope.event_type != "record_list.change"
            {
                continue;
            }
            let change: Value = serde_json::from_slice(envelope.data.into_inner())
                .expect("a change's data is JSON");
            return Made {
                from: who.to_string(),
                bytes: line.bytes.clone(),
                change,
            };
        }
        panic!("{who} put no change on the network");
    }

    /// Give `to` the change `made` as its link would, and let it take it.
    pub fn give(&mut self, made: &Made, to: &str) {
        self.world
            .deliver_again(&made.from, to, PEOPLE, made.bytes.clone());
        self.step();
    }

    /// `who`'s host folds its journal at the stable cut: the copy's state is the snapshot,
    /// and what the host held before it is dropped. Made only where the copies are at rest.
    pub fn fold(&mut self, who: &str) {
        if self.causal {
            assert_layers_rest(&mut self.world, &[who], "a fold is made at the cut");
        }
        let variables = record_variables_of(&mut self.world, who);
        let values = variables["values"]
            .as_array()
            .expect("values are a list")
            .clone();
        assert!(
            !values.is_empty(),
            "{who}: a stable state with no value would be handed over as no restore"
        );
        let snapshot = Snapshot {
            values,
            wall_time: variables["clock"]["wallTime"].clone(),
            counter: variables["clock"]["counter"].clone(),
        };
        let book = self.book(who);
        let mut book = lock(&book);
        book.snapshot = Some(snapshot);
        book.journal.clear();
        book.history.clear();
    }

    /// `who`'s copy is built again from what its host holds: a fresh machine, joined, given
    /// the snapshot of the last fold as `record_list.restore` and every change after it as
    /// `record_list.recall`, in the order the copy took them in. Its endpoint and its layer
    /// are kept: only the machine is new.
    pub fn rebuild(&mut self, who: &str) {
        let (restores, recalls) = {
            let book = self.book(who);
            let book = lock(&book);
            (restores_of(&book), recalls_of(&book))
        };
        self.world.restart::<RecordList>(who, |_, clock| {
            let mut engine = Engine::new(RecordList::new());
            engine.set_clock(clock);
            let Ok(()) = engine.initialize();
            engine
        });
        self.world.raise(
            who,
            "record_list.join",
            &format!(r#"{{"node":"{}","journal":true}}"#, record_node_of(who)),
        );
        for restore in restores {
            self.world.raise(who, "record_list.restore", &restore);
        }
        for recall in recalls {
            self.world.raise(who, "record_list.recall", &recall);
        }
        self.quiet(&format!("{who} built again"));
    }

    /// What `who` holds.
    pub fn held(&mut self, who: &str) -> Held {
        let variables = record_variables_of(&mut self.world, who);
        let mut adds: Vec<(i128, i128, i128, i128)> = variables["adds"]
            .as_array()
            .expect("adds are a list")
            .iter()
            .map(|add| {
                (
                    whole(&add["element"]),
                    whole(&add["wallTime"]),
                    whole(&add["counter"]),
                    whole(&add["nodeId"]),
                )
            })
            .collect();
        adds.sort_unstable();
        Held {
            records: records_of(&mut self.world, who),
            adds,
            clock: (
                whole(&variables["clock"]["wallTime"]),
                whole(&variables["clock"]["counter"]),
            ),
        }
    }
}

/// The snapshot of the last fold as the restores that hand it over, `PARTS` values each:
/// each value a write at its own dot, and the state's clock in every part.
fn restores_of(book: &Book) -> Vec<String> {
    let Some(snapshot) = &book.snapshot else {
        return Vec::new();
    };
    snapshot
        .values
        .chunks(PARTS)
        .map(|part| {
            let ops: Vec<Value> = part
                .iter()
                .map(|value| {
                    let mut op = value.clone();
                    op["kind"] = json!(0);
                    op
                })
                .collect();
            json!({
                "by": "0",
                "wallTime": snapshot.wall_time,
                "counter": snapshot.counter,
                "ops": ops,
            })
            .to_string()
        })
        .collect()
}

/// Every change after the last fold, the copy's own with the command that ended it and
/// the others' with none, in the order the copy took them in.
fn recalls_of(book: &Book) -> Vec<String> {
    let mut taken: Vec<(u64, Value)> = Vec::new();
    for own in &book.journal {
        let mut recall = own.change.clone();
        recall["command"] = own.command.clone();
        recall["provisional"] = json!(false);
        taken.push((own.at, recall));
    }
    for delivered in &book.history {
        let mut recall = delivered.change.clone();
        recall["command"] = json!("0");
        recall["provisional"] = json!(false);
        taken.push((delivered.at, recall));
    }
    taken.sort_by_key(|(at, _)| *at);
    taken
        .into_iter()
        .map(|(_, recall)| recall.to_string())
        .collect()
}

/// Every change of the list put on the network, once each, whatever became of it: the
/// changes a held link lost and a test then gave by hand are among them. Read off the
/// envelopes the network recorded, not off any copy.
pub fn changes_on_the_network(world: &World) -> Vec<Value> {
    let mut seen: Vec<(i128, i128, i128)> = Vec::new();
    let mut changes = Vec::new();
    for line in world.trace() {
        if !matches!(
            line.fate,
            Fate::Sent { .. } | Fate::Lost | Fate::Duplicated { .. }
        ) {
            continue;
        }
        let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&line.bytes)) else {
            continue;
        };
        if envelope.pattern == PatternKind::CausalVector
            || envelope.event_type != "record_list.change"
        {
            continue;
        }
        let change: Value =
            serde_json::from_slice(envelope.data.into_inner()).expect("a change's data is JSON");
        let stamp = stamp_of(&change);
        if !seen.contains(&stamp) {
            seen.push(stamp);
            changes.push(change);
        }
    }
    changes
}
