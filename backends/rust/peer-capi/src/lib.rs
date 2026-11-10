// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C ABI of the peer core of a device that holds a shared list (`sce-rust-peer`, ADR 0079).
//!
//! An application lane that is not Rust holds the core of one device of one list ([`ScePeer`]) and
//! speaks to it in texts: the machine's outbound events as their name and JSON payload
//! ([`sce_peer_outbound`]), the relay's requests and responses as the protocol's JSON
//! ([`sce_peer_poll_request`], [`sce_peer_respond`], [`sce_peer_fail`]), what a round came to as one
//! JSON text ([`sce_peer_take_outcome`]), and what the device keeps between runs, its secrets with
//! the rest, as one sealed state and the data key the host wraps with its platform's key store
//! ([`sce_peer_save_sealed`], [`sce_peer_restore_sealed`]). An administrator's device opens a list
//! ([`sce_peer_open_list`]), grants, admits and takes off devices ([`sce_peer_grant`],
//! [`sce_peer_add_member`], [`sce_peer_remove_member`]), and any device reads what it stands at in
//! the list as one JSON text ([`sce_peer_standing`]). No structure is laid out twice in C: the shapes
//! are the texts, written down in `docs/SCE_SHARED_LIST_PEER_ABI.md`.
//!
//! A lane that does not carry the texts itself runs an exchange over a link the library holds
//! ([`ScePeerLink`]): the relay's WebSocket binding ([`sce_peer_link_wss`], and
//! [`sce_peer_link_wss_believing`] for a relay under a deployment's own authority), or a transport the host
//! writes as one function the library calls ([`sce_peer_link_host`]). [`sce_peer_run`] runs the
//! exchange to its end on the calling thread, a background worker's, by a deadline and until a
//! cancellation another thread raises ([`ScePeerCancel`]); the host tells the link when the network
//! changed ([`sce_peer_link_network_changed`]), from any thread. The loop is the relay's one driver,
//! so framing, authentication and reconnection are written once, in Rust. A device that has no
//! token logs in with its person's key ([`sce_peer_login`], ADR 0080) over a link made with none,
//! and gives the link the token the outcome holds ([`sce_peer_link_set_token`]).
//!
//! Every function that does something returns 0 for success and a nonzero status otherwise, and
//! says why in [`sce_peer_last_error`] (per thread). Buffers the library returns ([`ScePeerBuf`])
//! are the library's and are given back with [`sce_peer_buf_free`]; an empty buffer (a null pointer
//! and a length of 0) is how a call says there is nothing to give: no request to put to the relay, no
//! outcome to take, no settlement. The header is `include/sce_peer.h`, which is written by hand and
//! kept equal to this file's exports by a test.
//!
//! This is the one crate of the peer core that holds `unsafe` code. Every pointer a caller gives is
//! checked for null, and is read for exactly the length it comes with; a panic is caught and
//! reported and never crosses the boundary.

#![deny(unsafe_op_in_unsafe_fn)]

use std::cell::{Cell, RefCell, UnsafeCell};
use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{OnceLock, PoisonError, RwLock};
use std::time::{Duration, Instant};

use sce_rust_peer::{
    drive, drive_with, Clock, CourierError, DataKey, Device, Driven, GroupStep, Kept, Outcome,
    Peer, PeerError, Profile, Publication, Round, RoundKind, Signal, Standing, Transport,
    TransportError,
};
use sce_rust_relay::tls::{self, Roots};
use sce_rust_relay::wss::WssLink;
use sce_rust_relay::{Code, Notice};
use sce_rust_trust::invitation::Ticket;
use sce_rust_trust::{encoding, Identity, PublicKey};
use serde_json::{json, Value};

/// The version of this ABI: a change to what a function takes or returns, or to the shape of a text
/// it gives, makes it a new number. A function added alone does not, since a lane built against the
/// number before calls none of it. Version 2: the outcome holds `owed`, and the save is the core's
/// version 2, which holds what the relay is owed. Version 3: what the device keeps is one sealed
/// state, its secrets with the rest, under a data key the host wraps
/// ([`sce_peer_save_sealed`], [`sce_peer_restore_sealed`]); the plain save and restore and the
/// group's state in the clear are gone, and [`sce_peer_with_group`] makes a new device only.
/// Version 4: a send the device may not sign yet is kept and not refused
/// (`SCE_PEER_KEPT_WAITING` from [`sce_peer_outbound`], an answer a lane of version 3 does not
/// know), the outcome holds `taken` and `dropped` and the standing `waiting`, and the sealed state
/// is the core's version 4, which holds what waits. Version 5: a device logs in (ADR 0080,
/// [`sce_peer_login`]) and the outcome of a login holds `token`; a link made with a null or empty
/// token dials a connection that presents none, where version 4 presented the empty text as a token
/// the relay refused, a changed meaning of an input; and a link is given the token a login earned
/// ([`sce_peer_link_set_token`]). The invitations and the join of ADR 0080 section 4
/// ([`sce_peer_invite`], [`sce_peer_invite_under`], [`sce_peer_invitation_read`],
/// [`sce_peer_invited_by`], [`sce_peer_join`], [`sce_peer_joined`]) are functions added alone, with
/// statuses appended (`SCE_PEER_ERR_GRANTED`, `_INVITATION`, `_NOT_JOINED`) that only they and the
/// acts on a node they invited give, and move no number; the sealed state is the core's version 5,
/// which holds the invitations. The creation of ADR 0080 section 3 moves none either: a login that
/// meets a relay holding no such list and asking work creates the list ([`sce_peer_login`]), work no
/// relay asked before, and its outcome has the shape of a login's; and [`sce_peer_certified_name`]
/// is a function added alone. Version 6: a run waits out the relay's refusal for the rate and puts
/// the same request again while the wait ends before its deadline, where version 5 ended the round
/// with the refusal, a changed meaning of [`sce_peer_run`]; a host's transport may say the relay
/// refused it for the rate (`SCE_PEER_TRANSPORT_RATE_LIMITED`) and give the wait its run takes on
/// the host's own clock ([`sce_peer_link_set_pause`]); and a host installs the sink of what the
/// library logs ([`sce_peer_set_log_sink`]), with a status appended (`SCE_PEER_ERR_SINK_TAKEN`).
/// Version 7: a change of the list is one event and one signed operation, `record_list.change`
/// (docs/adr/0085, section 1), so what [`sce_peer_outbound`] takes and the events an outcome holds
/// are of another shape than the parts `record_list.write` and `record_list.supersede` of version 6,
/// which the core no longer signs: a changed shape of a text, which makes a new number.
/// Version 8: the core of a copy that keeps a journal (docs/adr/0083, landing 3; docs/adr/0085,
/// landing 5; docs/adr/0093, section 6). A change the copy offers is signed in the call that takes
/// it ([`sce_peer_made`]), and is a record the host journals ([`sce_peer_take_records`]) that no
/// round gives the relay before the host says it recorded it ([`sce_peer_recorded`]), which a lane
/// of version 7 never says; the outcome holds `withdrawn` in place of `dropped`, a send that never
/// will be signed withdrawn with every send after it and the machine built again for none of them,
/// and `raised` when the device's own stamp rose; the word that an instant is settled
/// (`sce_peer_floor_event`) is gone, since the list reads no floor, and the stable cut
/// ([`sce_peer_stable_cut`]) and a device's mark ([`sce_peer_mark`]) say what every member was
/// told; and a host started again gives back its journal's records
/// ([`sce_peer_restore_records`]), recalls the history ([`sce_peer_history`]) and raises its
/// copy's clock to the device's own stamp ([`sce_peer_own_stamp`]). The sealed state is the core's
/// version 6, which holds the outbox. The fold of a list held before a change was one unit
/// (docs/adr/0085, section 8.3) moves no number: [`sce_peer_fold`] is a function added alone, and
/// [`sce_peer_restore_sealed`] opens and upgrades a state of the core's version 5, which it refused
/// as not opened before: a state no core of this number seals, and every output keeps its shape.
pub const SCE_PEER_ABI_VERSION: u32 = 8;

pub const SCE_PEER_OK: i32 = 0;
pub const SCE_PEER_ERR_NOTHING_ASKED: i32 = 1;
pub const SCE_PEER_ERR_OUTCOME_NOT_TAKEN: i32 = 2;
pub const SCE_PEER_ERR_NO_GROUP_DEVICE: i32 = 3;
pub const SCE_PEER_ERR_GROUP: i32 = 4;
pub const SCE_PEER_ERR_UNREADABLE: i32 = 5;
pub const SCE_PEER_ERR_NO_ADMINISTRATOR: i32 = 6;
pub const SCE_PEER_ERR_NO_PERSON: i32 = 7;
pub const SCE_PEER_ERR_OPENED: i32 = 8;
pub const SCE_PEER_ERR_LOG: i32 = 9;
pub const SCE_PEER_ERR_NOT_A_MEMBER: i32 = 10;
pub const SCE_PEER_ERR_ADMITTED_THROUGH_GROUP: i32 = 11;
pub const SCE_PEER_ERR_NO_KEY_PACKAGE: i32 = 12;
pub const SCE_PEER_ERR_DRAWN_KEYS: i32 = 13;
/// A sealed state did not open under the key given for this list and node: another key, a changed
/// or cut state, or another device's. Nothing of it was taken.
pub const SCE_PEER_ERR_NOT_OPENED: i32 = 14;
/// The operating system gave no randomness for a data key or an invitation's secret.
pub const SCE_PEER_ERR_NO_RANDOMNESS: i32 = 15;
/// The log has granted the node an invitation is asked for: an invitation is for a node it never
/// granted.
pub const SCE_PEER_ERR_GRANTED: i32 = 16;
/// An invitation that is not one, or one this core cannot make or join with (another list or
/// genesis, another node than its person's, a lifetime past a day, a list not opened yet).
pub const SCE_PEER_ERR_INVITATION: i32 = 17;
/// The core invited the node, and no join a round read proves the key given under its invitation.
pub const SCE_PEER_ERR_NOT_JOINED: i32 = 18;
pub const SCE_PEER_ERR_NULL_ARGUMENT: i32 = 20;
pub const SCE_PEER_ERR_BAD_TEXT: i32 = 21;
pub const SCE_PEER_ERR_PANIC: i32 = 22;
pub const SCE_PEER_ERR_BAD_ARGUMENT: i32 = 23;
/// [`sce_peer_run`] stopped because its cancellation was raised, or because its deadline passed:
/// the exchange ended as a failure of the link, and its outcome waits.
pub const SCE_PEER_ERR_CANCELLED: i32 = 24;
pub const SCE_PEER_ERR_DEADLINE: i32 = 25;
/// [`sce_peer_set_log_sink`]: the process has another sink of the library's records installed (a
/// Rust library in the same image installed its own), and the host's is not installed.
pub const SCE_PEER_ERR_SINK_TAKEN: i32 = 26;

/// What a host's transport says of one exchange ([`ScePeerExchangeFn`]): the response was given,
/// or why it was not ([`TransportError`]).
pub const SCE_PEER_TRANSPORT_ANSWERED: i32 = 0;
pub const SCE_PEER_TRANSPORT_LINK: i32 = 1;
pub const SCE_PEER_TRANSPORT_UNAUTHORIZED: i32 = 2;
pub const SCE_PEER_TRANSPORT_DEADLINE: i32 = 3;
pub const SCE_PEER_TRANSPORT_CANCELLED: i32 = 4;
/// The relay's endpoint refused the connection for the rate before it carried the request (an
/// HTTP 429): the text given with [`sce_peer_reply`] is the wait it asked for, in milliseconds, as
/// decimal digits. The run waits it out as it waits out the relay's `rate_limited`.
pub const SCE_PEER_TRANSPORT_RATE_LIMITED: i32 = 5;

/// What a host's pause says of the wait it was asked for ([`ScePeerPauseFn`]): it waited all of
/// it, or it returned before (the run's cancellation was raised, or a reason of the host's).
pub const SCE_PEER_PAUSE_WAITED: i32 = 0;
pub const SCE_PEER_PAUSE_WOKEN: i32 = 1;

/// The levels of what the library logs ([`sce_peer_set_log_sink`]): a sink is given the records at
/// its level and the levels before it; `SCE_PEER_LOG_OFF` is none.
pub const SCE_PEER_LOG_OFF: i32 = 0;
pub const SCE_PEER_LOG_ERROR: i32 = 1;
pub const SCE_PEER_LOG_WARN: i32 = 2;
pub const SCE_PEER_LOG_INFO: i32 = 3;
pub const SCE_PEER_LOG_DEBUG: i32 = 4;
pub const SCE_PEER_LOG_TRACE: i32 = 5;

/// What [`sce_peer_outbound`] says the core did with a send ([`Kept`]).
pub const SCE_PEER_KEPT_TAKEN: i32 = 0;
pub const SCE_PEER_KEPT_HELD: i32 = 1;
pub const SCE_PEER_KEPT_NOT_SIGNED: i32 = 2;
pub const SCE_PEER_KEPT_REFUSED: i32 = 3;
/// The device may not sign it yet: it is kept, and the round that reads what it waits for signs it
/// and gives it the relay, or refuses it when the log says the person never may.
pub const SCE_PEER_KEPT_WAITING: i32 = 4;

/// The kinds of round [`sce_peer_start`] begins ([`RoundKind`]).
pub const SCE_PEER_ROUND_SYNC: i32 = 0;
pub const SCE_PEER_ROUND_RESYNC: i32 = 1;
pub const SCE_PEER_ROUND_REREAD: i32 = 2;

/// Whether the hub of a list [`sce_peer_open_list`] opens is trusted ([`Profile`]).
pub const SCE_PEER_PROFILE_UNTRUSTED: i32 = 0;
pub const SCE_PEER_PROFILE_TRUSTED_HUB: i32 = 1;

/// Which authorities a link of [`sce_peer_link_wss_believing`] believes ([`Roots`]): the ones it is
/// given alone, or those and the public ones.
pub const SCE_PEER_ROOTS_ONLY: i32 = 0;
pub const SCE_PEER_ROOTS_AND_PUBLIC: i32 = 1;

/// The core of one device of one list, and the list's name, which the group's device is made for.
pub struct ScePeer {
    list: String,
    peer: Peer,
}

impl ScePeer {
    /// Apply one of the core's options that take the core and give it back ([`Peer::for_a_hub`],
    /// [`Peer::with_group`]) to the core this handle holds.
    fn rebuild(&mut self, with: impl FnOnce(Peer) -> Peer) {
        let peer = std::mem::replace(&mut self.peer, Peer::open(&self.list));
        self.peer = with(peer);
    }
}

/// A buffer the library made. It is freed with [`sce_peer_buf_free`] and nothing else.
#[repr(C)]
pub struct ScePeerBuf {
    pub ptr: *mut u8,
    pub len: usize,
}

impl ScePeerBuf {
    const EMPTY: ScePeerBuf = ScePeerBuf {
        ptr: std::ptr::null_mut(),
        len: 0,
    };

    fn of(bytes: Vec<u8>) -> Self {
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        Self {
            ptr: Box::into_raw(boxed).cast::<u8>(),
            len,
        }
    }
}

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

fn fail(status: i32, message: impl Into<Vec<u8>>) -> i32 {
    let text = CString::new(message).unwrap_or_else(|_| CString::from(c"an error"));
    LAST_ERROR.with(|last| *last.borrow_mut() = text);
    status
}

fn status_of(error: &PeerError) -> i32 {
    match error {
        PeerError::NothingAsked => SCE_PEER_ERR_NOTHING_ASKED,
        PeerError::OutcomeNotTaken => SCE_PEER_ERR_OUTCOME_NOT_TAKEN,
        PeerError::NoGroupDevice => SCE_PEER_ERR_NO_GROUP_DEVICE,
        PeerError::Group(_) => SCE_PEER_ERR_GROUP,
        PeerError::Unreadable(_) => SCE_PEER_ERR_UNREADABLE,
        PeerError::NoAdministrator => SCE_PEER_ERR_NO_ADMINISTRATOR,
        PeerError::NoPerson => SCE_PEER_ERR_NO_PERSON,
        PeerError::Opened => SCE_PEER_ERR_OPENED,
        PeerError::Log(_) => SCE_PEER_ERR_LOG,
        PeerError::NotAMember(_) => SCE_PEER_ERR_NOT_A_MEMBER,
        PeerError::NoSuchRole(_) => SCE_PEER_ERR_BAD_ARGUMENT,
        PeerError::AdmittedThroughGroup(_) => SCE_PEER_ERR_ADMITTED_THROUGH_GROUP,
        PeerError::NoKeyPackage(_) => SCE_PEER_ERR_NO_KEY_PACKAGE,
        PeerError::DrawnKeys => SCE_PEER_ERR_DRAWN_KEYS,
        PeerError::NotOpened => SCE_PEER_ERR_NOT_OPENED,
        PeerError::NoRandomness => SCE_PEER_ERR_NO_RANDOMNESS,
        PeerError::Granted(_) => SCE_PEER_ERR_GRANTED,
        PeerError::Invitation(_) => SCE_PEER_ERR_INVITATION,
        PeerError::NotJoined(_) => SCE_PEER_ERR_NOT_JOINED,
    }
}

/// A role as the core takes it: a number a lane gives wider than a role is refused, not cut.
fn role_of(role: u32) -> Result<u8, i32> {
    u8::try_from(role).map_err(|_| fail(SCE_PEER_ERR_BAD_ARGUMENT, format!("{role} is not a role")))
}

fn from_error(error: PeerError) -> i32 {
    let status = status_of(&error);
    fail(status, format!("{error:?}"))
}

/// Run `body`, turning a panic into a status.
fn guard(body: impl FnOnce() -> Result<(), i32>) -> i32 {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(())) => SCE_PEER_OK,
        Ok(Err(status)) => status,
        Err(_) => fail(SCE_PEER_ERR_PANIC, "the library panicked"),
    }
}

/// The bytes `ptr` points at for `len`, or null-argument when it is null and `len` is not 0.
///
/// # Safety
/// `ptr` is valid for reads of `len` bytes, as the caller promises of every pointer it passes.
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], i32> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(fail(SCE_PEER_ERR_NULL_ARGUMENT, "a pointer is null"));
    }
    // SAFETY: not null, and valid for `len` bytes by the caller's promise.
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// A fixed-size array a caller points at.
///
/// # Safety
/// `ptr` is valid for reads of `N` bytes.
unsafe fn array<const N: usize>(ptr: *const u8) -> Result<[u8; N], i32> {
    // SAFETY: the caller's promise of N readable bytes.
    let slice = unsafe { bytes(ptr, N) }?;
    slice
        .try_into()
        .map_err(|_| fail(SCE_PEER_ERR_NULL_ARGUMENT, "a pointer is null"))
}

/// A text the caller gives as a NUL-terminated string.
///
/// # Safety
/// `ptr` is null or points at a NUL-terminated string.
unsafe fn text<'a>(ptr: *const c_char) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Err(fail(SCE_PEER_ERR_NULL_ARGUMENT, "a string is null"));
    }
    // SAFETY: not null and NUL-terminated by the caller's promise.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| fail(SCE_PEER_ERR_BAD_TEXT, "a string is not UTF-8"))
}

/// The text of an invitation the caller gives as `len` bytes of UTF-8, read as an invitation
/// ([`Ticket`]).
///
/// # Safety
/// `ptr` is valid for reads of `len` bytes.
unsafe fn ticket_of(ptr: *const u8, len: usize) -> Result<Ticket, i32> {
    // SAFETY: the caller's promise of `len` readable bytes.
    let given = unsafe { bytes(ptr, len) }?;
    let text = std::str::from_utf8(given)
        .map_err(|_| fail(SCE_PEER_ERR_BAD_TEXT, "an invitation is not UTF-8"))?;
    Ticket::from_text(text).map_err(|error| {
        fail(
            SCE_PEER_ERR_INVITATION,
            format!("it is not an invitation: {error:?}"),
        )
    })
}

/// A token the caller gives, null for none: a link with none dials a connection that presents no
/// token, which a device logs in on.
///
/// # Safety
/// `ptr` is null or points at a NUL-terminated string.
unsafe fn token_or_none<'a>(ptr: *const c_char) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Ok("");
    }
    // SAFETY: not null, and NUL-terminated by the caller's promise.
    unsafe { text(ptr) }
}

/// The object a handle names.
///
/// # Safety
/// `ptr` is null or a live handle made by this library, not used by another thread meanwhile.
unsafe fn handle<'a, T>(ptr: *mut T) -> Result<&'a mut T, i32> {
    // SAFETY: the caller's promise.
    unsafe { ptr.as_mut() }.ok_or_else(|| fail(SCE_PEER_ERR_NULL_ARGUMENT, "a handle is null"))
}

/// The object a handle names, shared: what another thread may reach while one uses it (a link's
/// network signal, a cancellation), and nothing it holds is changed through it but by a lock.
///
/// # Safety
/// `ptr` is null or a live handle made by this library.
unsafe fn shared<'a, T>(ptr: *const T) -> Result<&'a T, i32> {
    // SAFETY: the caller's promise.
    unsafe { ptr.as_ref() }.ok_or_else(|| fail(SCE_PEER_ERR_NULL_ARGUMENT, "a handle is null"))
}

/// Put `value` where the caller said, when it said.
///
/// # Safety
/// `out` is null or valid for a write of a `T`.
unsafe fn put<T>(out: *mut T, value: T) {
    if !out.is_null() {
        // SAFETY: not null, and valid for a write by the caller's promise.
        unsafe { out.write(value) };
    }
}

/// A place the caller gives for what a call is for: refused when it is null, before anything is
/// done, so that nothing is taken from the core that would then be lost (an outcome is given once).
fn required<T>(out: *mut T) -> Result<(), i32> {
    if out.is_null() {
        Err(fail(SCE_PEER_ERR_NULL_ARGUMENT, "an out pointer is null"))
    } else {
        Ok(())
    }
}

/// Make an object (a core, a link, a cancellation) and hand it out as a handle; null when `make`
/// fails, and why is in [`sce_peer_last_error`].
fn made<T>(make: impl FnOnce() -> Result<T, i32>) -> *mut T {
    let mut pointer = std::ptr::null_mut();
    guard(|| {
        pointer = Box::into_raw(Box::new(make()?));
        Ok(())
    });
    pointer
}

/// A pair of 64-bit numbers as the texts the relay's protocol writes them as.
fn numbered(pairs: &[(u64, u64)]) -> impl Iterator<Item = Value> + '_ {
    pairs
        .iter()
        .map(|(node, seq)| json!([node.to_string(), seq.to_string()]))
}

/// What the group's device did with one thing it read.
fn group_step(step: &GroupStep) -> Value {
    match step {
        GroupStep::Joined(epoch) => json!({"step": "joined", "epoch": epoch.to_string()}),
        GroupStep::Applied(epoch) => json!({"step": "applied", "epoch": epoch.to_string()}),
        GroupStep::Removed => json!({"step": "removed"}),
        GroupStep::Refused(reason) => json!({"step": "refused", "reason": reason}),
    }
}

/// What an exchange came to, as the one text [`sce_peer_take_outcome`] gives.
fn outcome_text(outcome: &Result<Outcome, CourierError>) -> String {
    let value = match outcome {
        Ok(outcome) => {
            let rounds: Vec<&Round> = std::iter::once(&outcome.round)
                .chain(outcome.reread.as_ref())
                .collect();
            let mut value = json!({
                "outcome": "ended",
                "events": outcome
                    .events()
                    .into_iter()
                    .map(|(event, data)| json!([event, data]))
                    .collect::<Vec<_>>(),
                "refused": outcome.refused(),
                "pushed": rounds.iter().map(|round| round.pushed).sum::<usize>(),
                "evicted": rounds
                    .iter()
                    .copied()
                    .flat_map(|round| numbered(&round.evicted))
                    .collect::<Vec<_>>(),
                "restored": rounds
                    .iter()
                    .copied()
                    .flat_map(|round| numbered(&round.restored))
                    .collect::<Vec<_>>(),
                "taken": outcome
                    .taken()
                    .into_iter()
                    .map(|(event, data)| json!([event, data]))
                    .collect::<Vec<_>>(),
                "withdrawn": outcome
                    .withdrawn()
                    .into_iter()
                    .map(|one| json!({
                        "event": one.event,
                        "data": one.data,
                        "wallTime": one.wall_time.to_string(),
                        "counter": one.counter,
                        "why": one.why,
                    }))
                    .collect::<Vec<_>>(),
                "rebuild": outcome.rebuild(),
                "stalled": outcome.stalled(),
                "reread": outcome.reread.is_some(),
                "steps": outcome.steps.iter().map(group_step).collect::<Vec<_>>(),
                "owed": outcome.owed,
            });
            // A login's outcome alone holds a token: the member is not written for a round.
            if let Some(token) = &outcome.token {
                value["token"] = json!({
                    "token": token.token,
                    "expires_at_ms": token.expires_at_ms.to_string(),
                });
            }
            // Written only for an exchange that froze the device's own chain, so that the text of
            // every other outcome is the one it was: a lane reads it by its name, and meets it only
            // from a relay of protocol 2, which answers `conflict`, or a feed that shows the device
            // signed two histories.
            if let Some(frozen) = outcome.conflicted() {
                value["conflicted"] = json!(frozen.to_string());
            }
            // Written only for a round that raised the device's own stamp: the host gives its copy
            // `record_list.raise` with it (docs/adr/0093, section 4.3).
            if let Some((wall_time, counter)) = outcome.raised {
                value["raised"] = json!({
                    "wallTime": wall_time.to_string(),
                    "counter": counter,
                });
            }
            value
        }
        Err(CourierError::Link(reason)) => {
            json!({"outcome": "failed", "failure": "link", "reason": reason})
        }
        Err(CourierError::Relay { code, reason }) => {
            let mut value = json!({
                "outcome": "failed",
                "failure": "relay",
                "code": code.tag(),
                "reason": reason,
            });
            if let Code::RateLimited { retry_after_ms } = code {
                value["retry_after_ms"] = json!(retry_after_ms.to_string());
            }
            value
        }
        Err(CourierError::Unexpected(reason)) => {
            json!({"outcome": "failed", "failure": "unexpected", "reason": reason})
        }
    };
    value.to_string()
}

/// What a device stands at in its list, as the one text [`sce_peer_standing`] gives: a hash and a
/// key in the URL alphabet, as the log's entries write them, and a 64-bit number as text.
fn standing_text(standing: &Standing) -> String {
    json!({
        "genesis": standing.genesis.map(|hash| encoding::encode(&hash)),
        "node": standing.person.map(|(node, _)| node.to_string()),
        "key": standing.person.map(|(_, key)| key.to_text()),
        "role": standing.role,
        "administrator": standing.administrator,
        "awaits_own_chain": standing.awaits_own_chain,
        "members": standing
            .members
            .iter()
            .map(|(node, role)| json!({"node": node.to_string(), "role": role}))
            .collect::<Vec<_>>(),
        "epoch": standing.epoch,
        "keys": standing.keys,
        "owed": standing.owed,
        "waiting": standing.waiting,
    })
    .to_string()
}

/// The number of this ABI.
#[no_mangle]
pub extern "C" fn sce_peer_abi_version() -> u32 {
    SCE_PEER_ABI_VERSION
}

/// What went wrong in the last call of this thread that failed: a NUL-terminated text the library
/// owns, good until the next call of this thread.
#[no_mangle]
pub extern "C" fn sce_peer_last_error() -> *const c_char {
    LAST_ERROR.with(|last| last.borrow().as_ptr())
}

/// Give a buffer back.
///
/// # Safety
/// `buf` is one this library returned and has not given back, or empty.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_buf_free(buf: ScePeerBuf) {
    if !buf.ptr.is_null() {
        // SAFETY: made by `ScePeerBuf::of` from a boxed slice of this length.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(buf.ptr, buf.len)) });
    }
}

/// The core of the device of whoever makes the list `list` (NUL-terminated): it takes the genesis
/// it is first given and every entry after it ([`Peer::open`]). Null when it fails.
///
/// # Safety
/// `list` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_open(list: *const c_char) -> *mut ScePeer {
    made(|| {
        // SAFETY: the promise in the function's contract.
        let list = unsafe { text(list)? };
        Ok(ScePeer {
            list: list.to_string(),
            peer: Peer::open(list),
        })
    })
}

/// The core of a device invited to the list `list` with the hash (32 bytes) of its genesis entry,
/// which the person was told in a channel they trust ([`Peer::invited`]). Null when it fails.
///
/// # Safety
/// `list` is a NUL-terminated string; `genesis` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_invited(list: *const c_char, genesis: *const u8) -> *mut ScePeer {
    made(|| {
        // SAFETY: the promises in the function's contract.
        let (list, genesis) = unsafe { (text(list)?, array::<32>(genesis)?) };
        Ok(ScePeer {
            list: list.to_string(),
            peer: Peer::invited(list, genesis),
        })
    })
}

/// The name of a list to open at a relay that takes a creation with no token (ADR 0080 section 3),
/// into `out` as UTF-8: `n1.<salt>.<digest>`, which certifies the administrator's key the 32-byte
/// `administrator` seed makes, the key the list's genesis will name first
/// ([`Peer::certified_name`]). `salt` is the host's, from 1 to 32 bytes; with `salt_len` 0 the
/// library draws 16 bytes from the operating system ([`Peer::draw_certified_name`]). The host makes
/// the core with this name ([`sce_peer_open`]), gives it the same seed
/// ([`sce_peer_set_administrator`]) and opens the list ([`sce_peer_open_list`]); its login creates
/// the list at such a relay ([`sce_peer_login`]).
///
/// # Safety
/// `administrator` points at 32 readable bytes; `salt` at `salt_len` readable bytes, or is null
/// when `salt_len` is 0; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_certified_name(
    administrator: *const u8,
    salt: *const u8,
    salt_len: usize,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let (seed, salt) = unsafe {
            put(out, ScePeerBuf::EMPTY);
            (array::<32>(administrator)?, bytes(salt, salt_len)?)
        };
        let key = Identity::from_seed(seed).public();
        let name = if salt.is_empty() {
            Peer::draw_certified_name(&key).map_err(from_error)?
        } else {
            Peer::certified_name(&key, salt).ok_or_else(|| {
                fail(
                    SCE_PEER_ERR_BAD_ARGUMENT,
                    format!("a salt is 1 to 32 bytes, and not {}", salt.len()),
                )
            })?
        };
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(name.into_bytes())) };
        Ok(())
    })
}

/// Free a core.
///
/// # Safety
/// `peer` is null or a handle from this library not freed yet.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_free(peer: *mut ScePeer) {
    if !peer.is_null() {
        // SAFETY: made by `Box::into_raw` in this library.
        drop(unsafe { Box::from_raw(peer) });
    }
}

/// The device's person: the node the log names them by, and the key the 32-byte `seed` makes
/// ([`Peer::set_identity`]).
///
/// # Safety
/// `peer` is a live handle; `seed` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_set_identity(
    peer: *mut ScePeer,
    node: u64,
    seed: *const u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, seed) = unsafe { (handle(peer)?, array::<32>(seed)?) };
        held.peer.set_identity(node, Identity::from_seed(seed));
        Ok(())
    })
}

/// The device holds the administrator's key the 32-byte `seed` makes: the one the genesis names when
/// the device opens the list, and the one its administrator's acts are signed with
/// ([`Peer::set_administrator`]). A secret, given once: the sealed state keeps it
/// ([`sce_peer_save_sealed`]).
///
/// # Safety
/// `peer` is a live handle; `seed` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_set_administrator(peer: *mut ScePeer, seed: *const u8) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, seed) = unsafe { (handle(peer)?, array::<32>(seed)?) };
        held.peer.set_administrator(Identity::from_seed(seed));
        Ok(())
    })
}

/// The machine is a hub's: it is told who ever wrote, not who writes now ([`Peer::for_a_hub`]).
/// Called before the first exchange.
///
/// # Safety
/// `peer` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_for_a_hub(peer: *mut ScePeer) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        held.rebuild(Peer::for_a_hub);
        Ok(())
    })
}

/// The core holds a new device of `node` in the list's MLS group, signing with the key the 32-byte
/// `seed` makes ([`Peer::with_group`]). Called once, before the first exchange of a device that has
/// no sealed state yet: the sealed state keeps the group's device from then on
/// ([`sce_peer_save_sealed`]).
///
/// # Safety
/// `peer` is a live handle; `seed` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_with_group(
    peer: *mut ScePeer,
    node: u64,
    seed: *const u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, seed) = unsafe { (handle(peer)?, array::<32>(seed)?) };
        let device = Device::new(&held.list, node, Identity::from_seed(seed));
        held.rebuild(|peer| peer.with_group(device, node));
        Ok(())
    })
}

/// Open a new list as its administrator, with the hub's profile `profile` (`SCE_PEER_PROFILE_*`)
/// ([`Peer::open_list`]): the genesis names the administrator's key the device holds, a grant makes
/// the device's person a member with the highest role, and the group is made when the core holds a
/// device of it, with the entry that names its first epoch, so that the list is sealed from its
/// first write. `genesis` is set to the hash (32 bytes) of the genesis, which a person is invited
/// with. The entries go to the relay with the next round, and the commit of the first epoch after
/// it.
///
/// # Safety
/// `peer` is a live handle; `genesis` is valid for a write of 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_open_list(
    peer: *mut ScePeer,
    profile: i32,
    genesis: *mut u8,
) -> i32 {
    guard(|| {
        required(genesis)?;
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        let profile = match profile {
            SCE_PEER_PROFILE_UNTRUSTED => Profile::Untrusted,
            SCE_PEER_PROFILE_TRUSTED_HUB => Profile::TrustedHub,
            other => {
                return Err(fail(
                    SCE_PEER_ERR_BAD_ARGUMENT,
                    format!("{other} is not a profile"),
                ))
            }
        };
        let hash = held.peer.open_list(profile).map_err(from_error)?;
        // SAFETY: `genesis` is valid for a write of 32 bytes.
        unsafe { std::ptr::copy_nonoverlapping(hash.as_ptr(), genesis, hash.len()) };
        Ok(())
    })
}

/// Grant the device of `node`, which signs with the 32-byte public `key`, the role `role` (1 to 3)
/// ([`Peer::grant`]). On a list keyed by its group a device the group does not hold is admitted with
/// [`sce_peer_add_member`] instead.
///
/// # Safety
/// `peer` is a live handle; `key` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_grant(
    peer: *mut ScePeer,
    node: u64,
    role: u32,
    key: *const u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, key) = unsafe { (handle(peer)?, array::<32>(key)?) };
        held.peer
            .grant(node, role_of(role)?, PublicKey::from_bytes(key))
            .map_err(from_error)
    })
}

/// Admit the device of `node`, which signs with the 32-byte public `key`, to the list and its group
/// with the role `role` (1 to 3), from the newest key package of it a round read
/// ([`Peer::add_member`]): the grant and the commit are one change. The entries go to the relay with
/// the next round, the commit, the Welcome and the keys of the epochs before the device's own after
/// it.
///
/// # Safety
/// `peer` is a live handle; `key` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_add_member(
    peer: *mut ScePeer,
    node: u64,
    role: u32,
    key: *const u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, key) = unsafe { (handle(peer)?, array::<32>(key)?) };
        held.peer
            .add_member(node, role_of(role)?, PublicKey::from_bytes(key))
            .map_err(from_error)
    })
}

/// Take the device of `node` off the list, and off its group when the list is keyed by one
/// ([`Peer::remove_member`]). The entries go to the relay with the next round, the commit after it.
///
/// # Safety
/// `peer` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_remove_member(peer: *mut ScePeer, node: u64) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        held.peer.remove_member(node).map_err(from_error)
    })
}

/// What the device stands at in its list, as one JSON text in `out` ([`Peer::standing`]): the
/// genesis, its person and their role, whether it holds an administrator's key the log names,
/// whether it waits for its own chain, the members, the epochs, and what the relay is still owed.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_standing(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let text = standing_text(&held.peer.standing());
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        Ok(())
    })
}

/// What the machine sent to `#hub` or `#people`: the event's name and its JSON payload, both
/// NUL-terminated ([`Peer::outbound`]). `kept` is set to what the core did with it
/// (`SCE_PEER_KEPT_*`), and, when it refused it or keeps it waiting, `reason` (which may be null) to
/// why; `reason` is set to an empty buffer otherwise.
///
/// # Safety
/// `peer` is a live handle; `event` and `data` are NUL-terminated strings; `kept` is valid for a
/// write; `reason` is null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_outbound(
    peer: *mut ScePeer,
    event: *const c_char,
    data: *const c_char,
    kept: *mut i32,
    reason: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(kept)?;
        // SAFETY: the promises in the function's contract.
        let (held, event, data) = unsafe {
            put(reason, ScePeerBuf::EMPTY);
            (handle(peer)?, text(event)?, text(data)?)
        };
        let answer = held.peer.outbound(event, data);
        // SAFETY: `kept` is valid for a write, `reason` null or valid for one.
        unsafe { put_kept(answer, kept, reason) };
        Ok(())
    })
}

/// Put what the core did with a send where the caller said: its `SCE_PEER_KEPT_*` at `kept`, and
/// why at `reason` when it was refused or waits.
///
/// # Safety
/// `kept` is valid for a write; `reason` is null or valid for a write.
unsafe fn put_kept(answer: Kept, kept: *mut i32, reason: *mut ScePeerBuf) {
    let (answer, why) = match answer {
        Kept::Taken => (SCE_PEER_KEPT_TAKEN, None),
        Kept::Held => (SCE_PEER_KEPT_HELD, None),
        Kept::NotSigned => (SCE_PEER_KEPT_NOT_SIGNED, None),
        Kept::Refused(why) => (SCE_PEER_KEPT_REFUSED, Some(why)),
        Kept::Waiting(why) => (SCE_PEER_KEPT_WAITING, Some(why)),
    };
    // SAFETY: the promises in the function's contract.
    unsafe {
        put(kept, answer);
        if let Some(why) = why {
            put(reason, ScePeerBuf::of(why.into_bytes()));
        }
    }
}

/// The change a copy that keeps a journal offered its host: `record_list.made`'s JSON payload,
/// NUL-terminated, the change and the command that closed it ([`Peer::made`]). It is signed as one
/// operation in this call, and `kept` is set to the verdict the host gives the copy:
/// `SCE_PEER_KEPT_TAKEN` is `accepted`, `_HELD` `held`, `_WAITING` `waiting` and `_REFUSED`
/// `refused`, with `reason` (which may be null) set to why for the last two and to an empty buffer
/// otherwise. What is signed is a record ([`sce_peer_take_records`]) no round gives the relay
/// before the host says it recorded it ([`sce_peer_recorded`]).
///
/// # Safety
/// `peer` is a live handle; `data` is a NUL-terminated string; `kept` is valid for a write;
/// `reason` is null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_made(
    peer: *mut ScePeer,
    data: *const c_char,
    kept: *mut i32,
    reason: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(kept)?;
        // SAFETY: the promises in the function's contract.
        let (held, data) = unsafe {
            put(reason, ScePeerBuf::EMPTY);
            (handle(peer)?, text(data)?)
        };
        let answer = held.peer.made(data);
        // SAFETY: `kept` is valid for a write, `reason` null or valid for one.
        unsafe { put_kept(answer, kept, reason) };
        Ok(())
    })
}

/// The records of the device's own operations the host has not said it recorded, in the order of
/// their numbers, as one JSON array in `out`: each `{"seq", "event", "data"}`, the number as text,
/// and `data` the text the relay is given ([`Peer::take_records`]). They stay until
/// [`sce_peer_recorded`] says so; an empty array when there are none.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_take_records(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let records: Vec<Value> = held
            .peer
            .take_records()
            .into_iter()
            .map(|record| {
                json!({"seq": record.seq.to_string(), "event": record.event, "data": record.data})
            })
            .collect();
        let text = Value::Array(records).to_string();
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        Ok(())
    })
}

/// The host recorded every operation of the device's chain up to `through` in its journal: the next
/// round gives the relay what it lacks of them ([`Peer::recorded`]).
///
/// # Safety
/// `peer` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_recorded(peer: *mut ScePeer, through: u64) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        held.peer.recorded(through);
        Ok(())
    })
}

/// Take back what the host's journal holds of the device's own operations past the sealed state it
/// restored: `records` is a NUL-terminated JSON array in the form [`sce_peer_take_records`] gives
/// ([`Peer::restore_records`]). `kept` (may be null) is set to how many the core did not hold.
/// `SCE_PEER_ERR_BAD_ARGUMENT` when the text is not that array, `SCE_PEER_ERR_UNREADABLE` when a
/// record is not one of the device's own or does not hold, the records before it kept.
///
/// # Safety
/// `peer` is a live handle; `records` is a NUL-terminated string; `kept` is null or valid for a
/// write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_restore_records(
    peer: *mut ScePeer,
    records: *const c_char,
    kept: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, records) = unsafe { (handle(peer)?, text(records)?) };
        let pairs = records_of(records).ok_or_else(|| {
            fail(
                SCE_PEER_ERR_BAD_ARGUMENT,
                "the records are not a JSON array of {seq, event, data}",
            )
        })?;
        let restored = held.peer.restore_records(&pairs).map_err(from_error)?;
        // SAFETY: `kept` is null or valid for a write.
        unsafe { put(kept, restored as u64) };
        Ok(())
    })
}

/// The records a host gives back, read from the text [`sce_peer_take_records`] wrote.
fn records_of(text: &str) -> Option<Vec<(String, String)>> {
    let value: Value = serde_json::from_str(text).ok()?;
    value
        .as_array()?
        .iter()
        .map(|record| {
            Some((
                record.get("event")?.as_str()?.to_string(),
                record.get("data")?.as_str()?.to_string(),
            ))
        })
        .collect()
}

/// A cut a lane gives, as the pairs [`numbered`] writes: `[[node, seq], ...]`, each number as
/// text.
fn cut_of(text: &str) -> Option<Vec<(u64, u64)>> {
    let value: Value = serde_json::from_str(text).ok()?;
    value
        .as_array()?
        .iter()
        .map(|pair| {
            let pair = pair.as_array().filter(|pair| pair.len() == 2)?;
            let number = |at: usize| pair[at].as_str()?.parse::<u64>().ok();
            Some((number(0)?, number(1)?))
        })
        .collect()
}

/// The history a machine built again is given, as one JSON object in `out` ([`Peer::history`]):
/// `roster`, the pairs of the event's name and its payload a round tells the roster in, and
/// `changes`, every change the core holds that a round would tell, the device's own included, in
/// causal order, from the cut `after` on (a NUL-terminated JSON array of `[node, seq]` pairs, each
/// number as text, as [`sce_peer_stable_cut`] gives; null for the whole history), then the device's
/// own changes that wait to be signed. Each change is `{"event", "data", "provisional"}`, with
/// `author`, `seq` (as text) and `context` (pairs as text) when it has a tag; a change that waits has
/// none and is `provisional`. `SCE_PEER_ERR_BAD_ARGUMENT` when `after` is not such an array.
///
/// # Safety
/// `peer` is a live handle; `after` is null or a NUL-terminated string; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_history(
    peer: *mut ScePeer,
    after: *const c_char,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let cut = if after.is_null() {
            Vec::new()
        } else {
            // SAFETY: not null, and NUL-terminated by the caller's promise.
            let given = unsafe { text(after)? };
            cut_of(given).ok_or_else(|| {
                fail(
                    SCE_PEER_ERR_BAD_ARGUMENT,
                    "the cut is not a JSON array of [node, seq] pairs",
                )
            })?
        };
        let history = held.peer.history(&cut);
        let changes: Vec<Value> = history
            .changes
            .iter()
            .map(|told| {
                let mut change = json!({
                    "event": told.event,
                    "data": told.data,
                    "provisional": told.provisional(),
                });
                if let Some(tag) = &told.tag {
                    change["author"] = json!(tag.author.to_string());
                    change["seq"] = json!(tag.seq.to_string());
                    change["context"] = json!(numbered(&tag.context).collect::<Vec<_>>());
                }
                change
            })
            .collect();
        let text = json!({
            "roster": history
                .roster
                .iter()
                .map(|(event, data)| json!([event, data]))
                .collect::<Vec<_>>(),
            "changes": changes,
        })
        .to_string();
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        Ok(())
    })
}

/// What the parts of changes signed before a change was one unit that the core holds make
/// together, as one JSON object in `out` ([`Peer::fold`], docs/adr/0085, section 8.3): `restores`,
/// the state as `record_list.restore` payloads, each a JSON text of at most `parts` values (the
/// build's `shared_parts`; 1 when 0 is given), empty when no value is live; and `cut`, for each
/// device the highest number of its chain among the parts, as `[node, seq]` pairs, each number as
/// text. A machine built again is given the restores first, then the history
/// ([`sce_peer_history`]). A function added alone (`docs/SCE_SHARED_LIST_PEER_ABI.md`, the
/// paragraph on numbers): a lane of version 8 that does not call it sees nothing change.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_fold(
    peer: *mut ScePeer,
    parts: u32,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let fold = held.peer.fold();
        let parts = usize::try_from(parts).unwrap_or(usize::MAX);
        let text = json!({
            "restores": fold.restores(parts),
            "cut": numbered(&fold.cut).collect::<Vec<_>>(),
        })
        .to_string();
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        Ok(())
    })
}

/// The stable cut, as one JSON array of `[node, seq]` pairs in `out`, each number as text, lowest
/// node first ([`Peer::stable_cut`]): for each device, the highest number of its chain every member
/// is known to have been told. The host folds its journal at it.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_stable_cut(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let cut = held.peer.stable_cut();
        let text = Value::Array(numbered(&cut).collect()).to_string();
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        Ok(())
    })
}

/// Sign a mark of what the machine was told, when one is owed ([`Peer::mark`]): `seq` (may be null)
/// is set to the number the mark took, 0 when none was owed. The next round gives it the relay.
///
/// # Safety
/// `peer` is a live handle; `seq` is null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_mark(peer: *mut ScePeer, seq: *mut u64) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        let marked = held.peer.mark().unwrap_or(0);
        // SAFETY: `seq` is null or valid for a write.
        unsafe { put(seq, marked) };
        Ok(())
    })
}

/// The highest stamp of the device's own changes the core holds, 0 and 0 when it holds none, and
/// whether it has read its own chain back, 1 or 0 ([`Peer::own_stamp`], docs/adr/0093, section
/// 6.3): a copy's next stamp exceeds it, which the host gives as `record_list.raise`.
///
/// # Safety
/// `peer` is a live handle; `wall_time`, `counter` and `read_back` are each null or valid for a
/// write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_own_stamp(
    peer: *mut ScePeer,
    wall_time: *mut i64,
    counter: *mut u32,
    read_back: *mut i32,
) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        let own = held.peer.own_stamp();
        let (wall, count) = own.stamp.unwrap_or((0, 0));
        // A stamp's counter is a `uint32` of its change's schema, and the trust signs no wider one.
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        // SAFETY: each is null or valid for a write.
        unsafe {
            put(wall_time, wall);
            put(counter, count);
            put(read_back, i32::from(own.read_back));
        }
        Ok(())
    })
}

/// Begin a round of `kind` (`SCE_PEER_ROUND_*`) ([`Peer::start`]); its requests are taken by
/// [`sce_peer_poll_request`].
///
/// # Safety
/// `peer` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_start(peer: *mut ScePeer, kind: i32) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        let kind = match kind {
            SCE_PEER_ROUND_SYNC => RoundKind::Sync,
            SCE_PEER_ROUND_RESYNC => RoundKind::Resync,
            SCE_PEER_ROUND_REREAD => RoundKind::Reread,
            other => {
                return Err(fail(
                    SCE_PEER_ERR_BAD_ARGUMENT,
                    format!("{other} is not a kind of round"),
                ))
            }
        };
        held.peer.start(kind).map_err(from_error)
    })
}

/// Begin the exchange that logs the device in at the relay named `audience` (NUL-terminated): the
/// name the device was configured with, shipped with the relay's address or carried in an
/// invitation, never the one the relay says ([`Peer::login`], ADR 0080). Its requests are taken and
/// answered as a round's, over a link that presents no token or one that expired, and its outcome
/// holds `token`, which the host gives the link ([`sce_peer_link_set_token`]). At a relay that takes
/// a creation with no token, the login of a list the relay does not hold creates it, under a name
/// that certifies its genesis ([`sce_peer_certified_name`]): a list just opened, or one that lapsed
/// there, which the next round gives back what the device holds.
/// `SCE_PEER_ERR_NO_PERSON` when the core has no person to log in as.
///
/// # Safety
/// `peer` is a live handle; `audience` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_login(peer: *mut ScePeer, audience: *const c_char) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, audience) = unsafe { (handle(peer)?, text(audience)?) };
        held.peer.login(audience).map_err(from_error)
    })
}

/// An administrator's invitation of the device of `node`, which the log never granted, to the list
/// at the relay whose address is `relay` and whose name is `audience` (both NUL-terminated), from
/// `now_ms` (the host's clock) for `valid_ms` (1 to 86 400 000: at most a day) ([`Peer::invite`],
/// ADR 0080 section 4). `out` is set to the text a host shows as a QR code: the relay's address, the
/// signed invitation and the secret the core drew, of which it keeps the key the secret makes.
/// `SCE_PEER_ERR_NO_ADMINISTRATOR` without a key the log names as an administrator's,
/// `SCE_PEER_ERR_GRANTED` for a node the log granted, `SCE_PEER_ERR_INVITATION` for a lifetime past a
/// day or a list not opened.
///
/// # Safety
/// `peer` is a live handle; `relay` and `audience` are NUL-terminated strings; `out` is valid for a
/// write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_invite(
    peer: *mut ScePeer,
    relay: *const c_char,
    audience: *const c_char,
    node: u64,
    now_ms: u64,
    valid_ms: u64,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let (held, relay, audience) = unsafe {
            put(out, ScePeerBuf::EMPTY);
            (handle(peer)?, text(relay)?, text(audience)?)
        };
        let invitation = held
            .peer
            .invite(relay, audience, node, now_ms, valid_ms)
            .map_err(from_error)?;
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(invitation.into_bytes())) };
        Ok(())
    })
}

/// [`sce_peer_invite`] under the 32-byte `secret` the host gives ([`Peer::invite_under`]): one it
/// drew from a source of its own, or a recording's fixed one. The same secret and the same
/// invitation give the same text.
///
/// # Safety
/// `peer` is a live handle; `relay` and `audience` are NUL-terminated strings; `secret` points at 32
/// readable bytes; `out` is valid for a write.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sce_peer_invite_under(
    peer: *mut ScePeer,
    relay: *const c_char,
    audience: *const c_char,
    node: u64,
    now_ms: u64,
    valid_ms: u64,
    secret: *const u8,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let (held, relay, audience, secret) = unsafe {
            put(out, ScePeerBuf::EMPTY);
            (
                handle(peer)?,
                text(relay)?,
                text(audience)?,
                array::<32>(secret)?,
            )
        };
        let invitation = held
            .peer
            .invite_under(relay, audience, node, now_ms, valid_ms, secret)
            .map_err(from_error)?;
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(invitation.into_bytes())) };
        Ok(())
    })
}

/// What the invitation in the `len` bytes of UTF-8 at `invitation` says, as one JSON text in `out`
/// for a host to make the invited device with and show its person: `relay` (the address to dial),
/// `audience`, `list`, `node` and `expires_at_ms` (as text), and `genesis` in the URL alphabet. Its
/// secret is not in it. `SCE_PEER_ERR_INVITATION` when the bytes are not an invitation's text.
///
/// # Safety
/// `invitation` points at `len` readable bytes; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_invitation_read(
    invitation: *const u8,
    len: usize,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let ticket = unsafe {
            put(out, ScePeerBuf::EMPTY);
            ticket_of(invitation, len)?
        };
        let said = &ticket.invitation;
        let value = json!({
            "relay": ticket.relay,
            "audience": said.audience,
            "list": said.list,
            "node": said.node.to_string(),
            "genesis": encoding::encode(&said.genesis),
            "expires_at_ms": said.expires_at_ms.to_string(),
        });
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(value.to_string().into_bytes())) };
        Ok(())
    })
}

/// The core of the device the invitation in the `len` bytes of UTF-8 at `invitation` invites: the
/// one [`sce_peer_invited`] makes for its list and the hash of its genesis. Null when it fails.
///
/// # Safety
/// `invitation` points at `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_invited_by(invitation: *const u8, len: usize) -> *mut ScePeer {
    made(|| {
        // SAFETY: the promise in the function's contract.
        let ticket = unsafe { ticket_of(invitation, len)? };
        let said = &ticket.invitation;
        Ok(ScePeer {
            list: said.list.clone(),
            peer: Peer::invited(&said.list, said.genesis),
        })
    })
}

/// Begin the exchange that leaves the device's identity key, and its key package when the core
/// holds a device of the list's group, with the relay under the invitation in the `len` bytes of
/// UTF-8 at `invitation` ([`Peer::join`], ADR 0080 section 4). Its requests are taken and answered
/// as a round's, over a link that presents no token, and its outcome tells the machine nothing.
/// `fingerprint` is set to the digits the person compares with the ones the administrator's device
/// shows ([`sce_peer_joined`]). `SCE_PEER_ERR_INVITATION` for an invitation of another list, genesis
/// or node than the core's; `SCE_PEER_ERR_NO_PERSON` with no person.
///
/// # Safety
/// `peer` is a live handle; `invitation` points at `len` readable bytes; `fingerprint` is valid for a
/// write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_join(
    peer: *mut ScePeer,
    invitation: *const u8,
    len: usize,
    fingerprint: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(fingerprint)?;
        // SAFETY: the promises in the function's contract.
        let (held, given) = unsafe {
            put(fingerprint, ScePeerBuf::EMPTY);
            (handle(peer)?, bytes(invitation, len)?)
        };
        let text = std::str::from_utf8(given)
            .map_err(|_| fail(SCE_PEER_ERR_BAD_TEXT, "an invitation is not UTF-8"))?;
        let digits = held.peer.join(text).map_err(from_error)?;
        // SAFETY: `fingerprint` is valid for a write.
        unsafe { put(fingerprint, ScePeerBuf::of(digits.into_bytes())) };
        Ok(())
    })
}

/// The devices that joined under an invitation this core made for `node`, as the rounds read them,
/// as one JSON array in `out`, each `{"key": .., "fingerprint": ..}` (the key in the URL alphabet):
/// what the administrator's device shows before it admits one with [`sce_peer_add_member`] or
/// [`sce_peer_grant`] and that key ([`Peer::joined`]). An empty buffer when none was read, or the node
/// was admitted.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_joined(
    peer: *mut ScePeer,
    node: u64,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let joined = held.peer.joined(node);
        if !joined.is_empty() {
            let text = json!(joined
                .iter()
                .map(|device| json!({"key": device.key.to_text(),
                                     "fingerprint": device.fingerprint}))
                .collect::<Vec<_>>())
            .to_string();
            // SAFETY: `out` is valid for a write.
            unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        }
        Ok(())
    })
}

/// Begin the exchange that gives the relay the device's person's checkpoint: what their copy held
/// of everything up to `before`, the digest the machine computed ([`Peer::publish`] of
/// [`Publication::Checkpoint`]).
///
/// # Safety
/// `peer` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_publish_checkpoint(
    peer: *mut ScePeer,
    before: i64,
    digest: u32,
) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        held.peer
            .publish(&Publication::Checkpoint { before, digest })
            .map_err(from_error)
    })
}

/// Begin the exchange that leaves a key package of the group's device with the relay, for an
/// administrator to add it with ([`Peer::publish_key_package`]).
///
/// # Safety
/// `peer` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_publish_key_package(peer: *mut ScePeer) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let held = unsafe { handle(peer)? };
        held.peer.publish_key_package().map_err(from_error)
    })
}

/// The next request to put to the relay, as the protocol's text, in `out`; an empty buffer when no
/// exchange runs or the one that ran is over and its outcome waits ([`Peer::poll_request`]). Asked
/// again before it is answered, it is the same request.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_poll_request(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        if let Some(request) = held.peer.poll_request() {
            // SAFETY: `out` is valid for a write.
            unsafe { put(out, ScePeerBuf::of(request.into_bytes())) };
        }
        Ok(())
    })
}

/// The relay's response text (NUL-terminated) to the request [`sce_peer_poll_request`] gave
/// ([`Peer::respond`]). A text that is not a response of the protocol is taken as a failure of the
/// transport.
///
/// # Safety
/// `peer` is a live handle; `response` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_respond(peer: *mut ScePeer, response: *const c_char) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, response) = unsafe { (handle(peer)?, text(response)?) };
        held.peer.respond(response).map_err(from_error)
    })
}

/// The transport did not carry the request or its response, and why (NUL-terminated): the exchange
/// ends, and what it took in is handed over by the next round ([`Peer::fail`]).
///
/// # Safety
/// `peer` is a live handle; `reason` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_fail(peer: *mut ScePeer, reason: *const c_char) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, reason) = unsafe { (handle(peer)?, text(reason)?) };
        held.peer.fail(reason).map_err(from_error)
    })
}

/// What the exchange that ended came to, once, as one JSON text in `out`; an empty buffer when none
/// ended since the last was taken ([`Peer::take_outcome`]).
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_take_outcome(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        if let Some(outcome) = held.peer.take_outcome() {
            // SAFETY: `out` is valid for a write.
            unsafe { put(out, ScePeerBuf::of(outcome_text(&outcome).into_bytes())) };
        }
        Ok(())
    })
}

/// What the device keeps, its secrets with the rest, sealed under a data key drawn for this save
/// alone ([`Peer::save_sealed`]): `state` is set to the sealed state, and the 32 bytes at `key` to
/// the data key. The host wraps the key with a key its platform's key store holds, keeps the
/// wrapped key beside the state, and zeroes its copy of this one.
///
/// # Safety
/// `peer` is a live handle; `state` is valid for a write; `key` is valid for a write of 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_save_sealed(
    peer: *mut ScePeer,
    state: *mut ScePeerBuf,
    key: *mut u8,
) -> i32 {
    guard(|| {
        required(state)?;
        required(key)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(state, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let sealed = held.peer.save_sealed().map_err(from_error)?;
        let data_key = sealed.key.as_bytes();
        // SAFETY: `key` is valid for a write of 32 bytes, and `state` for a write.
        unsafe {
            std::ptr::copy_nonoverlapping(data_key.as_ptr(), key, data_key.len());
            put(state, ScePeerBuf::of(sealed.state));
        }
        Ok(())
    })
}

/// What [`sce_peer_save_sealed`] seals, under the 32-byte data `key` the host gives
/// ([`Peer::save_sealed_under`]): a key the host drew from a source of its own, or a recording's
/// fixed key. The same key and state give the same bytes. `state` is set to the sealed state.
///
/// # Safety
/// `peer` is a live handle; `key` points at 32 readable bytes; `state` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_save_sealed_under(
    peer: *mut ScePeer,
    key: *const u8,
    state: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(state)?;
        // SAFETY: the promises in the function's contract.
        let (held, key) = unsafe {
            put(state, ScePeerBuf::EMPTY);
            (handle(peer)?, DataKey::from_bytes(array::<32>(key)?))
        };
        let sealed = held.peer.save_sealed_under(&key).map_err(from_error)?;
        // SAFETY: `state` is valid for a write.
        unsafe { put(state, ScePeerBuf::of(sealed)) };
        Ok(())
    })
}

/// Take back the `len` bytes of `state` that [`sce_peer_save_sealed`] sealed for the device of
/// `node`, under the 32-byte data `key` the host unwrapped, into a core just made for the same list
/// and given nothing else ([`Peer::restore_sealed`]); `kept` (which may be null) is set to how many
/// operations and checkpoints the log verified. `SCE_PEER_ERR_NOT_OPENED` when the state does not
/// open under the key for this list and node, `SCE_PEER_ERR_UNREADABLE` when the bytes are no
/// sealed state; either way the core took nothing. The device then signs nothing new until a round
/// has read its own chain back from the relay. A state a core of version 5 sealed, before a change
/// was one unit, is opened and upgraded: the host builds its machine again from the fold
/// ([`sce_peer_fold`]) and the history ([`sce_peer_history`]).
///
/// # Safety
/// `peer` is a live handle; `state` points at `len` readable bytes; `key` at 32; `kept` is null or
/// valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_restore_sealed(
    peer: *mut ScePeer,
    node: u64,
    state: *const u8,
    len: usize,
    key: *const u8,
    kept: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, state, key) = unsafe {
            (
                handle(peer)?,
                bytes(state, len)?,
                DataKey::from_bytes(array::<32>(key)?),
            )
        };
        let count = held
            .peer
            .restore_sealed(node, state, &key)
            .map_err(from_error)?;
        // SAFETY: `kept` is null or valid for a write.
        unsafe { put(kept, count as u64) };
        Ok(())
    })
}

/// A link to a relay: the transport the library holds for it, and the signal the host raises when
/// the device's network changed.
///
/// The transport is used by the one thread that runs over the link ([`sce_peer_run`],
/// [`sce_peer_link_take_notices`]); the network's signal, which is outside the cell, by any thread
/// at any time ([`sce_peer_link_network_changed`]). So a function on a link makes no `&mut` of the
/// link itself, and reaches into the cell only on the thread that runs.
pub struct ScePeerLink {
    transport: UnsafeCell<Box<dyn Transport>>,
    network: Signal,
    /// The context of a host's transport, which its pause is called with; `None` for a transport
    /// of the library's own, whose time is the system's.
    host: Option<*mut c_void>,
    /// The wait of a run over a host's transport, on the host's clock ([`sce_peer_link_set_pause`]);
    /// `None` for the system's clock.
    pause: Cell<Option<ScePeerPauseFn>>,
}

impl ScePeerLink {
    /// A link over one of the library's own transports.
    fn over(transport: Box<dyn Transport>) -> Self {
        Self {
            network: transport.network(),
            transport: UnsafeCell::new(transport),
            host: None,
            pause: Cell::new(None),
        }
    }

    /// A link over a host's transport, whose functions are given `context`.
    fn hosted(transport: Box<dyn Transport>, context: *mut c_void) -> Self {
        Self {
            host: Some(context),
            ..Self::over(transport)
        }
    }
}

/// A run's clock when its link is a host's transport that gave its pause: the system's time, which
/// the run's deadline is given in, and the host's wait, on the host's own clock.
struct HostClock {
    pause: ScePeerPauseFn,
    context: *mut c_void,
}

impl Clock for HostClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wait(&self, wait: Duration, cancel: &Signal) {
        if cancel.is_raised() {
            return;
        }
        let wait_ms = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX);
        // SAFETY: the host's function, given the context it gave with its transport, on the thread
        // that runs (`sce_peer_link_host`, `sce_peer_link_set_pause`).
        let paused = unsafe { (self.pause)(self.context, wait_ms) };
        log::debug!(wait_ms = wait_ms, woken = paused == SCE_PEER_PAUSE_WOKEN; "host_paused");
    }
}

/// The cancellation of a run ([`sce_peer_run`]): raised from any thread, once, and it stays raised.
/// A host makes one for each run it may want to stop.
pub struct ScePeerCancel {
    signal: Signal,
}

/// Where a host's transport puts the text of one exchange ([`sce_peer_reply`]): the response, or
/// why there is none. The library's, alive for the one call of the host's function it is given to.
pub struct ScePeerReply {
    text: Option<Vec<u8>>,
}

/// A host's transport: carry the `len` bytes of `request` (UTF-8, not NUL-terminated, valid for the
/// call) to the relay, give the response text with [`sce_peer_reply`] on `reply`, and return
/// `SCE_PEER_TRANSPORT_ANSWERED`; or return why it was not carried (`SCE_PEER_TRANSPORT_*`), with the
/// reason given the same way when there is one. It may block for at most `timeout_ms` milliseconds
/// (`u64::MAX` when the run has no deadline). It is called on the thread that runs, one call at a
/// time, and calls no function of the library on that link or its core but [`sce_peer_reply`].
pub type ScePeerExchangeFn = unsafe extern "C" fn(
    context: *mut c_void,
    request: *const u8,
    len: usize,
    timeout_ms: u64,
    reply: *mut ScePeerReply,
) -> i32;

/// Called once with the host's context when the link that holds it is freed.
pub type ScePeerReleaseFn = unsafe extern "C" fn(context: *mut c_void);

/// A host's wait, on its own clock ([`sce_peer_link_set_pause`]): wait `wait_ms` milliseconds of the
/// host's time, the wait a relay asked for when it refused a request for the rate, and return
/// `SCE_PEER_PAUSE_WAITED`; or return earlier with `SCE_PEER_PAUSE_WOKEN`, at the latest once the
/// host raises the run's cancellation. It is called with the context of the host's transport, on
/// the thread that runs, between two exchanges, and calls no function of the library on that link
/// or its core.
pub type ScePeerPauseFn = unsafe extern "C" fn(context: *mut c_void, wait_ms: u64) -> i32;

/// The sink of what the library logs ([`sce_peer_set_log_sink`]): called with a record's level
/// (`SCE_PEER_LOG_*`), the module that decided it and the record as one line (both NUL-terminated
/// UTF-8, the library's, valid for the call), and the `user` pointer the host gave. It may be called
/// from any thread that calls the library, and from several at once; it calls no function of the
/// library.
pub type ScePeerLogFn = unsafe extern "C" fn(
    level: i32,
    target: *const c_char,
    message: *const c_char,
    user: *mut c_void,
);

/// A transport the host carries itself, through the function it gave.
struct HostTransport {
    exchange: ScePeerExchangeFn,
    release: Option<ScePeerReleaseFn>,
    context: *mut c_void,
    network: Signal,
}

// SAFETY: by the contract of `sce_peer_link_host`, the host's functions and its context may be used
// from whichever thread runs over the link, and a link is used by one thread at a time; the library
// never calls them from two threads at once.
unsafe impl Send for HostTransport {}

impl Transport for HostTransport {
    fn exchange(
        &mut self,
        request: &str,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<String, TransportError> {
        if cancel.is_raised() {
            return Err(TransportError::Cancelled);
        }
        let timeout_ms = deadline.map_or(u64::MAX, |deadline| {
            let left = deadline
                .saturating_duration_since(Instant::now())
                .as_millis();
            u64::try_from(left).unwrap_or(u64::MAX).min(u64::MAX - 1)
        });
        let mut reply = ScePeerReply { text: None };
        // SAFETY: the host's function, given the context it gave with it, a request valid for its
        // length for the call, and a reply valid for writes for the call (`sce_peer_link_host`).
        let status = unsafe {
            (self.exchange)(
                self.context,
                request.as_ptr(),
                request.len(),
                timeout_ms,
                &mut reply,
            )
        };
        let given = match reply.text {
            None => None,
            Some(bytes) => Some(String::from_utf8(bytes).map_err(|_| {
                TransportError::Link(
                    "the host's transport gave a text that is not UTF-8".to_string(),
                )
            })?),
        };
        match status {
            SCE_PEER_TRANSPORT_ANSWERED => given.ok_or_else(|| {
                TransportError::Link(
                    "the host's transport answered and gave no response".to_string(),
                )
            }),
            SCE_PEER_TRANSPORT_LINK => {
                Err(TransportError::Link(given.unwrap_or_else(|| {
                    "the host's transport did not carry the exchange".to_string()
                })))
            }
            SCE_PEER_TRANSPORT_UNAUTHORIZED => {
                Err(TransportError::Unauthorized(given.unwrap_or_default()))
            }
            SCE_PEER_TRANSPORT_DEADLINE => Err(TransportError::Deadline),
            SCE_PEER_TRANSPORT_CANCELLED => Err(TransportError::Cancelled),
            SCE_PEER_TRANSPORT_RATE_LIMITED => {
                match given.as_deref().map(str::trim).map(str::parse::<u64>) {
                    Some(Ok(retry_after_ms)) => Err(TransportError::RateLimited {
                        retry_after_ms,
                        reason: "the host's transport was refused for the rate".to_string(),
                    }),
                    _ => Err(TransportError::Link(
                        "the host's transport said it was refused for the rate and gave no wait \
                         in milliseconds"
                            .to_string(),
                    )),
                }
            }
            other => Err(TransportError::Link(format!(
                "the host's transport said {other}, which is no status of a transport"
            ))),
        }
    }

    fn take_notices(&mut self) -> Vec<Notice> {
        // The host carries the texts and knows what became of its connection.
        Vec::new()
    }

    fn network(&self) -> Signal {
        self.network.clone()
    }
}

impl Drop for HostTransport {
    fn drop(&mut self) {
        if let Some(release) = self.release {
            // SAFETY: the host's function, given its context once, as the link that held it goes.
            unsafe { release(self.context) };
        }
    }
}

/// A notice of a transport as the JSON object [`sce_peer_link_take_notices`] gives.
fn notice_value(notice: &Notice) -> Value {
    match notice {
        Notice::Connected => json!({"notice": "connected"}),
        Notice::Lost(reason) => json!({"notice": "lost", "reason": reason}),
        Notice::Pushed(text) => json!({"notice": "pushed", "text": text}),
    }
}

/// A link to the relay at `base` (`ws://host:port` or `wss://host:port`, NUL-terminated) over its
/// WebSocket binding, presenting `token` (NUL-terminated), the token the device's login earned, or
/// none when `token` is null or empty: a link that dials the connection a device logs in on
/// ([`sce_peer_login`]), and is then given its token ([`sce_peer_link_set_token`]). It dials on the
/// first run and again whenever it must. Null when it fails: an address that is not a WebSocket's.
///
/// # Safety
/// `base` is a NUL-terminated string; `token` is null or one.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_wss(
    base: *const c_char,
    token: *const c_char,
) -> *mut ScePeerLink {
    made(|| {
        // SAFETY: the promises in the function's contract.
        let (base, token) = unsafe { (text(base)?, token_or_none(token)?) };
        let link = WssLink::new(base, token).map_err(|error| {
            fail(
                SCE_PEER_ERR_BAD_ARGUMENT,
                format!("no link to the relay: {error}"),
            )
        })?;
        Ok(ScePeerLink::over(Box::new(link)))
    })
}

/// A link as [`sce_peer_link_wss`] makes it, which believes the certificate authorities in the `len`
/// bytes at `authorities` for a `wss://` relay's certificate: one or more certificates in PEM, as a
/// deployment hands its authority out and as an application ships it (an Android raw resource, a
/// file of an iOS bundle, read whole), or one in DER. With `roots` `SCE_PEER_ROOTS_ONLY` it believes
/// them alone, for a relay under a private authority; with `SCE_PEER_ROOTS_AND_PUBLIC` the public
/// authorities too, for a deployment whose relays are under either, or move from the one to the
/// other ([`Roots`]). The bytes are read here and not kept. Null when it fails: an address that is
/// not a WebSocket's, bytes that hold no certificate, a certificate that is not one, a `roots` that
/// is neither. A null or empty `token` is none, as for [`sce_peer_link_wss`].
///
/// # Safety
/// `base` is a NUL-terminated string; `token` is null or one; `authorities` points at `len`
/// readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_wss_believing(
    base: *const c_char,
    token: *const c_char,
    authorities: *const u8,
    len: usize,
    roots: i32,
) -> *mut ScePeerLink {
    made(|| {
        // SAFETY: the promises in the function's contract.
        let (base, token, given) =
            unsafe { (text(base)?, token_or_none(token)?, bytes(authorities, len)?) };
        let believed = tls::authorities(given).map_err(|error| {
            fail(
                SCE_PEER_ERR_BAD_ARGUMENT,
                format!("no authority to believe: {error}"),
            )
        })?;
        let roots = match roots {
            SCE_PEER_ROOTS_ONLY => Roots::Only(believed),
            SCE_PEER_ROOTS_AND_PUBLIC => Roots::PublicAnd(believed),
            other => {
                return Err(fail(
                    SCE_PEER_ERR_BAD_ARGUMENT,
                    format!("{other} is not a choice of authorities"),
                ))
            }
        };
        let link = WssLink::believing(base, token, &roots).map_err(|error| {
            fail(
                SCE_PEER_ERR_BAD_ARGUMENT,
                format!("no link to the relay: {error}"),
            )
        })?;
        Ok(ScePeerLink::over(Box::new(link)))
    })
}

/// A link whose transport the host carries itself: `exchange` is called for each request of a run
/// ([`ScePeerExchangeFn`]) with `context`, and `release` (which may be null) once with `context`
/// when the link is freed. Null when `exchange` is null, and then `release` is not called.
///
/// # Safety
/// `exchange` and `release` are functions as [`ScePeerExchangeFn`] and [`ScePeerReleaseFn`] say,
/// and they and `context` may be used from whichever thread runs over the link until `release` is
/// called.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_host(
    exchange: Option<ScePeerExchangeFn>,
    release: Option<ScePeerReleaseFn>,
    context: *mut c_void,
) -> *mut ScePeerLink {
    made(|| {
        let exchange = exchange.ok_or_else(|| {
            fail(
                SCE_PEER_ERR_NULL_ARGUMENT,
                "a host's transport has no exchange function",
            )
        })?;
        Ok(ScePeerLink::hosted(
            Box::new(HostTransport {
                exchange,
                release,
                context,
                network: Signal::new(),
            }),
            context,
        ))
    })
}

/// Free a link; a host's transport is released. Not while a run uses it.
///
/// # Safety
/// `link` is null or a handle from this library not freed yet, and no run uses it.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_free(link: *mut ScePeerLink) {
    if !link.is_null() {
        // SAFETY: made by `Box::into_raw` in this library.
        drop(unsafe { Box::from_raw(link) });
    }
}

/// The device's network changed: an interface came or went, or the operating system said the path
/// is another. The link gives its connection up and dials again on its next exchange, and an
/// exchange that waits when it is told fails as the link's failure. Any thread may call it, at any
/// time, a run on the link included.
///
/// # Safety
/// `link` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_network_changed(link: *const ScePeerLink) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract; the network's signal is shared.
        let link = unsafe { shared(link)? };
        link.network.raise();
        Ok(())
    })
}

/// What became of the link's connection and what the relay pushed since this was last asked, as one
/// JSON array in `out`, oldest first: `{"notice": "connected"}`, `{"notice": "lost", "reason": text}`,
/// `{"notice": "pushed", "text": text}`. An empty buffer when there is none. A host's transport gives
/// none.
///
/// # Safety
/// `link` is a live handle no run uses meanwhile; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_take_notices(
    link: *mut ScePeerLink,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let link = unsafe {
            put(out, ScePeerBuf::EMPTY);
            shared(link.cast_const())?
        };
        // SAFETY: this thread is the one that uses the link's transport now.
        let notices = unsafe { &mut **link.transport.get() }.take_notices();
        if !notices.is_empty() {
            let text =
                Value::from(notices.iter().map(notice_value).collect::<Vec<_>>()).to_string();
            // SAFETY: `out` is valid for a write.
            unsafe { put(out, ScePeerBuf::of(text.into_bytes())) };
        }
        Ok(())
    })
}

/// The token the link presents from its next dial on (NUL-terminated; null or empty for none): the
/// one a login earned ([`sce_peer_login`], the outcome's `token`). A connection open now that was
/// dialled with another is given up, and the next run dials again. `SCE_PEER_ERR_BAD_ARGUMENT` for
/// a link whose transport the host carries itself ([`sce_peer_link_host`]), whose credentials are
/// the host's to present.
///
/// # Safety
/// `link` is a live handle no run uses meanwhile; `token` is null or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_set_token(
    link: *mut ScePeerLink,
    token: *const c_char,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (link, token) = unsafe { (shared(link.cast_const())?, token_or_none(token)?) };
        // SAFETY: this thread is the one that uses the link's transport now.
        let transport = unsafe { &mut **link.transport.get() };
        if transport.set_token(token) {
            Ok(())
        } else {
            Err(fail(
                SCE_PEER_ERR_BAD_ARGUMENT,
                "a host's transport presents its own credentials, and carries no token of the \
                 library's",
            ))
        }
    })
}

/// The wait of a run over a host's own transport ([`sce_peer_link_host`]) is `pause`, on the host's
/// clock, called with the transport's context ([`ScePeerPauseFn`]); null for the system's clock,
/// which is what a host's transport has until it is given one. A run waits whenever the relay
/// refuses a request for the rate with a wait that ends before the run's deadline, and then puts the
/// same request again: on the system's clock the run's thread sleeps that long (woken by the run's
/// cancellation), which is what a host whose time is the system's wants, and what a host whose time
/// is its own (a simulation, whose relay counts the host's time) does not, since its relay would
/// not have moved on when the run asks again. `SCE_PEER_ERR_BAD_ARGUMENT` for a link of the
/// library's own transport, whose time is the system's.
///
/// # Safety
/// `link` is a live handle no run uses meanwhile; `pause` is null or a function as
/// [`ScePeerPauseFn`] says, which may be called with the transport's context from whichever thread
/// runs over the link, until the link is freed.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_set_pause(
    link: *mut ScePeerLink,
    pause: Option<ScePeerPauseFn>,
) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let link = unsafe { shared(link.cast_const())? };
        if link.host.is_none() {
            return Err(fail(
                SCE_PEER_ERR_BAD_ARGUMENT,
                "a link of the library's own transport waits on the system's clock, and takes no \
                 host's pause",
            ));
        }
        link.pause.set(pause);
        Ok(())
    })
}

/// Give the text of one exchange of a host's transport: the response, or why there is none. Called
/// by the host's [`ScePeerExchangeFn`] on the `reply` it was given, before it returns; a second call
/// replaces the first. The text is copied.
///
/// # Safety
/// `reply` is the one the host's function was given, during that call; `text` points at `len`
/// readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_reply(
    reply: *mut ScePeerReply,
    text: *const u8,
    len: usize,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (reply, text) = unsafe { (handle(reply)?, bytes(text, len)?) };
        reply.text = Some(text.to_vec());
        Ok(())
    })
}

/// A cancellation for a run, not raised. Null when it fails.
#[no_mangle]
pub extern "C" fn sce_peer_cancel_new() -> *mut ScePeerCancel {
    made(|| {
        Ok(ScePeerCancel {
            signal: Signal::new(),
        })
    })
}

/// Raise a cancellation: the run that was given it ends as soon as its transport gives up the
/// exchange that is out, and a run given it later carries nothing. Any thread may call it, at any
/// time, while the run goes on included.
///
/// # Safety
/// `cancel` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_cancel_raise(cancel: *const ScePeerCancel) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract; the signal is shared.
        let cancel = unsafe { shared(cancel)? };
        cancel.signal.raise();
        Ok(())
    })
}

/// Free a cancellation. Not while a run uses it.
///
/// # Safety
/// `cancel` is null or a handle from this library not freed yet, and no run uses it.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_cancel_free(cancel: *mut ScePeerCancel) {
    if !cancel.is_null() {
        // SAFETY: made by `Box::into_raw` in this library.
        drop(unsafe { Box::from_raw(cancel) });
    }
}

/// Run the exchange begun on `peer` ([`sce_peer_start`], a publication) over `link` until it ends,
/// blocking the calling thread: each request the core gives is carried by the link's transport and
/// the response, or the transport's failure, given back, by the one driver every transport is run
/// by. It stops `within_ms` milliseconds from the call (`u64::MAX`, or a time too large for the
/// clock, is no deadline) or once `cancel` (which may be null) is raised, and the transport gives up
/// the exchange that is
/// out. The outcome is then taken with [`sce_peer_take_outcome`], as after an exchange carried by
/// hand; a run that stopped leaves an outcome of a failure of the link, and what the exchange took
/// in is handed over by the next round. `SCE_PEER_OK` when the exchange ended (whatever it came to,
/// and at once when none runs), `SCE_PEER_ERR_CANCELLED` or `SCE_PEER_ERR_DEADLINE` when the run
/// stopped.
///
/// A relay's refusal of a request for the rate (`rate_limited`, or its endpoint's refusal of the
/// connection, `SCE_PEER_TRANSPORT_RATE_LIMITED`) is waited out and the same request put again,
/// as many times as the relay asks, while its wait ends before the deadline (or there is none):
/// on the host's clock when the link is a host's transport that gave its pause
/// ([`sce_peer_link_set_pause`]), on the system's otherwise. A wait that would end at the deadline
/// or after it ends the exchange with the refusal, which the outcome holds with `retry_after_ms`;
/// the cancellation raised during a wait stops the run as it does an exchange.
///
/// # Safety
/// `peer` and `link` are live handles no other thread uses meanwhile, but to raise the link's
/// network signal; `cancel` is null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_run(
    peer: *mut ScePeer,
    link: *mut ScePeerLink,
    within_ms: u64,
    cancel: *const ScePeerCancel,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, link, cancel) =
            unsafe { (handle(peer)?, shared(link.cast_const())?, cancel.as_ref()) };
        let never = Signal::new();
        let cancel = cancel.map_or(&never, |cancel| &cancel.signal);
        let deadline = match within_ms {
            u64::MAX => None,
            within_ms => Instant::now().checked_add(Duration::from_millis(within_ms)),
        };
        // SAFETY: this run is the one user of the link's transport; other threads reach only the
        // network's signal, which is outside the cell.
        let transport = unsafe { &mut **link.transport.get() };
        // A refusal for the rate is waited out on the clock of whoever carries the texts: the
        // host's, when its transport gave its pause, and the system's otherwise.
        let driven = match (link.host, link.pause.get()) {
            (Some(context), Some(pause)) => drive_with(
                &mut held.peer,
                transport,
                deadline,
                cancel,
                &HostClock { pause, context },
            ),
            _ => drive(&mut held.peer, transport, deadline, cancel),
        };
        match driven {
            Driven::Ended => Ok(()),
            Driven::Cancelled => Err(fail(
                SCE_PEER_ERR_CANCELLED,
                "the run was cancelled: the exchange ended as a failure of the link, and its \
                 outcome waits",
            )),
            Driven::DeadlinePassed => Err(fail(
                SCE_PEER_ERR_DEADLINE,
                "the deadline passed: the exchange ended as a failure of the link, and its \
                 outcome waits",
            )),
        }
    })
}

/// The sink a host installed ([`sce_peer_set_log_sink`]): its function, its pointer, and the most
/// verbose level it is given.
struct HostSink {
    sink: ScePeerLogFn,
    user: *mut c_void,
    most: log::LevelFilter,
}

// SAFETY: by the contract of `sce_peer_set_log_sink`, the host's function may be called from any
// thread, and from several at once, with the pointer it gave.
unsafe impl Send for HostSink {}
// SAFETY: as for `Send`: the function and its pointer are read, never written, while it is installed.
unsafe impl Sync for HostSink {}

/// The sink of the process, while a host has one installed. A record is handed to it under the
/// read lock, and a sink is replaced under the write lock, so that once a replacement returns the
/// function it replaced is called no more.
static SINK: RwLock<Option<HostSink>> = RwLock::new(None);

/// What the facade is given as the process's sink: the host's, when one is installed.
struct Facade;

impl log::Log for Facade {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        SINK.read()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_some_and(|host| sce_rust_relay::logs::admits(metadata, host.most))
    }

    fn log(&self, record: &log::Record<'_>) {
        let held = SINK.read().unwrap_or_else(PoisonError::into_inner);
        let Some(host) = held
            .as_ref()
            .filter(|host| sce_rust_relay::logs::admits(record.metadata(), host.most))
        else {
            return;
        };
        // The record's line escapes every control character, NUL among them; a target is a module's
        // path. Neither holds a NUL, and one that did would be written without the record.
        let line = sce_rust_relay::logs::line(record);
        let (Ok(target), Ok(message)) = (CString::new(record.target()), CString::new(line)) else {
            return;
        };
        let level = match record.level() {
            log::Level::Error => SCE_PEER_LOG_ERROR,
            log::Level::Warn => SCE_PEER_LOG_WARN,
            log::Level::Info => SCE_PEER_LOG_INFO,
            log::Level::Debug => SCE_PEER_LOG_DEBUG,
            log::Level::Trace => SCE_PEER_LOG_TRACE,
        };
        // SAFETY: the host's function and pointer, as `sce_peer_set_log_sink` was given them; the
        // texts live until it returns.
        unsafe { (host.sink)(level, target.as_ptr(), message.as_ptr(), host.user) };
    }

    fn flush(&self) {}
}

static FACADE: Facade = Facade;

/// Whether the facade's sink of the process is this library's: installed the first time a host
/// installs a sink, and never again.
fn facade_installed() -> bool {
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    *INSTALLED.get_or_init(|| log::set_logger(&FACADE).is_ok())
}

/// Install `sink` as the sink of what the library logs, process-wide, given `user` with each record
/// and the records at `max_level` (`SCE_PEER_LOG_*`) and the levels before it: each decision of the
/// relay's client side, the driver and the core (a dial and a connection, a wait for the rate, a
/// refusal given over, a failure of the link, an exchange ended or failed, what the relay is owed
/// given or kept), as one line of `key=value` words (`docs/SCE_SHARED_LIST_PEER_ABI.md` section
/// 14), which names counts, codes, nodes, durations and a list by a digest of its name, and never a
/// token, a key, a signature or anything a list holds.
///
/// A sink is replaceable: a later call replaces the one installed, and a null `sink` or
/// `SCE_PEER_LOG_OFF` removes it. When the call returns, the function it replaced is called no more,
/// and its `user` may be freed. The library logs nothing until a sink is installed.
/// `SCE_PEER_ERR_BAD_ARGUMENT` for a level that is none of `SCE_PEER_LOG_*`;
/// `SCE_PEER_ERR_SINK_TAKEN` when another sink of the facade was installed in the process first (a
/// Rust library in the same image installed its own), and nothing is installed.
///
/// # Safety
/// `sink` is null or a function as [`ScePeerLogFn`] says, which may be called from any thread, and
/// from several at once, with `user`, until a later call replaces it; it calls no function of the
/// library.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_set_log_sink(
    max_level: i32,
    sink: Option<ScePeerLogFn>,
    user: *mut c_void,
) -> i32 {
    guard(|| {
        let most = match max_level {
            SCE_PEER_LOG_OFF => log::LevelFilter::Off,
            SCE_PEER_LOG_ERROR => log::LevelFilter::Error,
            SCE_PEER_LOG_WARN => log::LevelFilter::Warn,
            SCE_PEER_LOG_INFO => log::LevelFilter::Info,
            SCE_PEER_LOG_DEBUG => log::LevelFilter::Debug,
            SCE_PEER_LOG_TRACE => log::LevelFilter::Trace,
            other => {
                return Err(fail(
                    SCE_PEER_ERR_BAD_ARGUMENT,
                    format!("{other} is not a level of the library's log"),
                ))
            }
        };
        if !facade_installed() {
            return Err(fail(
                SCE_PEER_ERR_SINK_TAKEN,
                "another sink of the library's records is installed in this process",
            ));
        }
        let installed = sink
            .filter(|_| most != log::LevelFilter::Off)
            .map(|sink| HostSink { sink, user, most });
        let level = installed
            .as_ref()
            .map_or(log::LevelFilter::Off, |host| host.most);
        *SINK.write().unwrap_or_else(PoisonError::into_inner) = installed;
        log::set_max_level(level);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An outcome names `conflicted`, the number as text, only for an exchange that froze the
    /// device's own chain; the text of every other outcome is the one a lane read before it, and
    /// nothing else of it moves when it is written.
    #[test]
    fn an_outcome_names_conflicted_only_when_the_exchange_froze_the_devices_own_chain() {
        let text = |conflicted: Option<u64>| -> Value {
            let outcome = Outcome {
                round: Round {
                    conflicted,
                    ..Round::default()
                },
                reread: None,
                steps: Vec::new(),
                owed: 0,
                token: None,
                raised: None,
            };
            serde_json::from_str(&outcome_text(&Ok(outcome))).unwrap()
        };
        assert!(text(None).get("conflicted").is_none(), "{}", text(None));
        assert_eq!(text(Some(3))["conflicted"], json!("3"));
        let mut without = text(Some(3));
        without.as_object_mut().unwrap().remove("conflicted");
        assert_eq!(without, text(None));
    }
}
