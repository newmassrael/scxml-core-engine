// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// ABI version 6: a run waits out a relay's refusal for the rate and puts the same request again,
// on the clock of whoever carries the texts, and what the library decides is handed to the sink a
// host installs. What is asked of the wait, over a transport the host carries (one function the
// library calls) that answers as each row says: with the host's pause installed, a wait of seconds
// takes no real time and every wait is the host's to take; with none, the run's thread sleeps the
// relay's wait; the host raising the run's cancellation during its pause stops the run; a pause that
// returns early puts the request again; a wait that would pass the run's deadline ends the exchange
// with the refusal; the host's transport saying the relay refused it for the rate (with a wait, or
// with none it can read) is the relay's refusal or the link's failure; and a link of the library's
// own takes no pause. What is asked of the sink: that it is given the records at its level and none
// past it, is replaced and removed, refuses a level that is none, and that over a login against a
// relay in this process it is given the relay's, the driver's and the core's records and none holds
// the token, the nonce, the signature or the device's key.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_void, CStr};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use sce_peer_capi::*;
use sce_rust_relay::{
    Authority, Code, Doorway, Hub, Limits, LoginPolicy, MemoryStore, Relay, Response,
};
use serde_json::Value;

/// The sink is the process's: the tests of this file take turns.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn last_error() -> String {
    // SAFETY: the library returns a NUL-terminated text it owns.
    unsafe { CStr::from_ptr(sce_peer_last_error()) }
        .to_string_lossy()
        .into_owned()
}

fn empty() -> ScePeerBuf {
    ScePeerBuf {
        ptr: std::ptr::null_mut(),
        len: 0,
    }
}

/// The text of a buffer the library gave, which is given back; `None` for an empty buffer.
///
/// # Safety
/// `buf` is one the library returned and has not given back, or empty.
unsafe fn taken(buf: ScePeerBuf) -> Option<String> {
    if buf.ptr.is_null() {
        return None;
    }
    // SAFETY: the library's buffer is `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(buf.ptr, buf.len) }.to_vec();
    // SAFETY: returned by the library and not given back yet.
    unsafe { sce_peer_buf_free(buf) };
    Some(String::from_utf8(bytes).expect("a text the library gives is UTF-8"))
}

/// What the exchange that ended came to.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn outcome_of(peer: *mut ScePeer) -> Value {
    let mut out = empty();
    // SAFETY: `peer` is live and `out` is valid for a write.
    assert_eq!(
        unsafe { sce_peer_take_outcome(peer, &mut out) },
        SCE_PEER_OK
    );
    // SAFETY: what the call just gave.
    let text = unsafe { taken(out) }.expect("the exchange ended");
    serde_json::from_str(&text).expect("an outcome is JSON")
}

/// What a host's transport answers one exchange with.
#[derive(Debug, Clone)]
enum Answer {
    /// The relay's `rate_limited` with this wait.
    Limited(u64),
    /// The relay's endpoint refused the connection for the rate: the transport's status, with
    /// this text.
    Refused(&'static str),
    /// A refusal that ends the round.
    Closed,
}

/// What a host's pause does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pause {
    /// None is installed: the system's clock.
    None,
    /// It waits all it is asked, on the host's clock, at once.
    Waits,
    /// It returns at once without the cancellation, as for a reason of the host's.
    Early,
    /// The host raises the run's cancellation during it.
    Cancels,
}

/// The context of the host's transport.
struct Script {
    answers: Vec<Answer>,
    carried: Vec<String>,
    pauses: Vec<u64>,
    pause: Pause,
    cancel: *const ScePeerCancel,
}

/// Give `text` as the reply.
///
/// # Safety
/// `reply` is the one the library gave the call.
unsafe fn reply_with(reply: *mut ScePeerReply, text: &str) {
    // SAFETY: `reply` is live for the call, and `text` is valid for its length.
    unsafe { sce_peer_reply(reply, text.as_ptr(), text.len()) };
}

/// The host's transport: each exchange answered as the script says, the last answer again once
/// the script is spent.
///
/// # Safety
/// As `ScePeerExchangeFn` says; `context` is a `Script`.
unsafe extern "C" fn scripted(
    context: *mut c_void,
    request: *const u8,
    len: usize,
    _timeout_ms: u64,
    reply: *mut ScePeerReply,
) -> i32 {
    // SAFETY: the context the link was made with, used by the running thread alone.
    let script = unsafe { &mut *context.cast::<Script>() };
    // SAFETY: the request is `len` readable bytes for the call.
    let request = unsafe { std::slice::from_raw_parts(request, len) };
    script
        .carried
        .push(String::from_utf8_lossy(request).into_owned());
    let at = (script.carried.len() - 1).min(script.answers.len() - 1);
    match script.answers[at].clone() {
        Answer::Limited(retry_after_ms) => {
            let text =
                Response::refused(Code::RateLimited { retry_after_ms }, "slow down").to_json();
            // SAFETY: `reply` is the call's.
            unsafe { reply_with(reply, &text) };
            SCE_PEER_TRANSPORT_ANSWERED
        }
        Answer::Refused(text) => {
            // SAFETY: `reply` is the call's.
            unsafe { reply_with(reply, text) };
            SCE_PEER_TRANSPORT_RATE_LIMITED
        }
        Answer::Closed => {
            let text = Response::refused(Code::Unavailable, "closed for the test").to_json();
            // SAFETY: `reply` is the call's.
            unsafe { reply_with(reply, &text) };
            SCE_PEER_TRANSPORT_ANSWERED
        }
    }
}

/// The host's pause, as the script says.
///
/// # Safety
/// As `ScePeerPauseFn` says; `context` is a `Script`.
unsafe extern "C" fn paused(context: *mut c_void, wait_ms: u64) -> i32 {
    // SAFETY: the context the link was made with, used by the running thread alone.
    let script = unsafe { &mut *context.cast::<Script>() };
    script.pauses.push(wait_ms);
    match script.pause {
        Pause::Waits => SCE_PEER_PAUSE_WAITED,
        Pause::None | Pause::Early => SCE_PEER_PAUSE_WOKEN,
        Pause::Cancels => {
            // SAFETY: the run's cancellation, alive for the run.
            unsafe { sce_peer_cancel_raise(script.cancel) };
            SCE_PEER_PAUSE_WOKEN
        }
    }
}

/// A row of the wait's table.
struct Row {
    name: &'static str,
    answers: Vec<Answer>,
    pause: Pause,
    within_ms: u64,
}

/// What a run of a row came to.
struct Ran {
    status: i32,
    outcome: Value,
    carried: usize,
    pauses: Vec<u64>,
    took: Duration,
}

/// A sync round of a core with no person over a host's transport that answers as `row` says.
fn run(row: &Row) -> Ran {
    // SAFETY: each call passes live handles; the script outlives the link, which is freed before
    // it is taken back.
    unsafe {
        let peer = sce_peer_open(c"home".as_ptr());
        assert!(!peer.is_null(), "{}", last_error());
        let cancel = sce_peer_cancel_new();
        let script = Box::into_raw(Box::new(Script {
            answers: row.answers.clone(),
            carried: Vec::new(),
            pauses: Vec::new(),
            pause: row.pause,
            cancel,
        }));
        let link = sce_peer_link_host(Some(scripted), None, script.cast::<c_void>());
        assert!(!link.is_null(), "{}", last_error());
        if row.pause != Pause::None {
            assert_eq!(sce_peer_link_set_pause(link, Some(paused)), SCE_PEER_OK);
        }
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        let began = Instant::now();
        let status = sce_peer_run(peer, link, row.within_ms, cancel);
        let took = began.elapsed();
        let outcome = outcome_of(peer);
        sce_peer_link_free(link);
        let script = Box::from_raw(script);
        sce_peer_cancel_free(cancel);
        sce_peer_free(peer);
        assert!(
            script.carried.windows(2).all(|pair| pair[0] == pair[1]),
            "{}: the same request every time",
            row.name
        );
        Ran {
            status,
            outcome,
            carried: script.carried.len(),
            pauses: script.pauses,
            took,
        }
    }
}

/// The oracle of a row: the status, the requests put, the pauses the host was asked for, and the
/// failure and code the outcome holds.
struct Expected {
    status: i32,
    carried: usize,
    pauses: Vec<u64>,
    failure: &'static str,
    code: Option<&'static str>,
}

#[test]
fn the_wait_is_the_hosts_when_it_gave_its_pause_and_the_systems_when_not() {
    let _turn = serial();
    let rows = [
        (
            Row {
                name: "installed: two waits of five seconds, on the host's clock",
                answers: vec![
                    Answer::Limited(5_000),
                    Answer::Limited(5_000),
                    Answer::Closed,
                ],
                pause: Pause::Waits,
                within_ms: u64::MAX,
            },
            Expected {
                status: SCE_PEER_OK,
                carried: 3,
                pauses: vec![5_000, 5_000],
                failure: "relay",
                code: Some("unavailable"),
            },
        ),
        (
            Row {
                name: "not installed: the system's clock sleeps 20 ms",
                answers: vec![Answer::Limited(20), Answer::Closed],
                pause: Pause::None,
                within_ms: u64::MAX,
            },
            Expected {
                status: SCE_PEER_OK,
                carried: 2,
                pauses: Vec::new(),
                failure: "relay",
                code: Some("unavailable"),
            },
        ),
        (
            Row {
                name: "the host raises the cancellation during its pause",
                answers: vec![Answer::Limited(5_000), Answer::Closed],
                pause: Pause::Cancels,
                within_ms: u64::MAX,
            },
            Expected {
                status: SCE_PEER_ERR_CANCELLED,
                carried: 1,
                pauses: vec![5_000],
                failure: "link",
                code: None,
            },
        ),
        (
            Row {
                name: "the host's pause returns early: the request is put again",
                answers: vec![
                    Answer::Limited(5_000),
                    Answer::Limited(5_000),
                    Answer::Closed,
                ],
                pause: Pause::Early,
                within_ms: u64::MAX,
            },
            Expected {
                status: SCE_PEER_OK,
                carried: 3,
                pauses: vec![5_000, 5_000],
                failure: "relay",
                code: Some("unavailable"),
            },
        ),
        (
            Row {
                name: "a wait past the run's deadline is the exchange's refusal",
                answers: vec![Answer::Limited(60_000), Answer::Closed],
                pause: Pause::Waits,
                within_ms: 30_000,
            },
            Expected {
                status: SCE_PEER_OK,
                carried: 1,
                pauses: Vec::new(),
                failure: "relay",
                code: Some("rate_limited"),
            },
        ),
        (
            Row {
                name: "the host's transport refused for the rate, with its wait",
                answers: vec![Answer::Refused("5000"), Answer::Closed],
                pause: Pause::Waits,
                within_ms: u64::MAX,
            },
            Expected {
                status: SCE_PEER_OK,
                carried: 2,
                pauses: vec![5_000],
                failure: "relay",
                code: Some("unavailable"),
            },
        ),
        (
            Row {
                name: "the host's transport refused for the rate, with no wait it can read",
                answers: vec![Answer::Refused("soon"), Answer::Closed],
                pause: Pause::Waits,
                within_ms: u64::MAX,
            },
            Expected {
                status: SCE_PEER_OK,
                carried: 1,
                pauses: Vec::new(),
                failure: "link",
                code: None,
            },
        ),
    ];
    for (row, expected) in &rows {
        let ran = run(row);
        assert_eq!(
            ran.status,
            expected.status,
            "{}: {}",
            row.name,
            last_error()
        );
        assert_eq!(ran.carried, expected.carried, "{}", row.name);
        assert_eq!(ran.pauses, expected.pauses, "{}", row.name);
        assert_eq!(
            ran.outcome["outcome"], "failed",
            "{}: {}",
            row.name, ran.outcome
        );
        assert_eq!(
            ran.outcome["failure"], expected.failure,
            "{}: {}",
            row.name, ran.outcome
        );
        assert_eq!(
            ran.outcome["code"].as_str(),
            expected.code,
            "{}: {}",
            row.name,
            ran.outcome
        );
        if row.pause == Pause::None {
            assert!(
                ran.took >= Duration::from_millis(20),
                "{}: the system's clock took the wait",
                row.name
            );
        } else {
            assert!(
                ran.took < Duration::from_secs(4),
                "{}: no real wait for a wait on the host's clock ({:?})",
                row.name,
                ran.took
            );
        }
    }
    let past = &rows[4].0;
    let ran = run(past);
    assert_eq!(ran.outcome["retry_after_ms"], "60000", "{}", ran.outcome);
    let cancelled = run(&rows[2].0);
    assert_eq!(
        cancelled.outcome["reason"], "the exchange was cancelled",
        "{}",
        cancelled.outcome
    );
    let unread = run(&rows[6].0);
    assert!(
        unread.outcome["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("no wait")),
        "{}",
        unread.outcome
    );
}

#[test]
fn a_link_of_the_librarys_own_takes_no_pause_and_a_host_link_takes_one_or_none() {
    let _turn = serial();
    // SAFETY: each call passes live handles or null where the contract allows it.
    unsafe {
        let wss = sce_peer_link_wss(c"ws://127.0.0.1:9".as_ptr(), std::ptr::null());
        assert!(!wss.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_link_set_pause(wss, Some(paused)),
            SCE_PEER_ERR_BAD_ARGUMENT
        );
        assert!(last_error().contains("system's clock"), "{}", last_error());
        sce_peer_link_free(wss);
        assert_eq!(
            sce_peer_link_set_pause(std::ptr::null_mut(), None),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let host = sce_peer_link_host(Some(scripted), None, std::ptr::null_mut());
        assert!(!host.is_null(), "{}", last_error());
        assert_eq!(sce_peer_link_set_pause(host, Some(paused)), SCE_PEER_OK);
        assert_eq!(
            sce_peer_link_set_pause(host, None),
            SCE_PEER_OK,
            "back to the system's"
        );
        sce_peer_link_free(host);
    }
    assert_eq!(SCE_PEER_ABI_VERSION, 7);
    assert_eq!(sce_peer_abi_version(), 7);
}

/// What a sink was given: each record's level, target and line.
type Records = Mutex<Vec<(i32, String, String)>>;

/// The host's sink: each record kept in the `Records` at `user`.
///
/// # Safety
/// As `ScePeerLogFn` says; `user` is a `Records` that outlives the sink's installation.
unsafe extern "C" fn keep(
    level: i32,
    target: *const c_char,
    message: *const c_char,
    user: *mut c_void,
) {
    // SAFETY: the library's texts, NUL-terminated and alive for the call.
    let (target, message) = unsafe {
        (
            CStr::from_ptr(target).to_string_lossy().into_owned(),
            CStr::from_ptr(message).to_string_lossy().into_owned(),
        )
    };
    // SAFETY: the `Records` the host gave with the sink.
    let records = unsafe { &*user.cast_const().cast::<Records>() };
    records
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push((level, target, message));
}

/// Records that live as long as the process, so that a sink a failing test left installed never
/// writes into what was freed.
fn records() -> &'static Records {
    Box::leak(Box::new(Records::default()))
}

fn install(level: i32, records: &'static Records) -> i32 {
    // SAFETY: `keep` is as the contract says, and the records live as long as the process.
    unsafe {
        sce_peer_set_log_sink(
            level,
            Some(keep),
            std::ptr::from_ref(records).cast_mut().cast::<c_void>(),
        )
    }
}

fn remove() {
    // SAFETY: no sink and no pointer.
    assert_eq!(
        unsafe { sce_peer_set_log_sink(SCE_PEER_LOG_OFF, None, std::ptr::null_mut()) },
        SCE_PEER_OK
    );
}

fn drained(records: &Records) -> Vec<(i32, String, String)> {
    std::mem::take(&mut *records.lock().unwrap())
}

#[test]
fn the_sink_is_given_its_level_and_none_past_it_and_is_replaced_and_removed() {
    let _turn = serial();
    let records = records();
    let waits_twice = Row {
        name: "two waits on the host's clock",
        answers: vec![
            Answer::Limited(5_000),
            Answer::Limited(5_000),
            Answer::Closed,
        ],
        pause: Pause::Waits,
        within_ms: u64::MAX,
    };

    assert_eq!(
        install(SCE_PEER_LOG_INFO, records),
        SCE_PEER_OK,
        "{}",
        last_error()
    );
    run(&waits_twice);
    let info = drained(records);
    assert!(
        info.iter().all(|(level, _, _)| *level <= SCE_PEER_LOG_INFO),
        "none past its level: {info:#?}"
    );
    let waits: Vec<&(i32, String, String)> = info
        .iter()
        .filter(|(_, _, line)| line.starts_with("event=rate_limited_wait "))
        .collect();
    assert_eq!(waits.len(), 2, "{info:#?}");
    assert_eq!(waits[0].0, SCE_PEER_LOG_INFO);
    assert_eq!(waits[0].1, "sce_rust_relay::driver");
    assert!(
        waits[0].2.contains("wait_ms=5000 limited=1"),
        "{}",
        waits[0].2
    );
    assert!(
        info.iter()
            .any(|(_, target, line)| target == "sce_rust_peer"
                && line.starts_with("event=exchange_failed ")
                && line.contains("code=unavailable")),
        "the core's record of the round: {info:#?}"
    );

    // Replaced at a more verbose level: the host's pauses, at debug, come too.
    assert_eq!(install(SCE_PEER_LOG_DEBUG, records), SCE_PEER_OK);
    run(&waits_twice);
    let debug = drained(records);
    assert!(
        debug
            .iter()
            .any(|(level, _, line)| *level == SCE_PEER_LOG_DEBUG
                && line.starts_with("event=host_paused wait_ms=5000")),
        "{debug:#?}"
    );

    // Removed: nothing more.
    remove();
    run(&waits_twice);
    assert!(
        drained(records).is_empty(),
        "a sink removed is given nothing"
    );

    // A level that is none, refused, and the sink left as it was.
    assert_eq!(install(9, records), SCE_PEER_ERR_BAD_ARGUMENT);
    assert!(last_error().contains("level"), "{}", last_error());
    run(&waits_twice);
    assert!(drained(records).is_empty());
}

/// A record of another crate the library links reaches the host only at `warn` and `error`, at
/// every level of the sink: their `debug` and `trace` were measured on a device to hold a WebSocket
/// frame's payload and an MLS key derivation's context (`sce_rust_relay::logs::admits`). A record
/// of this workspace at the same levels is given as the sink's level says.
#[test]
fn a_sink_is_given_another_crates_records_at_warn_at_most_and_this_workspaces_at_its_level() {
    let _turn = serial();
    let records = records();
    let sinks = [
        SCE_PEER_LOG_ERROR,
        SCE_PEER_LOG_WARN,
        SCE_PEER_LOG_INFO,
        SCE_PEER_LOG_DEBUG,
        SCE_PEER_LOG_TRACE,
    ];
    let levels = [
        (log::Level::Error, SCE_PEER_LOG_ERROR),
        (log::Level::Warn, SCE_PEER_LOG_WARN),
        (log::Level::Info, SCE_PEER_LOG_INFO),
        (log::Level::Debug, SCE_PEER_LOG_DEBUG),
        (log::Level::Trace, SCE_PEER_LOG_TRACE),
    ];
    for sink in sinks {
        assert_eq!(install(sink, records), SCE_PEER_OK, "{}", last_error());
        for (level, abi) in levels {
            log::log!(target: "tungstenite::protocol", level, "frame payload");
            log::log!(target: "sce_rust_relay::driver", level, "own record");
            let given = drained(records);
            let foreign = given
                .iter()
                .any(|(_, target, _)| target == "tungstenite::protocol");
            let own = given
                .iter()
                .any(|(_, target, _)| target == "sce_rust_relay::driver");
            assert_eq!(
                foreign,
                abi <= sink && abi <= SCE_PEER_LOG_WARN,
                "sink {sink}, another crate's record at {level}: {given:?}"
            );
            assert_eq!(
                own,
                abi <= sink,
                "sink {sink}, this workspace's record at {level}: {given:?}"
            );
        }
    }
    remove();
}

/// The context of a host's transport to a relay in this process.
struct ToRelay {
    relay: Arc<Mutex<Relay<MemoryStore>>>,
    token: Option<String>,
    carried: Vec<(String, String)>,
}

/// The host's transport: what it carries goes to the relay in this process, and is kept.
///
/// # Safety
/// As `ScePeerExchangeFn` says; `context` is a `ToRelay`.
unsafe extern "C" fn to_relay(
    context: *mut c_void,
    request: *const u8,
    len: usize,
    _timeout_ms: u64,
    reply: *mut ScePeerReply,
) -> i32 {
    // SAFETY: the context the link was made with, used by the running thread alone.
    let carrier = unsafe { &mut *context.cast::<ToRelay>() };
    // SAFETY: the request is `len` readable bytes for the call.
    let request =
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(request, len) }).into_owned();
    let response = carrier
        .relay
        .answer(carrier.token.as_deref(), None, &request, 1_000);
    // SAFETY: `reply` is the call's.
    unsafe { reply_with(reply, &response) };
    carrier.carried.push((request, response));
    SCE_PEER_TRANSPORT_ANSWERED
}

/// Run the exchange begun on `peer` over a host's transport to `relay`, presenting `token`: what
/// was carried, each request with its response.
///
/// # Safety
/// `peer` is a live handle with an exchange begun.
unsafe fn carried_to(
    peer: *mut ScePeer,
    relay: &Arc<Mutex<Relay<MemoryStore>>>,
    token: Option<String>,
) -> Vec<(String, String)> {
    let carrier = Box::into_raw(Box::new(ToRelay {
        relay: relay.clone(),
        token,
        carried: Vec::new(),
    }));
    // SAFETY: the carrier outlives the link, which is freed before it is taken back.
    unsafe {
        let link = sce_peer_link_host(Some(to_relay), None, carrier.cast::<c_void>());
        assert!(!link.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_run(peer, link, u64::MAX, std::ptr::null()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        sce_peer_link_free(link);
        Box::from_raw(carrier).carried
    }
}

/// A device logs in at a relay in this process, the sink installed at its most verbose: it is given
/// the relay's records, the driver's and the core's, and none holds the token the relay issued, the
/// challenge's nonce, the login's signature, the device's key or the list's name.
#[test]
fn over_a_login_the_sink_is_given_no_secret() {
    let _turn = serial();
    let authority = Authority::new([4; 32], "relay-a");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default())
            .unwrap()
            .with_login(Doorway::with_key(LoginPolicy::default(), [5; 32])),
    ));
    let records = records();
    let seed = [1u8; 32];
    let key = sce_rust_trust::Identity::from_seed(seed).public().to_text();
    // SAFETY: each call passes live handles, NUL-terminated texts and 32-byte seeds.
    let (login, outcome) = unsafe {
        let peer = sce_peer_open(c"home".as_ptr());
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(sce_peer_set_identity(peer, 1, seed.as_ptr()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_set_administrator(peer, [9u8; 32].as_ptr()),
            SCE_PEER_OK
        );
        let mut genesis = [0u8; 32];
        assert_eq!(
            sce_peer_open_list(peer, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        // The relay is given the list by a round with a token of its authority.
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        carried_to(peer, &relay, Some(authority.issue("home", 1, u64::MAX)));
        let given = outcome_of(peer);
        assert_eq!(given["outcome"], "ended", "{given}");

        assert_eq!(
            install(SCE_PEER_LOG_TRACE, records),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(sce_peer_login(peer, c"relay-a".as_ptr()), SCE_PEER_OK);
        let login = carried_to(peer, &relay, None);
        let outcome = outcome_of(peer);
        remove();
        sce_peer_free(peer);
        (login, outcome)
    };
    let token = outcome["token"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("the login came to a token: {outcome}"))
        .to_string();
    let mut secrets = vec![token, key, "home".to_string()];
    for (request, response) in &login {
        let request: Value = serde_json::from_str(request).unwrap();
        let response: Value = serde_json::from_str(response).unwrap();
        for value in [&request["nonce"], &request["signature"], &response["nonce"]] {
            if let Some(text) = value.as_str() {
                secrets.push(text.to_string());
            }
        }
    }
    assert!(
        secrets.len() >= 5,
        "the token, the key, the name, a nonce, a signature: {secrets:?}"
    );

    let records = drained(records);
    let has = |target: &str, event: &str| {
        records
            .iter()
            .any(|(_, at, line)| at == target && line.starts_with(&format!("event={event} ")))
    };
    assert!(has("sce_rust_relay::login", "token_issued"), "{records:#?}");
    assert!(
        has("sce_rust_relay::relay", "login_admitted"),
        "{records:#?}"
    );
    assert!(
        has("sce_rust_relay::driver", "drive_answered"),
        "{records:#?}"
    );
    assert!(has("sce_rust_peer", "exchange_ended"), "{records:#?}");
    for (_, _, line) in &records {
        for secret in &secrets {
            assert!(!line.contains(secret.as_str()), "{secret} in {line}");
        }
    }
}
