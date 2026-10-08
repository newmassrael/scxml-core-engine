// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a machine asked of its host, as a scenario writes it.
//!
//! A generated machine with `<sce:action>`s takes a host that performs them, and
//! the generator writes one that only records them (`Recording<Machine>Actions`).
//! One function per machine here turns what that host recorded into the entries
//! of a scenario's `host_calls` — `{"action": "showAttempts", "args": [0, false]}`,
//! the name the document gives the action and its arguments in the document's
//! order — and hands back only what is new since it was last asked, which is how
//! a step is judged on the calls of that step alone.

use crate::integration::static_datamodel::static_host_call_sm::{
    RecordingStaticHostCallActions, StaticHostCallActionsCall, StaticHostCallPolicy,
};
use sce_rust_runtime::Engine;
use serde_json::{json, Value};

/// The calls `static_host_call` made of its host since `seen` of them were read.
pub fn static_host_call(
    engine: &Engine<StaticHostCallPolicy<RecordingStaticHostCallActions>>,
    seen: &mut usize,
) -> Vec<Value> {
    let calls = engine.policy().actions().calls();
    let fresh = calls[*seen..]
        .iter()
        .map(|call| match call {
            StaticHostCallActionsCall::ShowAttempts { count, exhausted } => {
                json!({"action": "showAttempts", "args": [count, exhausted]})
            }
        })
        .collect();
    *seen = calls.len();
    fresh
}
