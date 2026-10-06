// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A generation request: one ask of an AI to write a model from a work's text.
//!
//! This module is the request alone and how it moves, with no folder and no clock of its
//! own: every function takes the moment it is to act at and answers the request that
//! results, or why it would not. The store (`store.rs`) is what keeps requests, under the
//! lock a save takes, and it is where two callers meet; what is decided here is only what
//! one request does when asked.
//!
//! ## The lease
//!
//! An executor (an AI adapter, or the authoring server of a person's own terminal) takes a
//! request by claiming it, and says it is still there by renewing the claim. The claim is a
//! LEASE: it runs out. A lease that ran out is a suspicion that the executor is gone, not a
//! revocation. A laptop that slept past it and wakes with the run finished has done work
//! that cost something, and nobody else took the request in the meantime. So what revokes a
//! claim is a thing that was done: the request was cancelled, its inputs moved
//! ([`Request::supersede`]), or somebody claimed it again ([`Request::claim`] with `resume`,
//! which starts a new ATTEMPT). An executor names the attempt it holds, and an attempt that
//! is not the current one is told so and writes nothing: the number is the fence.
//!
//! The request is never taken again unasked. An interrupted request is taken only by a
//! caller that says it is resuming, because a run is not free and the next one is the
//! person's to ask for.

use serde::{Deserialize, Serialize};

use crate::connection::{AdapterKind, ConnectionId, Limits};
use crate::revision::Revision;

/// The lease a claim is given when it does not ask for another, in seconds.
pub const LEASE_DEFAULT_SECONDS: u64 = 60;
/// The shortest lease a claim may ask for: a lease shorter than a renewal can be sent is
/// one that cannot be kept.
pub const LEASE_MIN_SECONDS: u64 = 10;
/// The longest: a lease is how long a request is held for an executor that has gone, so a
/// long one is a long wait for nothing.
pub const LEASE_MAX_SECONDS: u64 = 900;

/// Where a request is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// Registered, and nobody has taken it.
    Queued,
    /// An executor holds it.
    Running,
    /// The executor said it is done.
    Completed,
    /// The executor said it could not.
    Failed,
    /// The owner called it off.
    Cancelled,
    /// The executor's lease ran out, and nobody has said what became of it.
    Interrupted,
    /// Its inputs moved, or a newer request replaced it.
    Superseded,
}

impl State {
    /// Whether the request can still produce a result: queued, held, or let go of by an
    /// executor that may come back. At most one request of a work is open.
    pub fn is_open(self) -> bool {
        matches!(self, State::Queued | State::Running | State::Interrupted)
    }

    /// The word a program branches on.
    pub fn word(self) -> &'static str {
        match self {
            State::Queued => "queued",
            State::Running => "running",
            State::Completed => "completed",
            State::Failed => "failed",
            State::Cancelled => "cancelled",
            State::Interrupted => "interrupted",
            State::Superseded => "superseded",
        }
    }
}

/// The moment something is done at: the seconds a lease is measured in, and the text a
/// person reads in the history. One value, so a request never records two different
/// moments for one act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moment {
    pub epoch: u64,
    pub text: String,
}

impl Moment {
    pub fn of(clock: &impl crate::clock::Clock) -> Self {
        // The text is written from the seconds that were read, not read a second time:
        // two reads of a clock are two moments.
        let epoch = clock.epoch();
        Moment {
            epoch,
            text: crate::clock::utc_timestamp(epoch),
        }
    }
}

/// The saved revisions a request was registered against. They are fixed when the request
/// is made: an executor that reads the work later and finds it changed is working from
/// something else than what was asked, and the request says so instead of being re-aimed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inputs {
    pub source: Revision,
    /// `None` when the owner had answered nothing.
    #[serde(default)]
    pub answers: Option<Revision>,
}

/// A connection as an executor or a screen names it: which one, and the revision it has seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionRef {
    pub id: ConnectionId,
    pub revision: Revision,
}

/// The connection a request was made for, as it was when the request was made.
///
/// The core copies it from the person's settings and not the screen: a request that carried
/// whatever its caller said would carry whatever its caller chose. It keeps what a run needs
/// to be the same run (which client, which model, what it may spend) and nothing that says
/// where or as whom (a server's address, what the person calls it, the path of a program):
/// the works folder is shared, and an executor that needs those reads the connection at this
/// revision, from the settings of whoever is running it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    pub connection: ConnectionId,
    pub revision: Revision,
    pub adapter: AdapterKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Limits::is_empty")]
    pub limits: Limits,
}

impl Pin {
    /// The connection this pins, as an executor offers it.
    pub fn reference(&self) -> ConnectionRef {
        ConnectionRef {
            id: self.connection.clone(),
            revision: self.revision.clone(),
        }
    }
}

/// An executor's claim on a request, and the moment it runs out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub holder: String,
    /// Which attempt this claim is: the fence an executor names with everything it says.
    pub attempt: u32,
    /// Seconds since the Unix epoch.
    pub granted_at: u64,
    pub expires_at: u64,
}

/// Why a request would not do what it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The request ended, so it takes nothing more (`state` says how).
    Ended { state: State },
    /// An executor holds it, and its lease has not run out.
    Held { holder: String, until: u64 },
    /// The caller is not the executor of the current attempt, or names an attempt that is
    /// not the current one. `holder` and `attempt` say who is.
    NotHolder {
        holder: Option<String>,
        attempt: u32,
    },
    /// The request was let go of; it is taken again only by a caller that says it resumes.
    NotResuming,
    /// The request is for another connection than the one the caller offers: a person who
    /// chose one AI is not answered by another. A request with no connection is taken by a
    /// caller that offers none.
    WrongConnection {
        pinned: Option<ConnectionRef>,
        offered: Option<ConnectionRef>,
    },
}

/// What an executor has written for the request so far, by revision. The texts are kept
/// where every revision of a model and of a requirement list is, named by what they hold;
/// what the request keeps is which ones it means. Nothing here is the work's model: a
/// candidate becomes it only when the request is published, as one bundle with the other.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    #[serde(default)]
    pub model: Option<Revision>,
    #[serde(default)]
    pub requirements: Option<Revision>,
    /// The version of the working instructions the executor was given, in its own words
    /// (`claude-code/<digest>`): what a bundle says it was made to, so that a result can be
    /// told apart from one made to other instructions. An executor that says nothing of them
    /// has none recorded.
    #[serde(default)]
    pub instructions: Option<String>,
}

impl Candidate {
    /// What is still missing for a bundle, by name: a bundle is a model and the requirement
    /// list read out of the same text, and one without the other is not one.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.model.is_none() {
            missing.push("model");
        }
        if self.requirements.is_none() {
            missing.push("requirements");
        }
        missing
    }
}

/// What a completed request made: the bundle it published.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub bundle: Revision,
}

/// One generation request, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    /// The order requests of a work were made in, from 1.
    pub seq: u64,
    /// What the caller said to make a repeated registration the same request.
    pub key: String,
    /// Where it was asked from: `gui`, or the name of a client.
    pub origin: String,
    pub inputs: Inputs,
    pub created_at: String,
    /// The state as last written. A running request whose lease ran out is read as
    /// interrupted ([`Request::effective`]) before anything writes that down.
    pub state: State,
    /// How many times an executor has taken it.
    pub attempt: u32,
    /// The last executor's claim, kept after the request ends so that a repeated
    /// completion of the same attempt can be told from another's.
    #[serde(default)]
    pub lease: Option<Lease>,
    #[serde(default)]
    pub ended_at: Option<String>,
    /// Why it ended or was let go of, in a person's words.
    #[serde(default)]
    pub note: Option<String>,
    /// What the executor has written for it so far; `None` until it writes something.
    #[serde(default)]
    pub candidate: Option<Candidate>,
    /// The bundle a completed request published; `None` for every request that did not.
    #[serde(default)]
    pub outcome: Option<Outcome>,
    /// The connection it was made for. `None` for a request nobody chose a connection for (one
    /// an AI client asks for through the authoring server), which an executor that offers no
    /// connection takes; and for every request written before connections existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<Pin>,
}

/// Whether `seconds` is a lease a claim may ask for.
pub fn lease_is_valid(seconds: u64) -> bool {
    (LEASE_MIN_SECONDS..=LEASE_MAX_SECONDS).contains(&seconds)
}

impl Request {
    /// A request registered at `now` and nobody has taken.
    pub fn queued(
        id: String,
        seq: u64,
        key: String,
        origin: String,
        inputs: Inputs,
        pin: Option<Pin>,
        now: &Moment,
    ) -> Self {
        Request {
            id,
            seq,
            key,
            origin,
            inputs,
            created_at: now.text.clone(),
            state: State::Queued,
            attempt: 0,
            lease: None,
            ended_at: None,
            note: None,
            candidate: None,
            outcome: None,
            pin,
        }
    }

    /// The state as it is at `now`: what is written, except that a running request whose
    /// lease ran out is interrupted. Reading never writes, so this is how a request that
    /// nobody has touched since its executor went away is seen to have been let go of.
    pub fn effective(&self, now: u64) -> State {
        match (&self.state, &self.lease) {
            (State::Running, Some(lease)) if lease.expires_at <= now => State::Interrupted,
            (state, _) => *state,
        }
    }

    /// The request with that written down, when it is not already.
    pub fn normalized(&self, now: &Moment) -> Option<Request> {
        (self.state == State::Running && self.effective(now.epoch) == State::Interrupted).then(
            || Request {
                state: State::Interrupted,
                note: Some("the executor stopped renewing its lease".to_string()),
                ..self.clone()
            },
        )
    }

    /// `holder` takes the request for `ttl` seconds.
    ///
    /// A queued request is taken as the first attempt. An interrupted one is taken as the
    /// next attempt, and only by a caller that says it `resume`s. One held under a lease
    /// that has not run out is not taken, by anyone: the executor that holds it renews.
    pub fn claim(
        &self,
        holder: &str,
        ttl: u64,
        resume: bool,
        now: &Moment,
    ) -> Result<Request, Refusal> {
        self.claim_for(holder, ttl, resume, None, now)
    }

    /// The same, by an executor that runs for the connection `offered`, or for none.
    ///
    /// A request is taken by the executor of the connection it was made for and by nobody
    /// else: a request pinned to a connection is refused to a caller that offers another, or
    /// none, and a request with none is refused to a caller that offers one. That is said
    /// before anything about the request's state, because it is what the caller can act on.
    pub fn claim_for(
        &self,
        holder: &str,
        ttl: u64,
        resume: bool,
        offered: Option<&ConnectionRef>,
        now: &Moment,
    ) -> Result<Request, Refusal> {
        let pinned = self.pin.as_ref().map(Pin::reference);
        if pinned.as_ref() != offered {
            return Err(Refusal::WrongConnection {
                pinned,
                offered: offered.cloned(),
            });
        }
        let attempt = match self.effective(now.epoch) {
            State::Queued => self.attempt + 1,
            State::Interrupted if resume => self.attempt + 1,
            State::Interrupted => return Err(Refusal::NotResuming),
            State::Running => {
                let lease = self.lease.as_ref();
                return Err(Refusal::Held {
                    holder: lease.map(|l| l.holder.clone()).unwrap_or_default(),
                    until: lease.map_or(0, |l| l.expires_at),
                });
            }
            ended => return Err(Refusal::Ended { state: ended }),
        };
        Ok(Request {
            state: State::Running,
            attempt,
            lease: Some(Lease {
                holder: holder.to_string(),
                attempt,
                granted_at: now.epoch,
                expires_at: now.epoch + ttl,
            }),
            note: None,
            ..self.clone()
        })
    }

    /// Whether `holder` is the executor of attempt `attempt`, the current one.
    ///
    /// A lease that ran out does not stop it being: nobody else has taken the request, and
    /// an attempt is only displaced by a claim or by the request ending.
    fn held_by(&self, holder: &str, attempt: u32) -> Result<(), Refusal> {
        match self.state {
            State::Completed | State::Failed | State::Cancelled | State::Superseded => {
                Err(Refusal::Ended { state: self.state })
            }
            State::Queued => Err(Refusal::NotHolder {
                holder: None,
                attempt: 0,
            }),
            State::Running | State::Interrupted => match &self.lease {
                Some(lease)
                    if lease.holder == holder
                        && lease.attempt == attempt
                        && attempt == self.attempt =>
                {
                    Ok(())
                }
                lease => Err(Refusal::NotHolder {
                    holder: lease.as_ref().map(|l| l.holder.clone()),
                    attempt: self.attempt,
                }),
            },
        }
    }

    /// The executor says it is still there, and keeps the request for `ttl` more seconds
    /// from `now`. A request whose lease had run out is running again.
    pub fn heartbeat(
        &self,
        holder: &str,
        attempt: u32,
        ttl: u64,
        now: &Moment,
    ) -> Result<Request, Refusal> {
        self.held_by(holder, attempt)?;
        let lease = self.lease.as_ref().expect("a held request has a lease");
        Ok(Request {
            state: State::Running,
            lease: Some(Lease {
                expires_at: now.epoch + ttl,
                ..lease.clone()
            }),
            note: None,
            ..self.clone()
        })
    }

    /// Whether this request already ended the way the executor now says, as the same attempt:
    /// a repeated completion is the same completion, not a second one.
    fn already(&self, state: State, holder: &str, attempt: u32) -> bool {
        self.state == state
            && self
                .lease
                .as_ref()
                .is_some_and(|l| l.holder == holder && l.attempt == attempt)
    }

    /// Whether `holder` speaks for the current attempt, as a request that is not theirs, or
    /// that ended, is refused before anything else about the call is looked at.
    pub fn check_held_by(&self, holder: &str, attempt: u32) -> Result<(), Refusal> {
        self.held_by(holder, attempt)
    }

    /// The executor wrote something for the request: what it names replaces what it named
    /// before, and what it leaves out stays. The executor says it as it says everything, so it
    /// is the executor of the current attempt or it is refused.
    pub fn with_candidate(
        &self,
        holder: &str,
        attempt: u32,
        written: Candidate,
    ) -> Result<Request, Refusal> {
        self.held_by(holder, attempt)?;
        let before = self.candidate.clone().unwrap_or_default();
        Ok(Request {
            candidate: Some(Candidate {
                model: written.model.or(before.model),
                requirements: written.requirements.or(before.requirements),
                instructions: written.instructions.or(before.instructions),
            }),
            ..self.clone()
        })
    }

    /// The executor says it is done and the bundle it made is the work's now. The same bundle
    /// said again by the same attempt is the same completion.
    pub fn publish(
        &self,
        holder: &str,
        attempt: u32,
        bundle: &Revision,
        now: &Moment,
    ) -> Result<Request, Refusal> {
        if self.already(State::Completed, holder, attempt) && self.outcome_is(bundle) {
            return Ok(self.clone());
        }
        self.held_by(holder, attempt)?;
        Ok(Request {
            outcome: Some(Outcome {
                bundle: bundle.clone(),
            }),
            ..self.ended(State::Completed, None, now)
        })
    }

    fn outcome_is(&self, bundle: &Revision) -> bool {
        self.outcome.as_ref().is_some_and(|o| &o.bundle == bundle)
    }

    /// The executor says it could not, and why.
    pub fn fail(
        &self,
        holder: &str,
        attempt: u32,
        reason: &str,
        now: &Moment,
    ) -> Result<Request, Refusal> {
        if self.already(State::Failed, holder, attempt) {
            return Ok(self.clone());
        }
        self.held_by(holder, attempt)?;
        Ok(self.ended(State::Failed, Some(reason.to_string()), now))
    }

    /// The owner calls it off. Whoever held it finds, at its next word, that the request
    /// ended. Cancelling a cancelled request is the same cancellation.
    pub fn cancel(&self, now: &Moment) -> Result<Request, Refusal> {
        match self.state {
            State::Cancelled => Ok(self.clone()),
            state if !state.is_open() => Err(Refusal::Ended { state }),
            _ => Ok(self.ended(
                State::Cancelled,
                Some("cancelled by the owner".to_string()),
                now,
            )),
        }
    }

    /// The request is replaced, or what it was asked about moved. `None` when it had
    /// already ended: there is nothing to replace.
    pub fn supersede(&self, why: &str, now: &Moment) -> Option<Request> {
        self.state
            .is_open()
            .then(|| self.ended(State::Superseded, Some(why.to_string()), now))
    }

    fn ended(&self, state: State, note: Option<String>, now: &Moment) -> Request {
        Request {
            state,
            ended_at: Some(now.text.clone()),
            note,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_791_190_800;

    fn at(epoch: u64) -> Moment {
        Moment {
            epoch,
            text: crate::clock::utc_timestamp(epoch),
        }
    }

    fn revision(text: &str) -> Revision {
        Revision::of(text.as_bytes())
    }

    fn queued() -> Request {
        Request::queued(
            "req-1".to_string(),
            1,
            "key-1".to_string(),
            "gui".to_string(),
            Inputs {
                source: revision("text"),
                answers: None,
            },
            None,
            &at(T0),
        )
    }

    fn running() -> Request {
        queued().claim("adapter-a", 60, false, &at(T0)).unwrap()
    }

    #[test]
    fn a_request_is_queued_until_an_executor_takes_it_and_is_then_its_first_attempt() {
        let request = queued();
        assert_eq!((request.state, request.attempt), (State::Queued, 0));
        assert!(request.lease.is_none());

        let taken = request.claim("adapter-a", 60, false, &at(T0)).unwrap();

        assert_eq!((taken.state, taken.attempt), (State::Running, 1));
        assert_eq!(
            taken.lease,
            Some(Lease {
                holder: "adapter-a".to_string(),
                attempt: 1,
                granted_at: T0,
                expires_at: T0 + 60,
            })
        );
    }

    #[test]
    fn a_request_held_under_a_lease_that_has_not_run_out_is_taken_by_nobody() {
        let held = running();

        for who in ["adapter-a", "adapter-b"] {
            let refused = held.claim(who, 60, true, &at(T0 + 59)).unwrap_err();
            assert_eq!(
                refused,
                Refusal::Held {
                    holder: "adapter-a".to_string(),
                    until: T0 + 60
                },
                "{who}"
            );
        }
    }

    #[test]
    fn a_lease_that_ran_out_is_an_interrupted_request_before_anything_writes_that_down() {
        let held = running();
        assert_eq!(held.effective(T0 + 59), State::Running);
        assert_eq!(held.effective(T0 + 60), State::Interrupted);
        assert_eq!(held.state, State::Running);

        let written = held.normalized(&at(T0 + 61)).expect("it ran out");

        assert_eq!(written.state, State::Interrupted);
        assert_eq!(written.effective(T0 + 61), State::Interrupted);
        assert!(
            held.normalized(&at(T0 + 10)).is_none(),
            "a live lease is not written down"
        );
        assert!(queued().normalized(&at(T0 + 1_000)).is_none());
    }

    #[test]
    fn an_executor_that_comes_back_after_its_lease_ran_out_is_still_the_executor() {
        // A laptop slept past the lease and wakes with the run done: nobody took the
        // request, so the run is not thrown away.
        let held = running();

        let renewed = held.heartbeat("adapter-a", 1, 60, &at(T0 + 500)).unwrap();

        assert_eq!(renewed.state, State::Running);
        assert_eq!(renewed.effective(T0 + 559), State::Running);
        assert_eq!(renewed.lease.as_ref().map(|l| l.expires_at), Some(T0 + 560));
        let late = held
            .publish("adapter-a", 1, &revision("bundle"), &at(T0 + 500))
            .unwrap();
        assert_eq!(late.state, State::Completed);
    }

    #[test]
    fn an_interrupted_request_is_taken_again_only_by_a_caller_that_resumes_it() {
        let written = running().normalized(&at(T0 + 100)).unwrap();

        assert_eq!(
            written.claim("adapter-b", 60, false, &at(T0 + 100)),
            Err(Refusal::NotResuming)
        );
        let resumed = written.claim("adapter-b", 60, true, &at(T0 + 100)).unwrap();

        assert_eq!((resumed.state, resumed.attempt), (State::Running, 2));
        assert_eq!(
            resumed.lease.as_ref().map(|l| l.holder.as_str()),
            Some("adapter-b")
        );
        // The same holds for one that was never written down: its lease ran out, and that is
        // what makes it interrupted, so it is taken as the next attempt.
        let unwritten = running()
            .claim("adapter-b", 60, true, &at(T0 + 100))
            .unwrap();
        assert_eq!(unwritten.attempt, 2);
        assert_eq!(
            running().claim("adapter-b", 60, false, &at(T0 + 100)),
            Err(Refusal::NotResuming)
        );
    }

    #[test]
    fn an_attempt_that_was_taken_again_is_fenced_out_of_everything_it_says() {
        let resumed = running()
            .normalized(&at(T0 + 100))
            .unwrap()
            .claim("adapter-b", 60, true, &at(T0 + 100))
            .unwrap();
        let fenced = Refusal::NotHolder {
            holder: Some("adapter-b".to_string()),
            attempt: 2,
        };

        assert_eq!(
            resumed.heartbeat("adapter-a", 1, 60, &at(T0 + 101)),
            Err(fenced.clone())
        );
        assert_eq!(
            resumed.publish("adapter-a", 1, &revision("bundle"), &at(T0 + 101)),
            Err(fenced.clone())
        );
        assert_eq!(
            resumed.fail("adapter-a", 1, "late", &at(T0 + 101)),
            Err(fenced)
        );
        // The same holder under the old attempt number is fenced too: the number is the fence.
        assert!(resumed
            .heartbeat("adapter-b", 1, 60, &at(T0 + 101))
            .is_err());
        assert!(resumed.heartbeat("adapter-b", 2, 60, &at(T0 + 101)).is_ok());
    }

    #[test]
    fn nobody_but_the_holder_of_the_current_attempt_renews_or_finishes_it() {
        let held = running();

        for refused in [
            held.heartbeat("adapter-b", 1, 60, &at(T0 + 1)),
            held.heartbeat("adapter-a", 2, 60, &at(T0 + 1)),
            held.publish("adapter-b", 1, &revision("bundle"), &at(T0 + 1)),
            held.fail("adapter-a", 7, "no", &at(T0 + 1)),
        ] {
            assert_eq!(
                refused,
                Err(Refusal::NotHolder {
                    holder: Some("adapter-a".to_string()),
                    attempt: 1
                })
            );
        }
        // A request nobody took has no holder to be.
        assert_eq!(
            queued().heartbeat("adapter-a", 1, 60, &at(T0)),
            Err(Refusal::NotHolder {
                holder: None,
                attempt: 0
            })
        );
    }

    #[test]
    fn a_completion_said_twice_by_the_same_attempt_is_one_completion() {
        let bundle = revision("bundle");
        let done = running()
            .publish("adapter-a", 1, &bundle, &at(T0 + 5))
            .unwrap();
        assert_eq!(done.state, State::Completed);
        assert_eq!(done.ended_at, Some(at(T0 + 5).text));

        let again = done.publish("adapter-a", 1, &bundle, &at(T0 + 9)).unwrap();

        assert_eq!(
            again, done,
            "the second saying changes nothing, the time included"
        );
        assert_eq!(
            done.publish("adapter-b", 1, &bundle, &at(T0 + 9)),
            Err(Refusal::Ended {
                state: State::Completed
            })
        );
        assert_eq!(
            done.heartbeat("adapter-a", 1, 60, &at(T0 + 9)),
            Err(Refusal::Ended {
                state: State::Completed
            })
        );
    }

    #[test]
    fn a_failure_says_why_and_is_said_once() {
        let failed = running()
            .fail("adapter-a", 1, "the model was refused", &at(T0 + 5))
            .unwrap();

        assert_eq!(failed.state, State::Failed);
        assert_eq!(failed.note.as_deref(), Some("the model was refused"));
        assert_eq!(
            failed
                .fail("adapter-a", 1, "other words", &at(T0 + 6))
                .unwrap(),
            failed
        );
    }

    #[test]
    fn a_cancelled_request_ends_whoever_holds_it_and_is_cancelled_once() {
        for before in [
            queued(),
            running(),
            running().normalized(&at(T0 + 100)).unwrap(),
        ] {
            let cancelled = before.cancel(&at(T0 + 100)).unwrap();

            assert_eq!(cancelled.state, State::Cancelled);
            assert_eq!(cancelled.cancel(&at(T0 + 200)).unwrap(), cancelled);
            assert_eq!(
                cancelled.heartbeat("adapter-a", 1, 60, &at(T0 + 101)),
                Err(Refusal::Ended {
                    state: State::Cancelled
                })
            );
            assert_eq!(
                cancelled.claim("adapter-b", 60, true, &at(T0 + 101)),
                Err(Refusal::Ended {
                    state: State::Cancelled
                })
            );
        }
        let done = running()
            .publish("adapter-a", 1, &revision("bundle"), &at(T0 + 5))
            .unwrap();
        assert_eq!(
            done.cancel(&at(T0 + 6)),
            Err(Refusal::Ended {
                state: State::Completed
            })
        );
    }

    #[test]
    fn a_superseded_request_ends_unless_it_already_had() {
        let superseded = running()
            .supersede("the text was saved", &at(T0 + 3))
            .unwrap();

        assert_eq!(superseded.state, State::Superseded);
        assert_eq!(superseded.note.as_deref(), Some("the text was saved"));
        assert_eq!(superseded.supersede("again", &at(T0 + 4)), None);
        assert!(queued().supersede("x", &at(T0)).is_some());
        assert_eq!(
            superseded.publish("adapter-a", 1, &revision("bundle"), &at(T0 + 4)),
            Err(Refusal::Ended {
                state: State::Superseded
            })
        );
    }

    #[test]
    fn which_states_are_open_and_the_words_a_program_branches_on() {
        let open: Vec<State> = [
            State::Queued,
            State::Running,
            State::Completed,
            State::Failed,
            State::Cancelled,
            State::Interrupted,
            State::Superseded,
        ]
        .into_iter()
        .filter(|s| s.is_open())
        .collect();
        assert_eq!(
            open,
            vec![State::Queued, State::Running, State::Interrupted]
        );
        assert_eq!(State::Interrupted.word(), "interrupted");
        assert_eq!(
            serde_json::to_string(&State::Superseded).unwrap(),
            "\"superseded\""
        );
    }

    #[test]
    fn a_lease_is_asked_for_within_the_limits() {
        assert!(!lease_is_valid(LEASE_MIN_SECONDS - 1));
        assert!(lease_is_valid(LEASE_MIN_SECONDS));
        assert!(lease_is_valid(LEASE_DEFAULT_SECONDS));
        assert!(lease_is_valid(LEASE_MAX_SECONDS));
        assert!(!lease_is_valid(LEASE_MAX_SECONDS + 1));
    }

    #[test]
    fn what_the_executor_writes_for_a_request_is_kept_by_revision_and_added_to() {
        let held = running();
        let model = revision("model");
        let list = revision("list");

        let first = held
            .with_candidate(
                "adapter-a",
                1,
                Candidate {
                    model: Some(model.clone()),
                    ..Candidate::default()
                },
            )
            .unwrap();
        assert_eq!(
            first.candidate.as_ref().unwrap().missing(),
            vec!["requirements"]
        );

        // A second word names the other half, and leaves the first where it was.
        let second = first
            .with_candidate(
                "adapter-a",
                1,
                Candidate {
                    requirements: Some(list.clone()),
                    ..Candidate::default()
                },
            )
            .unwrap();
        assert!(second.candidate.as_ref().unwrap().missing().is_empty());
        assert_eq!(second.candidate.as_ref().unwrap().model, Some(model));
        // A later model replaces the earlier one.
        let replaced = second
            .with_candidate(
                "adapter-a",
                1,
                Candidate {
                    model: Some(revision("model, again")),
                    ..Candidate::default()
                },
            )
            .unwrap();
        assert_eq!(
            replaced.candidate.unwrap().model,
            Some(revision("model, again"))
        );
        // It does not move the lease or the state.
        assert_eq!((second.state, second.lease), (held.state, held.lease));
    }

    #[test]
    fn only_the_holder_of_the_current_attempt_writes_for_a_request() {
        let held = running();
        let written = Candidate {
            model: Some(revision("model")),
            ..Candidate::default()
        };

        assert_eq!(
            held.with_candidate("adapter-b", 1, written.clone()),
            Err(Refusal::NotHolder {
                holder: Some("adapter-a".to_string()),
                attempt: 1
            })
        );
        assert_eq!(
            queued().with_candidate("adapter-a", 1, written.clone()),
            Err(Refusal::NotHolder {
                holder: None,
                attempt: 0
            })
        );
        let cancelled = held.cancel(&at(T0 + 1)).unwrap();
        assert_eq!(
            cancelled.with_candidate("adapter-a", 1, written),
            Err(Refusal::Ended {
                state: State::Cancelled
            })
        );
    }

    #[test]
    fn a_published_request_says_which_bundle_it_made_and_says_it_once() {
        let bundle = revision("bundle");

        let done = running()
            .publish("adapter-a", 1, &bundle, &at(T0 + 5))
            .unwrap();

        assert_eq!(done.state, State::Completed);
        assert_eq!(
            done.outcome,
            Some(Outcome {
                bundle: bundle.clone()
            })
        );
        assert_eq!(
            done.publish("adapter-a", 1, &bundle, &at(T0 + 9)).unwrap(),
            done
        );
        // Another bundle from the same attempt is not the completion it made.
        assert_eq!(
            done.publish("adapter-a", 1, &revision("another"), &at(T0 + 9)),
            Err(Refusal::Ended {
                state: State::Completed
            })
        );
        assert!(running().outcome.is_none() && running().candidate.is_none());
    }

    #[test]
    fn a_request_is_read_back_from_what_was_written() {
        let held = running();
        let text = serde_json::to_string(&held).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&text).unwrap(), held);
        // A file written without the optional fields (an older build, or by hand) is read.
        let minimal = r#"{"id":"r","seq":1,"key":"k","origin":"gui","created_at":"2026-10-05T09:00:00Z",
            "state":"queued","attempt":0,"inputs":{"source":"0000000000000000000000000000000000000000000000000000000000000000"}}"#;
        let read: Request = serde_json::from_str(minimal).unwrap();
        assert_eq!(
            (read.state, read.inputs.answers, read.lease),
            (State::Queued, None, None)
        );
    }
}
