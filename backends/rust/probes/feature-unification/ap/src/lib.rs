// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The application half of the feature-unification probe (docs/adr/0053).
//!
//! Compiled only in the host `cargo check --workspace`, beside a crate that
//! asks the runtime for no features. What it holds compiles only where the
//! runtime has the defaults this crate asks for, so a build that handed this
//! crate the bare-metal profile, or took the HTTP processor or the macrostep
//! report away from it, fails here rather than in a downstream workspace.

#![deny(warnings)]

use sce_rust_runtime::{
    sce_string_from_str, Engine, HttpSendRequest, HttpSendResponse, StatePolicy,
};

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

/// What a host reads after driving a machine: how many macrosteps the engine
/// stopped short and how many calls it handed back with work still queued.
/// The accessors are the runtime's `macrostep-diagnostics`, which this crate's
/// defaults ask for and the bare-metal crate beside it does not; they used to
/// be removed by a feature (`no_macrostep_diagnostics`), and a build that
/// unified that feature with this crate stopped compiling here.
pub fn chains_cut_short<P: StatePolicy>(engine: &Engine<P>) -> (u32, u32) {
    (
        engine.truncated_macrosteps(),
        engine.truncated_event_chains(),
    )
}
