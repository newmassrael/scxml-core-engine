// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The decisions of the HTTPS binding's dialing side (SCE_MESH.md section 22), as a state machine
//! that moves nothing: it has no thread, lock, clock, timer, random source or transport. A shell
//! gives [`ClientCore::step`] what happened, as an [`Input`], and carries out what it answers, as
//! [`Effect`]s; the native shell is the client of the module `client`.
//!
//! A step is a function of the core and its input alone, and both sides of it are plain data, so
//! the inputs of a run with the effects each gave are the run's trace, and the same inputs give
//! the same effects again. A trace holds each request whole, its `Authorization` among it: it is
//! as secret as the credential.
//!
//! **What the shell owes the core.** It gives one input at a time. It tells its host the
//! [`Effect`]s of a step in their order, and those of one step before those of the next. It does
//! what a lane is asked ([`Effect::Ask`]) one task at a time in the order asked, and gives back the
//! input each [`Task`] names. A [`Task::Stop`] ends a [`Task::Wake`] still being waited out; nothing
//! else is asked of a lane that has a task unanswered.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::mem;
use core::time::Duration;

use sce_rust_crypto::cose::TrustedKey;
use sce_rust_metrics::ExchangeOutcome;

use super::signature::{
    content_digest, field_value, verify_response, Answered, CONTENT_DIGEST_HEADER, NONCE_HEADER,
};
use super::{
    is_epoch, join_sequence, read_retry_after, split_sequence, HttpsFailure, HttpsRequest,
    HttpsResponse, LinkEvent, RetryAfter, ACK_HEADER, CONTENT_TYPE, EPOCH_HEADER, FIRST_HEADER,
    HOLD_HEADER,
};
use crate::reconnect::Backoff;

/// Headers, in order, each a name and its value.
pub(super) type Fields = Vec<(String, String)>;

/// What a core decides by: one client's binding, as the shell read it from its settings.
pub(super) struct CoreSettings {
    /// The server machine's name: the peer every [`LinkEvent`] is of.
    pub(super) server: String,
    /// The URL every exchange posts to.
    pub(super) url: String,
    /// The same URL as a signature covers it (`@target-uri`, SCE_MESH.md section 22.9).
    pub(super) target_uri: String,
    /// The `Authorization` header, when the credential is one.
    pub(super) authorization: Option<String>,
    /// Whether the client signs its requests (the `key` authenticator). The key is the shell's.
    pub(super) signs: bool,
    /// The keys a `200` is verified under, when the binding verifies what its server sends.
    pub(super) verify: Option<Vec<TrustedKey>>,
    /// How long a poll asks the server to hold it while nothing waits.
    pub(super) hold: Duration,
    /// How long an exchange may take beyond the hold it asks for before it is a timeout.
    pub(super) exchange_timeout: Duration,
    /// The most bytes of envelopes one exchange carries; a larger envelope is carried alone.
    pub(super) max_body_bytes: usize,
    /// The waits of a lane between exchanges after a failed one.
    pub(super) backoff: Backoff,
    /// deploy.yaml's `receipt: durable` (docs/adr/0109, D10; SCE_MESH.md section 10.11).
    pub(super) durable: bool,
}

/// One of the client's two lines of exchanges, each with at most one in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lane {
    /// One held exchange after another, carrying nothing.
    Poll,
    /// An exchange that asks for no hold whenever the outbox holds something.
    Send,
}

/// What happened, as the shell tells the core. One that a lane is not waiting for gives no effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Input {
    /// The shell started: it carries out what it is asked from now on.
    Started,
    /// The core of the machine transmitted an envelope to the server.
    Transmitted(Vec<u8>),
    /// [`Task::Draw`] is done: what was drawn, or `None` when the random source gave nothing.
    Drawn(Lane, Option<Drawn>),
    /// [`Task::Sign`] is done: the fields that sign the request, or `None` when they could not be
    /// made.
    Signed(Lane, Option<Fields>),
    /// [`Task::Carry`] is done: what the exchange ended as.
    Answered(Lane, Result<HttpsResponse, HttpsFailure>),
    /// [`Task::Wake`] is done: the moment the lane waited for has come.
    Woken(Lane),
    /// The host reported what a durable checkpoint of its own holds (docs/adr/0109, D10).
    Held(Received),
    /// The client stops.
    Stop,
}

/// What a lane's request needs that a core without a random source and a clock cannot make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Drawn {
    /// A fresh nonce (`draw_nonce` of the module `signature`).
    pub(super) nonce: String,
    /// The moment, since the Unix epoch.
    pub(super) moment: Duration,
}

/// What the shell does, in the order a step gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Effect {
    /// Report an exchange by the status its server answered, or none (docs/adr/0109, D16).
    Count(ExchangeOutcome),
    /// Tell the host.
    Tell(LinkEvent),
    /// Do this for a lane.
    Ask(Lane, Task),
}

/// What a lane needs done, and the [`Input`] that says it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Task {
    /// Draw a nonce and read the moment, for a request that is verified against or signed:
    /// [`Input::Drawn`].
    Draw,
    /// Make the fields that sign the request of these `headers` to `target_uri`, as made at
    /// `created` (seconds since the Unix epoch): [`Input::Signed`].
    Sign {
        target_uri: String,
        headers: Fields,
        created: i64,
    },
    /// Carry this exchange: [`Input::Answered`].
    Carry(HttpsRequest),
    /// Wait: [`Input::Woken`]. A wait of no time lets what the host is doing go on first, so that
    /// what it transmits together is carried together.
    Wake(Wait),
    /// The lane is over: it makes no exchange any more, and no input of it is awaited.
    Stop,
}

/// What a lane waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Wait {
    /// This long, from now.
    For(Duration),
    /// This long, from now, spread as [`Backoff::delay`] spreads a wait with `jitter`.
    Spread(Duration),
    /// This moment, since the Unix epoch; one already past is no wait.
    Until(Duration),
}

/// Where a client stands in its server's mailbox (SCE_MESH.md section 22.3): the incarnation
/// the mailbox numbers in (`None` before the first `200`) and the highest number the client
/// handed its host. Read by a host that checkpoints under `receipt: durable` before it applies
/// what the client handed it, and reported back once a checkpoint that holds it is durable
/// (`HttpsClient::received`, `HttpsClient::held`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Received {
    pub epoch: Option<String>,
    pub number: u64,
}

/// What a request was sent with that its response is read against: where its acknowledgement
/// stood, and, when the client verifies, what of it the response's signature covers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Sent {
    mark: Mark,
    answered: Option<Answered>,
}

/// Where an acknowledgement stands (§mesh-22.3): the highest mailbox number handed to the core,
/// and the incarnation of the mailbox it counts in, `None` before the first `200`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Mark {
    ack: u64,
    epoch: Option<String>,
}

/// An exchange from the moment its lane began it: the envelopes its request carries, and what the
/// request was sent with.
struct Exchange {
    carried: Vec<Vec<u8>>,
    sent: Sent,
}

/// Where a lane is.
enum Phase {
    /// It waits for something to carry, or for the shell to start.
    Idle,
    /// It waits for [`Input::Woken`].
    Waiting,
    /// It waits for [`Input::Drawn`], which this request needs.
    Drawing(Exchange, HttpsRequest),
    /// It waits for [`Input::Signed`], which this request needs.
    Signing(Exchange, HttpsRequest),
    /// It waits for [`Input::Answered`].
    InFlight(Exchange),
    /// It makes no exchange any more.
    Stopped,
}

struct LaneState {
    phase: Phase,
    /// The exchanges since the lane's last `200` that ended otherwise, which its backoff grows by.
    failures: u32,
}

impl LaneState {
    fn new() -> Self {
        LaneState {
            phase: Phase::Idle,
            failures: 0,
        }
    }
}

/// What an exchange's outcome asks of the lane that made it.
enum Then {
    /// Go on at once.
    Continue,
    /// Wait at least this long first: the server asked (`Retry-After`) or the exchange failed.
    Wait(Wait),
    /// Make no exchange any more: the client stopped.
    Stop,
}

/// The client's state, and the one function that moves it ([`ClientCore::step`]).
pub(super) struct ClientCore {
    settings: CoreSettings,
    /// What the core of the machine transmitted and no exchange has carried yet, oldest first.
    outbox: VecDeque<Vec<u8>>,
    /// What every request acknowledges: the highest mailbox number handed to the core of the
    /// machine, in the mailbox incarnation the client last saw.
    mark: Mark,
    ready: bool,
    stopped: bool,
    /// Whether the last `200` did not verify under `verify:`, so that a run of them is told once
    /// (SCE_MESH.md section 16.7 row 19, once per edge).
    unauthenticated: bool,
    /// Under `receipt: durable` (docs/adr/0109, D10), the highest mailbox number, in the
    /// incarnation of `mark`, that the host reported a durable checkpoint of its own holds
    /// ([`Input::Held`]): what every request acknowledges, never above `mark.ack`. `None`
    /// on a binding that is not durable, whose requests acknowledge `mark.ack`.
    held: Option<u64>,
    poll: LaneState,
    send: LaneState,
}

impl ClientCore {
    pub(super) fn new(settings: CoreSettings) -> Self {
        let held = settings.durable.then_some(0);
        ClientCore {
            settings,
            outbox: VecDeque::new(),
            mark: Mark::default(),
            ready: false,
            stopped: false,
            unauthenticated: false,
            held,
            poll: LaneState::new(),
            send: LaneState::new(),
        }
    }

    /// Whether the client stopped: it takes no envelope from the core of the machine any more.
    pub(super) fn stopped(&self) -> bool {
        self.stopped
    }

    /// Where the client stands in its server's mailbox.
    pub(super) fn received(&self) -> Received {
        Received {
            epoch: self.mark.epoch.clone(),
            number: self.mark.ack,
        }
    }

    /// Take `input`, and answer what the shell does now.
    pub(super) fn step(&mut self, input: Input) -> Vec<Effect> {
        let mut effects = Vec::new();
        match input {
            Input::Started => self.start(&mut effects),
            Input::Transmitted(envelope) => self.transmit(envelope, &mut effects),
            Input::Drawn(lane, drawn) => self.take_drawn(lane, drawn, &mut effects),
            Input::Signed(lane, fields) => self.take_signed(lane, fields, &mut effects),
            Input::Answered(lane, outcome) => self.take_answer(lane, outcome, &mut effects),
            Input::Woken(lane) => self.wake(lane, &mut effects),
            Input::Held(received) => self.hold(&received),
            Input::Stop => self.stop(&mut effects),
        }
        effects
    }

    /// The first poll is made at once.
    fn start(&mut self, effects: &mut Vec<Effect>) {
        if matches!(self.poll.phase, Phase::Idle) {
            self.wait(Lane::Poll, Wait::For(Duration::ZERO), effects);
        }
    }

    /// What the core of the machine transmits waits for the send lane's next exchange. A client
    /// that stopped hands it back.
    fn transmit(&mut self, envelope: Vec<u8>, effects: &mut Vec<Effect>) {
        if self.stopped {
            self.hand_back(alloc::vec![envelope], effects);
            return;
        }
        self.outbox.push_back(envelope);
        if matches!(self.send.phase, Phase::Idle) {
            self.wait(Lane::Send, Wait::For(Duration::ZERO), effects);
        }
    }

    fn wake(&mut self, lane: Lane, effects: &mut Vec<Effect>) {
        if matches!(self.lane(lane).phase, Phase::Waiting) {
            self.begin(lane, effects);
        }
    }

    /// A durable checkpoint of the host holds every envelope up to `received` (docs/adr/0109,
    /// D10). A mark of an incarnation the client has left counts in a numbering that is gone, and
    /// moves nothing; on a binding that is not durable this does nothing.
    fn hold(&mut self, received: &Received) {
        if self.mark.epoch != received.epoch {
            return;
        }
        if let Some(held) = self.held.as_mut() {
            *held = (*held).max(received.number);
        }
    }

    /// No exchange is begun after this, what waits in the outbox is handed back as not sent, and
    /// the server is lost. A lane with an exchange begun is over once that exchange ended.
    fn stop(&mut self, effects: &mut Vec<Effect>) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        let left: Vec<Vec<u8>> = self.outbox.drain(..).collect();
        self.hand_back(left, effects);
        // A request waiting on the server can be answered only by an exchange this client makes,
        // and it makes none from now on: the link is lost whether or not it was ready.
        self.ready = false;
        effects.push(Effect::Tell(LinkEvent::Lost(
            self.settings.server.clone(),
            "the HTTPS client stopped".to_string(),
        )));
        for lane in [Lane::Poll, Lane::Send] {
            if matches!(self.lane(lane).phase, Phase::Idle | Phase::Waiting) {
                self.end(lane, effects);
            }
        }
    }

    /// Begin the lane's next exchange: a poll carries nothing and is held; a send carries what the
    /// outbox holds, up to the bound of one exchange, and asks for no hold, and waits when the
    /// outbox holds nothing.
    fn begin(&mut self, lane: Lane, effects: &mut Vec<Effect>) {
        let (carried, hold) = match lane {
            Lane::Poll => (Vec::new(), self.settings.hold),
            Lane::Send if self.outbox.is_empty() => {
                self.send.phase = Phase::Idle;
                return;
            }
            Lane::Send => (self.take_outbox(), Duration::ZERO),
        };
        let mark = self.acknowledging();
        // §mesh-22.1: what every request carries besides its body, the media type of the
        // §mesh-22.2 framing, the acknowledgement and the incarnation it counts in (none before
        // the first `200`), the hold, and the credential.
        let mut headers = alloc::vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), mark.ack.to_string()),
            (HOLD_HEADER.to_string(), hold.as_millis().to_string()),
        ];
        if let Some(epoch) = &mark.epoch {
            headers.push((EPOCH_HEADER.to_string(), epoch.clone()));
        }
        if let Some(authorization) = &self.settings.authorization {
            headers.push(("Authorization".to_string(), authorization.clone()));
        }
        let request = HttpsRequest {
            url: self.settings.url.clone(),
            headers,
            body: join_sequence(carried.iter().map(Vec::as_slice)),
            timeout: hold.saturating_add(self.settings.exchange_timeout),
        };
        let exchange = Exchange {
            carried,
            sent: Sent {
                mark,
                answered: None,
            },
        };
        // §mesh-22.9: a client that verifies or signs draws a nonce for each
        // exchange, which a response's signature covers so that it answers this request and no
        // other; a client of the `key` authenticator signs the request over its body's digest.
        if self.settings.verify.is_some() || self.settings.signs {
            self.lane(lane).phase = Phase::Drawing(exchange, request);
            effects.push(Effect::Ask(lane, Task::Draw));
        } else {
            self.carry(lane, exchange, request, effects);
        }
    }

    /// The envelopes the next exchange carries, oldest first: as many as its bound holds, and at
    /// least one.
    fn take_outbox(&mut self) -> Vec<Vec<u8>> {
        let mut carried = Vec::new();
        let mut size = 0usize;
        while let Some(length) = self.outbox.front().map(Vec::len) {
            let grown = size.saturating_add(length);
            if !carried.is_empty() && grown > self.settings.max_body_bytes {
                break;
            }
            size = grown;
            if let Some(envelope) = self.outbox.pop_front() {
                carried.push(envelope);
            }
        }
        carried
    }

    /// What the next request acknowledges: the highest number handed to the core, or, under
    /// `receipt: durable`, the highest a durable checkpoint of the host holds (docs/adr/0109,
    /// D10), so that the server never deletes what a crash of this client would lose.
    fn acknowledging(&self) -> Mark {
        match self.held {
            Some(held) => Mark {
                ack: held.min(self.mark.ack),
                epoch: self.mark.epoch.clone(),
            },
            None => self.mark.clone(),
        }
    }

    /// The nonce and the moment the lane's request needs. A request they could not be drawn for
    /// ends as a failure at the transport.
    fn take_drawn(&mut self, lane: Lane, drawn: Option<Drawn>, effects: &mut Vec<Effect>) {
        let (mut exchange, mut request) = match self.leave(lane) {
            Phase::Drawing(exchange, request) => (exchange, request),
            other => {
                self.lane(lane).phase = other;
                return;
            }
        };
        let Some(Drawn { nonce, moment }) = drawn else {
            let failure = HttpsFailure::Other("the random source could not be read".to_string());
            self.settle(lane, exchange, Err(failure), effects);
            return;
        };
        request
            .headers
            .push((NONCE_HEADER.to_string(), nonce.clone()));
        if self.settings.verify.is_some() {
            exchange.sent.answered = Some(Answered {
                target_uri: self.settings.target_uri.clone(),
                ack: exchange.sent.mark.ack.to_string(),
                nonce,
            });
        }
        if !self.settings.signs {
            self.carry(lane, exchange, request, effects);
            return;
        }
        request.headers.push((
            CONTENT_DIGEST_HEADER.to_string(),
            content_digest(&request.body),
        ));
        effects.push(Effect::Ask(
            lane,
            Task::Sign {
                target_uri: self.settings.target_uri.clone(),
                headers: request.headers.clone(),
                created: i64::try_from(moment.as_secs()).unwrap_or(i64::MAX),
            },
        ));
        self.lane(lane).phase = Phase::Signing(exchange, request);
    }

    /// The fields that sign the lane's request. A request they could not be made for ends as a
    /// failure at the transport.
    fn take_signed(&mut self, lane: Lane, fields: Option<Fields>, effects: &mut Vec<Effect>) {
        let (exchange, mut request) = match self.leave(lane) {
            Phase::Signing(exchange, request) => (exchange, request),
            other => {
                self.lane(lane).phase = other;
                return;
            }
        };
        let Some(fields) = fields else {
            let failure =
                HttpsFailure::Other("the request lacks a field its signature covers".to_string());
            self.settle(lane, exchange, Err(failure), effects);
            return;
        };
        request.headers.extend(fields);
        self.carry(lane, exchange, request, effects);
    }

    fn take_answer(
        &mut self,
        lane: Lane,
        outcome: Result<HttpsResponse, HttpsFailure>,
        effects: &mut Vec<Effect>,
    ) {
        match self.leave(lane) {
            Phase::InFlight(exchange) => self.settle(lane, exchange, outcome, effects),
            other => self.lane(lane).phase = other,
        }
    }

    /// The exchange of `lane` ended as `outcome`: take it, and go on as it asks.
    fn settle(
        &mut self,
        lane: Lane,
        exchange: Exchange,
        outcome: Result<HttpsResponse, HttpsFailure>,
        effects: &mut Vec<Effect>,
    ) {
        match self.take_outcome(lane, exchange, outcome, effects) {
            Then::Continue => self.begin(lane, effects),
            Then::Wait(wait) => self.wait(lane, wait, effects),
            Then::Stop => self.end(lane, effects),
        }
    }

    /// Take what an exchange returned: the envelopes its request carried and what it was sent
    /// with are `exchange`'s. Returns what the lane does next; a response the server shed puts
    /// what was carried back at the front of the outbox, to go again after the wait.
    fn take_outcome(
        &mut self,
        lane: Lane,
        exchange: Exchange,
        outcome: Result<HttpsResponse, HttpsFailure>,
        effects: &mut Vec<Effect>,
    ) -> Then {
        let Exchange { carried, sent } = exchange;
        // docs/adr/0109, D16: every exchange, by the status its server answered, or none.
        effects.push(Effect::Count(match &outcome {
            Ok(response) => ExchangeOutcome::of_status(response.status),
            Err(_) => ExchangeOutcome::NoResponse,
        }));
        if self.stopped {
            // The client stopped while the exchange was in flight: it has told the host its server
            // is lost, and takes nothing more in. What the mailbox sent stays there,
            // unacknowledged.
            self.hand_back(carried, effects);
            return Then::Stop;
        }
        match outcome {
            Ok(response) if response.status == 200 => {
                // §mesh-22.9: a `200` that did not verify proves nothing of the server. The
                // client takes nothing of it and acknowledges nothing, so what the mailbox holds
                // stays there for a response that verifies; what the request carried is handed
                // back as not sent, since nothing says the server took it; readiness stands as it
                // was; and the host is told once per edge (section 16.7 row 19). The client tries
                // again on its backoff.
                if let Some(why) = self.unverified(&sent, &response) {
                    self.hand_back(carried, effects);
                    if !self.unauthenticated {
                        self.unauthenticated = true;
                        effects.push(Effect::Tell(LinkEvent::Unauthenticated(
                            self.settings.server.clone(),
                            why,
                        )));
                    }
                    return Then::Wait(self.back_off(lane));
                }
                self.lane(lane).failures = 0;
                self.unauthenticated = false;
                if !self.ready {
                    self.ready = true;
                    effects.push(Effect::Tell(LinkEvent::Ready(self.settings.server.clone())));
                }
                // docs/adr/0109, D10: under `receipt: durable` the server answers `200` only once
                // a durable checkpoint holds what the request carried, which is the binding's
                // acknowledgement of it: the host's core holds it no more.
                if self.settings.durable && !carried.is_empty() {
                    effects.push(Effect::Tell(LinkEvent::Carried(
                        self.settings.server.clone(),
                        carried,
                    )));
                }
                self.take_mailbox(&sent.mark, &response, effects);
                Then::Continue
            }
            // §mesh-22.4: load shed is neither a failure nor a lost server. What the request
            // carried goes again after at least the wait `Retry-After` asks for.
            Ok(response) if matches!(response.status, 429 | 503) => {
                for envelope in carried.into_iter().rev() {
                    self.outbox.push_front(envelope);
                }
                let backed_off = self.back_off(lane);
                let asked = response
                    .header("Retry-After")
                    .and_then(read_retry_after)
                    .map(|asked| match asked {
                        RetryAfter::Delay(delay) => Wait::For(delay),
                        RetryAfter::Date(moment) => Wait::Until(moment),
                    });
                Then::Wait(asked.unwrap_or(backed_off))
            }
            // A status the binding does not take in: the server answered, so the client stays as
            // ready as it was, and what the request carried was not taken (§mesh-22.4).
            Ok(_) => {
                self.hand_back(carried, effects);
                Then::Wait(self.back_off(lane))
            }
            // §mesh-22.4: a failure at the transport hands the envelopes back as not sent and ends
            // readiness until the next `200`.
            Err(failure) => {
                self.hand_back(carried, effects);
                // A failure at the transport ends readiness and loses nothing (section 22.4): the
                // server keeps the mailbox, a reply to a request this client waits on among it,
                // and a later exchange carries it, so the link is interrupted, not lost.
                if self.ready {
                    self.ready = false;
                    effects.push(Effect::Tell(LinkEvent::Interrupted(
                        self.settings.server.clone(),
                        failure.to_string(),
                    )));
                }
                Then::Wait(self.back_off(lane))
            }
        }
    }

    /// Hand the core the envelopes of a `200` response, in mailbox order and each once, and move
    /// the acknowledgement past them (§mesh-22.3). `sent` is what the response's request
    /// acknowledged.
    ///
    /// The response's incarnation (`Sce-Mesh-Epoch`) says which numbering its numbers count in:
    ///
    /// * the one the client last saw: what is at or below the client's acknowledgement (which the
    ///   other lane's response may already have handed on) is passed over;
    /// * another, which the request was sent in: an incarnation the client has left since, because
    ///   the other lane's response named a newer one. What is above the request's acknowledgement
    ///   is handed on, the section 10.5 window dropping what reached the core before, and the
    ///   client's acknowledgement stays with the newer incarnation;
    /// * any other: a mailbox that numbers anew (a server that started again with its mailboxes
    ///   empty). The acknowledgement starts over at 0 in it, so all of the response is new. When it
    ///   replaces an incarnation the client knew, what that mailbox held is gone, a reply among it:
    ///   the host is told the server restarted ([`LinkEvent::Restarted`]) before the response's
    ///   envelopes, so the core asks again what it waits on. The first incarnation the client
    ///   learns replaces none.
    ///
    /// A response that names no incarnation says nothing of what its numbers count in, and a body
    /// without a first number cannot be acknowledged: what a server that breaks the binding sends
    /// is not taken.
    fn take_mailbox(&mut self, sent: &Mark, response: &HttpsResponse, effects: &mut Vec<Effect>) {
        // §mesh-22.2: a client takes nothing of a `200` without `Sce-Mesh-Epoch`, reads the body
        // as a CBOR sequence, and numbers it from `Sce-Mesh-First`.
        let Some(epoch) = response
            .header(EPOCH_HEADER)
            .map(str::trim)
            .filter(|epoch| is_epoch(epoch))
        else {
            return;
        };
        let current = if self.mark.epoch.as_deref() == Some(epoch) {
            true
        } else if sent.epoch.as_deref() == Some(epoch) {
            false
        } else {
            let replaced = self.mark.epoch.is_some();
            self.mark = Mark {
                ack: 0,
                epoch: Some(epoch.to_string()),
            };
            // What a checkpoint held counted in the numbering that is gone.
            if let Some(held) = self.held.as_mut() {
                *held = 0;
            }
            if replaced {
                effects.push(Effect::Tell(LinkEvent::Restarted(
                    self.settings.server.clone(),
                )));
            }
            true
        };
        let envelopes = split_sequence(&response.body);
        if envelopes.is_empty() {
            return;
        }
        let first = response
            .header(FIRST_HEADER)
            .and_then(|value| value.trim().parse::<u64>().ok());
        let Some(first) = first.filter(|first| *first >= 1) else {
            return;
        };
        let taken = if current { self.mark.ack } else { sent.ack };
        let last = first.saturating_add(envelopes.len() as u64 - 1);
        for (number, envelope) in (first..).zip(envelopes) {
            if number <= taken {
                continue;
            }
            effects.push(Effect::Tell(LinkEvent::Received(
                self.settings.server.clone(),
                envelope,
            )));
        }
        if current {
            self.mark.ack = self.mark.ack.max(last);
        }
    }

    /// Why `response`, a `200`, is not one the client may take: `None` when the binding does not
    /// verify or the response verifies (SCE_MESH.md section 22.9).
    fn unverified(&self, sent: &Sent, response: &HttpsResponse) -> Option<String> {
        let trusted = self.settings.verify.as_ref()?;
        let Some(answered) = &sent.answered else {
            return Some("the request was sent without a nonce".to_string());
        };
        let field = |name: &str| field_value(&response.headers, name);
        verify_response(trusted, response.status, &field, &response.body, answered)
            .err()
            .map(|refusal| refusal.to_string())
    }

    /// The envelopes were not sent: hand them back, oldest first.
    fn hand_back(&self, envelopes: Vec<Vec<u8>>, effects: &mut Vec<Effect>) {
        for envelope in envelopes {
            effects.push(Effect::Tell(LinkEvent::Unsent(
                self.settings.server.clone(),
                envelope,
            )));
        }
    }

    /// The wait the lane's exchanges since its last `200` ask for, this one the last of them.
    fn back_off(&mut self, lane: Lane) -> Wait {
        let backoff = self.settings.backoff;
        let failures = &mut self.lane(lane).failures;
        let delay = backoff.base_delay(*failures);
        *failures = failures.saturating_add(1);
        if backoff.jitter {
            Wait::Spread(delay)
        } else {
            Wait::For(delay)
        }
    }

    fn lane(&mut self, lane: Lane) -> &mut LaneState {
        match lane {
            Lane::Poll => &mut self.poll,
            Lane::Send => &mut self.send,
        }
    }

    /// Take the lane out of its phase, which the caller gives it again or replaces.
    fn leave(&mut self, lane: Lane) -> Phase {
        mem::replace(&mut self.lane(lane).phase, Phase::Idle)
    }

    fn carry(
        &mut self,
        lane: Lane,
        exchange: Exchange,
        request: HttpsRequest,
        effects: &mut Vec<Effect>,
    ) {
        self.lane(lane).phase = Phase::InFlight(exchange);
        effects.push(Effect::Ask(lane, Task::Carry(request)));
    }

    fn wait(&mut self, lane: Lane, wait: Wait, effects: &mut Vec<Effect>) {
        self.lane(lane).phase = Phase::Waiting;
        effects.push(Effect::Ask(lane, Task::Wake(wait)));
    }

    fn end(&mut self, lane: Lane, effects: &mut Vec<Effect>) {
        self.lane(lane).phase = Phase::Stopped;
        effects.push(Effect::Ask(lane, Task::Stop));
    }
}

#[cfg(test)]
mod tests {
    use super::super::signature::{Refusal, ResponseSigner, SIGNATURE_HEADER};
    use super::*;
    use sce_rust_crypto::ed25519::KeyPair;

    /// The incarnation the server's mailbox numbers in, and one it numbered in before it started
    /// again.
    const EPOCH: &str = "0123456789abcdef0123456789abcdef";
    const EARLIER: &str = "fedcba9876543210fedcba9876543210";

    /// What a step that does nothing answers.
    const NOTHING: [Effect; 0] = [];

    const URL: &str = "https://cal.example/api/sce-mesh/1/phone";

    fn settings() -> CoreSettings {
        CoreSettings {
            server: "cloud".to_string(),
            url: URL.to_string(),
            target_uri: URL.to_string(),
            authorization: Some("Bearer t".to_string()),
            signs: false,
            verify: None,
            hold: Duration::from_millis(500),
            exchange_timeout: Duration::from_secs(30),
            max_body_bytes: 1024,
            backoff: Backoff {
                initial: Duration::from_millis(250),
                max: Duration::from_secs(1),
                factor: 2,
                jitter: false,
            },
            durable: false,
        }
    }

    /// A client of the `key` authenticator.
    fn signing() -> CoreSettings {
        CoreSettings {
            authorization: None,
            signs: true,
            ..settings()
        }
    }

    /// A core whose shell started it and whose first poll was begun.
    fn begun(settings: CoreSettings) -> ClientCore {
        let mut core = ClientCore::new(settings);
        core.step(Input::Started);
        core.step(Input::Woken(Lane::Poll));
        core
    }

    /// [`begun`], ready: its first poll was answered `200` with nothing, and its second is in
    /// flight.
    fn ready(settings: CoreSettings) -> ClientCore {
        let mut core = begun(settings);
        let effects = core.step(answered(Lane::Poll, ok(None, &[])));
        assert!(effects.contains(&told(LinkEvent::Ready("cloud".to_string()))));
        core
    }

    /// A `200` of the mailbox incarnation `epoch`, its body's first number `first`.
    fn ok_in(epoch: &str, first: Option<u64>, body: &[u8]) -> HttpsResponse {
        let mut headers = alloc::vec![(EPOCH_HEADER.to_string(), epoch.to_string())];
        if let Some(first) = first {
            headers.push((FIRST_HEADER.to_string(), first.to_string()));
        }
        HttpsResponse {
            status: 200,
            headers,
            body: body.to_vec(),
        }
    }

    fn ok(first: Option<u64>, body: &[u8]) -> HttpsResponse {
        ok_in(EPOCH, first, body)
    }

    fn status(code: u16, headers: &[(&str, &str)]) -> HttpsResponse {
        HttpsResponse {
            status: code,
            headers: headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            body: Vec::new(),
        }
    }

    /// The request of a client that neither verifies nor signs.
    fn request(ack: u64, epoch: Option<&str>, hold_ms: u64, body: &[u8]) -> HttpsRequest {
        let mut headers = alloc::vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), ack.to_string()),
            (HOLD_HEADER.to_string(), hold_ms.to_string()),
        ];
        if let Some(epoch) = epoch {
            headers.push((EPOCH_HEADER.to_string(), epoch.to_string()));
        }
        headers.push(("Authorization".to_string(), "Bearer t".to_string()));
        HttpsRequest {
            url: URL.to_string(),
            headers,
            body: body.to_vec(),
            timeout: Duration::from_millis(hold_ms) + Duration::from_secs(30),
        }
    }

    fn answered(lane: Lane, response: HttpsResponse) -> Input {
        Input::Answered(lane, Ok(response))
    }

    fn failed(lane: Lane) -> Input {
        Input::Answered(lane, Err(HttpsFailure::Connect))
    }

    /// A nonce and a moment drawn for a request of `lane`.
    fn drawn(lane: Lane, nonce: &str) -> Input {
        Input::Drawn(
            lane,
            Some(Drawn {
                nonce: nonce.to_string(),
                moment: Duration::from_millis(1_700_000_000_900),
            }),
        )
    }

    fn told(event: LinkEvent) -> Effect {
        Effect::Tell(event)
    }

    fn unsent(envelope: &[u8]) -> Effect {
        told(LinkEvent::Unsent("cloud".to_string(), envelope.to_vec()))
    }

    fn arrived(envelope: &[u8]) -> Effect {
        told(LinkEvent::Received("cloud".to_string(), envelope.to_vec()))
    }

    fn interrupted(words: &str) -> Effect {
        told(LinkEvent::Interrupted(
            "cloud".to_string(),
            words.to_string(),
        ))
    }

    fn lost() -> Effect {
        told(LinkEvent::Lost(
            "cloud".to_string(),
            "the HTTPS client stopped".to_string(),
        ))
    }

    fn counted(outcome: ExchangeOutcome) -> Effect {
        Effect::Count(outcome)
    }

    fn carry(lane: Lane, request: HttpsRequest) -> Effect {
        Effect::Ask(lane, Task::Carry(request))
    }

    fn wait_ms(lane: Lane, ms: u64) -> Effect {
        Effect::Ask(lane, Task::Wake(Wait::For(Duration::from_millis(ms))))
    }

    /// The one request the effects of a step carry.
    fn carried(effects: &[Effect]) -> HttpsRequest {
        let mut requests = effects.iter().filter_map(|effect| match effect {
            Effect::Ask(_, Task::Carry(request)) => Some(request.clone()),
            _ => None,
        });
        let request = requests.next().expect("an exchange to carry");
        assert_eq!(requests.next(), None, "one exchange to carry");
        request
    }

    /// The lane begins with a poll at once, held as the settings say and carrying nothing; its
    /// answer makes the server ready once, hands on what the mailbox held, and the next poll
    /// acknowledges it. Red if `start` asks for no wake, or `begin` holds a send or not a poll.
    #[test]
    fn a_poll_is_held_and_the_next_acknowledges_what_it_brought() {
        let mut core = ClientCore::new(settings());
        assert_eq!(core.step(Input::Started), [wait_ms(Lane::Poll, 0)]);
        assert_eq!(
            core.step(Input::Woken(Lane::Poll)),
            [carry(Lane::Poll, request(0, None, 500, &[]))]
        );
        assert_eq!(
            core.step(answered(Lane::Poll, ok(Some(1), &[0x01, 0x02]))),
            [
                counted(ExchangeOutcome::Ok),
                told(LinkEvent::Ready("cloud".to_string())),
                arrived(&[0x01]),
                arrived(&[0x02]),
                carry(Lane::Poll, request(2, Some(EPOCH), 500, &[])),
            ]
        );
        assert_eq!(
            core.step(answered(Lane::Poll, ok(None, &[]))),
            [
                counted(ExchangeOutcome::Ok),
                carry(Lane::Poll, request(2, Some(EPOCH), 500, &[])),
            ],
            "ready once"
        );
    }

    /// What is transmitted before the send lane is woken is carried together, up to the bound of
    /// one exchange, the rest by the next, and an envelope larger than the bound alone; a lane
    /// with nothing to carry waits. Red if `take_outbox` passes its bound or `transmit` wakes a
    /// lane that is not idle.
    #[test]
    fn a_send_carries_what_the_outbox_holds_up_to_its_bound() {
        let mut core = ready(CoreSettings {
            max_body_bytes: 4,
            ..settings()
        });
        assert_eq!(
            core.step(Input::Transmitted(alloc::vec![1, 2, 3])),
            [wait_ms(Lane::Send, 0)]
        );
        assert_eq!(core.step(Input::Transmitted(alloc::vec![4])), NOTHING);
        assert_eq!(core.step(Input::Transmitted(alloc::vec![5])), NOTHING);
        assert_eq!(
            core.step(Input::Transmitted(alloc::vec![6, 7, 8, 9, 10])),
            NOTHING
        );
        let sent = |body: &[u8]| carry(Lane::Send, request(0, Some(EPOCH), 0, body));
        assert_eq!(
            core.step(Input::Woken(Lane::Send)),
            [sent(&[1, 2, 3, 4])],
            "as many as the bound holds"
        );
        assert_eq!(
            core.step(answered(Lane::Send, ok(None, &[]))),
            [counted(ExchangeOutcome::Ok), sent(&[5])]
        );
        assert_eq!(
            core.step(answered(Lane::Send, ok(None, &[]))),
            [counted(ExchangeOutcome::Ok), sent(&[6, 7, 8, 9, 10])],
            "larger than the bound, alone"
        );
        assert_eq!(
            core.step(answered(Lane::Send, ok(None, &[]))),
            [counted(ExchangeOutcome::Ok)],
            "nothing to carry"
        );
        assert_eq!(
            core.step(Input::Transmitted(alloc::vec![11])),
            [wait_ms(Lane::Send, 0)]
        );
    }

    /// Load shed puts what the exchange carried back before what was transmitted meanwhile, tells
    /// the host nothing, and waits what `Retry-After` asks, seconds or a date, or the backoff when
    /// it asks nothing. Red if `take_outcome` hands a shed envelope back, or puts it at the end of
    /// the outbox.
    #[test]
    fn a_shed_exchange_goes_again_in_order_after_the_wait_it_is_asked() {
        let mut core = ready(settings());
        core.step(Input::Transmitted(alloc::vec![1]));
        core.step(Input::Transmitted(alloc::vec![2]));
        let sent = |body: &[u8]| carry(Lane::Send, request(0, Some(EPOCH), 0, body));
        assert_eq!(core.step(Input::Woken(Lane::Send)), [sent(&[1, 2])]);
        assert_eq!(core.step(Input::Transmitted(alloc::vec![3])), NOTHING);
        assert_eq!(
            core.step(answered(Lane::Send, status(503, &[("Retry-After", "2")]))),
            [
                counted(ExchangeOutcome::ServiceUnavailable),
                wait_ms(Lane::Send, 2000),
            ]
        );
        assert_eq!(
            core.step(Input::Transmitted(alloc::vec![4])),
            NOTHING,
            "the lane waits out what it was asked"
        );
        assert_eq!(core.step(Input::Woken(Lane::Send)), [sent(&[1, 2, 3, 4])]);
        let dated = status(429, &[("Retry-After", "Sun, 06 Nov 1994 08:49:37 GMT")]);
        assert_eq!(
            core.step(answered(Lane::Send, dated)),
            [
                counted(ExchangeOutcome::TooManyRequests),
                Effect::Ask(
                    Lane::Send,
                    Task::Wake(Wait::Until(Duration::from_secs(784_111_777)))
                ),
            ]
        );
        assert_eq!(core.step(Input::Woken(Lane::Send)), [sent(&[1, 2, 3, 4])]);
        assert_eq!(
            core.step(answered(Lane::Send, status(503, &[]))),
            [
                counted(ExchangeOutcome::ServiceUnavailable),
                wait_ms(Lane::Send, 1000),
            ],
            "the third in a row, on the backoff"
        );
    }

    /// Each lane's waits grow from its own failures, whatever they are, up to the longest, and
    /// start over at its next `200`. Red if `take_outcome` leaves the count at a `200`, or the
    /// lanes share one.
    #[test]
    fn a_lanes_backoff_grows_with_its_failures_and_starts_over_at_a_200() {
        let mut core = begun(settings());
        core.step(Input::Transmitted(alloc::vec![9]));
        for ms in [250, 500, 1000, 1000] {
            assert_eq!(
                core.step(failed(Lane::Poll)),
                [
                    counted(ExchangeOutcome::NoResponse),
                    wait_ms(Lane::Poll, ms),
                ]
            );
            core.step(Input::Woken(Lane::Poll));
        }
        core.step(Input::Woken(Lane::Send));
        assert_eq!(
            core.step(answered(Lane::Send, status(401, &[]))),
            [
                counted(ExchangeOutcome::Unauthorized),
                unsent(&[9]),
                wait_ms(Lane::Send, 250),
            ],
            "the other lane's own first"
        );
        core.step(answered(Lane::Poll, ok(None, &[])));
        assert_eq!(
            core.step(answered(Lane::Poll, status(502, &[]))),
            [counted(ExchangeOutcome::Other), wait_ms(Lane::Poll, 250)]
        );
    }

    /// A backoff with jitter asks the shell to spread the wait, which a core without a random
    /// source cannot. Red if `back_off` passes over `jitter`.
    #[test]
    fn a_backoff_with_jitter_is_spread_by_the_shell() {
        let mut core = begun(CoreSettings {
            backoff: Backoff {
                jitter: true,
                ..settings().backoff
            },
            ..settings()
        });
        assert_eq!(
            core.step(failed(Lane::Poll)),
            [
                counted(ExchangeOutcome::NoResponse),
                Effect::Ask(
                    Lane::Poll,
                    Task::Wake(Wait::Spread(Duration::from_millis(250)))
                ),
            ]
        );
    }

    /// What signs as the server does.
    fn signer() -> ResponseSigner {
        ResponseSigner::new("server-2026", KeyPair::from_seed(&[3u8; 32]))
    }

    /// A `200` the server signed that answers `request` with `body`, numbered from `first`.
    fn signed(request: &HttpsRequest, first: Option<u64>, body: &[u8]) -> HttpsResponse {
        let field = |name: &str| field_value(&request.headers, name).unwrap_or_default();
        let answered = Answered {
            target_uri: request.url.clone(),
            ack: field(ACK_HEADER),
            nonce: field(NONCE_HEADER),
        };
        let digest = content_digest(body);
        let first = first.map(|first| first.to_string());
        let mut response = ok(None, body);
        if let Some(first) = &first {
            response
                .headers
                .push((FIRST_HEADER.to_string(), first.clone()));
        }
        response
            .headers
            .push((CONTENT_DIGEST_HEADER.to_string(), digest.clone()));
        response
            .headers
            .extend(signer().sign(&digest, EPOCH, first.as_deref(), &answered));
        response
    }

    /// Under `verify:` every request draws a nonce, a `200` that does not verify is taken nothing
    /// of and hands back what its request carried, a run of them on either lane is told once, and
    /// the next that verifies is taken and starts another run. Red if `take_outcome` takes an
    /// unverified `200`, tells each one, or leaves `unauthenticated` at one that verifies.
    #[test]
    fn a_200_that_does_not_verify_is_taken_nothing_of_and_told_once_per_edge() {
        let mut core = begun(CoreSettings {
            verify: Some(alloc::vec![TrustedKey {
                id: "server-2026".to_string(),
                public_key: KeyPair::from_seed(&[3u8; 32]).public_key(),
            }]),
            ..settings()
        });
        let unauthenticated = told(LinkEvent::Unauthenticated(
            "cloud".to_string(),
            Refusal::Digest.to_string(),
        ));
        let asked = carried(&core.step(drawn(Lane::Poll, "n1")));
        assert_eq!(
            field_value(&asked.headers, NONCE_HEADER).as_deref(),
            Some("n1")
        );
        assert_eq!(
            core.step(answered(Lane::Poll, ok(Some(1), &[0x05]))),
            [
                counted(ExchangeOutcome::Ok),
                unauthenticated.clone(),
                wait_ms(Lane::Poll, 250),
            ],
            "no Ready, no envelope"
        );
        core.step(Input::Transmitted(alloc::vec![9]));
        assert_eq!(
            core.step(Input::Woken(Lane::Send)),
            [Effect::Ask(Lane::Send, Task::Draw)]
        );
        core.step(drawn(Lane::Send, "n2"));
        assert_eq!(
            core.step(answered(Lane::Send, ok(None, &[]))),
            [
                counted(ExchangeOutcome::Ok),
                unsent(&[9]),
                wait_ms(Lane::Send, 250),
            ],
            "told once"
        );
        core.step(Input::Woken(Lane::Poll));
        let asked = carried(&core.step(drawn(Lane::Poll, "n3")));
        assert_eq!(
            field_value(&asked.headers, ACK_HEADER).as_deref(),
            Some("0"),
            "nothing is acknowledged before a response verified"
        );
        assert_eq!(
            core.step(answered(Lane::Poll, signed(&asked, Some(1), &[0x05]))),
            [
                counted(ExchangeOutcome::Ok),
                told(LinkEvent::Ready("cloud".to_string())),
                arrived(&[0x05]),
                Effect::Ask(Lane::Poll, Task::Draw),
            ]
        );
        let replayed = signed(&asked, Some(2), &[0x06]);
        core.step(drawn(Lane::Poll, "n4"));
        assert_eq!(
            core.step(answered(Lane::Poll, replayed)),
            [
                counted(ExchangeOutcome::Ok),
                told(LinkEvent::Unauthenticated(
                    "cloud".to_string(),
                    Refusal::Signature.to_string(),
                )),
                wait_ms(Lane::Poll, 250),
            ],
            "it answers the earlier request; another run, told"
        );
    }

    /// A client of the `key` authenticator draws a nonce and a moment for a request, has its
    /// fields signed over the body's digest, and carries it with the signature's fields last.
    /// Red if `take_drawn` carries a request it signs unsigned, or signs it without a field.
    #[test]
    fn a_request_is_drawn_for_and_signed_before_it_is_carried() {
        let mut core = begun(signing());
        core.step(Input::Transmitted(alloc::vec![9]));
        assert_eq!(
            core.step(Input::Woken(Lane::Send)),
            [Effect::Ask(Lane::Send, Task::Draw)]
        );
        let headers: Fields = [
            ("Content-Type", CONTENT_TYPE),
            (ACK_HEADER, "0"),
            (HOLD_HEADER, "0"),
            (NONCE_HEADER, "n1"),
            (CONTENT_DIGEST_HEADER, content_digest(&[9]).as_str()),
        ]
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
        assert_eq!(
            core.step(drawn(Lane::Send, "n1")),
            [Effect::Ask(
                Lane::Send,
                Task::Sign {
                    target_uri: URL.to_string(),
                    headers: headers.clone(),
                    created: 1_700_000_000,
                }
            )]
        );
        let signature = (SIGNATURE_HEADER.to_string(), "sce=:AA==:".to_string());
        let sent = carried(&core.step(Input::Signed(
            Lane::Send,
            Some(alloc::vec![signature.clone()]),
        )));
        assert_eq!(sent.body, [9]);
        assert_eq!(sent.headers[..5], headers[..]);
        assert_eq!(sent.headers[5..], [signature]);
    }

    /// A request no nonce could be drawn for, and one that could not be signed, end as a failure
    /// at the transport: counted as no response, what it carried handed back, the link
    /// interrupted when it was ready, and tried again on the backoff. Red if either drops what
    /// the lane carried or leaves the lane where it was.
    #[test]
    fn a_failure_to_draw_or_to_sign_is_a_failure_at_the_transport() {
        let mut core = begun(signing());
        core.step(drawn(Lane::Poll, "n1"));
        core.step(Input::Signed(Lane::Poll, Some(Vec::new())));
        core.step(answered(Lane::Poll, ok(None, &[])));
        core.step(Input::Transmitted(alloc::vec![9]));
        core.step(Input::Woken(Lane::Send));
        assert_eq!(
            core.step(Input::Drawn(Lane::Send, None)),
            [
                counted(ExchangeOutcome::NoResponse),
                unsent(&[9]),
                interrupted("the random source could not be read"),
                wait_ms(Lane::Send, 250),
            ]
        );
        // The poll the first answer began: drawn for, and not signed.
        core.step(drawn(Lane::Poll, "n2"));
        assert_eq!(
            core.step(Input::Signed(Lane::Poll, None)),
            [
                counted(ExchangeOutcome::NoResponse),
                wait_ms(Lane::Poll, 250),
            ],
            "interrupted already"
        );
        assert_eq!(
            core.step(Input::Woken(Lane::Poll)),
            [Effect::Ask(Lane::Poll, Task::Draw)]
        );
        core.step(drawn(Lane::Poll, "n3"));
        core.step(Input::Signed(Lane::Poll, Some(Vec::new())));
        core.step(answered(Lane::Poll, ok(None, &[])));
        core.step(drawn(Lane::Poll, "n4"));
        assert_eq!(
            core.step(Input::Signed(Lane::Poll, None)),
            [
                counted(ExchangeOutcome::NoResponse),
                interrupted("the request lacks a field its signature covers"),
                wait_ms(Lane::Poll, 250),
            ]
        );
    }

    /// A failure at the transport interrupts a ready link once, on whichever lane, hands back what
    /// was carried, and the next `200` makes it ready again; a status the server answered leaves
    /// it ready. Red if `take_outcome` tells every failure, or a status ends readiness.
    #[test]
    fn a_link_is_interrupted_once_per_edge_and_only_by_the_transport() {
        let mut core = ready(settings());
        core.step(Input::Transmitted(alloc::vec![9]));
        core.step(Input::Woken(Lane::Send));
        assert_eq!(
            core.step(failed(Lane::Poll)),
            [
                counted(ExchangeOutcome::NoResponse),
                interrupted("no connection could be opened"),
                wait_ms(Lane::Poll, 250),
            ]
        );
        assert_eq!(
            core.step(Input::Answered(Lane::Send, Err(HttpsFailure::Timeout))),
            [
                counted(ExchangeOutcome::NoResponse),
                unsent(&[9]),
                wait_ms(Lane::Send, 250),
            ],
            "told once"
        );
        core.step(Input::Woken(Lane::Poll));
        assert_eq!(
            core.step(answered(Lane::Poll, ok(None, &[])))[1],
            told(LinkEvent::Ready("cloud".to_string()))
        );
        assert_eq!(
            core.step(answered(Lane::Poll, status(401, &[]))),
            [
                counted(ExchangeOutcome::Unauthorized),
                wait_ms(Lane::Poll, 250),
            ],
            "the server answered"
        );
        core.step(Input::Woken(Lane::Poll));
        assert_eq!(
            core.step(Input::Answered(
                Lane::Poll,
                Err(HttpsFailure::Other("closed".to_string()))
            ))[1],
            interrupted("closed"),
            "the next edge"
        );
    }

    /// docs/adr/0109, D10: under `receipt: durable` a request acknowledges what the host reported
    /// a checkpoint holds, never above what it was handed, a mark of another incarnation moves
    /// nothing, a `200` that carried envelopes is told, and a new incarnation starts what is held
    /// over. Red if `acknowledging` answers `mark` alone or `take_mailbox` leaves `held`.
    #[test]
    fn a_durable_client_acknowledges_only_what_a_checkpoint_holds() {
        let mut core = ready(CoreSettings {
            durable: true,
            ..settings()
        });
        let poll = |ack, epoch| carry(Lane::Poll, request(ack, Some(epoch), 500, &[]));
        assert_eq!(
            core.step(answered(Lane::Poll, ok(Some(1), &[0x01, 0x02, 0x03]))),
            [
                counted(ExchangeOutcome::Ok),
                arrived(&[0x01]),
                arrived(&[0x02]),
                arrived(&[0x03]),
                poll(0, EPOCH),
            ],
            "nothing a checkpoint holds yet"
        );
        let received = core.received();
        assert_eq!(
            received,
            Received {
                epoch: Some(EPOCH.to_string()),
                number: 3,
            }
        );
        let earlier = Received {
            epoch: Some(EARLIER.to_string()),
            number: 3,
        };
        assert_eq!(core.step(Input::Held(earlier)), NOTHING);
        let empty = || answered(Lane::Poll, ok(None, &[]));
        assert_eq!(core.step(empty())[1], poll(0, EPOCH));
        assert_eq!(core.step(Input::Held(received.clone())), NOTHING);
        assert_eq!(core.step(empty())[1], poll(3, EPOCH));
        let beyond = Received {
            number: 9,
            ..received
        };
        assert_eq!(core.step(Input::Held(beyond)), NOTHING);
        assert_eq!(
            core.step(empty())[1],
            poll(3, EPOCH),
            "never above the mark"
        );

        core.step(Input::Transmitted(alloc::vec![9]));
        core.step(Input::Woken(Lane::Send));
        assert_eq!(
            core.step(answered(Lane::Send, ok(None, &[]))),
            [
                counted(ExchangeOutcome::Ok),
                told(LinkEvent::Carried(
                    "cloud".to_string(),
                    alloc::vec![alloc::vec![9]]
                )),
            ]
        );
        assert_eq!(
            core.step(answered(Lane::Poll, ok_in(EARLIER, Some(1), &[0x04]))),
            [
                counted(ExchangeOutcome::Ok),
                told(LinkEvent::Restarted("cloud".to_string())),
                arrived(&[0x04]),
                poll(0, EARLIER),
            ],
            "what was held counted in the numbering that is gone"
        );
    }

    /// On a binding that is not durable a report of a checkpoint moves nothing. Red if `hold`
    /// makes a client durable.
    #[test]
    fn a_client_that_is_not_durable_acknowledges_what_it_handed_on() {
        let mut core = ready(settings());
        assert_eq!(core.step(Input::Held(core.received())), NOTHING);
        let effects = core.step(answered(Lane::Poll, ok(Some(1), &[0x01])));
        assert_eq!(
            carried(&effects),
            request(1, Some(EPOCH), 500, &[]),
            "what was handed on, not what was held"
        );
    }

    /// A stop hands back the outbox, oldest first, then tells the server lost, once, though the
    /// link was never ready, and ends each lane that has no exchange begun; nothing moves the
    /// core after. Red if `stop` tells `Lost` before what it hands back, or a lane that waits
    /// begins again.
    #[test]
    fn a_stop_hands_back_the_outbox_and_tells_the_server_lost_once() {
        let mut core = begun(settings());
        core.step(Input::Transmitted(alloc::vec![1]));
        core.step(Input::Transmitted(alloc::vec![2]));
        assert!(!core.stopped());
        assert_eq!(
            core.step(Input::Stop),
            [
                unsent(&[1]),
                unsent(&[2]),
                lost(),
                Effect::Ask(Lane::Send, Task::Stop),
            ]
        );
        assert!(core.stopped());
        assert_eq!(core.step(Input::Stop), NOTHING);
        assert_eq!(core.step(Input::Woken(Lane::Send)), NOTHING);
        assert_eq!(
            core.step(Input::Transmitted(alloc::vec![3])),
            [unsent(&[3])],
            "taken by no exchange, and not lost unseen"
        );

        let mut unstarted = ClientCore::new(settings());
        assert_eq!(
            unstarted.step(Input::Stop),
            [
                lost(),
                Effect::Ask(Lane::Poll, Task::Stop),
                Effect::Ask(Lane::Send, Task::Stop),
            ]
        );
        assert_eq!(unstarted.step(Input::Started), NOTHING);
    }

    /// An exchange in flight at the stop is counted when it ends, is taken nothing of, though a
    /// `200` that carries envelopes, hands back what it carried, and is its lane's last. Red if
    /// `take_outcome` takes an answer in after the stop, or a lane goes on.
    #[test]
    fn an_answer_after_a_stop_is_taken_nothing_of() {
        let mut core = begun(settings());
        core.step(Input::Transmitted(alloc::vec![1]));
        core.step(Input::Woken(Lane::Send));
        assert_eq!(core.step(Input::Stop), [lost()], "both lanes in flight");
        assert_eq!(
            core.step(answered(Lane::Poll, ok(Some(1), &[0x05]))),
            [
                counted(ExchangeOutcome::Ok),
                Effect::Ask(Lane::Poll, Task::Stop),
            ]
        );
        assert_eq!(core.received(), Received::default());
        assert_eq!(
            core.step(answered(Lane::Send, status(503, &[("Retry-After", "0")]))),
            [
                counted(ExchangeOutcome::ServiceUnavailable),
                unsent(&[1]),
                Effect::Ask(Lane::Send, Task::Stop),
            ],
            "shed or not, it does not go again"
        );
    }

    /// An exchange whose request was being made at the stop was begun before it: it goes on, and
    /// is its lane's last. Red if `stop` ends a lane that has an exchange begun, whose envelopes
    /// would be dropped.
    #[test]
    fn a_request_being_made_at_a_stop_is_carried_and_is_the_last() {
        let mut core = begun(signing());
        core.step(Input::Transmitted(alloc::vec![1]));
        core.step(Input::Woken(Lane::Send));
        assert_eq!(core.step(Input::Stop), [lost()]);
        core.step(drawn(Lane::Send, "n1"));
        let sent = carried(&core.step(Input::Signed(Lane::Send, Some(Vec::new()))));
        assert_eq!(sent.body, [1]);
        assert_eq!(
            core.step(failed(Lane::Send)),
            [
                counted(ExchangeOutcome::NoResponse),
                unsent(&[1]),
                Effect::Ask(Lane::Send, Task::Stop),
            ]
        );
        assert_eq!(
            core.step(Input::Drawn(Lane::Poll, None)),
            [
                counted(ExchangeOutcome::NoResponse),
                Effect::Ask(Lane::Poll, Task::Stop),
            ]
        );
    }

    /// An input a lane is not waiting for moves nothing, and the lane goes on waiting for the one
    /// it is. Red if `take_drawn`, `take_signed` or `take_answer` leaves the lane out of its phase.
    #[test]
    fn an_input_no_lane_waits_for_moves_nothing() {
        let mut core = ready(settings());
        for input in [
            Input::Started,
            Input::Woken(Lane::Poll),
            Input::Woken(Lane::Send),
            drawn(Lane::Poll, "n"),
            Input::Drawn(Lane::Send, None),
            Input::Signed(Lane::Poll, None),
            Input::Signed(Lane::Send, Some(Vec::new())),
            answered(Lane::Send, ok(Some(1), &[0x01])),
        ] {
            assert_eq!(core.step(input.clone()), NOTHING, "{input:?}");
        }
        assert_eq!(
            core.step(answered(Lane::Poll, ok(Some(1), &[0x01]))),
            [
                counted(ExchangeOutcome::Ok),
                arrived(&[0x01]),
                carry(Lane::Poll, request(1, Some(EPOCH), 500, &[])),
            ]
        );
    }

    /// Two cores given the same inputs give the same effects at each: the list of both is a
    /// run's whole trace. Red if a step reads a clock, a random source or a hash's order.
    #[test]
    fn the_same_inputs_give_the_same_effects() {
        let inputs = [
            Input::Started,
            Input::Woken(Lane::Poll),
            Input::Transmitted(alloc::vec![1]),
            drawn(Lane::Poll, "n1"),
            Input::Woken(Lane::Send),
            Input::Signed(Lane::Poll, Some(Vec::new())),
            Input::Drawn(Lane::Send, None),
            answered(Lane::Poll, ok(Some(1), &[0x01, 0x02])),
            Input::Transmitted(alloc::vec![2]),
            Input::Held(Received {
                epoch: Some(EPOCH.to_string()),
                number: 1,
            }),
            Input::Woken(Lane::Send),
            drawn(Lane::Send, "n2"),
            Input::Signed(Lane::Send, Some(Vec::new())),
            answered(Lane::Send, status(429, &[])),
            drawn(Lane::Poll, "n3"),
            Input::Signed(Lane::Poll, Some(Vec::new())),
            failed(Lane::Poll),
            Input::Stop,
        ];
        let run = || -> Vec<(Input, Vec<Effect>)> {
            let mut core = ClientCore::new(CoreSettings {
                durable: true,
                backoff: Backoff {
                    jitter: true,
                    ..settings().backoff
                },
                ..signing()
            });
            inputs
                .iter()
                .map(|input| (input.clone(), core.step(input.clone())))
                .collect()
        };
        let trace = run();
        assert_eq!(trace, run());
        let effects: Vec<Effect> = trace.into_iter().flat_map(|(_, effects)| effects).collect();
        for expected in [
            unsent(&[1]),
            unsent(&[2]),
            arrived(&[0x02]),
            interrupted("no connection could be opened"),
            lost(),
        ] {
            assert!(effects.contains(&expected), "{expected:?}");
        }
        assert!(
            effects.iter().any(|effect| matches!(
                effect,
                Effect::Ask(Lane::Send, Task::Wake(Wait::Spread(_)))
            )),
            "the shed send waits"
        );
    }

    /// Where a request's acknowledgement stood, in `epoch`.
    fn mark(ack: u64, epoch: Option<&str>) -> Mark {
        Mark {
            ack,
            epoch: epoch.map(str::to_string),
        }
    }

    /// What [`ClientCore::take_mailbox`] tells of `response` to a request sent with `sent`.
    fn take(core: &mut ClientCore, sent: &Mark, response: &HttpsResponse) -> Vec<Effect> {
        let mut effects = Vec::new();
        core.take_mailbox(sent, response, &mut effects);
        effects
    }

    /// Three one-byte CBOR items (the unsigned integers 1, 2 and 3), which split as three.
    const THREE: [u8; 3] = [0x01, 0x02, 0x03];

    #[test]
    fn a_response_that_continues_the_numbering_is_taken_once() {
        let mut core = ClientCore::new(settings());
        core.mark = mark(4, Some(EPOCH));
        let sent = core.mark.clone();
        // The request acknowledged 4; the mailbox answers 5, 6, 7.
        assert_eq!(
            take(&mut core, &sent, &ok(Some(5), &THREE)),
            [arrived(&[1]), arrived(&[2]), arrived(&[3])]
        );
        assert_eq!(core.mark, mark(7, Some(EPOCH)));
        // The other lane's request also acknowledged 4 and was answered 5 to 8: only 8 is new.
        assert_eq!(
            take(&mut core, &sent, &ok(Some(5), &[0x01, 0x02, 0x03, 0x04])),
            [arrived(&[4])]
        );
        assert_eq!(core.mark, mark(8, Some(EPOCH)));
    }

    /// A server that started again with its mailboxes empty numbers them from 1 in a new
    /// incarnation: the acknowledgement starts over in it, so what it sends is all new though its
    /// numbers are below what the client acknowledged. The other lane's response to a request
    /// sent in the earlier incarnation, answered in the new one, is taken from where the new
    /// numbering stands; one answered in the earlier incarnation is handed on above what its
    /// request acknowledged, and leaves the acknowledgement in the new one. The host is told the
    /// server restarted once, when the new incarnation replaces the one the client knew, and
    /// before what it sends: neither the other lane's response in the new one nor the late one
    /// in the earlier one is a restart.
    #[test]
    fn a_new_incarnation_starts_the_acknowledgement_over() {
        let mut core = ClientCore::new(settings());
        core.mark = mark(57, Some(EARLIER));
        let sent = core.mark.clone();
        assert_eq!(
            take(&mut core, &sent, &ok(Some(1), &THREE)),
            [
                told(LinkEvent::Restarted("cloud".to_string())),
                arrived(&[1]),
                arrived(&[2]),
                arrived(&[3]),
            ],
            "the mailbox the client knew is gone"
        );
        assert_eq!(core.mark, mark(3, Some(EPOCH)));

        assert_eq!(
            take(&mut core, &sent, &ok(Some(1), &[0x01, 0x02, 0x03, 0x04])),
            [arrived(&[4])],
            "taken once in the new numbering"
        );
        assert_eq!(core.mark, mark(4, Some(EPOCH)));

        assert_eq!(
            take(&mut core, &sent, &ok_in(EARLIER, Some(57), &[0x09, 0x0a])),
            [arrived(&[0x0a])],
            "above what the request acknowledged"
        );
        assert_eq!(
            core.mark,
            mark(4, Some(EPOCH)),
            "the newer incarnation stays"
        );
    }

    #[test]
    fn a_body_without_a_number_or_an_incarnation_is_not_taken() {
        let mut core = ClientCore::new(settings());
        assert_eq!(
            take(&mut core, &Mark::default(), &ok(None, &THREE)),
            NOTHING
        );
        assert_eq!(core.mark, mark(0, Some(EPOCH)), "the incarnation is learnt");
        assert_eq!(
            take(&mut core, &Mark::default(), &ok(Some(0), &THREE)),
            NOTHING,
            "mailbox numbers start at 1"
        );
        let unnamed = HttpsResponse {
            status: 200,
            headers: alloc::vec![(FIRST_HEADER.to_string(), "1".to_string())],
            body: THREE.to_vec(),
        };
        assert_eq!(
            take(&mut core, &Mark::default(), &unnamed),
            NOTHING,
            "no incarnation names the numbers"
        );
        assert_eq!(core.mark, mark(0, Some(EPOCH)));
    }
}
