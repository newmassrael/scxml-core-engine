// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The relay: it verifies what a device sends it against the list's membership log, keeps it, and
//! gives it back.
//!
//! The relay holds, for each list, the membership log, the operations of every device's chain and
//! the members' checkpoints, in the order they arrived. It holds no key of a member and none of an
//! administrator, and reads no content: what is sealed stays sealed. What it applies is the rules
//! of `sce-rust-trust` that need no key (the entry an operation names, its author's role under it,
//! the cutoff the log set for the device, the signature, the consistency of the chain), so that it
//! can refuse spam and the removed without reading anything. It decides nothing a member's host
//! would then trust: every host checks what a relay gives it as it checks what anyone gives it.
//!
//! A request is made by a session, which a login token names ([`Authority`]). What a session may
//! ask is decided by the log at the relay's head: a device reads when it is a member, and writes
//! when it may write. Each request spends the session's allowance ([`Limits`]).

use std::collections::{BTreeMap, BTreeSet};
use std::io;

use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Entry, LogError};
use sce_rust_trust::{Checkpoint, CheckpointError, TakeError, Trust};
use sha2::{Digest, Sha256};

use crate::limits::{Allowances, Limits};
use crate::protocol::{Code, Record, Request, Response};
use crate::store::Store;
use crate::token::{Authority, Session, TokenError};

/// The role at which a device may write, and the one at which it may read.
const WRITER: u8 = 2;
const READER: u8 = 1;

struct ListState {
    /// Holds the log, the chains and the checkpoints, with no key and no person: a verifier.
    trust: Trust,
    /// What the relay was given, in the order it was given it.
    records: Vec<Record>,
    /// The epochs whose commit the relay holds, and what it holds of the MLS messages by the hash
    /// of their bytes, so that one is kept once.
    commits: BTreeSet<u32>,
    welcomes: BTreeSet<(u32, u64, [u8; 32])>,
    key_packages: BTreeMap<u64, BTreeSet<[u8; 32]>>,
}

impl ListState {
    fn new(list: &str) -> Self {
        Self {
            trust: Trust::open(list),
            records: Vec::new(),
            commits: BTreeSet::new(),
            welcomes: BTreeSet::new(),
            key_packages: BTreeMap::new(),
        }
    }

    /// Remember an MLS message that was kept, so that the same one is not kept twice.
    fn note(&mut self, record: &Record) {
        match record {
            Record::Commit { epoch, .. } => {
                self.commits.insert(*epoch);
            }
            Record::Welcome { epoch, node, data } => {
                if let Some(hash) = digest_of(data) {
                    self.welcomes.insert((*epoch, *node, hash));
                }
            }
            Record::KeyPackage { node, data } => {
                if let Some(hash) = digest_of(data) {
                    self.key_packages.entry(*node).or_default().insert(hash);
                }
            }
            Record::Entry(_) | Record::Operation { .. } | Record::Checkpoint(_) => {}
        }
    }
}

/// The SHA-256 of what `text` (in the URL alphabet) writes, when it is that.
fn digest_of(text: &str) -> Option<[u8; 32]> {
    Some(Sha256::digest(encoding::decode(text)?).into())
}

/// A relay over a store.
pub struct Relay<S: Store> {
    store: S,
    authority: Authority,
    limits: Limits,
    lists: BTreeMap<String, ListState>,
    allowances: Allowances,
    /// Why the relay takes nothing: a record it was given could not be kept.
    failed: Option<String>,
}

fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl<S: Store> Relay<S> {
    /// A relay that starts from what `store` holds, every record read back through the checks that
    /// took it in: a store that holds what the members did not sign does not start a relay.
    pub fn open(mut store: S, authority: Authority, limits: Limits) -> io::Result<Self> {
        let mut lists: BTreeMap<String, ListState> = BTreeMap::new();
        for (index, (list, record)) in store.load()?.into_iter().enumerate() {
            let state = lists
                .entry(list.clone())
                .or_insert_with(|| ListState::new(&list));
            match verify(state, &record, &limits) {
                Ok(true) => {
                    state.note(&record);
                    state.records.push(record);
                }
                Ok(false) => {
                    return Err(invalid_data(format!(
                        "record {index} of `{list}` is one the relay held already"
                    )))
                }
                Err((_, reason)) => {
                    return Err(invalid_data(format!(
                        "record {index} of `{list}` does not verify: {reason}"
                    )))
                }
            }
        }
        Ok(Self {
            store,
            authority,
            limits,
            lists,
            allowances: Allowances::default(),
            failed: None,
        })
    }

    /// The device a token names, when this relay issued it and it has not expired: what a
    /// transport asks before it lets a connection stay.
    pub fn authenticate(&self, token: &str, now_ms: u64) -> Result<Session, TokenError> {
        self.authority.verify(token, now_ms)
    }

    /// Answer a request text made with `token` at `now_ms`, as a response text.
    pub fn handle(&mut self, token: &str, request: &str, now_ms: u64) -> String {
        let response = match self.authority.verify(token, now_ms) {
            Err(error) => Response::refused(Code::Unauthorized, token_reason(error)),
            Ok(session) => {
                if request.len() > self.limits.max_request_bytes {
                    Response::refused(
                        Code::TooLarge,
                        format!(
                            "a request is at most {} bytes",
                            self.limits.max_request_bytes
                        ),
                    )
                } else {
                    match Request::from_json(request) {
                        Ok(request) => self.call(&session, request, now_ms),
                        Err(reason) => Response::refused(Code::Invalid, reason),
                    }
                }
            }
        };
        response.to_json()
    }

    /// Answer `request` made by `session` at `now_ms`.
    pub fn call(&mut self, session: &Session, request: Request, now_ms: u64) -> Response {
        if let Some(reason) = &self.failed {
            return Response::refused(Code::Unavailable, reason.clone());
        }
        let cost = if request.is_write() {
            self.limits.write_cost
        } else {
            1
        };
        if let Err(retry_after_ms) =
            self.allowances
                .take(&session.list, session.node, cost, now_ms, &self.limits)
        {
            return Response::refused(
                Code::RateLimited { retry_after_ms },
                "this device has spent its allowance",
            );
        }
        let role = self
            .lists
            .get(&session.list)
            .map_or(0, |state| state.trust.role(session.node));
        let needs = match &request {
            Request::Entries { .. }
            | Request::Append { .. }
            | Request::PublishKeyPackage { .. } => 0,
            Request::Submit { .. }
            | Request::Checkpoint { .. }
            | Request::PublishCommit { .. }
            | Request::PublishWelcome { .. } => WRITER,
            Request::Feed { .. } | Request::Operations { .. } | Request::Heads => READER,
        };
        if role < needs {
            return Response::refused(
                Code::Forbidden,
                format!(
                    "node {} has role {role} in this list and this needs {needs}",
                    session.node
                ),
            );
        }
        match request {
            Request::Entries { from } => self.entries(&session.list, from),
            Request::Append { entry } => match Entry::from_json(&entry) {
                Some(parsed) => self.append(&session.list, entry, parsed),
                None => Response::refused(Code::Invalid, "it is not an entry of a log"),
            },
            Request::Submit { event, data } => self.submit(&session.list, event, data),
            Request::Checkpoint { checkpoint } => match Checkpoint::from_json(&checkpoint) {
                Some(parsed) => self.checkpoint(&session.list, checkpoint, parsed),
                None => Response::refused(Code::Invalid, "it is not a checkpoint"),
            },
            Request::PublishKeyPackage { data } => self.publish_group(
                &session.list,
                Record::KeyPackage {
                    node: session.node,
                    data,
                },
            ),
            Request::PublishCommit { epoch, data } => {
                self.publish_group(&session.list, Record::Commit { epoch, data })
            }
            Request::PublishWelcome { epoch, node, data } => {
                self.publish_group(&session.list, Record::Welcome { epoch, node, data })
            }
            Request::Feed { cursor, limit } => self.feed(&session.list, cursor, limit),
            Request::Operations { node, from, to } => {
                self.operations(&session.list, node, from, to)
            }
            Request::Heads => self.heads(&session.list),
        }
    }

    /// Keep an MLS message of the list's group: a key package, the commit the log names for an
    /// epoch, a Welcome. The relay reads none of them; it keeps each once, the commit only when its
    /// hash is the one the log names, and bounds how many a device leaves.
    fn publish_group(&mut self, list: &str, record: Record) -> Response {
        if let Err(full) = self.room(list) {
            return full;
        }
        let Some(state) = self.lists.get(list) else {
            return Response::refused(Code::Behind, "the relay holds no log of this list");
        };
        match check_group_record(state, &record, &self.limits) {
            Ok(false) => Response::Accepted {
                new: false,
                cursor: state.records.len() as u64,
            },
            Ok(true) => match self.keep(list, record) {
                Ok(cursor) => Response::Accepted { new: true, cursor },
                Err(response) => response,
            },
            Err((code, reason)) => Response::refused(code, reason),
        }
    }

    /// The number of records the relay holds of `list`.
    pub fn records(&self, list: &str) -> usize {
        self.lists.get(list).map_or(0, |state| state.records.len())
    }

    fn entries(&self, list: &str, from: u64) -> Response {
        let held = self
            .lists
            .get(list)
            .map(|state| state.trust.entries())
            .unwrap_or_default();
        let start = usize::try_from(from).unwrap_or(usize::MAX).min(held.len());
        let end = start
            .saturating_add(self.limits.max_range as usize)
            .min(held.len());
        Response::Entries {
            entries: held[start..end].iter().map(Entry::to_json).collect(),
            len: held.len() as u64,
        }
    }

    /// Keep one more record of `list`: the relay has already verified it. A record the relay cannot
    /// write down was taken in memory and is not durable, so the relay takes nothing more until it
    /// is restarted from what it did write.
    fn keep(&mut self, list: &str, record: Record) -> Result<u64, Response> {
        if let Err(error) = self.store.append(list, &record) {
            let reason = format!("the store did not take a record: {error}");
            self.failed = Some(reason.clone());
            return Err(Response::refused(Code::Unavailable, reason));
        }
        let state = self
            .lists
            .get_mut(list)
            .ok_or_else(|| Response::refused(Code::Unavailable, "the list is gone"))?;
        state.note(&record);
        state.records.push(record);
        Ok(state.records.len() as u64)
    }

    /// Whether the list has room for one more record.
    fn room(&self, list: &str) -> Result<(), Response> {
        if self.records(list) >= self.limits.max_records_per_list {
            return Err(Response::refused(
                Code::TooLarge,
                "this list holds as many records as the relay keeps",
            ));
        }
        Ok(())
    }

    fn append(&mut self, list: &str, text: String, entry: Entry) -> Response {
        if let Err(full) = self.room(list) {
            return full;
        }
        let state = self
            .lists
            .entry(list.to_string())
            .or_insert_with(|| ListState::new(list));
        match state.trust.append(&entry) {
            Ok(true) => match self.keep(list, Record::Entry(text)) {
                Ok(_) => Response::Appended {
                    new: true,
                    len: self.lists[list].trust.log_len(),
                },
                Err(response) => response,
            },
            Ok(false) => Response::Appended {
                new: false,
                len: state.trust.log_len(),
            },
            Err(LogError::Gap { expected }) => Response::refused(
                Code::Behind,
                format!("the relay holds the log up to {expected} and not this far"),
            ),
            Err(error) => Response::refused(Code::Invalid, format!("{error:?}")),
        }
    }

    fn submit(&mut self, list: &str, event: String, data: String) -> Response {
        if let Err(full) = self.room(list) {
            return full;
        }
        let Some(state) = self.lists.get(list) else {
            return Response::refused(Code::Behind, "the relay holds no log of this list");
        };
        match state.trust.take_operation(&event, &data) {
            Ok(taken) if taken.new => match self.keep(list, Record::Operation { event, data }) {
                Ok(cursor) => Response::Accepted { new: true, cursor },
                Err(response) => response,
            },
            Ok(_) => Response::Accepted {
                new: false,
                cursor: state.records.len() as u64,
            },
            Err(TakeError::NotReached(reason)) => Response::refused(Code::Behind, reason),
            Err(TakeError::Refused(reason) | TakeError::NoKey(reason)) => {
                Response::refused(Code::Invalid, reason)
            }
        }
    }

    fn checkpoint(&mut self, list: &str, text: String, checkpoint: Checkpoint) -> Response {
        if let Err(full) = self.room(list) {
            return full;
        }
        let Some(state) = self.lists.get(list) else {
            return Response::refused(Code::Behind, "the relay holds no log of this list");
        };
        match state.trust.take_checkpoint(&checkpoint) {
            Ok(true) => match self.keep(list, Record::Checkpoint(text)) {
                Ok(cursor) => Response::Accepted { new: true, cursor },
                Err(response) => response,
            },
            Ok(false) => Response::Accepted {
                new: false,
                cursor: state.records.len() as u64,
            },
            Err(CheckpointError::LogNotHeld { log }) => Response::refused(
                Code::Behind,
                format!("the relay does not hold entry {log} of the log"),
            ),
            Err(error) => Response::refused(Code::Invalid, format!("{error:?}")),
        }
    }

    fn feed(&self, list: &str, cursor: u64, limit: u32) -> Response {
        let Some(state) = self.lists.get(list) else {
            return Response::refused(Code::Forbidden, "the relay holds no such list");
        };
        let total = state.records.len() as u64;
        if cursor > total {
            return Response::refused(
                Code::Invalid,
                format!("the cursor is {cursor} and the relay holds {total} records"),
            );
        }
        let count = u64::from(limit.min(self.limits.max_feed));
        let end = cursor.saturating_add(count).min(total);
        Response::Feed {
            records: state.records[cursor as usize..end as usize].to_vec(),
            next: end,
            more: end < total,
        }
    }

    fn operations(&self, list: &str, node: u64, from: u64, to: u64) -> Response {
        let Some(state) = self.lists.get(list) else {
            return Response::refused(Code::Forbidden, "the relay holds no such list");
        };
        let to = to.min(from.saturating_add(self.limits.max_range.saturating_sub(1)));
        Response::Operations {
            operations: state.trust.operations(node, from, to),
        }
    }

    fn heads(&self, list: &str) -> Response {
        let Some(state) = self.lists.get(list) else {
            return Response::refused(Code::Forbidden, "the relay holds no such list");
        };
        Response::Heads {
            heads: state.trust.heads(),
            log_len: state.trust.log_len(),
            cursor: state.records.len() as u64,
        }
    }
}

/// Whether a record that was kept verifies against what a list holds before it: `Ok(true)` when it
/// is new and holds, `Ok(false)` when the list held it already.
fn verify(state: &ListState, record: &Record, limits: &Limits) -> Result<bool, (Code, String)> {
    let trust = &state.trust;
    match record {
        Record::KeyPackage { .. } | Record::Commit { .. } | Record::Welcome { .. } => {
            check_group_record(state, record, limits)
        }
        Record::Entry(text) => {
            let entry = Entry::from_json(text)
                .ok_or((Code::Invalid, "it is not an entry of a log".to_string()))?;
            trust
                .append(&entry)
                .map_err(|error| (Code::Invalid, format!("{error:?}")))
        }
        Record::Operation { event, data } => trust
            .take_operation(event, data)
            .map(|taken| taken.new)
            .map_err(|error| match error {
                TakeError::NotReached(reason)
                | TakeError::NoKey(reason)
                | TakeError::Refused(reason) => (Code::Invalid, reason),
            }),
        Record::Checkpoint(text) => {
            let checkpoint = Checkpoint::from_json(text)
                .ok_or((Code::Invalid, "it is not a checkpoint".to_string()))?;
            trust
                .take_checkpoint(&checkpoint)
                .map_err(|error| (Code::Invalid, format!("{error:?}")))
        }
    }
}

/// Whether an MLS message of a list holds against what the list has: `Ok(true)` when it is new and
/// holds, `Ok(false)` when the relay holds it already. Nothing is read of it, only its hash taken: a
/// commit must be the one the log names for its epoch, a Welcome must be for an epoch the log began,
/// and a device leaves a bounded number of key packages.
fn check_group_record(
    state: &ListState,
    record: &Record,
    limits: &Limits,
) -> Result<bool, (Code, String)> {
    let too_large = || {
        (
            Code::TooLarge,
            format!(
                "an MLS message is at most {} bytes",
                limits.max_group_message_bytes
            ),
        )
    };
    let bytes_of = |data: &String| -> Result<Vec<u8>, (Code, String)> {
        let bytes = encoding::decode(data)
            .ok_or((Code::Invalid, "it is not in the URL alphabet".to_string()))?;
        if bytes.is_empty() {
            return Err((Code::Invalid, "it is empty".to_string()));
        }
        if bytes.len() > limits.max_group_message_bytes {
            return Err(too_large());
        }
        Ok(bytes)
    };
    match record {
        Record::KeyPackage { node, data } => {
            let hash: [u8; 32] = Sha256::digest(bytes_of(data)?).into();
            let held = state.key_packages.get(node);
            if held.is_some_and(|held| held.contains(&hash)) {
                return Ok(false);
            }
            if held.map_or(0, BTreeSet::len) >= limits.max_key_packages_per_node {
                return Err((
                    Code::TooLarge,
                    format!(
                        "a device leaves at most {} key packages",
                        limits.max_key_packages_per_node
                    ),
                ));
            }
            Ok(true)
        }
        Record::Commit { epoch, data } => {
            let hash: [u8; 32] = Sha256::digest(bytes_of(data)?).into();
            match state.trust.commit_of(*epoch) {
                None => Err((
                    Code::Behind,
                    format!("the log the relay holds names no commit for epoch {epoch}"),
                )),
                Some(named) if named != hash => Err((
                    Code::Invalid,
                    format!("this is not the commit the log names for epoch {epoch}"),
                )),
                Some(_) => Ok(!state.commits.contains(epoch)),
            }
        }
        Record::Welcome { epoch, node, data } => {
            let hash: [u8; 32] = Sha256::digest(bytes_of(data)?).into();
            if state.trust.commit_of(*epoch).is_none() {
                return Err((
                    Code::Behind,
                    format!("the log the relay holds has not begun epoch {epoch}"),
                ));
            }
            Ok(!state.welcomes.contains(&(*epoch, *node, hash)))
        }
        _ => Err((Code::Invalid, "it is not an MLS message".to_string())),
    }
}

fn token_reason(error: TokenError) -> &'static str {
    match error {
        TokenError::Malformed => "the token is not one this relay issues",
        TokenError::BadTag => "the token was not issued by this relay",
        TokenError::WrongAudience => "the token is for another relay",
        TokenError::Expired => "the token has expired",
    }
}
