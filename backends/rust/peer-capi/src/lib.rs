// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C ABI of the peer core of a device that holds a shared list (`sce-rust-peer`, ADR 0079).
//!
//! An application lane that is not Rust holds the core of one device of one list ([`ScePeer`]) and
//! speaks to it in texts: the machine's outbound events as their name and JSON payload
//! ([`sce_peer_outbound`]), the relay's requests and responses as the protocol's JSON
//! ([`sce_peer_poll_request`], [`sce_peer_respond`], [`sce_peer_fail`]), what a round came to as one
//! JSON text ([`sce_peer_take_outcome`]), and what the device keeps between runs as one text
//! ([`sce_peer_save`], [`sce_peer_restore`]). No structure is laid out twice in C: the shapes are the
//! texts, written down in `docs/SCE_SHARED_LIST_PEER_ABI.md`.
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

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

use sce_rust_peer::{
    CourierError, Device, GroupStep, Kept, Outcome, Peer, PeerError, Publication, Round, RoundKind,
};
use sce_rust_relay::Code;
use sce_rust_trust::Identity;
use serde_json::{json, Value};

/// The version of this ABI: a change to what a function takes or returns, or to the shape of a text
/// it gives, makes it a new number.
pub const SCE_PEER_ABI_VERSION: u32 = 1;

pub const SCE_PEER_OK: i32 = 0;
pub const SCE_PEER_ERR_NOTHING_ASKED: i32 = 1;
pub const SCE_PEER_ERR_OUTCOME_NOT_TAKEN: i32 = 2;
pub const SCE_PEER_ERR_NO_GROUP_DEVICE: i32 = 3;
pub const SCE_PEER_ERR_GROUP: i32 = 4;
pub const SCE_PEER_ERR_UNREADABLE: i32 = 5;
pub const SCE_PEER_ERR_NULL_ARGUMENT: i32 = 20;
pub const SCE_PEER_ERR_BAD_TEXT: i32 = 21;
pub const SCE_PEER_ERR_PANIC: i32 = 22;
pub const SCE_PEER_ERR_BAD_ARGUMENT: i32 = 23;

/// What [`sce_peer_outbound`] says the core did with a send ([`Kept`]).
pub const SCE_PEER_KEPT_TAKEN: i32 = 0;
pub const SCE_PEER_KEPT_HELD: i32 = 1;
pub const SCE_PEER_KEPT_NOT_SIGNED: i32 = 2;
pub const SCE_PEER_KEPT_REFUSED: i32 = 3;

/// The kinds of round [`sce_peer_start`] begins ([`RoundKind`]).
pub const SCE_PEER_ROUND_SYNC: i32 = 0;
pub const SCE_PEER_ROUND_RESYNC: i32 = 1;
pub const SCE_PEER_ROUND_REREAD: i32 = 2;

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
    }
}

fn from_error(error: PeerError) -> i32 {
    let status = status_of(&error);
    fail(status, format!("{error:?}"))
}

/// The group's device refused, and why.
fn from_group_error(error: impl std::fmt::Debug) -> i32 {
    fail(SCE_PEER_ERR_GROUP, format!("{error:?}"))
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

/// The object a handle names.
///
/// # Safety
/// `ptr` is null or a live handle made by this library, not used by another thread meanwhile.
unsafe fn handle<'a, T>(ptr: *mut T) -> Result<&'a mut T, i32> {
    // SAFETY: the caller's promise.
    unsafe { ptr.as_mut() }.ok_or_else(|| fail(SCE_PEER_ERR_NULL_ARGUMENT, "a handle is null"))
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

/// Make a core and hand it out as a handle; null when `make` fails, and why is in
/// [`sce_peer_last_error`].
fn made(make: impl FnOnce() -> Result<ScePeer, i32>) -> *mut ScePeer {
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

/// The word the relay's protocol names a refusal by.
fn code_tag(code: &Code) -> &'static str {
    match code {
        Code::Unauthorized => "unauthorized",
        Code::Forbidden => "forbidden",
        Code::RateLimited { .. } => "rate_limited",
        Code::TooLarge => "too_large",
        Code::Invalid => "invalid",
        Code::Behind => "behind",
        Code::Unavailable => "unavailable",
    }
}

/// What an exchange came to, as the one text [`sce_peer_take_outcome`] gives.
fn outcome_text(outcome: &Result<Outcome, CourierError>) -> String {
    let value = match outcome {
        Ok(outcome) => {
            let rounds: Vec<&Round> = std::iter::once(&outcome.round)
                .chain(outcome.reread.as_ref())
                .collect();
            json!({
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
                "rebuild": outcome.rebuild(),
                "stalled": outcome.stalled(),
                "reread": outcome.reread.is_some(),
                "steps": outcome.steps.iter().map(group_step).collect::<Vec<_>>(),
            })
        }
        Err(CourierError::Link(reason)) => {
            json!({"outcome": "failed", "failure": "link", "reason": reason})
        }
        Err(CourierError::Relay { code, reason }) => {
            let mut value = json!({
                "outcome": "failed",
                "failure": "relay",
                "code": code_tag(code),
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

/// The core holds the device of `node` in the list's MLS group, signing with the key the 32-byte
/// `seed` makes ([`Peer::with_group`]): a new device when `len` is 0, or the one
/// [`sce_peer_group_state`] gave, read from `state`. Called before the first exchange.
///
/// # Safety
/// `peer` is a live handle; `seed` points at 32 readable bytes; `state` at `len`.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_with_group(
    peer: *mut ScePeer,
    node: u64,
    seed: *const u8,
    state: *const u8,
    len: usize,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, seed, state) =
            unsafe { (handle(peer)?, array::<32>(seed)?, bytes(state, len)?) };
        let identity = Identity::from_seed(seed);
        let device = if state.is_empty() {
            Device::new(&held.list, node, identity)
        } else {
            Device::from_bytes(&held.list, node, identity, state).map_err(from_group_error)?
        };
        held.rebuild(|peer| peer.with_group(device, node));
        Ok(())
    })
}

/// What the group's device keeps (its group, its secrets, the private halves of its key packages),
/// secret, for the host's store: [`sce_peer_save`] does not hold it, and [`sce_peer_with_group`]
/// takes it back.
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_group_state(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let member = held
            .peer
            .group()
            .ok_or_else(|| from_error(PeerError::NoGroupDevice))?;
        let state = member.device().to_bytes().map_err(from_group_error)?;
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(state)) };
        Ok(())
    })
}

/// What the machine sent to `#hub` or `#people`: the event's name and its JSON payload, both
/// NUL-terminated ([`Peer::outbound`]). `kept` is set to what the core did with it
/// (`SCE_PEER_KEPT_*`), and, when it refused, `reason` (which may be null) to why; `reason` is set to
/// an empty buffer otherwise.
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
        let (answer, why) = match held.peer.outbound(event, data) {
            Kept::Taken => (SCE_PEER_KEPT_TAKEN, None),
            Kept::Held => (SCE_PEER_KEPT_HELD, None),
            Kept::NotSigned => (SCE_PEER_KEPT_NOT_SIGNED, None),
            Kept::Refused(why) => (SCE_PEER_KEPT_REFUSED, Some(why)),
        };
        // SAFETY: `kept` is valid for a write, `reason` null or valid for one.
        unsafe {
            put(kept, answer);
            if let Some(why) = why {
                put(reason, ScePeerBuf::of(why.into_bytes()));
            }
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

/// The word the machine is told that the instant `before` is settled, as the JSON pair of the
/// event's name and its payload text, when the checkpoints the device holds make a settlement of it
/// with `digest`; an empty buffer when they do not ([`Peer::floor_event`]).
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_floor_event(
    peer: *mut ScePeer,
    before: i64,
    digest: u32,
    out: *mut ScePeerBuf,
) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        if let Some((event, data)) = held.peer.floor_event(before, digest) {
            let pair = json!([event, data]).to_string();
            // SAFETY: `out` is valid for a write.
            unsafe { put(out, ScePeerBuf::of(pair.into_bytes())) };
        }
        Ok(())
    })
}

/// What the device keeps, as one text in `out` ([`Peer::save`]). It does not hold the device's
/// secrets: the person's seed and the group's state are given again before [`sce_peer_restore`].
///
/// # Safety
/// `peer` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_save(peer: *mut ScePeer, out: *mut ScePeerBuf) -> i32 {
    guard(|| {
        required(out)?;
        // SAFETY: the promises in the function's contract.
        let held = unsafe {
            put(out, ScePeerBuf::EMPTY);
            handle(peer)?
        };
        let saved = held.peer.save();
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, ScePeerBuf::of(saved.into_bytes())) };
        Ok(())
    })
}

/// Take back what [`sce_peer_save`] wrote (NUL-terminated), into a core just made for the same list
/// after its person was given again ([`Peer::restore`]); `kept` (which may be null) is set to how
/// many operations and checkpoints the log verified. The device then signs nothing new until a
/// round has read its own chain back from the relay.
///
/// # Safety
/// `peer` is a live handle; `saved` is a NUL-terminated string; `kept` is null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_restore(
    peer: *mut ScePeer,
    saved: *const c_char,
    kept: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (held, saved) = unsafe { (handle(peer)?, text(saved)?) };
        let count = held.peer.restore(saved).map_err(from_error)?;
        // SAFETY: `kept` is null or valid for a write.
        unsafe { put(kept, count as u64) };
        Ok(())
    })
}
