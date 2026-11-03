// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A relay's `rate_limited` is flow control, which the one driver waits out on its clock and does not
// hand to the exchange while the wait ends before the deadline. What is asked here, over every
// combination a drive can meet, each row's end computed by an oracle written apart from the driver:
// the answer a link gives (accepted, another refusal, a failure of the link, and `rate_limited` with
// a wait of nothing, one that fits, one of exactly the time left, one past it, and never), how many
// times in a row the relay refuses for the rate (none, once, twice, and every time), the deadline
// (none, far, near, and passed before the first request), and the cancellation (never, raised before
// the first request, and raised during the first wait). For each: how the drive ended, how many times
// the request was put, every wait the clock was asked for, and the answer the exchange was given; and
// that every request put was the same one. The clock is the test's, so a wait of a minute takes no
// time; the system's clock is asked apart for what only it does (its waits take their time, and a
// cancellation raised on another thread wakes it), and so is a clock that returns before its time.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sce_rust_relay::logs::capture;
use sce_rust_relay::{
    drive, drive_with, Clock, Code, Driven, Exchange, Link, LinkError, Request, Response, Signal,
    TransportError,
};

/// The deadline the test calls near, and the one it calls far, in milliseconds from the start.
const NEAR_MS: u64 = 100;
const FAR_MS: u64 = 1_000_000;
/// The wait that fits whatever is left, but for a near deadline after three of them.
const FITS_MS: u64 = 30;
/// With no deadline there is no time left to wait exactly or to pass: these stand for the two.
const EXACTLY_WITH_NONE_MS: u64 = 50;
const EXCEEDS_WITH_NONE_MS: u64 = 2_000_000;

/// The wait a relay asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Retry {
    Zero,
    Fits,
    /// Exactly the time left to the deadline.
    Exactly,
    /// One millisecond past the time left.
    Exceeds,
    /// `u64::MAX`: the relay's word for never.
    Never,
}

const RETRIES: [Retry; 5] = [
    Retry::Zero,
    Retry::Fits,
    Retry::Exactly,
    Retry::Exceeds,
    Retry::Never,
];

/// What answers the request once the relay stops refusing it for the rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    Accepted,
    Refused,
    Failed,
}

const LASTS: [Last; 3] = [Last::Accepted, Last::Refused, Last::Failed];

/// How many times in a row the relay refuses for the rate before the last answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Limited {
    No,
    Once(Retry),
    Twice(Retry),
    /// Every time, with a wait that fits.
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Deadline {
    None,
    Far,
    Near,
    /// Passed before the first request.
    Passed,
}

const DEADLINES: [Deadline; 4] = [
    Deadline::None,
    Deadline::Far,
    Deadline::Near,
    Deadline::Passed,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cancel {
    Never,
    BeforeTheRequest,
    DuringTheFirstWait,
}

const CANCELS: [Cancel; 3] = [
    Cancel::Never,
    Cancel::BeforeTheRequest,
    Cancel::DuringTheFirstWait,
];

#[derive(Debug, Clone, Copy)]
struct Row {
    limited: Limited,
    last: Last,
    deadline: Deadline,
    cancel: Cancel,
}

impl Deadline {
    /// The deadline in milliseconds from the start.
    fn at_ms(self) -> Option<u64> {
        match self {
            Deadline::None => None,
            Deadline::Far => Some(FAR_MS),
            Deadline::Near => Some(NEAR_MS),
            Deadline::Passed => Some(0),
        }
    }
}

/// The wait `retry` stands for at `now_ms`, with the deadline at `deadline_ms`.
fn resolve(retry: Retry, now_ms: u64, deadline_ms: Option<u64>) -> u64 {
    match (retry, deadline_ms) {
        (Retry::Zero, _) => 0,
        (Retry::Fits, _) => FITS_MS,
        (Retry::Exactly, Some(deadline)) => deadline - now_ms,
        (Retry::Exactly, None) => EXACTLY_WITH_NONE_MS,
        (Retry::Exceeds, Some(deadline)) => deadline - now_ms + 1,
        (Retry::Exceeds, None) => EXCEEDS_WITH_NONE_MS,
        (Retry::Never, _) => u64::MAX,
    }
}

/// What the link gives for the request the `nth` time (from 0) it is put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Given {
    Limited(Retry),
    Last(Last),
}

fn given(row: &Row, nth: usize) -> Given {
    match (row.limited, nth) {
        (Limited::Always, _) => Given::Limited(Retry::Fits),
        (Limited::Once(retry), 0) | (Limited::Twice(retry), 0 | 1) => Given::Limited(retry),
        _ => Given::Last(row.last),
    }
}

fn answer_of(given: Given, now_ms: u64, deadline_ms: Option<u64>) -> Result<Response, LinkError> {
    match given {
        Given::Limited(retry) => Ok(Response::refused(
            Code::RateLimited {
                retry_after_ms: resolve(retry, now_ms, deadline_ms),
            },
            "this device has spent its allowance",
        )),
        Given::Last(Last::Accepted) => Ok(Response::Accepted {
            new: true,
            cursor: 1,
        }),
        Given::Last(Last::Refused) => Ok(Response::refused(Code::Forbidden, "no")),
        Given::Last(Last::Failed) => Err(LinkError("the connection closed".to_string())),
    }
}

/// What a drive of a row comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct End {
    driven: Driven,
    sent: usize,
    waits: Vec<u64>,
    answer: Result<Response, LinkError>,
}

fn stopped(driven: Driven, error: TransportError, sent: usize, waits: Vec<u64>) -> End {
    End {
        driven,
        sent,
        waits,
        answer: Err(LinkError(error.to_string())),
    }
}

/// The oracle: what a drive of `row` comes to, by the rules of flow control and not by the driver.
/// The cancellation is read before the deadline, and both before each request; a wait is taken
/// when it ends before the deadline (or there is none) and is not never; a cancellation raised
/// during a wait stops the drive when it returns; the test's clock moves only by the waits.
fn oracle(row: &Row) -> End {
    let deadline = row.deadline.at_ms();
    if row.cancel == Cancel::BeforeTheRequest {
        return stopped(Driven::Cancelled, TransportError::Cancelled, 0, Vec::new());
    }
    if deadline == Some(0) {
        return stopped(
            Driven::DeadlinePassed,
            TransportError::Deadline,
            0,
            Vec::new(),
        );
    }
    let mut now = 0u64;
    let mut waits = Vec::new();
    let mut sent = 0;
    loop {
        let this = given(row, sent);
        sent += 1;
        let answer = answer_of(this, now, deadline);
        let Given::Limited(retry) = this else {
            return End {
                driven: Driven::Ended,
                sent,
                waits,
                answer,
            };
        };
        let wait = resolve(retry, now, deadline);
        let taken = wait != u64::MAX && deadline.is_none_or(|deadline| now + wait < deadline);
        if !taken {
            return End {
                driven: Driven::Ended,
                sent,
                waits,
                answer,
            };
        }
        waits.push(wait);
        if row.cancel == Cancel::DuringTheFirstWait {
            return stopped(Driven::Cancelled, TransportError::Cancelled, sent, waits);
        }
        now += wait;
    }
}

/// The test's clock: it stands still but for the waits it is asked for, which move it at once, and
/// raises the cancellation during the wait it is told to.
struct TestClock {
    start: Instant,
    now_ms: Rc<Cell<u64>>,
    waits: RefCell<Vec<u64>>,
    /// The wait (from 1) during which the cancellation is raised.
    cancel_during: Option<usize>,
    /// Whether a wait returns at once without moving the clock, as a host's clock may.
    early: bool,
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        self.start + Duration::from_millis(self.now_ms.get())
    }

    fn wait(&self, wait: Duration, cancel: &Signal) {
        let wait_ms = u64::try_from(wait.as_millis()).unwrap();
        self.waits.borrow_mut().push(wait_ms);
        if self.cancel_during == Some(self.waits.borrow().len()) {
            cancel.raise();
            return;
        }
        if !self.early {
            self.now_ms.set(self.now_ms.get() + wait_ms);
        }
    }
}

/// A link that answers as a row says, by the test's clock.
struct TestLink {
    row: Row,
    now_ms: Rc<Cell<u64>>,
    put: Vec<String>,
}

impl Link for TestLink {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
        let nth = self.put.len();
        self.put.push(request.to_json());
        answer_of(
            given(&self.row, nth),
            self.now_ms.get(),
            self.row.deadline.at_ms(),
        )
    }
}

/// An exchange that asks one request and keeps what it is given.
#[derive(Default)]
struct OneRequest {
    given: Vec<Result<Response, LinkError>>,
}

impl Exchange for OneRequest {
    fn request(&mut self) -> Option<Request> {
        self.given.is_empty().then_some(Request::Feed {
            cursor: 7,
            limit: 100,
        })
    }

    fn answer(&mut self, answer: Result<Response, LinkError>) {
        self.given.push(answer);
    }
}

fn run(row: &Row, early: bool) -> (End, Vec<String>) {
    let now_ms = Rc::new(Cell::new(0));
    let clock = TestClock {
        start: Instant::now(),
        now_ms: Rc::clone(&now_ms),
        waits: RefCell::new(Vec::new()),
        cancel_during: (row.cancel == Cancel::DuringTheFirstWait).then_some(1),
        early,
    };
    let deadline = row
        .deadline
        .at_ms()
        .map(|at| clock.start + Duration::from_millis(at));
    let cancel = Signal::new();
    if row.cancel == Cancel::BeforeTheRequest {
        cancel.raise();
    }
    let mut link = TestLink {
        row: *row,
        now_ms,
        put: Vec::new(),
    };
    let mut exchange = OneRequest::default();
    let driven = drive_with(&mut exchange, &mut link, deadline, &cancel, &clock);
    assert_eq!(
        exchange.given.len(),
        1,
        "{row:?}: one answer for one request"
    );
    let end = End {
        driven,
        sent: link.put.len(),
        waits: clock.waits.into_inner(),
        answer: exchange.given.remove(0),
    };
    (end, link.put)
}

fn rows() -> Vec<Row> {
    let mut sequences = vec![Limited::No, Limited::Always];
    for retry in RETRIES {
        sequences.push(Limited::Once(retry));
        sequences.push(Limited::Twice(retry));
    }
    let mut rows = Vec::new();
    for limited in sequences {
        // The last answer of a relay that always refuses is never given.
        let lasts: &[Last] = if limited == Limited::Always {
            &[Last::Accepted]
        } else {
            &LASTS
        };
        for &last in lasts {
            for deadline in DEADLINES {
                for cancel in CANCELS {
                    // A relay that always refuses, no deadline and no cancellation: a drive the
                    // host gave no end to, and that the relay gives none. Asked apart below.
                    if limited == Limited::Always
                        && deadline == Deadline::None
                        && cancel == Cancel::Never
                    {
                        continue;
                    }
                    rows.push(Row {
                        limited,
                        last,
                        deadline,
                        cancel,
                    });
                }
            }
        }
    }
    rows
}

#[test]
fn every_combination_a_drive_meets_ends_as_the_oracle_says() {
    let rows = rows();
    assert_eq!(
        rows.len(),
        (2 * RETRIES.len() + 1) * LASTS.len() * 12 + 12 - 1
    );
    let mut waited = 0;
    let mut given_limited = 0;
    for row in &rows {
        let (end, put) = run(row, false);
        assert_eq!(end, oracle(row), "{row:?}");
        assert!(
            put.windows(2).all(|pair| pair[0] == pair[1]),
            "{row:?}: the same request every time"
        );
        waited += usize::from(!end.waits.is_empty());
        given_limited += usize::from(matches!(
            end.answer,
            Ok(Response::Refused {
                code: Code::RateLimited { .. },
                ..
            })
        ));
    }
    // The table reaches both ends of flow control: waits taken, and refusals given over.
    assert!(waited > 0 && given_limited > 0, "{waited} {given_limited}");
}

/// Spot rows the oracle and the driver must agree on, written out, so that an oracle that drifted
/// with the driver is caught too.
#[test]
fn the_oracle_says_what_flow_control_means_on_rows_written_out() {
    let row = |limited, last, deadline, cancel| Row {
        limited,
        last,
        deadline,
        cancel,
    };
    let limited = |retry_after_ms| {
        Ok(Response::refused(
            Code::RateLimited { retry_after_ms },
            "this device has spent its allowance",
        ))
    };
    // The phone of the field: a wait of 20 ms, no deadline: waited out, and the write accepted.
    let once = oracle(&row(
        Limited::Once(Retry::Fits),
        Last::Accepted,
        Deadline::None,
        Cancel::Never,
    ));
    assert_eq!(once.driven, Driven::Ended);
    assert_eq!((once.sent, once.waits.clone()), (2, vec![FITS_MS]));
    assert_eq!(
        once.answer,
        Ok(Response::Accepted {
            new: true,
            cursor: 1
        })
    );
    // Exactly the time left to a near deadline: given over, with its wait, after one request.
    let exactly = oracle(&row(
        Limited::Once(Retry::Exactly),
        Last::Accepted,
        Deadline::Near,
        Cancel::Never,
    ));
    assert_eq!((exactly.sent, exactly.waits.len()), (1, 0));
    assert_eq!(exactly.answer, limited(NEAR_MS));
    // Always refused before a near deadline: three waits of 30 fit, the fourth does not.
    let always = oracle(&row(
        Limited::Always,
        Last::Accepted,
        Deadline::Near,
        Cancel::Never,
    ));
    assert_eq!(always.waits, vec![FITS_MS; 3]);
    assert_eq!((always.sent, always.answer.clone()), (4, limited(FITS_MS)));
    // Cancelled during the first wait: the link's failure, after one request.
    let cancelled = oracle(&row(
        Limited::Twice(Retry::Zero),
        Last::Accepted,
        Deadline::Far,
        Cancel::DuringTheFirstWait,
    ));
    assert_eq!(cancelled.driven, Driven::Cancelled);
    assert_eq!(cancelled.sent, 1);
    assert_eq!(
        cancelled.answer,
        Err(LinkError(TransportError::Cancelled.to_string()))
    );
    // Never: given over at once, whatever the deadline.
    let never = oracle(&row(
        Limited::Once(Retry::Never),
        Last::Accepted,
        Deadline::None,
        Cancel::Never,
    ));
    assert_eq!((never.sent, never.answer), (1, limited(u64::MAX)));
}

/// A relay that always refuses, with no deadline: the drive waits for as long as the relay asks,
/// and the cancellation is what ends it, during whichever wait it is raised in.
#[test]
fn a_drive_with_no_deadline_waits_until_it_is_cancelled() {
    for during in [1usize, 5, 40] {
        let row = Row {
            limited: Limited::Always,
            last: Last::Accepted,
            deadline: Deadline::None,
            cancel: Cancel::Never,
        };
        let now_ms = Rc::new(Cell::new(0));
        let clock = TestClock {
            start: Instant::now(),
            now_ms: Rc::clone(&now_ms),
            waits: RefCell::new(Vec::new()),
            cancel_during: Some(during),
            early: false,
        };
        let mut link = TestLink {
            row,
            now_ms,
            put: Vec::new(),
        };
        let mut exchange = OneRequest::default();
        let driven = drive_with(&mut exchange, &mut link, None, &Signal::new(), &clock);
        assert_eq!(driven, Driven::Cancelled);
        assert_eq!(link.put.len(), during, "one request for each wait");
        assert_eq!(clock.waits.into_inner(), vec![FITS_MS; during]);
        assert_eq!(
            exchange.given,
            vec![Err(LinkError(TransportError::Cancelled.to_string()))]
        );
    }
}

/// A clock that returns before its time, and not for a cancellation (a host whose own reason it
/// is): the request is put again at once, and the relay is asked again what is left.
#[test]
fn a_wait_that_returns_early_puts_the_request_again() {
    let row = Row {
        limited: Limited::Twice(Retry::Fits),
        last: Last::Accepted,
        deadline: Deadline::Near,
        cancel: Cancel::Never,
    };
    let (end, put) = run(&row, true);
    assert_eq!(end.driven, Driven::Ended);
    assert_eq!(end.sent, 3);
    assert_eq!(end.waits, vec![FITS_MS, FITS_MS]);
    assert_eq!(
        end.answer,
        Ok(Response::Accepted {
            new: true,
            cursor: 1
        })
    );
    assert_eq!(put.len(), 3);
}

/// Each wait is one record with the request's kind, how long and how many refusals in a row; a
/// refusal given over because its wait does not fit is one with the time left; the answer that
/// ended it is one at `debug`.
#[test]
fn each_wait_and_each_refusal_given_over_is_one_record() {
    capture::start();
    let row = Row {
        limited: Limited::Twice(Retry::Fits),
        last: Last::Accepted,
        deadline: Deadline::Near,
        cancel: Cancel::Never,
    };
    run(&row, false);
    let lines: Vec<String> = capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect();
    assert_eq!(
        lines,
        vec![
            "event=rate_limited_wait op=feed wait_ms=30 limited=1 deadline=true".to_string(),
            "event=rate_limited_wait op=feed wait_ms=30 limited=2 deadline=true".to_string(),
            "event=drive_answered op=feed answer=accepted limited=2".to_string(),
        ]
    );
    let row = Row {
        limited: Limited::Once(Retry::Exceeds),
        last: Last::Accepted,
        deadline: Deadline::Near,
        cancel: Cancel::Never,
    };
    run(&row, false);
    let lines: Vec<String> = capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect();
    assert_eq!(
        lines,
        vec![
            "event=rate_limited_given op=feed retry_after_ms=101 left_ms=100 limited=1".to_string()
        ]
    );
}

/// A link of real waits: a relay that asks for 20 ms twice is waited out on the system's clock,
/// which takes the time.
struct Twice {
    put: usize,
}

impl Link for Twice {
    fn call(&mut self, _: &Request) -> Result<Response, LinkError> {
        self.put += 1;
        if self.put <= 2 {
            Ok(Response::refused(
                Code::RateLimited { retry_after_ms: 20 },
                "slow down",
            ))
        } else {
            Ok(Response::Accepted {
                new: true,
                cursor: 1,
            })
        }
    }
}

#[test]
fn the_systems_clock_takes_the_time_of_each_wait() {
    let mut link = Twice { put: 0 };
    let mut exchange = OneRequest::default();
    let began = Instant::now();
    assert_eq!(
        drive(&mut exchange, &mut link, None, &Signal::new()),
        Driven::Ended
    );
    assert!(began.elapsed() >= Duration::from_millis(40));
    assert_eq!(link.put, 3);
    assert!(matches!(exchange.given[0], Ok(Response::Accepted { .. })));
}

/// A wait of a minute on the system's clock, ended by a cancellation another thread raises: the
/// drive stops at once with the link's failure.
#[test]
fn a_cancellation_from_another_thread_ends_a_wait_on_the_systems_clock() {
    struct Minute;
    impl Link for Minute {
        fn call(&mut self, _: &Request) -> Result<Response, LinkError> {
            Ok(Response::refused(
                Code::RateLimited {
                    retry_after_ms: 60_000,
                },
                "slow down",
            ))
        }
    }
    let cancel = Signal::new();
    let raiser = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            cancel.raise();
        })
    };
    let mut exchange = OneRequest::default();
    let began = Instant::now();
    let deadline = Some(Instant::now() + Duration::from_secs(600));
    assert_eq!(
        drive(&mut exchange, &mut Minute, deadline, &cancel),
        Driven::Cancelled
    );
    assert!(
        began.elapsed() < Duration::from_secs(30),
        "woken, not waited out"
    );
    assert_eq!(
        exchange.given,
        vec![Err(LinkError(TransportError::Cancelled.to_string()))]
    );
    raiser.join().unwrap();
}
