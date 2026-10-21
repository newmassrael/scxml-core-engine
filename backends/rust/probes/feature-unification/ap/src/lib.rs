// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The application half of the feature-unification probe (docs/adr/0053).
//!
//! Compiled only in the host `cargo check --workspace`, beside a crate that
//! asks the runtime for no features. What it holds compiles only where the
//! runtime is its `std` profile, so a build that handed this crate the
//! bare-metal profile, or took the HTTP processor away from it, fails here
//! rather than in a downstream workspace.

#![deny(warnings)]

use sce_rust_runtime::{sce_string_from_str, HttpSendRequest, HttpSendResponse};

/// An event string, which is the std `String` in this crate's build: under the
/// bare-metal profile `sce_string_from_str` answers a `heapless::String` and
/// this does not compile.
pub fn event_text(text: &str) -> String {
    sce_string_from_str(text)
}

/// What a host answers a BasicHTTP send with, through the processor's request
/// and response types, which `http-send` asks for and the bare-metal profile
/// does not have.
pub fn echo(request: &HttpSendRequest) -> HttpSendResponse {
    HttpSendResponse {
        event_name: request.event_name.clone(),
        event_data: request.content.clone(),
    }
}
