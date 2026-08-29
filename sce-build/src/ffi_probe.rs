// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A MEASUREMENT PROBE for the C-callable lowering surface. Not the
//! surface, and not a decision that there will be one.
//!
//! `docs/SCE_LUA_TRANSLATION_SEAM.md` prices a C surface over the
//! ECMAScript frontend, and the largest number in that pricing is how
//! much compiled code a C++ consumer would take on. That number can only
//! be measured by exporting the entry points and asking the linker what
//! it kept — which is why the first version of it was measured with a
//! throwaway probe that was deleted when the round ended. The document
//! then had to cite a figure nobody could reproduce, and it says so
//! against itself.
//!
//! So the probe is committed and gated behind an off-by-default feature.
//! It changes nothing about a normal build: `default = ["xsd"]` does not
//! name `ffi-probe`, no CMake target links a Rust artifact, and
//! `sce-build` stays `crate-type = ["rlib"]`. Turning the feature on
//! measures; it does not decide. The decision this feeds — whether a
//! C++ consumer carries the link at all, and whether unconditionally —
//! is a person's, and neither this file nor the document makes it.
//!
//! ## Why it returns the string instead of a status
//!
//! The earlier probe returned `NULL` from every entry point. That is a
//! measurement hazard, not just a leak: a wrapper that never uses the
//! lowered text lets the linker drop the emitter the measurement exists
//! to weigh, so the figure comes out too small. These wrappers hand the
//! text back through `CString::into_raw`, which keeps the whole
//! parse -> resolve -> emit path reachable, and `sce_lower_free` is the
//! matching release. Ownership is the caller's from `into_raw` to
//! `sce_lower_free`, exactly once.
//!
//! ## What a reader may conclude from a build of this
//!
//! Only the size of the reachable lowering code. Not the ergonomics of
//! the surface, not its error contract (the document counts that
//! separately, 15 codes against 0), and not whether the four entry
//! points are the right four.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::ecmascript::{
    to_lua_condition, to_lua_location, to_lua_script, to_lua_value, DocumentScope,
};

/// Borrow a caller's C string, or return null on invalid UTF-8.
///
/// # Safety
/// `s` must be a valid NUL-terminated C string or null.
unsafe fn borrow<'a>(s: *const c_char) -> Option<&'a str> {
    if s.is_null() {
        return None;
    }
    CStr::from_ptr(s).to_str().ok()
}

fn hand_back(result: Result<String, crate::forge::error::ExprError>) -> *mut c_char {
    match result {
        Ok(text) => CString::new(text)
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Open a scope handle. The scope is one flat set of names, so a handle
/// plus `declare` is the whole of what crosses the boundary.
///
/// # Safety
/// The returned pointer must be released with [`sce_scope_free`].
#[no_mangle]
pub unsafe extern "C" fn sce_scope_new() -> *mut DocumentScope {
    Box::into_raw(Box::new(DocumentScope::installed()))
}

/// Declare one name — the `<data id>` half of the scope obligation,
/// which the census measures as discharging 298 of 301 sites.
///
/// # Safety
/// `scope` must come from [`sce_scope_new`] and not yet be freed.
#[no_mangle]
pub unsafe extern "C" fn sce_scope_declare(scope: *mut DocumentScope, name: *const c_char) {
    let (Some(scope), Some(name)) = (scope.as_mut(), borrow(name)) else {
        return;
    };
    scope.declare(name);
}

/// Declare what a document-level `<script>` chunk introduces — the
/// remaining three sites. W3C SCXML 5.8 evaluates those at load time, so
/// a run-time caller can make this call before the first macrostep and
/// never maintain scope again.
///
/// # Safety
/// `scope` must come from [`sce_scope_new`] and not yet be freed.
#[no_mangle]
pub unsafe extern "C" fn sce_scope_declare_chunk(scope: *mut DocumentScope, source: *const c_char) {
    let (Some(scope), Some(source)) = (scope.as_mut(), borrow(source)) else {
        return;
    };
    scope.declare_chunk(source);
}

/// # Safety
/// `scope` must come from [`sce_scope_new`] and not already be freed.
#[no_mangle]
pub unsafe extern "C" fn sce_scope_free(scope: *mut DocumentScope) {
    if !scope.is_null() {
        drop(Box::from_raw(scope));
    }
}

/// # Safety
/// `source` must be a valid C string; `scope` must come from
/// [`sce_scope_new`]. The result is released with [`sce_lower_free`].
#[no_mangle]
pub unsafe extern "C" fn sce_lower_value(
    source: *const c_char,
    scope: *const DocumentScope,
) -> *mut c_char {
    let (Some(source), Some(scope)) = (borrow(source), scope.as_ref()) else {
        return std::ptr::null_mut();
    };
    hand_back(to_lua_value(source, scope))
}

/// # Safety
/// As [`sce_lower_value`].
#[no_mangle]
pub unsafe extern "C" fn sce_lower_condition(
    source: *const c_char,
    scope: *const DocumentScope,
) -> *mut c_char {
    let (Some(source), Some(scope)) = (borrow(source), scope.as_ref()) else {
        return std::ptr::null_mut();
    };
    hand_back(to_lua_condition(source, scope))
}

/// # Safety
/// As [`sce_lower_value`].
#[no_mangle]
pub unsafe extern "C" fn sce_lower_script(
    source: *const c_char,
    scope: *const DocumentScope,
) -> *mut c_char {
    let (Some(source), Some(scope)) = (borrow(source), scope.as_ref()) else {
        return std::ptr::null_mut();
    };
    hand_back(to_lua_script(source, scope))
}

/// # Safety
/// `source` must be a valid C string. The result is released with
/// [`sce_lower_free`].
#[no_mangle]
pub unsafe extern "C" fn sce_lower_location(source: *const c_char) -> *mut c_char {
    let Some(source) = borrow(source) else {
        return std::ptr::null_mut();
    };
    hand_back(to_lua_location(source))
}

/// # Safety
/// `text` must have come from one of the `sce_lower_*` entry points and
/// not already be freed.
#[no_mangle]
pub unsafe extern "C" fn sce_lower_free(text: *mut c_char) {
    if !text.is_null() {
        drop(CString::from_raw(text));
    }
}
