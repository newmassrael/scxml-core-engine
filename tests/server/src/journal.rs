// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The event journal on a server (docs/adr/0114, D1 and D4): the ledger that says each tag it takes
//! ([`Ledgers::journaling`](crate::ledger::Ledgers::journaling)), and a view that follows it.
//!
//! The view answers [`COUNT`], how many tags of its key it holds; a request may carry the position
//! of the ledger's journal it asks the view to have read (docs/adr/0116, D5; deploy.yaml's
//! `sessions.position_from`, here [`POSITION_FIELD`]), and the pool holds it until its reader has
//! passed that position.
//!
//! Both keep their sessions in one store, each machine its own ([`VIEW`] and
//! [`MACHINE_NAME`]), and the view reads the ledger's journal from that store. A view of a key
//! holds the tags the ledger of that key took, in the order the journal holds them; what makes it
//! hold each once is the position it keeps in its checkpoint, which a restore reads back.
//! Written by hand in the shape a generated machine has to the pool, as the ledger is.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError};

use sce_rust_mesh::checkpoint;
use sce_rust_mesh::endpoint::{
    EngineCall, Environment, Transport, MESH_ORIGIN_SCHEME, MESH_PROCESSOR_TYPE,
};
use sce_rust_mesh::peers::{FollowedEntry, Follows, Machine};
use sce_rust_mesh::pool::{SessionEndpoint, SessionFactory, SessionMachine};
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_runtime::HostSendRequest;
use sce_rust_session_store::{JournalStore, SessionStore, StreamName};

use crate::ledger::{pooled, Binding, MACHINE_NAME, TAKEN_FACT, TAKEN_RECORD};

/// The view's name: the machine whose sessions follow the ledger's journal, and the name its
/// reader's mark is kept under.
pub const VIEW: &str = "view";

/// What the view follows: the ledger's facts.
pub const FOLLOWS: Follows = Follows {
    journals: &[MACHINE_NAME],
    entries: &[FollowedEntry {
        event: TAKEN_FACT,
        record: TAKEN_RECORD,
    }],
};

/// The request a view answers: how many tags of its key it holds (`{"key", "after"?}`).
pub const COUNT: &str = "service.request.count";
/// Its reply: `{"taken"}`.
pub const COUNTED: &str = "service.response.count";

/// The field of a [`COUNT`] request that carries the position of the ledger's journal the view is
/// asked to have read (deploy.yaml's `sessions.position_from`, docs/adr/0116, D5).
pub const POSITION_FIELD: &str = "after";

/// The view as deploy.yaml would declare it: a keyed pool, keyed by the `key` field of the
/// ledger's facts, that follows the ledger (`follows: [ledger]`) and answers [`COUNT`], whose
/// position of the ledger's journal is read from [`POSITION_FIELD`]; its one peer is the client,
/// as every machine of the suite's.
pub fn view_machine(binding: Binding, idle_ms: u64) -> Machine {
    let mut machine = pooled(VIEW, &[COUNTED], binding, idle_ms);
    if let Some(sessions) = machine.sessions.as_mut() {
        sessions.follows = FOLLOWS;
        sessions.position_from = Some(POSITION_FIELD);
    }
    machine
}

/// The data of a [`COUNT`] request for `key`, asking the view to have read the ledger's journal
/// through `after` when it is given.
pub fn count_data(key: &str, after: Option<u64>) -> String {
    match after {
        Some(after) => format!(
            r#"{{"key":{},"{POSITION_FIELD}":{after}}}"#,
            json::quote(key)
        ),
        None => format!(r#"{{"key":{}}}"#, json::quote(key)),
    }
}

/// One key's view: the tags the ledger of the key took, in the order it took them in.
pub struct View<T, E> {
    taken: Vec<String>,
    endpoint: SessionEndpoint<T, E>,
}

impl<T: Transport, E: Environment> View<T, E> {
    /// Answer the [`COUNT`] request `invoke_id` of `peer` with how many tags the view holds.
    fn count(&self, peer: &str, invoke_id: &str) {
        let request = HostSendRequest {
            processor_type: MESH_PROCESSOR_TYPE.to_string(),
            event_name: COUNTED.to_string(),
            target: format!("#{peer}"),
            event_data: format!(r#"{{"taken":{}}}"#, self.taken.len()),
            invoke_id: invoke_id.to_string(),
            ..HostSendRequest::default()
        };
        self.endpoint
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(&request);
    }
}

impl<T, E> SessionMachine for View<T, E>
where
    T: Transport + 'static,
    E: Environment + 'static,
{
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            if event.name == COUNT {
                stepped = true;
                let peer = event
                    .metadata
                    .origin_address()
                    .strip_prefix(MESH_ORIGIN_SCHEME)
                    .unwrap_or_default()
                    .to_string();
                self.count(&peer, &event.metadata.invoke_id);
                continue;
            }
            if event.name != TAKEN_FACT {
                continue;
            }
            stepped = true;
            let data = json::parse(&event.metadata.data).unwrap_or(Value::Null);
            if let Some(Value::Text(tag)) = data.member("tag") {
                self.taken.push(tag.clone());
            }
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: VIEW.to_string(),
            configuration: vec!["viewing".to_string()],
            current: "viewing".to_string(),
            variables: vec![(
                "taken".to_string(),
                Value::Array(self.taken.iter().cloned().map(Value::Text).collect()),
            )],
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: VIEW.to_string(),
            invoke_starts: Vec::new(),
            host_invokes: Vec::new(),
            host_invoke_starts: Vec::new(),
            host_invoke_token: 0,
            auto_send_seq: 0,
            external: Vec::new(),
            revisions: Vec::new(),
            features: None,
        })
    }

    fn finished(&self) -> bool {
        false
    }
}

/// The tags a view's saved `state` holds; refused for a state of another machine.
fn taken_of(state: &SavedState) -> Result<Vec<String>, String> {
    if state.shape != VIEW {
        return Err(format!("a state of shape {} is not a view's", state.shape));
    }
    match state.variable("taken") {
        Ok(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Text(tag) => Ok(tag.clone()),
                other => Err(format!("a tag that is {other:?}")),
            })
            .collect(),
        other => Err(format!("taken is {other:?}")),
    }
}

/// How the pool builds the view of a key.
#[derive(Default, Clone, Copy)]
pub struct Views;

impl<T, E> SessionFactory<T, E> for Views
where
    T: Transport + 'static,
    E: Environment + 'static,
{
    type Machine = View<T, E>;

    fn start(
        &self,
        _key: &str,
        endpoint: &SessionEndpoint<T, E>,
        _wall_now_ms: u64,
    ) -> Result<View<T, E>, String> {
        Ok(View {
            taken: Vec::new(),
            endpoint: Arc::clone(endpoint),
        })
    }

    fn restore(
        &self,
        _key: &str,
        state: &SavedState,
        endpoint: &SessionEndpoint<T, E>,
        _wall_now_ms: u64,
    ) -> Result<View<T, E>, String> {
        Ok(View {
            taken: taken_of(state)?,
            endpoint: Arc::clone(endpoint),
        })
    }
}

/// What the view of `key` holds in `store`, the view's handle, as its last durable checkpoint
/// keeps it: the tags it took in, and the position of the ledger's journal it took in; `None`
/// when the store holds no view of the key.
pub fn read_view<S: SessionStore + ?Sized>(
    store: &S,
    key: &str,
) -> Result<Option<(Vec<String>, u64)>, String> {
    let Some(loaded) = checkpoint::load(store, key).map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let taken = taken_of(&loaded.checkpoint.state)?;
    let position = loaded
        .checkpoint
        .followed
        .get(MACHINE_NAME)
        .copied()
        .unwrap_or(0);
    Ok(Some((taken, position)))
}

/// The position of the last tag the ledger's journal in `store` holds of each key, read up to the
/// horizon: how far a view must have read the journal to hold every tag of the key.
pub fn last_position_by_key(store: &impl JournalStore) -> Result<BTreeMap<String, u64>, String> {
    let mut last = BTreeMap::new();
    let mut after = 0;
    loop {
        let page = store.after(after, 512).map_err(|error| error.to_string())?;
        let Some(end) = page.entries.last().map(|entry| entry.position) else {
            return Ok(last);
        };
        for entry in page.entries {
            if let StreamName::Session { machine, key } = entry.stream {
                if machine == MACHINE_NAME {
                    last.insert(key, entry.position);
                }
            }
        }
        after = end;
    }
}

/// The tags the ledger's journal in `store` holds of each key, in the order of their positions,
/// and the position of the last entry, read up to the horizon.
pub fn journal_by_key(
    store: &impl JournalStore,
) -> Result<(BTreeMap<String, Vec<String>>, u64), String> {
    let mut by_key: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut after = 0;
    loop {
        let page = store.after(after, 512).map_err(|error| error.to_string())?;
        let Some(last) = page.entries.last().map(|entry| entry.position) else {
            return Ok((by_key, after));
        };
        for entry in page.entries {
            let StreamName::Session { machine, key } = &entry.stream else {
                return Err(format!("an observed entry at {}", entry.position));
            };
            if machine != MACHINE_NAME || entry.schema != TAKEN_FACT {
                return Err(format!(
                    "an entry of {machine} of {} at {}",
                    entry.schema, entry.position
                ));
            }
            let record = sce_rust_runtime::record_cbor::decode(&entry.data, TAKEN_RECORD)
                .map_err(|refusal| format!("the entry at {}: {refusal:?}", entry.position))?;
            let record = json::parse(&record).map_err(|error| error.to_string())?;
            let (Some(Value::Text(said_key)), Some(Value::Text(tag))) =
                (record.member("key"), record.member("tag"))
            else {
                return Err(format!(
                    "the entry at {} names no key and tag",
                    entry.position
                ));
            };
            if said_key != key {
                return Err(format!(
                    "the entry at {} of the stream of {key} says the key {said_key}",
                    entry.position
                ));
            }
            by_key.entry(key.clone()).or_default().push(tag.clone());
        }
        after = last;
    }
}
