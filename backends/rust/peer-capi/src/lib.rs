// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C ABI of the peer core of a device that holds a shared list (`sce-rust-peer`, ADR 0079).
//!
//! An application lane that is not Rust holds the core of one device of one list ([`ScePeer`]) and
//! speaks to it in texts: the machine's outbound events as their name and JSON payload
//! ([`sce_peer_outbound`]), the relay's requests and responses as the protocol's JSON
//! ([`sce_peer_poll_request`], [`sce_peer_respond`], [`sce_peer_fail`]), what a round came to as one
//! JSON text ([`sce_peer_take_outcome`]), and what the device keeps between runs as one text
//! ([`sce_peer_save`], [`sce_peer_restore`]). An administrator's device opens a list
//! ([`sce_peer_open_list`]), grants, admits and takes off devices ([`sce_peer_grant`],
//! [`sce_peer_add_member`], [`sce_peer_remove_member`]), and any device reads what it stands at in
//! the list as one JSON text ([`sce_peer_standing`]). No structure is laid out twice in C: the shapes
//! are the texts, written down in `docs/SCE_SHARED_LIST_PEER_ABI.md`.
//!
//! A lane that does not carry the texts itself runs an exchange over a link the library holds
//! ([`ScePeerLink`]): the relay's WebSocket binding ([`sce_peer_link_wss`]), or a transport the host
//! writes as one function the library calls ([`sce_peer_link_host`]). [`sce_peer_run`] runs the
//! exchange to its end on the calling thread, a background worker's, by a deadline and until a
//! cancellation another thread raises ([`ScePeerCancel`]); the host tells the link when the network
//! changed ([`sce_peer_link_network_changed`]), from any thread. The loop is the relay's one driver,
//! so framing, authentication and reconnection are written once, in Rust.
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

use std::cell::{RefCell, UnsafeCell};
use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

use sce_rust_peer::{
    drive, CourierError, Device, Driven, GroupStep, Kept, Outcome, Peer, PeerError, Profile,
    Publication, Round, RoundKind, Signal, Standing, Transport, TransportError,
};
use sce_rust_relay::wss::WssLink;
use sce_rust_relay::{Code, Notice};
use sce_rust_trust::{encoding, Identity, PublicKey};
use serde_json::{json, Value};

/// The version of this ABI: a change to what a function takes or returns, or to the shape of a text
/// it gives, makes it a new number. A function added alone does not, since a lane built against the
/// number before calls none of it. Version 2: the outcome holds `owed`, and the save is the core's
/// version 2, which holds what the relay is owed.
pub const SCE_PEER_ABI_VERSION: u32 = 2;

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
pub const SCE_PEER_ERR_NULL_ARGUMENT: i32 = 20;
pub const SCE_PEER_ERR_BAD_TEXT: i32 = 21;
pub const SCE_PEER_ERR_PANIC: i32 = 22;
pub const SCE_PEER_ERR_BAD_ARGUMENT: i32 = 23;
/// [`sce_peer_run`] stopped because its cancellation was raised, or because its deadline passed:
/// the exchange ended as a failure of the link, and its outcome waits.
pub const SCE_PEER_ERR_CANCELLED: i32 = 24;
pub const SCE_PEER_ERR_DEADLINE: i32 = 25;

/// What a host's transport says of one exchange ([`ScePeerExchangeFn`]): the response was given,
/// or why it was not ([`TransportError`]).
pub const SCE_PEER_TRANSPORT_ANSWERED: i32 = 0;
pub const SCE_PEER_TRANSPORT_LINK: i32 = 1;
pub const SCE_PEER_TRANSPORT_UNAUTHORIZED: i32 = 2;
pub const SCE_PEER_TRANSPORT_DEADLINE: i32 = 3;
pub const SCE_PEER_TRANSPORT_CANCELLED: i32 = 4;

/// What [`sce_peer_outbound`] says the core did with a send ([`Kept`]).
pub const SCE_PEER_KEPT_TAKEN: i32 = 0;
pub const SCE_PEER_KEPT_HELD: i32 = 1;
pub const SCE_PEER_KEPT_NOT_SIGNED: i32 = 2;
pub const SCE_PEER_KEPT_REFUSED: i32 = 3;

/// The kinds of round [`sce_peer_start`] begins ([`RoundKind`]).
pub const SCE_PEER_ROUND_SYNC: i32 = 0;
pub const SCE_PEER_ROUND_RESYNC: i32 = 1;
pub const SCE_PEER_ROUND_REREAD: i32 = 2;

/// Whether the hub of a list [`sce_peer_open_list`] opens is trusted ([`Profile`]).
pub const SCE_PEER_PROFILE_UNTRUSTED: i32 = 0;
pub const SCE_PEER_PROFILE_TRUSTED_HUB: i32 = 1;

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
                "owed": outcome.owed,
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
/// ([`Peer::set_administrator`]). A secret, given again before [`sce_peer_restore`].
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

/// Open a new list as its administrator, with the hub's profile `profile` (`SCE_PEER_PROFILE_*`)
/// ([`Peer::open_list`]): the genesis names the administrator's key the device holds, a grant makes
/// the device's person a member with the highest role, and the group is made when the core holds a
/// device of it. `genesis` is set to the hash (32 bytes) of the genesis, which a person is invited
/// with. The entries go to the relay with the next round.
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
/// the next round, the commit and the Welcome after it.
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

/// What the device keeps, as one text in `out` ([`Peer::save`]), the commits and Welcomes the relay
/// is still owed included. It does not hold the device's secrets: the person's seed, the
/// administrator's seed and the group's state are given again before [`sce_peer_restore`].
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
}

impl ScePeerLink {
    fn over(transport: Box<dyn Transport>) -> Self {
        Self {
            network: transport.network(),
            transport: UnsafeCell::new(transport),
        }
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
/// WebSocket binding, presenting `token` (NUL-terminated), the token the device's login earned. It
/// dials on the first run and again whenever it must. Null when it fails: an address that is not a
/// WebSocket's.
///
/// # Safety
/// `base` and `token` are NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn sce_peer_link_wss(
    base: *const c_char,
    token: *const c_char,
) -> *mut ScePeerLink {
    made(|| {
        // SAFETY: the promises in the function's contract.
        let (base, token) = unsafe { (text(base)?, text(token)?) };
        let link = WssLink::new(base, token).map_err(|error| {
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
        Ok(ScePeerLink::over(Box::new(HostTransport {
            exchange,
            release,
            context,
            network: Signal::new(),
        })))
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
        match drive(&mut held.peer, transport, deadline, cancel) {
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
