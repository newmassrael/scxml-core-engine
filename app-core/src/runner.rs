// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Hosting an executor: find the requests nobody has taken, take one, keep it while a
//! generator writes, and say how it came out.
//!
//! The works folder keeps requests and what comes of them (`requests.rs`, `bundle.rs`); a
//! [`Generator`] is whatever writes a model from a specification (an AI client, a script in a
//! test); this is what stands between the two. It is the same executor an authoring client in
//! a person's own terminal is (`tools/authoring`, `works_begin_generation`), with the same
//! words to the same folder, and it differs in what it is: a process of the application, that
//! the owner asked to be there, rather than a client the owner is typing to.
//!
//! - **It takes what was asked and nothing else.** A request is taken when it is `queued`. One
//!   a lease let go of is the owner's to ask again for: a run costs something, and nothing here
//!   spends it unasked.
//! - **It keeps what it takes.** A generator can take longer than a lease, and never says it is
//!   still there, so the runner does, from a thread of its own, and says in the same breath that
//!   the adapter is there. A renewal that is refused because the request ended (the owner called
//!   it off, or the text it was asked about was saved) stops the generator at its next look
//!   ([`Cancel`]) and nothing is renewed for the request again.
//! - **What it writes is a candidate, and the core decides.** The draft is saved for the request
//!   and completed through the same step the `complete_request` command takes
//!   ([`complete_generation`]): the core checks the model itself. A model it refuses comes back
//!   with the product's own records, the generator is given them and writes again, a few times,
//!   and a request that never gets a model the core accepts fails with what it last said.
//! - **A request that ended under it is not failed.** What the owner or the text did to it is
//!   not the generator's failure, and a runner that told the owner it had failed would be
//!   contradicting what the owner just did.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::answers::Answers;
use crate::clock::Clock;
use crate::commands::{complete_generation, CommandError};
use crate::connection::ConnectionId;
use crate::error::StoreError;
use crate::model_set::ModelFiles;
use crate::requests::{ConnectionRef, Pin, State};
use crate::requirements::Requirements;
use crate::review::Product;
use crate::revision::Revision;
use crate::store::{AdapterReport, CandidateWrite, RequestView, WorkId, WorkStore};

/// What a refusal of the request's own state is called when the request is no longer the
/// runner's to work on.
const ENDED_KINDS: [&str; 4] = ["request-ended", "not-holder", "request-held", "not-found"];

/// A word to a generator that its work is no longer wanted: the owner called the request off,
/// or the text it was asked about moved, or the application is closing. Cloned freely; one
/// `cancel` is seen by every clone.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// What a generator is asked to write a model from.
#[derive(Debug, Clone)]
pub struct Job {
    pub work: WorkId,
    pub title: String,
    pub request: String,
    /// Which attempt of the request this is the work of.
    pub attempt: u32,
    /// The text the request is about, as it was when the request was made.
    pub source: String,
    pub source_revision: Revision,
    /// The owner's answers to what an earlier model left open, by question id. What the
    /// request was made about: an answer given later makes another request.
    pub answers: BTreeMap<String, String>,
    /// The model and the list the work has now, to be written from again and not from nothing.
    pub previous: Option<Previous>,
    /// What the core said of the last draft it refused, in the product's words: set from the
    /// second try, and what the draft has to answer.
    pub refusal: Option<String>,
    /// The owner asked for every requirement of the list to be issued a new id (ADR 0012): the
    /// list is built with `fresh`, and one that carries an id is refused before it is published.
    pub fresh_ids: bool,
}

/// The model and the requirement list a work had before the request.
#[derive(Debug, Clone)]
pub struct Previous {
    pub model: ModelFiles,
    pub requirements: Option<Requirements>,
}

/// What a generator wrote: the model, and the requirement list read from the same text.
#[derive(Debug, Clone)]
pub struct Draft {
    pub model: ModelFiles,
    pub requirements: Requirements,
}

/// Why a generator did not write a draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    /// It was told to stop ([`Cancel`]) and did.
    Cancelled,
    /// It could not, and says why in a sentence the owner reads.
    Failed(String),
    /// It wrote something that is not a draft (a list that is not JSON, a set whose documents
    /// name each other wrongly). Not the end: it is told what was wrong, as it is told what the
    /// core refused, and writes again.
    Unusable(String),
}

/// Whatever writes a model from a specification.
pub trait Generator: Send + Sync {
    /// Which kind of client it runs (`claude-code`): what the owner's screen names.
    fn kind(&self) -> &str;

    fn version(&self) -> Option<String> {
        None
    }

    /// The version of the working instructions it gives the client (`claude-code/<digest>`),
    /// which changes when the wording does: a bundle records it, so that a result can be told
    /// apart from one made to other instructions.
    fn instructions(&self) -> Option<String> {
        None
    }

    /// What it can do; the screen offers only what is here (`cancel`, for a generator that
    /// stops when told to).
    fn capabilities(&self) -> Vec<String> {
        vec!["generate".to_string(), "cancel".to_string()]
    }

    /// Write a draft for `job`, looking at `cancel` as often as it can: a run that goes on
    /// after the request ended is spent for nothing.
    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError>;
}

/// Why a request pinned to a connection is not one a runner can run, in a sentence a person
/// reads: there is no adapter for that kind of connection in this build, the way it signs in is
/// not one the build uses, nobody has signed in, the settings it was made with are not on this
/// computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrunnable {
    pub reason: String,
}

impl Unrunnable {
    pub fn new(reason: impl Into<String>) -> Self {
        Unrunnable {
            reason: reason.into(),
        }
    }
}

/// Where a runner finds the generator for the connection a request was made for.
///
/// A request pinned to a connection is taken by the executor of that connection and by no other,
/// so a runner does not take it with whatever generator it holds. The directory knows how to run
/// each kind of connection (it reads the settings at the revision the request was made with,
/// asks the client who is signed in, and judges the way of signing in by the build's table), and
/// says why when it cannot.
pub trait Directory: Send + Sync {
    fn generator_for(&self, pin: &Pin) -> Result<Arc<dyn Generator>, Unrunnable>;
}

/// A request the runner took, with the generator that writes for it.
type Taken = (WorkId, RequestView, Arc<dyn Generator>);

/// A request the runner left queued at its last look, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    pub work: WorkId,
    pub request: String,
    pub connection: ConnectionId,
    pub reason: String,
}

/// How a runner holds its requests.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// The name every request is taken under, and the adapter is reported as.
    pub name: String,
    /// How long a claim lasts without a renewal.
    pub lease_seconds: u64,
    /// How often the claim and the adapter's presence are renewed: a third of the lease, so
    /// that two renewals can fail before the request is read as let go of.
    pub heartbeat: Duration,
    /// How many more times a model the core refused is written again.
    pub repairs: u32,
    /// How long to wait for a request when there is none.
    pub poll: Duration,
    /// The vocabulary SCE words its records in; its default when absent.
    pub lexicon: Option<String>,
}

impl RunnerConfig {
    pub fn named(name: impl Into<String>) -> Self {
        RunnerConfig {
            name: name.into(),
            lease_seconds: 60,
            heartbeat: Duration::from_secs(20),
            repairs: 2,
            poll: Duration::from_secs(2),
            lexicon: None,
        }
    }
}

/// How one request came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// No request was waiting to be taken.
    NothingToDo,
    /// The core accepted the model and published it with the list.
    Completed {
        work: WorkId,
        request: String,
        bundle: Revision,
    },
    /// The request could not be done, and the owner was told why.
    Failed {
        work: WorkId,
        request: String,
        reason: String,
    },
    /// The request stopped being the runner's: the owner called it off, or the text it was
    /// asked about was saved. Nothing was failed.
    Lost {
        work: WorkId,
        request: String,
        kind: String,
    },
    /// The runner was told to stop while it worked. The request is left to run out its lease,
    /// which is how the owner reads that its executor went away.
    Stopped { work: WorkId, request: String },
}

/// Finds requests, takes them, and has a [`Generator`] write for them.
pub struct Runner<C: Clock, G> {
    store: Arc<WorkStore<C>>,
    product: Arc<dyn Product>,
    /// What writes for a request nobody chose a connection for.
    generator: Arc<G>,
    /// Where the generator for a pinned request is found; a runner without one leaves every
    /// pinned request alone.
    directory: Option<Arc<dyn Directory>>,
    /// Whether a request nobody chose a connection for is this runner's to write. It is for a
    /// runner that knows no connections (one generator, as a command line tool or a test has);
    /// it is not for one that runs for connections, which does not guess which of them such a
    /// request was meant for and leaves it to an authoring client of the person's own.
    takes_unpinned: bool,
    /// What the last look left queued, and why.
    waiting: Mutex<Vec<Waiting>>,
    config: RunnerConfig,
    shutdown: Cancel,
}

impl<C, G> Runner<C, G>
where
    C: Clock + Send + Sync,
    G: Generator + 'static,
{
    pub fn new(
        store: Arc<WorkStore<C>>,
        product: Arc<dyn Product>,
        generator: Arc<G>,
        config: RunnerConfig,
    ) -> Self {
        Runner {
            store,
            product,
            generator,
            directory: None,
            takes_unpinned: true,
            waiting: Mutex::new(Vec::new()),
            config,
            shutdown: Cancel::new(),
        }
    }

    /// The same, running for the connections `directory` can run: a request pinned to one is
    /// written by the generator the directory gives for it. A request nobody chose a connection
    /// for is then left alone: which AI it was meant for is the person's to say, and no default is
    /// assigned to it.
    pub fn with_connections(mut self, directory: Arc<dyn Directory>) -> Self {
        self.directory = Some(directory);
        self.takes_unpinned = false;
        self
    }

    /// What the runner left queued at its last look, and why: the requests it could not run, for
    /// the screen to say, so that a person is not left waiting for an AI that will not come.
    pub fn waiting(&self) -> Vec<Waiting> {
        self.waiting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// A word that stops this runner: [`Self::run`] returns and a generator at work is
    /// told to stop. Ask for it before running.
    pub fn shutdown(&self) -> Cancel {
        self.shutdown.clone()
    }

    /// The same runner, stopped by `shutdown` and not by a word of its own: for a host that has
    /// one word for everything it runs, including a runner it only makes later.
    pub fn with_shutdown(mut self, shutdown: Cancel) -> Self {
        self.shutdown = shutdown;
        self
    }

    /// Say the adapter is there, and what it can do. Said again by every renewal, so that it
    /// is there for as long as the runner is and for a short while after.
    pub fn report(&self) -> Result<(), StoreError> {
        report_adapter(&self.store, self.generator.as_ref(), &self.config)
    }

    /// Take the oldest request that is waiting, have the generator write for it, and say how
    /// it came out; or [`Outcome::NothingToDo`].
    pub fn run_once(&self) -> Result<Outcome, StoreError> {
        self.report()?;
        let mut waiting = Vec::new();
        let taken = self.take_one(&mut waiting);
        *self.waiting.lock().unwrap_or_else(|e| e.into_inner()) = waiting;
        match taken? {
            Some((work, claimed, generator)) => self.work_on(&work, claimed, generator.as_ref()),
            None => Ok(Outcome::NothingToDo),
        }
    }

    /// Take the oldest request that is waiting and that this runner can run, with the generator
    /// that writes for it. A request pinned to a connection is taken by offering that connection,
    /// and only when the directory has a generator for it; the rest are left, and said in
    /// `waiting`.
    fn take_one(&self, waiting: &mut Vec<Waiting>) -> Result<Option<Taken>, StoreError> {
        for (work, view) in self.store.open_requests()? {
            if view.state != State::Queued {
                continue;
            }
            let (offered, generator): (Option<ConnectionRef>, Arc<dyn Generator>) =
                match &view.request.pin {
                    None if !self.takes_unpinned => continue,
                    None => (None, Arc::clone(&self.generator) as Arc<dyn Generator>),
                    Some(pin) => {
                        let found = match &self.directory {
                            Some(directory) => directory.generator_for(pin),
                            None => Err(Unrunnable::new(
                                "this executor runs for no connection, and the request was made \
                                 for one",
                            )),
                        };
                        match found {
                            Ok(generator) => (Some(pin.reference()), generator),
                            Err(why) => {
                                waiting.push(Waiting {
                                    work: work.clone(),
                                    request: view.request.id.clone(),
                                    connection: pin.connection.clone(),
                                    reason: why.reason,
                                });
                                continue;
                            }
                        }
                    }
                };
            match self.store.claim_request_for(
                &work,
                &view.request.id,
                &self.config.name,
                Some(self.config.lease_seconds),
                false,
                offered.as_ref(),
            ) {
                Ok(claimed) => return Ok(Some((work, claimed, generator))),
                // Somebody else took it, or the owner called it off, between looking and
                // taking: it is not this runner's, and the next one may be.
                Err(StoreError::Refused { .. }) => continue,
                Err(other) => return Err(other),
            }
        }
        Ok(None)
    }

    /// Run until told to stop ([`Self::shutdown`]): take requests as they are made. A store
    /// that cannot be read for a moment is waited out, not given up on.
    pub fn run(&self) {
        while !self.shutdown.is_cancelled() {
            let idle = match self.run_once() {
                Ok(Outcome::NothingToDo) | Err(_) => true,
                Ok(_) => false,
            };
            if idle {
                self.wait(self.config.poll);
            }
        }
    }

    /// Wait for `span`, or until told to stop.
    fn wait(&self, span: Duration) {
        let until = Instant::now() + span;
        while !self.shutdown.is_cancelled() {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return;
            }
            thread::sleep(left.min(Duration::from_millis(50)));
        }
    }

    /// The request is this runner's: renew it from a thread of its own while the generator
    /// writes, and see it to its end.
    fn work_on(
        &self,
        work: &WorkId,
        claimed: RequestView,
        generator: &dyn Generator,
    ) -> Result<Outcome, StoreError> {
        let request = claimed.request.id.clone();
        let attempt = claimed.request.attempt;
        let cancel = Cancel::new();
        let lost: Mutex<Option<String>> = Mutex::new(None);
        let (stop_beating, beating) = mpsc::channel::<()>();

        let (request_ref, cancel_ref, lost_ref) = (request.as_str(), &cancel, &lost);
        thread::scope(|scope| {
            // The receiver is the beating thread's own: it is not shared, so it moves.
            scope.spawn(move || {
                self.beat_until_told(&beating, work, request_ref, attempt, cancel_ref, lost_ref);
            });
            let outcome =
                self.write_for(work, request_ref, attempt, cancel_ref, lost_ref, generator);
            // The renewing stops with the work, before the scope waits for it.
            let _ = stop_beating.send(());
            outcome
        })
    }

    /// Renew the claim every `heartbeat`, and look at whether the runner was told to stop
    /// or the request ended in between.
    fn beat_until_told(
        &self,
        told: &mpsc::Receiver<()>,
        work: &WorkId,
        request: &str,
        attempt: u32,
        cancel: &Cancel,
        lost: &Mutex<Option<String>>,
    ) {
        let tick = Duration::from_millis(25).min(self.config.heartbeat);
        let mut last = Instant::now();
        loop {
            match told.recv_timeout(tick) {
                Err(RecvTimeoutError::Timeout) => {}
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
            }
            if self.shutdown.is_cancelled() {
                cancel.cancel();
            }
            if last.elapsed() < self.config.heartbeat {
                continue;
            }
            last = Instant::now();
            match self.store.heartbeat_request(
                work,
                request,
                &self.config.name,
                attempt,
                Some(self.config.lease_seconds),
            ) {
                Ok(_) => {}
                Err(StoreError::Refused { kind, .. }) if ENDED_KINDS.contains(&kind) => {
                    *lost.lock().unwrap_or_else(|e| e.into_inner()) = Some(kind.to_string());
                    cancel.cancel();
                    return;
                }
                // The folder did not answer this time: the next beat asks again, and the
                // lease runs out only if it keeps not answering.
                Err(_) => {}
            }
            // The adapter is there for as long as it is working.
            let _ = self.report();
        }
    }

    /// What the generator is asked, and what is done with what it writes.
    fn write_for(
        &self,
        work: &WorkId,
        request: &str,
        attempt: u32,
        cancel: &Cancel,
        lost: &Mutex<Option<String>>,
        generator: &dyn Generator,
    ) -> Result<Outcome, StoreError> {
        let holder = self.config.name.as_str();
        let mut job = match self.job_for(work, request, attempt) {
            Ok(job) => job,
            Err(reason) => return self.fail(work, request, attempt, reason),
        };
        let mut last_refusal = String::new();
        for _ in 0..=self.config.repairs {
            let draft = match generator.generate(&job, cancel) {
                Ok(draft) => draft,
                Err(GenerateError::Failed(reason)) => {
                    return self.fail(work, request, attempt, reason)
                }
                Err(GenerateError::Cancelled) => return Ok(self.ended(work, request, lost)),
                Err(GenerateError::Unusable(wrong)) => {
                    last_refusal = wrong.clone();
                    job.refusal = Some(wrong);
                    continue;
                }
            };
            if cancel.is_cancelled() {
                return Ok(self.ended(work, request, lost));
            }
            let written = self.store.save_candidate(
                work,
                request,
                holder,
                attempt,
                CandidateWrite {
                    model: Some(draft.model.stored_text()),
                    requirements: Some(draft.requirements.stored_text()),
                    instructions: generator.instructions(),
                },
            );
            match written {
                Ok(_) => {}
                Err(StoreError::Refused { kind, .. }) if ENDED_KINDS.contains(&kind) => {
                    return Ok(Outcome::Lost {
                        work: work.clone(),
                        request: request.to_string(),
                        kind: kind.to_string(),
                    })
                }
                Err(other) => return Err(other),
            }
            match complete_generation(
                &self.store,
                &*self.product,
                work,
                request,
                holder,
                attempt,
                Vec::new(),
                self.config.lexicon.as_deref(),
            ) {
                Ok(published) => {
                    return Ok(Outcome::Completed {
                        work: work.clone(),
                        request: request.to_string(),
                        bundle: published.bundle,
                    })
                }
                // What the core refused the draft for, and the draft can be written again from:
                // a model SCE refused, and a list that carries an id the owner asked to retire.
                // Any other refusal is the work's, not the draft's, and fails the request.
                Err(refused)
                    if refused.kind == "check-refused"
                        || refused.kind == "fresh-ids-not-issued" =>
                {
                    last_refusal = refusal_text(&refused);
                    job.refusal = Some(last_refusal.clone());
                }
                Err(refused) if ENDED_KINDS.contains(&refused.kind.as_str()) => {
                    return Ok(Outcome::Lost {
                        work: work.clone(),
                        request: request.to_string(),
                        kind: refused.kind,
                    })
                }
                Err(other) => return self.fail(work, request, attempt, other.message),
            }
        }
        self.fail(
            work,
            request,
            attempt,
            format!(
                "no draft the core accepted in {} try(ies); the last thing said of it was: {last_refusal}",
                self.config.repairs + 1
            ),
        )
    }

    /// What the generator is asked, from the request: the text and the answers it was made
    /// about, and the model the work had.
    fn job_for(&self, work: &WorkId, request: &str, attempt: u32) -> Result<Job, String> {
        let said = |e: StoreError| e.to_string();
        let view = self.store.read_request(work, request).map_err(said)?;
        let inputs = &view.request.inputs;
        let source = self
            .store
            .read_source(work, Some(&inputs.source))
            .map_err(said)?
            .ok_or_else(|| "the text the request is about is not there".to_string())?;
        let answers = match &inputs.answers {
            None => BTreeMap::new(),
            Some(revision) => {
                let saved = self
                    .store
                    .read_answers(work, Some(revision))
                    .map_err(said)?
                    .ok_or_else(|| "the answers the request is about are not there".to_string())?;
                Answers::parse(&saved.text)
                    .map_err(|e| e.to_string())?
                    .entries()
                    .iter()
                    .map(|(id, entry)| (id.clone(), entry.answer.clone()))
                    .collect()
            }
        };
        let previous = match self.store.read_model(work, None).map_err(said)? {
            Some(model) => ModelFiles::parse(&model.text).ok().map(|files| Previous {
                model: files,
                requirements: self
                    .store
                    .read_requirements(work, None)
                    .ok()
                    .flatten()
                    .and_then(|list| Requirements::parse(&list.text).ok()),
            }),
            None => None,
        };
        Ok(Job {
            work: work.clone(),
            title: self
                .store
                .read_work_snapshot(work)
                .map_err(said)?
                .work
                .title,
            request: request.to_string(),
            attempt,
            source: source.text,
            source_revision: source.revision,
            answers,
            previous,
            refusal: None,
            fresh_ids: view.request.fresh_ids,
        })
    }

    /// Say the request could not be done. A request that ended in the meantime is not failed:
    /// what ended it is not the generator's failure.
    fn fail(
        &self,
        work: &WorkId,
        request: &str,
        attempt: u32,
        reason: String,
    ) -> Result<Outcome, StoreError> {
        match self
            .store
            .fail_request(work, request, &self.config.name, attempt, &reason)
        {
            Ok(_) => Ok(Outcome::Failed {
                work: work.clone(),
                request: request.to_string(),
                reason,
            }),
            Err(StoreError::Refused { kind, .. }) if ENDED_KINDS.contains(&kind) => {
                Ok(Outcome::Lost {
                    work: work.clone(),
                    request: request.to_string(),
                    kind: kind.to_string(),
                })
            }
            Err(other) => Err(other),
        }
    }

    /// The generator stopped because it was told to: by the request ending, or by the runner
    /// being stopped.
    fn ended(&self, work: &WorkId, request: &str, lost: &Mutex<Option<String>>) -> Outcome {
        match lost.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            Some(kind) => Outcome::Lost {
                work: work.clone(),
                request: request.to_string(),
                kind,
            },
            None => Outcome::Stopped {
                work: work.clone(),
                request: request.to_string(),
            },
        }
    }
}

fn report_adapter<C: Clock, G: Generator + ?Sized>(
    store: &WorkStore<C>,
    generator: &G,
    config: &RunnerConfig,
) -> Result<(), StoreError> {
    let version = generator.version();
    store
        .report_adapter(AdapterReport {
            name: &config.name,
            kind: generator.kind(),
            capabilities: generator.capabilities(),
            version: version.as_deref(),
        })
        .map(|_| ())
}

/// What the core said of a draft it refused, in the words an executor can write again from:
/// the sentence, and each record the product wrote with where it found it.
fn refusal_text(refused: &CommandError) -> String {
    let mut text = refused.message.clone();
    if let Some(records) = refused.detail["records"].as_array() {
        for record in records {
            let code = record["code"].as_str().unwrap_or("record");
            let message = record["message"].as_str().unwrap_or("");
            match record["line"].as_u64() {
                Some(line) => text.push_str(&format!("\n- {code} (line {line}): {message}")),
                None => text.push_str(&format!("\n- {code}: {message}")),
            }
        }
    }
    text
}
