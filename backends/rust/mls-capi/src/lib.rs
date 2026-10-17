// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C ABI of the group key agreement of a shared list (`sce-rust-mls`).
//!
//! A lane that is not Rust holds a device ([`SceMlsDevice`]) and tells it the membership log as data
//! ([`SceMlsLog`]: who the members are with which key, the commit the log names for each epoch, which
//! epoch the log is in). The lane is responsible for that being what its own verification of the log
//! gave; the group applies a commit only when the data names it, and refuses a roster that is not the
//! data's, as it does for a Rust host that gives it a `Trust`.
//!
//! Every function returns 0 for success and a nonzero status otherwise, and says why in
//! [`sce_mls_last_error`] (per thread). Buffers the library returns ([`SceMlsBuf`]) are the library's
//! and are given back with [`sce_mls_buf_free`]. The header is `include/sce_mls.h`, which is written
//! by hand and kept equal to this file's exports by a test.
//!
//! This is the one crate of the group that holds `unsafe` code. Every pointer a caller gives is
//! checked for null, and is read for exactly the length it comes with; a panic is caught and
//! reported and never crosses the boundary.

#![deny(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

use sce_rust_mls::{Applied, Device, MlsError, Roster};
use sce_rust_trust::Identity;

/// The version of this ABI: a change to what a function takes or returns makes it a new number.
pub const SCE_MLS_ABI_VERSION: u32 = 1;

pub const SCE_MLS_OK: i32 = 0;
pub const SCE_MLS_ERR_LIBRARY: i32 = 1;
pub const SCE_MLS_ERR_NO_GROUP: i32 = 2;
pub const SCE_MLS_ERR_IN_GROUP: i32 = 3;
pub const SCE_MLS_ERR_PENDING_COMMIT: i32 = 4;
pub const SCE_MLS_ERR_NO_PENDING_COMMIT: i32 = 5;
pub const SCE_MLS_ERR_COMMIT_NOT_NAMED: i32 = 6;
pub const SCE_MLS_ERR_NOT_IN_LOG: i32 = 7;
pub const SCE_MLS_ERR_STILL_A_MEMBER: i32 = 8;
pub const SCE_MLS_ERR_ROSTER_DIFFERS: i32 = 9;
pub const SCE_MLS_ERR_WELCOME_NOT_THE_LOGS: i32 = 10;
pub const SCE_MLS_ERR_MALFORMED: i32 = 11;
pub const SCE_MLS_ERR_REMOVED: i32 = 12;
pub const SCE_MLS_ERR_NULL_ARGUMENT: i32 = 20;
pub const SCE_MLS_ERR_BAD_TEXT: i32 = 21;
pub const SCE_MLS_ERR_PANIC: i32 = 22;

/// A device of one list: its MLS state and its key.
pub struct SceMlsDevice {
    device: Device,
}

/// The membership log as data.
pub struct SceMlsLog {
    roster: Roster,
}

/// A buffer the library made. It is freed with [`sce_mls_buf_free`] and nothing else.
#[repr(C)]
pub struct SceMlsBuf {
    pub ptr: *mut u8,
    pub len: usize,
}

impl SceMlsBuf {
    const EMPTY: SceMlsBuf = SceMlsBuf {
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

fn status_of(error: &MlsError) -> i32 {
    match error {
        MlsError::Library(_) => SCE_MLS_ERR_LIBRARY,
        MlsError::NoGroup => SCE_MLS_ERR_NO_GROUP,
        MlsError::InGroup => SCE_MLS_ERR_IN_GROUP,
        MlsError::PendingCommit => SCE_MLS_ERR_PENDING_COMMIT,
        MlsError::NoPendingCommit => SCE_MLS_ERR_NO_PENDING_COMMIT,
        MlsError::CommitNotNamed { .. } => SCE_MLS_ERR_COMMIT_NOT_NAMED,
        MlsError::NotInLog(_) => SCE_MLS_ERR_NOT_IN_LOG,
        MlsError::StillAMember(_) => SCE_MLS_ERR_STILL_A_MEMBER,
        MlsError::RosterDiffers(_) => SCE_MLS_ERR_ROSTER_DIFFERS,
        MlsError::WelcomeNotTheLogs => SCE_MLS_ERR_WELCOME_NOT_THE_LOGS,
        MlsError::Malformed(_) => SCE_MLS_ERR_MALFORMED,
        MlsError::Removed => SCE_MLS_ERR_REMOVED,
    }
}

fn from_error(error: MlsError) -> i32 {
    let status = status_of(&error);
    fail(status, format!("{error:?}"))
}

/// Run `body`, turning a panic into a status.
fn guard(body: impl FnOnce() -> Result<(), i32>) -> i32 {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(())) => SCE_MLS_OK,
        Ok(Err(status)) => status,
        Err(_) => fail(SCE_MLS_ERR_PANIC, "the library panicked"),
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
        return Err(fail(SCE_MLS_ERR_NULL_ARGUMENT, "a pointer is null"));
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
        .map_err(|_| fail(SCE_MLS_ERR_NULL_ARGUMENT, "a pointer is null"))
}

/// A text the caller gives as a NUL-terminated string.
///
/// # Safety
/// `ptr` is null or points at a NUL-terminated string.
unsafe fn text<'a>(ptr: *const c_char) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Err(fail(SCE_MLS_ERR_NULL_ARGUMENT, "a string is null"));
    }
    // SAFETY: not null and NUL-terminated by the caller's promise.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| fail(SCE_MLS_ERR_BAD_TEXT, "a string is not UTF-8"))
}

/// The object a handle names.
///
/// # Safety
/// `ptr` is null or a live handle made by this library, not used by another thread meanwhile.
unsafe fn handle<'a, T>(ptr: *mut T) -> Result<&'a mut T, i32> {
    // SAFETY: the caller's promise.
    unsafe { ptr.as_mut() }.ok_or_else(|| fail(SCE_MLS_ERR_NULL_ARGUMENT, "a handle is null"))
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

/// The number of this ABI.
#[no_mangle]
pub extern "C" fn sce_mls_abi_version() -> u32 {
    SCE_MLS_ABI_VERSION
}

/// What went wrong in the last call of this thread that failed: a NUL-terminated text the library
/// owns, good until the next call of this thread.
#[no_mangle]
pub extern "C" fn sce_mls_last_error() -> *const c_char {
    LAST_ERROR.with(|last| last.borrow().as_ptr())
}

/// Give a buffer back.
///
/// # Safety
/// `buf` is one this library returned and has not given back, or empty.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_buf_free(buf: SceMlsBuf) {
    if !buf.ptr.is_null() {
        // SAFETY: made by `SceMlsBuf::of` from a boxed slice of this length.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(buf.ptr, buf.len)) });
    }
}

/// A log of the epoch `epoch`, with no member and no named commit.
#[no_mangle]
pub extern "C" fn sce_mls_log_new(epoch: u32) -> *mut SceMlsLog {
    Box::into_raw(Box::new(SceMlsLog {
        roster: Roster::new(epoch),
    }))
}

/// Tell the log that the device of `node` is a member and signs with `key` (32 bytes).
///
/// # Safety
/// `log` is a live handle; `key` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_log_add_member(
    log: *mut SceMlsLog,
    node: u64,
    key: *const u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (log, key) = unsafe { (handle(log)?, array::<32>(key)?) };
        log.roster.add_member(node, key);
        Ok(())
    })
}

/// Tell the log that the commit whose SHA-256 is `hash` (32 bytes) is the one that begins `epoch`.
///
/// # Safety
/// `log` is a live handle; `hash` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_log_name_commit(
    log: *mut SceMlsLog,
    epoch: u32,
    hash: *const u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (log, hash) = unsafe { (handle(log)?, array::<32>(hash)?) };
        log.roster.name_commit(epoch, hash);
        Ok(())
    })
}

/// Free a log.
///
/// # Safety
/// `log` is null or a handle from [`sce_mls_log_new`] not freed yet.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_log_free(log: *mut SceMlsLog) {
    if !log.is_null() {
        // SAFETY: made by `Box::into_raw` in `sce_mls_log_new`.
        drop(unsafe { Box::from_raw(log) });
    }
}

/// A device of the list `list` (NUL-terminated) for `node`, signing with the key `seed` (32 bytes)
/// makes. Null when it fails, and why is in [`sce_mls_last_error`].
///
/// # Safety
/// `list` is a NUL-terminated string; `seed` points at 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_device_new(
    list: *const c_char,
    node: u64,
    seed: *const u8,
) -> *mut SceMlsDevice {
    let mut made = std::ptr::null_mut();
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (list, seed) = unsafe { (text(list)?, array::<32>(seed)?) };
        let device = Device::new(list, node, Identity::from_seed(seed));
        made = Box::into_raw(Box::new(SceMlsDevice { device }));
        Ok(())
    });
    made
}

/// A device as [`sce_mls_device_to_bytes`] kept it. Null when it fails.
///
/// # Safety
/// `list` is a NUL-terminated string; `seed` points at 32 readable bytes; `state` at `len`.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_device_from_bytes(
    list: *const c_char,
    node: u64,
    seed: *const u8,
    state: *const u8,
    len: usize,
) -> *mut SceMlsDevice {
    let mut made = std::ptr::null_mut();
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (list, seed, state) = unsafe { (text(list)?, array::<32>(seed)?, bytes(state, len)?) };
        let device =
            Device::from_bytes(list, node, Identity::from_seed(seed), state).map_err(from_error)?;
        made = Box::into_raw(Box::new(SceMlsDevice { device }));
        Ok(())
    });
    made
}

/// Free a device.
///
/// # Safety
/// `device` is null or a handle from this library not freed yet.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_device_free(device: *mut SceMlsDevice) {
    if !device.is_null() {
        // SAFETY: made by `Box::into_raw` in this library.
        drop(unsafe { Box::from_raw(device) });
    }
}

/// What the device keeps of its group, secret, for the host's store.
///
/// # Safety
/// `device` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_device_to_bytes(
    device: *mut SceMlsDevice,
    out: *mut SceMlsBuf,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let device = unsafe { handle(device)? };
        let kept = device.device.to_bytes().map_err(from_error)?;
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, SceMlsBuf::of(kept)) };
        Ok(())
    })
}

/// A key package of this device, for an administrator to add it with.
///
/// # Safety
/// `device` is a live handle; `out` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_key_package(
    device: *mut SceMlsDevice,
    out: *mut SceMlsBuf,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let device = unsafe { handle(device)? };
        let package = device.device.key_package().map_err(from_error)?;
        // SAFETY: `out` is valid for a write.
        unsafe { put(out, SceMlsBuf::of(package)) };
        Ok(())
    })
}

/// Begin the list's group with this device alone in it.
///
/// # Safety
/// `device` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_create_group(device: *mut SceMlsDevice) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let device = unsafe { handle(device)? };
        device.device.create_group().map_err(from_error)
    })
}

/// The commit that adds the device of one key package, made and not merged. `commit` and `welcome`
/// are given back (free them); `commit_hash` (32 bytes) is what the log names it by, and `epoch` the
/// epoch it makes. Any of the out pointers may be null.
///
/// # Safety
/// `device` and `log` are live handles; `package` points at `package_len` bytes; each out pointer is
/// null or valid for a write (`commit_hash` for 32 bytes).
#[no_mangle]
pub unsafe extern "C" fn sce_mls_stage_add(
    device: *mut SceMlsDevice,
    package: *const u8,
    package_len: usize,
    log: *mut SceMlsLog,
    commit: *mut SceMlsBuf,
    welcome: *mut SceMlsBuf,
    commit_hash: *mut u8,
    epoch: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (device, log, package) =
            unsafe { (handle(device)?, handle(log)?, bytes(package, package_len)?) };
        let pending = device
            .device
            .stage_add(&[package.to_vec()], &log.roster)
            .map_err(from_error)?;
        // SAFETY: each out pointer is null or valid for a write.
        unsafe {
            put(commit, SceMlsBuf::of(pending.commit));
            put(
                welcome,
                pending.welcome.map_or(SceMlsBuf::EMPTY, SceMlsBuf::of),
            );
            if !commit_hash.is_null() {
                std::ptr::copy_nonoverlapping(pending.commit_hash.as_ptr(), commit_hash, 32);
            }
            put(epoch, pending.epoch);
        }
        Ok(())
    })
}

/// The commit that removes the device of `node`, made and not merged. Outputs as
/// [`sce_mls_stage_add`], without a Welcome.
///
/// # Safety
/// As [`sce_mls_stage_add`].
#[no_mangle]
pub unsafe extern "C" fn sce_mls_stage_remove(
    device: *mut SceMlsDevice,
    node: u64,
    log: *mut SceMlsLog,
    commit: *mut SceMlsBuf,
    commit_hash: *mut u8,
    epoch: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (device, log) = unsafe { (handle(device)?, handle(log)?) };
        let pending = device
            .device
            .stage_remove(&[node], &log.roster)
            .map_err(from_error)?;
        // SAFETY: each out pointer is null or valid for a write.
        unsafe {
            put(commit, SceMlsBuf::of(pending.commit));
            if !commit_hash.is_null() {
                std::ptr::copy_nonoverlapping(pending.commit_hash.as_ptr(), commit_hash, 32);
            }
            put(epoch, pending.epoch);
        }
        Ok(())
    })
}

/// Merge the commit made here, when the log names it.
///
/// # Safety
/// `device` and `log` are live handles; `epoch` is null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_merge_pending(
    device: *mut SceMlsDevice,
    log: *mut SceMlsLog,
    epoch: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (device, log) = unsafe { (handle(device)?, handle(log)?) };
        let merged = device
            .device
            .merge_pending(&log.roster)
            .map_err(from_error)?;
        // SAFETY: `epoch` is null or valid for a write.
        unsafe { put(epoch, merged) };
        Ok(())
    })
}

/// Give up the commit made here and not merged.
///
/// # Safety
/// `device` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_clear_pending(device: *mut SceMlsDevice) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let device = unsafe { handle(device)? };
        device.device.clear_pending().map_err(from_error)
    })
}

/// Apply a commit another device made, when the log names it. `removed` is set to 1 when the commit
/// removed this device (it then derives no later key) and to 0 otherwise; `epoch` is the epoch the
/// group is in, when it is.
///
/// # Safety
/// `device` and `log` are live handles; `commit` points at `commit_len` bytes; the out pointers are
/// null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_apply_commit(
    device: *mut SceMlsDevice,
    commit: *const u8,
    commit_len: usize,
    log: *mut SceMlsLog,
    removed: *mut i32,
    epoch: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (device, log, commit) =
            unsafe { (handle(device)?, handle(log)?, bytes(commit, commit_len)?) };
        let applied = device
            .device
            .apply_commit(commit, &log.roster)
            .map_err(from_error)?;
        // SAFETY: the out pointers are null or valid for a write.
        unsafe {
            match applied {
                Applied::Epoch(now) => {
                    put(removed, 0);
                    put(epoch, now);
                }
                Applied::Removed => put(removed, 1),
            }
        }
        Ok(())
    })
}

/// Join the group from a Welcome, with the commit that made its epoch.
///
/// # Safety
/// `device` and `log` are live handles; `welcome` and `commit` point at their lengths; `epoch` is
/// null or valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_join(
    device: *mut SceMlsDevice,
    welcome: *const u8,
    welcome_len: usize,
    commit: *const u8,
    commit_len: usize,
    log: *mut SceMlsLog,
    epoch: *mut u64,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (device, log, welcome, commit) = unsafe {
            (
                handle(device)?,
                handle(log)?,
                bytes(welcome, welcome_len)?,
                bytes(commit, commit_len)?,
            )
        };
        let joined = device
            .device
            .join(welcome, commit, &log.roster)
            .map_err(from_error)?;
        // SAFETY: `epoch` is null or valid for a write.
        unsafe { put(epoch, joined) };
        Ok(())
    })
}

/// The epoch the group is in.
///
/// # Safety
/// `device` is a live handle; `epoch` is valid for a write.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_epoch(device: *mut SceMlsDevice, epoch: *mut u64) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let device = unsafe { handle(device)? };
        let now = device.device.epoch().map_err(from_error)?;
        // SAFETY: `epoch` is valid for a write.
        unsafe { put(epoch, now) };
        Ok(())
    })
}

/// 1 when the device is in the group, 0 when it is not (it was removed, or has not joined).
///
/// # Safety
/// `device` is null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_is_active(device: *mut SceMlsDevice) -> i32 {
    // SAFETY: the promise in the function's contract.
    match unsafe { device.as_ref() } {
        Some(device) => i32::from(device.device.is_active()),
        None => 0,
    }
}

/// The key (32 bytes) of the group's current epoch for sealing the list's content.
///
/// # Safety
/// `device` is a live handle; `out` is valid for a write of 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_epoch_key(device: *mut SceMlsDevice, out: *mut u8) -> i32 {
    guard(|| {
        // SAFETY: the promise in the function's contract.
        let device = unsafe { handle(device)? };
        let key = device.device.epoch_key().map_err(from_error)?;
        if out.is_null() {
            return Err(fail(SCE_MLS_ERR_NULL_ARGUMENT, "a pointer is null"));
        }
        // SAFETY: `out` is valid for 32 bytes.
        unsafe { std::ptr::copy_nonoverlapping(key.as_ptr(), out, 32) };
        Ok(())
    })
}

/// The key (32 bytes) of the section `name` (NUL-terminated) in the current epoch.
///
/// # Safety
/// `device` is a live handle; `name` is a NUL-terminated string; `out` is valid for 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sce_mls_section_key(
    device: *mut SceMlsDevice,
    name: *const c_char,
    out: *mut u8,
) -> i32 {
    guard(|| {
        // SAFETY: the promises in the function's contract.
        let (device, name) = unsafe { (handle(device)?, text(name)?) };
        let key = device.device.section_key(name).map_err(from_error)?;
        if out.is_null() {
            return Err(fail(SCE_MLS_ERR_NULL_ARGUMENT, "a pointer is null"));
        }
        // SAFETY: `out` is valid for 32 bytes.
        unsafe { std::ptr::copy_nonoverlapping(key.as_ptr(), out, 32) };
        Ok(())
    })
}
