// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The machines a replay reads back through what their host observes of them
//! ([`crate::replay_observed`]) rather than through a saved state: those a saved
//! state cannot hold yet. A machine that holds an optional value is one until
//! docs/adr/0055 step 6 writes `null` into a saved state; until then it has no
//! `Persist`, and its reader here answers the members of a saved state a step
//! is held to (`current`, `configuration`, `variables`), each value as a saved
//! state writes it: a 64-bit integer as the text of its digits, a value of an
//! enum as the name its document declares, an absent value as `null`.

use crate::integration::static_datamodel::static_optional_sm::{
    StaticOptionalDueRecord, StaticOptionalObserve, StaticOptionalPolicy, StaticOptionalStatusEnum,
};
use sce_rust_runtime::{Engine, StatePolicy};
use serde_json::{json, Value};

/// A value as a saved state writes it.
trait Observed {
    fn observed(&self) -> Value;
}

/// A 64-bit integer is the text of its digits: a JSON number past 2^53 is not
/// exact everywhere.
impl Observed for i64 {
    fn observed(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl Observed for u32 {
    fn observed(&self) -> Value {
        Value::from(*self)
    }
}

impl Observed for bool {
    fn observed(&self) -> Value {
        Value::Bool(*self)
    }
}

impl Observed for String {
    fn observed(&self) -> Value {
        Value::String(self.clone())
    }
}

/// An absent value is `null` (docs/adr/0055, section 4).
impl<T: Observed> Observed for Option<T> {
    fn observed(&self) -> Value {
        match self {
            Some(present) => present.observed(),
            None => Value::Null,
        }
    }
}

impl<T: Observed> Observed for Vec<T> {
    fn observed(&self) -> Value {
        Value::Array(self.iter().map(Observed::observed).collect())
    }
}

impl Observed for StaticOptionalStatusEnum {
    fn observed(&self) -> Value {
        Value::String(self.sce_name().to_string())
    }
}

impl Observed for StaticOptionalDueRecord {
    fn observed(&self) -> Value {
        json!({
            "id": self.id.observed(),
            "due": self.due.observed(),
            "title": self.title.observed(),
            "status": self.status.observed(),
        })
    }
}

/// `static_optional` as its host observes it: its active states, and every
/// variable it publishes.
pub fn static_optional(engine: &Engine<StaticOptionalPolicy>) -> Value {
    let snapshot = engine.snapshot();
    let data = &snapshot.data;
    let configuration: Vec<&str> = snapshot
        .configuration
        .iter()
        .map(|state| StaticOptionalPolicy::get_state_name(*state))
        .collect();
    json!({
        "current": StaticOptionalPolicy::get_state_name(engine.get_current_state()),
        "configuration": configuration,
        "variables": {
            "due": data.due.observed(),
            "other": data.other.observed(),
            "title": data.title.observed(),
            "status": data.status.observed(),
            "item": data.item.observed(),
            "items": data.items.observed(),
            "days": data.days.observed(),
            "following": data.following.observed(),
            "late": data.late.observed(),
            "early": data.early.observed(),
            "label": data.label.observed(),
            "plain": data.plain.observed(),
            "kept": data.kept.observed(),
            "listed": data.listed.observed(),
            "missing": data.missing.observed(),
            "same": data.same.observed(),
            "absent": data.absent.observed(),
            "statusDone": data.status_done.observed(),
            "echoed": data.echoed.observed(),
            "echoTitle": data.echo_title.observed(),
            "echoStatus": data.echo_status.observed(),
            "fallbacks": data.fallbacks.observed(),
            "errors": data.errors.observed(),
        }
    })
}
