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
//!
//! A relay that offers login ([`Relay::with_login`]) also answers a connection that presents no
//! token it takes: a member's device logs in there against the log the relay holds, and is issued
//! its token ([`crate::login`]).

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::net::IpAddr;

use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Entry, LogError};
use sce_rust_trust::{Checkpoint, CheckpointError, PublicKey, TakeError, Trust};
use sha2::{Digest, Sha256};

use crate::limits::{Allowances, Limits};
use crate::login::{self, Asked, Checked, Doorway, Opened};
use crate::protocol::{Code, Join, Joined, Login, Record, Request, Response};
use crate::store::{Openly, Store};
use crate::token::{Authority, Session, TokenError};

/// The role at which a device may write, and the one at which it may read.
const WRITER: u8 = 2;
const READER: u8 = 1;

/// How long a login's touch of a list created openly may go before its mark is written again: a
/// relay writes a mark at most once an hour for a list, however often its devices log in, and a
/// list lapses that much later than its last login's instant, so that the instant the mark holds
/// never makes it lapse early.
pub const TOUCH_STEP_MS: u64 = 3_600_000;

/// What a sweep of lapsed lists did ([`Relay::lapse`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lapsed {
    /// How many lists created openly were removed, for no login had touched them for long enough.
    pub removed: usize,
    /// How many lists created openly the relay still holds.
    pub kept: usize,
    /// Why a removal failed, for each that did: the relay takes nothing more until it is started
    /// again from its store, as for a record it could not write.
    pub failures: Vec<String>,
}

impl Lapsed {
    /// What two sweeps did together: a relay's of each of its slots.
    pub fn and(mut self, other: Lapsed) -> Lapsed {
        self.removed += other.removed;
        self.kept += other.kept;
        self.failures.extend(other.failures);
        self
    }
}

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
    /// The nodes a key package was kept for under an invitation, each with when its invitation
    /// expires: the record of the invitation's use (ADR 0080 section 4).
    joins: BTreeMap<u64, u64>,
    /// The mark the store keeps of a list a device created with no token, which lapses; `None` for
    /// a list created with a token of the authority, which does not.
    openly: Option<Openly>,
    /// When a login last touched the list, by the relay's clock: what the store's mark says once
    /// the relay starts, and every login since, which the mark is written with at most once a
    /// [`TOUCH_STEP_MS`].
    touched_at_ms: u64,
}

impl ListState {
    fn new(list: &str) -> Self {
        Self {
            trust: Trust::open(list),
            records: Vec::new(),
            commits: BTreeSet::new(),
            welcomes: BTreeSet::new(),
            key_packages: BTreeMap::new(),
            joins: BTreeMap::new(),
            openly: None,
            touched_at_ms: 0,
        }
    }

    /// Whether the list was created openly and no login has touched it for `idle_ms`, and the
    /// step a mark lags a login by, at `now_ms`: so that a list never lapses before its time, even
    /// on a relay that started again since its last login and holds the mark's instant alone.
    fn lapsed(&self, idle_ms: u64, now_ms: u64) -> bool {
        self.openly.is_some_and(|mark| {
            let touched = self.touched_at_ms.max(mark.touched_at_ms);
            now_ms
                >= touched
                    .saturating_add(idle_ms)
                    .saturating_add(TOUCH_STEP_MS)
        })
    }

    /// How many joins wait for an administrator at `now_ms`: their invitation has not expired, and
    /// the log has not granted their node.
    fn outstanding_joins(&self, now_ms: u64) -> usize {
        self.joins
            .iter()
            .filter(|(node, expires_at_ms)| {
                **expires_at_ms > now_ms && !self.trust.has_granted(**node)
            })
            .count()
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
            Record::KeyPackage { node, data, join } => {
                if !data.is_empty() {
                    if let Some(hash) = digest_of(data) {
                        self.key_packages.entry(*node).or_default().insert(hash);
                    }
                }
                if let Some(join) = join {
                    self.joins.insert(*node, join.invitation.expires_at_ms);
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
    /// What answers a connection with no token, when the relay offers login.
    doorway: Option<Doorway>,
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
        // A mark of a list the store holds no record of is what a crash between a creation's mark
        // and its first record, or between a removal's records and its mark, left: it is removed.
        for (list, mark) in store.marks()? {
            match lists.get_mut(&list) {
                Some(state) => {
                    state.openly = Some(mark);
                    state.touched_at_ms = mark.touched_at_ms;
                }
                None => store.remove(&list)?,
            }
        }
        Ok(Self {
            store,
            authority,
            limits,
            lists,
            allowances: Allowances::default(),
            failed: None,
            doorway: None,
        })
    }

    /// This relay offers login through `doorway`: a member's device logs in on a connection with
    /// no token and is issued its token. A relay offers none until it is given one.
    pub fn with_login(mut self, doorway: Doorway) -> Self {
        self.doorway = Some(doorway);
        self
    }

    /// Whether a connection that presents no token the relay takes is taken from `source` at
    /// `now_ms`.
    pub fn open_anonymous(&mut self, source: Option<IpAddr>, now_ms: u64) -> Opened {
        match self.doorway.as_mut() {
            None => Opened::NotOffered,
            Some(doorway) => doorway.open(source, now_ms),
        }
    }

    /// Answer a request text made at `now_ms` on a connection from `source` that presents no token
    /// the relay takes, as a response text: a challenge, a login, and a refusal of the rest.
    pub fn anonymous(&mut self, source: Option<IpAddr>, request: &str, now_ms: u64) -> String {
        let asked = match self.doorway.as_mut() {
            None => return login::not_offered(request).to_json(),
            Some(doorway) => doorway.ask(source, request, self.limits.max_request_bytes, now_ms),
        };
        let response = match asked {
            Err(refused) => refused,
            Ok(Asked::Challenge { list, node }) => {
                let held = self.holds(&list);
                match self.doorway.as_mut() {
                    None => login::not_offered(request),
                    Some(doorway) => {
                        doorway.challenge(&list, node, held, self.authority.audience(), now_ms)
                    }
                }
            }
            Ok(Asked::Login(checked)) => {
                let audience = self.authority.audience().to_string();
                let admitted = self.admit_login(
                    checked.login(),
                    |key| checked.signed_by(key, &audience),
                    now_ms,
                );
                match (admitted, self.doorway.as_mut()) {
                    (Err(refused), _) => refused,
                    (Ok(()), None) => login::not_offered(request),
                    (Ok(()), Some(doorway)) => doorway.redeem(&checked, &self.authority, now_ms),
                }
            }
            Ok(Asked::Create(checked)) => {
                let spent = match self.doorway.as_mut() {
                    None => Err(login::not_offered(request)),
                    Some(doorway) => doorway.spend_create(&checked, now_ms),
                };
                let audience = self.authority.audience().to_string();
                let created = spent.and_then(|()| self.take_create(&checked, &audience, now_ms));
                match (created, self.doorway.as_ref()) {
                    (Err(refused), _) => refused,
                    (Ok(()), None) => login::not_offered(request),
                    (Ok(()), Some(doorway)) => {
                        doorway.token(checked.login(), &self.authority, now_ms)
                    }
                }
            }
            Ok(Asked::Join(checked)) => {
                let spent = match self.doorway.as_mut() {
                    None => Err(login::not_offered(request)),
                    Some(doorway) => doorway
                        .spend_join(&checked, now_ms)
                        .map(|()| doorway.policy().joins_outstanding_per_list),
                };
                match spent {
                    Err(refused) => refused,
                    Ok(outstanding) => {
                        let audience = self.authority.audience().to_string();
                        self.take_join(checked.into_join(), &audience, outstanding, now_ms)
                    }
                }
            }
        };
        response.to_json()
    }

    /// Keep the key package a device leaves under an administrator's invitation (ADR 0080 section
    /// 4), at `now_ms`, at the relay named `audience`, when the list holds no more than
    /// `outstanding` joins waiting for an administrator: the invitation is for this relay and this
    /// list's genesis, an administrator the log names signed it, it has not expired, its node was
    /// never granted and has no key package kept under an invitation (the record of its use), and the
    /// proof holds under the invitation's key for the device's identity key and key package. The
    /// answer is `accepted`; no token is issued.
    pub(crate) fn take_join(
        &mut self,
        join: Box<Join>,
        audience: &str,
        outstanding: usize,
        now_ms: u64,
    ) -> Response {
        if let Some(reason) = &self.failed {
            return Response::refused(Code::Unavailable, reason.clone());
        }
        let Join {
            invitation,
            proof,
            key_package,
        } = *join;
        let list = invitation.list.clone();
        if invitation.audience != audience {
            return Response::refused(Code::Unauthorized, "the invitation is for another relay");
        }
        let Some(state) = self.lists.get(&list) else {
            return Response::refused(Code::Behind, "the relay holds no log of this list");
        };
        if now_ms >= invitation.expires_at_ms {
            return Response::refused(Code::Unauthorized, "the invitation has expired");
        }
        if state.outstanding_joins(now_ms) >= outstanding {
            return Response::refused(
                Code::TooLarge,
                "this list holds as many joins waiting for an administrator as the relay keeps",
            );
        }
        if let Err(full) = self.room(&list) {
            return full;
        }
        let record = Record::KeyPackage {
            node: invitation.node,
            data: key_package.unwrap_or_default(),
            join: Some(Box::new(Joined { invitation, proof })),
        };
        if let Err((code, reason)) = check_group_record(state, &record, &self.limits) {
            return Response::refused(code, reason);
        }
        match self.keep(&list, record) {
            Ok(cursor) => Response::Accepted { new: true, cursor },
            Err(response) => response,
        }
    }

    /// Whether the relay holds a log of `list`.
    pub(crate) fn holds(&self, list: &str) -> bool {
        self.lists.contains_key(list)
    }

    /// Whether the device `login` names may log in at `now_ms`, by the log at the relay's head once
    /// the entries it brought are appended: its latest grant gives it a role of at least 1 and a
    /// key, under which `signed` holds. A list the relay holds no log of is not made by a login. A
    /// login admitted touches its list, which keeps a list created openly from lapsing.
    pub(crate) fn admit_login(
        &mut self,
        login: &Login,
        signed: impl Fn(&PublicKey) -> bool,
        now_ms: u64,
    ) -> Result<(), Response> {
        let Some(held) = self
            .lists
            .get(&login.list)
            .map(|state| state.trust.log_len())
        else {
            return Err(Response::refused(
                Code::Behind,
                "the relay holds no log of this list",
            ));
        };
        for text in &login.entries {
            let Some(entry) = Entry::from_json(text) else {
                return Err(Response::refused(
                    Code::Invalid,
                    "an entry brought with the login is not an entry of a log",
                ));
            };
            // An entry the relay holds a position for is its own to judge; one past its head is
            // taken as `append` takes it, and the first that is not ends the login.
            if entry.index < held {
                continue;
            }
            match self.append(&login.list, text.clone(), entry) {
                Response::Appended { .. } => {}
                refused => return Err(refused),
            }
        }
        let key = self.lists.get(&login.list).and_then(|state| {
            state
                .trust
                .current_members()
                .into_iter()
                .find(|(node, _)| *node == login.node)
                .map(|(_, key)| key)
        });
        let Some(key) = key else {
            return Err(Response::refused(
                Code::Forbidden,
                format!("node {} is no member of this list", login.node),
            ));
        };
        if !signed(&key) {
            return Err(Response::refused(
                Code::Unauthorized,
                "the login is not signed by the key the log gives the node",
            ));
        }
        self.touch(&login.list, now_ms)
    }

    /// A login touched `list` at `now_ms`. The mark of a list created openly is written again when
    /// the one the store holds is a [`TOUCH_STEP_MS`] old or more; a mark the store did not take
    /// stops the relay taking anything, as a record it did not take does.
    fn touch(&mut self, list: &str, now_ms: u64) -> Result<(), Response> {
        let Some(state) = self.lists.get_mut(list) else {
            return Ok(());
        };
        state.touched_at_ms = state.touched_at_ms.max(now_ms);
        let Some(mark) = state.openly else {
            return Ok(());
        };
        if now_ms < mark.touched_at_ms.saturating_add(TOUCH_STEP_MS) {
            return Ok(());
        }
        let touched = Openly {
            touched_at_ms: now_ms,
            ..mark
        };
        if let Err(error) = self.store.mark(list, &touched) {
            let reason = self.store_failed("take a list's mark", &error);
            return Err(Response::refused(Code::Unavailable, reason));
        }
        if let Some(state) = self.lists.get_mut(list) {
            state.openly = Some(touched);
        }
        Ok(())
    }

    /// Take the creation `checked` (ADR 0080 section 3), which the doorway found holds its
    /// challenge, its work and its name, at the relay named `audience` at `now_ms`: its entries, from
    /// the list's genesis, verify as `append` verifies them, the latest grant of the node they give
    /// it a role of at least 1 and a key, and the login is signed by that key for this relay. The
    /// store keeps the list's mark first and then its entries, so that the list lapses as every
    /// list created openly does. A creation of a list the relay holds already (two devices that
    /// create a lapsed list at once) is a login. A relay that holds as many lists as it keeps takes
    /// no other.
    pub(crate) fn take_create(
        &mut self,
        checked: &Checked,
        audience: &str,
        now_ms: u64,
    ) -> Result<(), Response> {
        if let Some(reason) = &self.failed {
            return Err(Response::refused(Code::Unavailable, reason.clone()));
        }
        let login = checked.login();
        let signed = |key: &PublicKey| checked.signed_by(key, audience);
        if self.lists.contains_key(&login.list) {
            return self.admit_login(login, signed, now_ms);
        }
        if self.lists.len() >= self.limits.max_lists {
            return Err(Response::refused(
                Code::TooLarge,
                "the relay holds as many lists as it keeps",
            ));
        }
        if login.entries.len() > self.limits.max_records_per_list {
            return Err(Response::refused(
                Code::TooLarge,
                "this list holds as many records as the relay keeps",
            ));
        }
        let mut state = ListState::new(&login.list);
        for text in &login.entries {
            let entry = Entry::from_json(text).ok_or_else(|| {
                Response::refused(Code::Invalid, "an entry of the creation is not one")
            })?;
            match state.trust.append(&entry) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(Response::refused(
                        Code::Invalid,
                        "an entry of the creation is given twice",
                    ))
                }
                Err(error) => return Err(Response::refused(Code::Invalid, format!("{error:?}"))),
            }
        }
        let key = state
            .trust
            .current_members()
            .into_iter()
            .find(|(node, _)| *node == login.node)
            .map(|(_, key)| key)
            .ok_or_else(|| {
                Response::refused(
                    Code::Forbidden,
                    format!(
                        "node {} is no member of the list its creation gives",
                        login.node
                    ),
                )
            })?;
        if !signed(&key) {
            return Err(Response::refused(
                Code::Unauthorized,
                "the creation is not signed by the key the log gives the node",
            ));
        }
        let mark = Openly {
            created_at_ms: now_ms,
            touched_at_ms: now_ms,
        };
        if let Err(error) = self.store.mark(&login.list, &mark) {
            let reason = self.store_failed("take a list's mark", &error);
            return Err(Response::refused(Code::Unavailable, reason));
        }
        state.openly = Some(mark);
        state.touched_at_ms = now_ms;
        self.lists.insert(login.list.clone(), state);
        for text in &login.entries {
            self.keep(&login.list, Record::Entry(text.clone()))?;
        }
        Ok(())
    }

    /// Remove every list created openly that no login has touched for `idle_ms` at `now_ms` (and
    /// the [`TOUCH_STEP_MS`] a mark may lag a login by), whole: its records and its mark from the
    /// store, then what the relay holds of it, its allowances among them. A list created with a
    /// token of the authority never lapses. A removal the store refused stops the relay taking
    /// anything, as a record it did not take does, and the sweep there: a relay started again reads
    /// the list whole or not at all. What the sweep did is returned, and names no list.
    pub fn lapse(&mut self, idle_ms: u64, now_ms: u64) -> Lapsed {
        let mut lapsed = Lapsed::default();
        if let Some(reason) = &self.failed {
            lapsed.failures.push(reason.clone());
            return lapsed;
        }
        let due: Vec<String> = self
            .lists
            .iter()
            .filter(|(_, state)| state.lapsed(idle_ms, now_ms))
            .map(|(list, _)| list.clone())
            .collect();
        for list in due {
            if let Err(error) = self.store.remove(&list) {
                let reason = self.store_failed("remove a list that lapsed", &error);
                lapsed.failures.push(reason);
                break;
            }
            self.lists.remove(&list);
            self.allowances.forget(&list);
            lapsed.removed += 1;
        }
        lapsed.kept = self
            .lists
            .values()
            .filter(|state| state.openly.is_some())
            .count();
        lapsed
    }

    /// Whether `list` is one a device created with no token, which lapses.
    pub fn is_created_openly(&self, list: &str) -> bool {
        self.lists
            .get(list)
            .is_some_and(|state| state.openly.is_some())
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
            Request::Challenge { .. }
            | Request::Login(_)
            | Request::Join(_)
            | Request::Create(_) => 0,
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
                    join: None,
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
            // A session's token names its list and node already: a challenge, a login, a join and
            // a creation are answered on a connection with no token (`Self::anonymous`), as a hub
            // routes them.
            Request::Challenge { .. }
            | Request::Login(_)
            | Request::Join(_)
            | Request::Create(_) => Response::refused(
                Code::Invalid,
                "a challenge, a login, a join and a creation are not a session's requests",
            ),
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
            let reason = self.store_failed("take a record", &error);
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

    /// The store did not do `what` (a record, a list's mark, a removal) and `error` says why: what
    /// the relay holds in memory is no longer what it could start again from, so it takes nothing
    /// more until it is restarted from what the store holds, and says why to whoever asks.
    fn store_failed(&mut self, what: &str, error: &io::Error) -> String {
        let reason = format!("the store did not {what}: {error}");
        self.failed = Some(reason.clone());
        reason
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
        let new_list = !self.lists.contains_key(list);
        if new_list && self.lists.len() >= self.limits.max_lists {
            return Response::refused(Code::TooLarge, "the relay holds as many lists as it keeps");
        }
        let state = self
            .lists
            .entry(list.to_string())
            .or_insert_with(|| ListState::new(list));
        let taken = state.trust.append(&entry);
        let log_len = state.trust.log_len();
        if new_list && !matches!(taken, Ok(true)) {
            // A list is held from the first entry the relay takes of it: one it refused takes no room.
            self.lists.remove(list);
        }
        match taken {
            Ok(true) => match self.keep(list, Record::Entry(text)) {
                Ok(_) => Response::Appended {
                    new: true,
                    len: self.lists[list].trust.log_len(),
                },
                Err(response) => response,
            },
            Ok(false) => Response::Appended {
                new: false,
                len: log_len,
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
            since: state.openly.map(|mark| mark.created_at_ms),
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
        Record::KeyPackage {
            node,
            data,
            join: Some(join),
        } => {
            let package = if data.is_empty() {
                Vec::new()
            } else {
                bytes_of(data)?
            };
            check_join(state, *node, &package, join)?;
            Ok(true)
        }
        Record::KeyPackage { node, data, .. } => {
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

/// Whether a key package of the device of `node` (`package`, empty for none) left under the
/// invitation and proof of `join` holds against what the list has before it: the invitation is
/// for this node of this list and its genesis, an administrator the log names now signed it, the
/// log has never granted the node, no key package was kept for it under an invitation, and the
/// proof holds under the invitation's key. What depends on the time and on the relay it reached
/// (its expiry, its audience) is judged as it arrives (`Relay::take_join`), not when the relay
/// reads its store again.
fn check_join(
    state: &ListState,
    node: u64,
    package: &[u8],
    join: &Joined,
) -> Result<(), (Code, String)> {
    let unauthorized = |why: &str| Err((Code::Unauthorized, why.to_string()));
    let invitation = &join.invitation;
    if invitation.node != node || invitation.list != state.trust.list() {
        return Err((
            Code::Invalid,
            "the invitation is for another node or list".to_string(),
        ));
    }
    if state.trust.genesis() != Some(invitation.genesis) {
        return unauthorized("the invitation is not for this list's genesis");
    }
    if !state
        .trust
        .administrators()
        .contains(&invitation.administrator)
    {
        return unauthorized("the invitation is not signed by an administrator the log names");
    }
    if !invitation.is_signed() {
        return unauthorized("the invitation's signature does not hold");
    }
    if state.trust.has_granted(node) {
        return Err((
            Code::Forbidden,
            format!("node {node} was granted already: an invitation is for a node never granted"),
        ));
    }
    if state.joins.contains_key(&node) {
        return Err((
            Code::Forbidden,
            format!("the invitation of node {node} was used"),
        ));
    }
    if !join.proof.holds(invitation, package) {
        return unauthorized("the proof does not hold under the invitation's key");
    }
    Ok(())
}

pub(crate) fn token_reason(error: TokenError) -> &'static str {
    match error {
        TokenError::Malformed => "the token is not one this relay issues",
        TokenError::BadTag => "the token was not issued by this relay",
        TokenError::WrongAudience => "the token is for another relay",
        TokenError::Expired => "the token has expired",
    }
}
