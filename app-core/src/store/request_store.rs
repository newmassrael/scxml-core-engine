// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Generation requests in a work's folder.
//!
//! ```text
//! <work-id>/
//!   requests/<request-id>.json   one file per request, replaced whole (atomically)
//!   requests.log                 one JSON line per CHANGE OF STATE, never per renewal
//!   requests.head                how many lines the log holds, as the count a reader compares
//! ```
//!
//! A request is not a chain of immutable revisions as a text is: it changes in place (it
//! is taken, renewed, finished), and a renewal every few seconds would make a log nobody
//! reads. So the request's file is the truth, replaced whole by an atomic rename, and the
//! log keeps what a person asks of a history: what changed state, when, and why.
//!
//! Every change is made under the lock a save takes, so a request and the text it was asked
//! about cannot move apart: a save that ends the request is one step with the save. Reads
//! take no lock. The head is written last (the request's file, then the log line, then the
//! count), so a reader that sees the count move can read what it counts; a process that stops
//! between the log line and the count leaves the count behind, and the next change takes the
//! log's own length and writes the count that is true.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{
    atomic_write, bundle_store, drop_torn_tail, is_removed, read_pointer, removed_work, Artifact,
    WorkId, WorkStore, LOCK_FILE, LOCK_WAIT,
};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;
use crate::lock;
use crate::requests::{
    lease_is_valid, ConnectionRef, Inputs, Moment, Pin, Refusal, Request, State,
    LEASE_DEFAULT_SECONDS,
};
use crate::revision::Revision;

const REQUESTS_DIR: &str = "requests";
const REQUESTS_LOG: &str = "requests.log";
const REQUESTS_HEAD: &str = "requests.head";
const ID_PREFIX: &str = "req-";
const ID_HEX: usize = 12;
const NAME_MAX: usize = 64;
const KEY_MAX: usize = 200;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// What a caller says to make a request.
#[derive(Debug, Clone)]
pub struct Registration<'a> {
    /// What makes a repeated registration the request it repeats: a button pressed twice, a
    /// call sent again after a connection dropped.
    pub key: &'a str,
    /// Where it was asked from: `gui`, or the name of a client.
    pub origin: &'a str,
    /// The revisions the caller read and is asking about. The request is refused when the
    /// work is no longer at them, and is registered against them when it is.
    pub expect: Inputs,
    /// Replace the open request of the work, if there is one. Without it an open request
    /// is a refusal: whether a run in progress is thrown away is the person's to say.
    pub supersede: bool,
}

/// A request as the clock sees it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestView {
    /// The request as it is kept.
    pub request: Request,
    /// Its state at this moment: what is kept, except that a lease that ran out is an
    /// interrupted request.
    pub state: State,
}

/// What registering a request made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    pub request: Request,
    pub state: State,
    /// `false` when the key was one that registered a request before, and this is that request.
    pub created: bool,
}

/// One line of the history of requests: a request changed state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    /// The place of the line among the history of the work's requests, from 1.
    pub n: u64,
    pub request: String,
    /// `None` for the line that made the request.
    pub from: Option<State>,
    pub to: State,
    pub at: String,
    pub attempt: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Where the latest request of a work stands, for a screen that asks whether the work moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestHead {
    pub id: String,
    /// The state at the moment of the question: a lease that ran out is interrupted.
    pub state: State,
    pub attempt: u32,
}

/// Whether `id` is the shape of a request id this store makes. A name that is not names no
/// file, whatever it holds: it is the only thing between a request id from a caller and a
/// path.
fn valid_request_id(id: &str) -> bool {
    id.strip_prefix(ID_PREFIX).is_some_and(|rest| {
        (1..=40).contains(&rest.len())
            && rest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Whether `name` can name an executor, an origin or an adapter: letters, digits and
/// `._:-`, up to 64, starting with a letter or a digit. It names a file in one case, so
/// nothing in it climbs out of a folder and nothing begins with a dot.
pub(super) fn valid_name(name: &str) -> bool {
    (1..=NAME_MAX).contains(&name.len())
        && name.as_bytes()[0].is_ascii_alphanumeric()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn request_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(REQUESTS_DIR).join(format!("{id}.json"))
}

fn new_request_id(work: &WorkId, key: &str, epoch: u64) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = Sha256::new();
    hasher.update(work.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(key.as_bytes());
    hasher.update([0]);
    hasher.update(epoch.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    hasher.update(counter.to_le_bytes());
    let digest = hasher.finalize();
    let mut id = String::from(ID_PREFIX);
    for byte in digest.iter().take(ID_HEX / 2) {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

/// The count a reader compares: how many changes of state the work's requests have made.
/// A work that never had a request has made none.
pub(super) fn read_requests_head(dir: &Path) -> Result<u64, StoreError> {
    let path = dir.join(REQUESTS_HEAD);
    match fs::read_to_string(&path) {
        Ok(text) => text
            .trim()
            .parse()
            .map_err(|_| StoreError::corrupt(&path, "not a count of changes")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(StoreError::io(&path, e)),
    }
}

/// The request `id`, or `None` when there is none. A file that is not a request, or that
/// says it is another, is a damaged folder.
fn load_request(dir: &Path, id: &str) -> Result<Option<Request>, StoreError> {
    let path = request_path(dir, id);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(StoreError::io(&path, e)),
    };
    let request: Request = serde_json::from_slice(&bytes)
        .map_err(|e| StoreError::corrupt(&path, format!("not a request: {e}")))?;
    if request.id != id {
        return Err(StoreError::corrupt(
            &path,
            format!("the file says it is the request `{}`", request.id),
        ));
    }
    Ok(Some(request))
}

/// Every request of the work, oldest first.
fn load_requests(dir: &Path) -> Result<Vec<Request>, StoreError> {
    let folder = dir.join(REQUESTS_DIR);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(StoreError::io(&folder, e)),
    };
    let mut requests = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| StoreError::io(&folder, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // What a half-written replacement leaves beside a request is not a request.
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        if !valid_request_id(id) {
            continue;
        }
        if let Some(request) = load_request(dir, id)? {
            requests.push(request);
        }
    }
    requests.sort_by_key(|r| r.seq);
    Ok(requests)
}

fn write_request(dir: &Path, request: &Request) -> Result<(), StoreError> {
    let folder = dir.join(REQUESTS_DIR);
    fs::create_dir_all(&folder).map_err(|e| StoreError::io(&folder, e))?;
    let path = request_path(dir, &request.id);
    let mut bytes = serde_json::to_vec_pretty(request)
        .map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
    bytes.push(b'\n');
    atomic_write(&path, &bytes)
}

/// The history, oldest first. A final line cut short by a process that stopped is left out.
pub(super) fn read_transitions(dir: &Path) -> Result<Vec<Transition>, StoreError> {
    let path = dir.join(REQUESTS_LOG);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(StoreError::io(&path, e)),
    };
    let complete = match text.rfind('\n') {
        Some(end) => &text[..=end],
        None => "",
    };
    complete
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|e| {
                StoreError::corrupt(&path, format!("a line of the history is not one: {e}"))
            })
        })
        .collect()
}

/// Append a line to the history and move the count to it. Under the lock.
fn record_transition(
    dir: &Path,
    from: Option<State>,
    to: &Request,
    now: &Moment,
) -> Result<(), StoreError> {
    let path = dir.join(REQUESTS_LOG);
    drop_torn_tail(&path).map_err(|e| StoreError::io(&path, e))?;
    let n = read_transitions(dir)?.len() as u64 + 1;
    let line = Transition {
        n,
        request: to.id.clone(),
        from,
        to: to.state,
        at: now.text.clone(),
        attempt: to.attempt,
        note: to.note.clone(),
    };
    let mut text =
        serde_json::to_string(&line).map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
    text.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| StoreError::io(&path, e))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| StoreError::io(&path, e))?;
    atomic_write(&dir.join(REQUESTS_HEAD), format!("{n}\n").as_bytes())
}

/// Keep `to` as the request, in place of `from` (`None` for a request just made). A change of
/// state is a line of the history; a renewal that changes nothing a person reads is not.
pub(super) fn persist(
    dir: &Path,
    from: Option<&Request>,
    to: &Request,
    now: &Moment,
) -> Result<(), StoreError> {
    write_request(dir, to)?;
    let before = from.map(|r| r.state);
    if before != Some(to.state) {
        record_transition(dir, before, to, now)?;
    }
    Ok(())
}

/// The request, as the work's current bundle says it is: a request that is still open and
/// that the current bundle says made it is completed.
///
/// Publishing a bundle moves a pointer and then writes the request down, and a process can
/// stop between the two. The bundle is the work's model by then, and the request that made it
/// would be read as running, taken again, and made to publish a second bundle of the same work.
/// The bundle names the request and the attempt, so it is not a guess: the request is read as
/// the completion it was, and written down as that by the next change.
pub(super) fn reconcile(dir: &Path, request: Request) -> Result<Request, StoreError> {
    if !request.state.is_open() {
        return Ok(request);
    }
    let Some((revision, bundle)) = bundle_store::current_bundle(dir)? else {
        return Ok(request);
    };
    if bundle.request != request.id || bundle.attempt != request.attempt {
        return Ok(request);
    }
    let at = Moment {
        epoch: crate::clock::parse_utc_timestamp(&bundle.published_at).unwrap_or(0),
        text: bundle.published_at.clone(),
    };
    Ok(request
        .publish(&bundle.executor, bundle.attempt, &revision, &at)
        .unwrap_or(request))
}

/// [`reconcile`], and the request written down as that when it differs from what is kept.
/// Under the lock, so the write is nobody else's.
pub(super) fn heal(dir: &Path, stored: Request, now: &Moment) -> Result<Request, StoreError> {
    let healed = reconcile(dir, stored.clone())?;
    if healed != stored {
        persist(dir, Some(&stored), &healed, now)?;
    }
    Ok(healed)
}

/// The request `id`, or `None` for one that is not there, and for a name that is not the
/// shape of a request id (which names no file, whatever is at that path).
pub(super) fn find_request(dir: &Path, id: &str) -> Result<Option<Request>, StoreError> {
    if valid_request_id(id) {
        load_request(dir, id)
    } else {
        Ok(None)
    }
}

/// A request's refusal as the error a caller branches on.
pub(super) fn refusal_error(request: &Request, refusal: Refusal) -> StoreError {
    let id = &request.id;
    match refusal {
        Refusal::Ended { state } => StoreError::refused(
            "request-ended",
            format!(
                "the request {id} ended as {}, so it takes nothing more",
                state.word()
            ),
            json!({ "request": id, "state": state.word() }),
        ),
        Refusal::Held { holder, until } => StoreError::refused(
            "request-held",
            format!(
                "the request {id} is held by `{holder}` until {}; the one that holds it renews",
                utc_timestamp(until)
            ),
            json!({ "request": id, "holder": holder, "until": utc_timestamp(until) }),
        ),
        Refusal::NotHolder { holder, attempt } => StoreError::refused(
            "not-holder",
            match &holder {
                Some(holder) => format!(
                    "the request {id} is held by `{holder}` as attempt {attempt}, and that is not who spoke"
                ),
                None => format!("the request {id} is held by nobody"),
            },
            json!({ "request": id, "holder": holder, "attempt": attempt }),
        ),
        Refusal::NotResuming => StoreError::refused(
            "not-resuming",
            format!(
                "the request {id} was let go of; it is taken again only by a caller that says it resumes"
            ),
            json!({ "request": id }),
        ),
        Refusal::WrongConnection { pinned, offered } => {
            let named = |connection: &Option<ConnectionRef>| match connection {
                Some(c) => format!("connection `{}` at {}", c.id, c.revision.short()),
                None => "no connection".to_string(),
            };
            StoreError::refused(
                "wrong-connection",
                format!(
                    "the request {id} is for {}, and the caller runs for {}: it is taken by the \
                     executor of the connection it was made for and by no other",
                    named(&pinned),
                    named(&offered)
                ),
                json!({ "request": id, "pinned": pinned, "offered": offered }),
            )
        }
    }
}

fn bad(kind: &'static str, what: &str, detail: serde_json::Value) -> StoreError {
    StoreError::refused(kind, what.to_string(), detail)
}

impl<C: Clock> WorkStore<C> {
    /// Ask for a model of the work's text, as it is now.
    ///
    /// The request is registered against the revisions the caller says it read (`expect`),
    /// and refused as `moved` when the work is no longer at them: the same rule `accept`
    /// holds the owner to, for the same reason. The work has one open request at most; a
    /// second is refused (`active-request`) unless it says it replaces the first.
    pub fn register_request(
        &self,
        id: &WorkId,
        registration: Registration<'_>,
    ) -> Result<Registered, StoreError> {
        self.register_request_for(id, registration, None)
    }

    /// The same, for the connection `pin` copies: the request is made for it, and is taken by
    /// the executor that runs for it. `None` is a request nobody chose a connection for.
    ///
    /// A key that was used before answers the request it made, with the pin it made it with:
    /// the connection is not an input of the key, so a press sent again is not made again for
    /// another.
    pub fn register_request_for(
        &self,
        id: &WorkId,
        registration: Registration<'_>,
        pin: Option<Pin>,
    ) -> Result<Registered, StoreError> {
        if registration.key.is_empty() || registration.key.len() > KEY_MAX {
            return Err(bad(
                "bad-key",
                "a registration key is 1 to 200 characters",
                json!({ "limit": KEY_MAX }),
            ));
        }
        if !valid_name(registration.origin) {
            return Err(bad(
                "bad-origin",
                "an origin is 1 to 64 letters, digits and `._:-`",
                json!(null),
            ));
        }
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        // A removal that came first, while this call waited for the lock.
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        let now = Moment::of(&self.clock);
        let mut requests = Vec::new();
        for stored in load_requests(&dir)? {
            requests.push(heal(&dir, stored, &now)?);
        }

        if let Some(same) = requests.iter().find(|r| r.key == registration.key) {
            if same.inputs != registration.expect {
                return Err(StoreError::refused(
                    "key-reused",
                    format!(
                        "the key was used for another request ({}), about other revisions",
                        same.id
                    ),
                    json!({ "request": same.id }),
                ));
            }
            return Ok(Registered {
                request: same.clone(),
                state: same.effective(now.epoch),
                created: false,
            });
        }

        let source = read_pointer(&dir, Artifact::Source)?.map(|p| p.revision);
        let answers = read_pointer(&dir, Artifact::Answers)?.map(|p| p.revision);
        let Some(source) = source else {
            return Err(StoreError::NotFound {
                what: format!("a text of work `{id}` (none was saved)"),
            });
        };
        let mut moved = Vec::new();
        if source != registration.expect.source {
            moved.push("source");
        }
        if answers != registration.expect.answers {
            moved.push("answers");
        }
        if !moved.is_empty() {
            return Err(StoreError::refused(
                "moved",
                format!(
                    "{} changed after the request was asked about it, so nothing was registered; read the work again",
                    moved.join(" and ")
                ),
                json!({ "moved": moved, "current": { "source": source, "answers": answers } }),
            ));
        }

        let open: Vec<&Request> = requests.iter().filter(|r| r.state.is_open()).collect();
        if let Some(first) = open.first() {
            if !registration.supersede {
                return Err(StoreError::refused(
                    "active-request",
                    format!(
                        "the work already has an open request ({}, {}); ask to replace it, or cancel it",
                        first.id,
                        first.effective(now.epoch).word()
                    ),
                    json!({ "request": first.id, "state": first.effective(now.epoch).word() }),
                ));
            }
            for request in open {
                if let Some(ended) = request.supersede("a newer request replaced it", &now) {
                    persist(&dir, Some(request), &ended, &now)?;
                }
            }
        }

        let seq = requests.iter().map(|r| r.seq).max().unwrap_or(0) + 1;
        let request = Request::queued(
            new_request_id(id, registration.key, now.epoch),
            seq,
            registration.key.to_string(),
            registration.origin.to_string(),
            registration.expect,
            pin,
            &now,
        );
        persist(&dir, None, &request, &now)?;
        Ok(Registered {
            state: State::Queued,
            request,
            created: true,
        })
    }

    /// One request of the work, as the clock sees it now.
    pub fn read_request(&self, id: &WorkId, request: &str) -> Result<RequestView, StoreError> {
        let dir = self.existing(id)?;
        let found = if valid_request_id(request) {
            load_request(&dir, request)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, request));
        };
        self.view(&dir, found)
    }

    /// Every request of the work, the newest first.
    pub fn list_requests(&self, id: &WorkId) -> Result<Vec<RequestView>, StoreError> {
        let dir = self.existing(id)?;
        let mut views = load_requests(&dir)?
            .into_iter()
            .map(|r| self.view(&dir, r))
            .collect::<Result<Vec<RequestView>, StoreError>>()?;
        views.reverse();
        Ok(views)
    }

    /// Every request of every work that can still produce a result, oldest first, as the
    /// clock reads it: what an executor looks at to find something to do. A work that cannot
    /// be read (removed while this listed, or damaged) holds back its own requests and no
    /// other work's.
    pub fn open_requests(&self) -> Result<Vec<(WorkId, RequestView)>, StoreError> {
        let mut open = Vec::new();
        for work in self.list_works()?.works {
            let Ok(views) = self.list_requests(&work.id) else {
                continue;
            };
            open.extend(
                views
                    .into_iter()
                    .filter(|view| view.state.is_open())
                    .map(|view| (work.id.clone(), view)),
            );
        }
        open.sort_by(|(a_work, a), (b_work, b)| {
            (&a.request.created_at, a_work.as_str(), a.request.seq).cmp(&(
                &b.request.created_at,
                b_work.as_str(),
                b.request.seq,
            ))
        });
        Ok(open)
    }

    /// Every change of state of the work's requests, oldest first.
    pub fn request_history(&self, id: &WorkId) -> Result<Vec<Transition>, StoreError> {
        let dir = self.existing(id)?;
        read_transitions(&dir)
    }

    /// The request as the clock and the work's current bundle say it is now.
    pub(super) fn view(&self, dir: &Path, request: Request) -> Result<RequestView, StoreError> {
        let request = reconcile(dir, request)?;
        Ok(RequestView {
            state: request.effective(self.clock.epoch()),
            request,
        })
    }

    /// Where the latest request of the work stands as the clock says it now.
    pub(super) fn latest_request_head(
        &self,
        dir: &Path,
        _id: &WorkId,
    ) -> Result<Option<RequestHead>, StoreError> {
        let Some(latest) = load_requests(dir)?.pop() else {
            return Ok(None);
        };
        let request = reconcile(dir, latest)?;
        Ok(Some(RequestHead {
            state: request.effective(self.clock.epoch()),
            attempt: request.attempt,
            id: request.id,
        }))
    }

    /// `holder` takes the request for `ttl` seconds (a minute when it does not say).
    ///
    /// A request nobody has taken is taken as its first attempt. One an executor let go of
    /// (its lease ran out) is taken again only when `resume` says so, as the next attempt:
    /// the one before it is fenced out of everything it says from then on.
    pub fn claim_request(
        &self,
        id: &WorkId,
        request: &str,
        holder: &str,
        ttl: Option<u64>,
        resume: bool,
    ) -> Result<RequestView, StoreError> {
        self.claim_request_for(id, request, holder, ttl, resume, None)
    }

    /// The same, by an executor that runs for the connection `offered`, or for none. A request
    /// is taken by the executor of the connection it was made for and by no other
    /// (`wrong-connection`).
    pub fn claim_request_for(
        &self,
        id: &WorkId,
        request: &str,
        holder: &str,
        ttl: Option<u64>,
        resume: bool,
        offered: Option<&ConnectionRef>,
    ) -> Result<RequestView, StoreError> {
        let ttl = checked_lease(ttl)?;
        checked_holder(holder)?;
        self.change_request(id, request, |current, now| {
            current.claim_for(holder, ttl, resume, offered, now)
        })
    }

    /// The executor says it is still there, and keeps the request for `ttl` more seconds.
    pub fn heartbeat_request(
        &self,
        id: &WorkId,
        request: &str,
        holder: &str,
        attempt: u32,
        ttl: Option<u64>,
    ) -> Result<RequestView, StoreError> {
        let ttl = checked_lease(ttl)?;
        checked_holder(holder)?;
        self.change_request(id, request, |current, now| {
            current.heartbeat(holder, attempt, ttl, now)
        })
    }

    /// The executor says it could not, and why.
    pub fn fail_request(
        &self,
        id: &WorkId,
        request: &str,
        holder: &str,
        attempt: u32,
        reason: &str,
    ) -> Result<RequestView, StoreError> {
        checked_holder(holder)?;
        self.change_request(id, request, |current, now| {
            current.fail(holder, attempt, reason, now)
        })
    }

    /// The owner calls the request off. Whoever holds it finds out at its next word.
    pub fn cancel_request(&self, id: &WorkId, request: &str) -> Result<RequestView, StoreError> {
        self.change_request(id, request, |current, now| current.cancel(now))
    }

    /// Apply `change` to the request, under the lock a save takes.
    ///
    /// A lease that ran out is written down first when the change goes through, so that the
    /// history says the request was let go of before it says what was done to it. A change
    /// that is refused writes nothing: the request is read as interrupted at the next look
    /// anyway, by the clock.
    fn change_request(
        &self,
        id: &WorkId,
        request: &str,
        change: impl FnOnce(&Request, &Moment) -> Result<Request, Refusal>,
    ) -> Result<RequestView, StoreError> {
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        let current = if valid_request_id(request) {
            load_request(&dir, request)?
        } else {
            None
        };
        let Some(current) = current else {
            return Err(not_found(id, request));
        };
        let now = Moment::of(&self.clock);
        // A bundle published and not written down is written down first: a request it made
        // is not called off, or taken again, as if it had not.
        let current = heal(&dir, current, &now)?;
        let next = change(&current, &now).map_err(|refusal| refusal_error(&current, refusal))?;
        if let Some(written_down) = current.normalized(&now) {
            persist(&dir, Some(&current), &written_down, &now)?;
            persist(&dir, Some(&written_down), &next, &now)?;
        } else {
            persist(&dir, Some(&current), &next, &now)?;
        }
        Ok(RequestView {
            state: next.effective(now.epoch),
            request: next,
        })
    }

    /// A save of the text or of the answers has just moved a pointer, under the lock: the
    /// open requests that were asked about what it replaced are ended.
    ///
    /// What the executor itself saves (a model, a requirement list) moves nothing a request
    /// was asked about. And this never fails the save: the pointer has moved, the owner's
    /// writing is kept, and a request file that cannot be read is no reason to tell them
    /// otherwise. A request that outlives its inputs this way is caught when it is completed.
    pub(super) fn supersede_moved(&self, dir: &Path, artifact: Artifact, revision: &Revision) {
        let what = match artifact {
            Artifact::Source => "the text was saved after the request was made",
            Artifact::Answers => "the answers were saved after the request was made",
            _ => return,
        };
        let Ok(requests) = load_requests(dir) else {
            return;
        };
        let now = Moment::of(&self.clock);
        // A request the current bundle says is done is done, not open: it is not ended
        // because the text moved after it made its bundle.
        let requests: Vec<Request> = requests
            .into_iter()
            .filter_map(|stored| heal(dir, stored, &now).ok())
            .collect();
        for request in requests.iter().filter(|r| r.state.is_open()) {
            let pinned = match artifact {
                Artifact::Source => Some(&request.inputs.source),
                _ => request.inputs.answers.as_ref(),
            };
            if pinned == Some(revision) {
                continue;
            }
            if let Some(ended) = request.supersede(what, &now) {
                // Best effort, for the reason above.
                let _ = persist(dir, Some(request), &ended, &now);
            }
        }
    }
}

pub(super) fn not_found(id: &WorkId, request: &str) -> StoreError {
    StoreError::NotFound {
        what: format!("request `{request}` of work `{id}`"),
    }
}

fn checked_lease(ttl: Option<u64>) -> Result<u64, StoreError> {
    let seconds = ttl.unwrap_or(LEASE_DEFAULT_SECONDS);
    if lease_is_valid(seconds) {
        Ok(seconds)
    } else {
        Err(bad(
            "bad-lease",
            &format!(
                "a lease of {seconds} seconds cannot be kept or is not worth holding: ask for {} to {}",
                crate::requests::LEASE_MIN_SECONDS,
                crate::requests::LEASE_MAX_SECONDS
            ),
            json!({
                "min": crate::requests::LEASE_MIN_SECONDS,
                "max": crate::requests::LEASE_MAX_SECONDS,
            }),
        ))
    }
}

pub(super) fn checked_holder(holder: &str) -> Result<(), StoreError> {
    if valid_name(holder) {
        Ok(())
    } else {
        Err(bad(
            "bad-holder",
            "an executor is named by 1 to 64 letters, digits and `._:-`",
            json!(null),
        ))
    }
}
