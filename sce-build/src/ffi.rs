// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The C-callable surface over the ECMAScript frontend.
//!
//! ## What this is, and what it stopped being
//!
//! This module began as a MEASUREMENT PROBE, committed behind an
//! off-by-default feature so the D1 ledger's largest number could be
//! re-derived rather than cited from a throwaway. It said of itself, in
//! this docstring, "not the surface, and not a decision that there will
//! be one".
//!
//! **That is no longer true.** The owner decided on 2026-08-29 to link
//! the frontend into the C++ engine and retire `EcmaScriptToLuaTransformer`,
//! and `docs/SCE_LUA_TRANSLATION_SEAM.md`'s D1 ledger carries that
//! decision as a row. A file whose name and prose still said "probe"
//! while a shipped engine called it would be the exact drift the ledger
//! exists to catch, so the feature is `ffi`, the module is `ffi`, and
//! this paragraph is here instead of the old disclaimer.
//!
//! It stays behind a feature because it is not needed by the code
//! generator: `default = ["xsd"]` does not name `ffi`, and a `cargo
//! build` for the generator compiles none of it. What turns it on is the
//! C++ build (`cmake/SCEBuildLowering.cmake`) and
//! `scripts/measure-lowering-footprint.sh`.
//!
//! ## The contract, and where its other half lives
//!
//! `sce/include/scripting/SceLowering.h` declares these functions for
//! C++. The two are held together by `ffi_header_matches_surface`, which
//! fails if either side grows, loses or renames an entry point — a hand
//! written header that drifts from its library is a segfault with a
//! clean compile.
//!
//! ## Ownership
//!
//! Every `sce_lower_*` hands back a heap string through
//! `CString::into_raw`; the caller owns it from that moment and releases
//! it with [`sce_lower_free`], exactly once. A `NULL` return means the
//! frontend REFUSED — the expression did not parse, or it named
//! something the scope does not declare — and there is nothing to free.
//!
//! ⚠ Refusal is a normal answer, not an error to route around. It is
//! what lets a caller send only the expressions the frontend can answer
//! and leave the rest on its existing path, which is precisely how
//! `LuaEngine` adopts this without the rewriter having to retire first.
//!
//! ## Why the wrappers return the string rather than a status
//!
//! An earlier form returned `NULL` from every entry point. That is a
//! measurement hazard as well as a useless API: a wrapper that never
//! uses the lowered text lets the linker drop the emitter, so the size
//! the ledger reports comes out too small.

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
/// A scope with nothing declared is not a degenerate case: it is what a
/// caller uses to ask "can you answer this expression without me telling
/// you any name?", and 11 of the 23 divergences this repository tracks
/// are answered that way.
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
/// remaining three sites. A processor evaluates those at document load
/// time (§scxml-5.8), so a run-time caller can make this call before
/// the first macrostep and never maintain scope again.
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
