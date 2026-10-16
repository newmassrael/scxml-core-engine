// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Notifications (docs/SCE_HOST_FEATURES.md, section 4), kept in memory: the
//! platform's tray is two lists.

use crate::clock::utc_now;
use crate::{failure, record, Outcome, ReferenceHost, Transport};
use serde_json::json;

/// One notification the host holds.
pub(crate) struct Notification {
    pub(crate) id: String,
    // What was posted is held as it was given: the host shows text as written.
    #[allow(dead_code)]
    pub(crate) title: String,
    #[allow(dead_code)]
    pub(crate) body: String,
}

pub(crate) struct Notifications {
    pub(crate) permission: String,
    pub(crate) user_decision: String,
    /// Kept to be shown at their instant, in the order posted.
    pub(crate) waiting: Vec<Notification>,
    /// Handed to the platform, in the order posted.
    pub(crate) shown: Vec<Notification>,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            permission: "undetermined".to_string(),
            user_decision: "none".to_string(),
            waiting: Vec::new(),
            shown: Vec::new(),
        }
    }
}

impl Notifications {
    /// Remove what `id` names from both lists; whether it named anything.
    fn withdraw(&mut self, id: &str) -> bool {
        let before = self.waiting.len() + self.shown.len();
        self.waiting.retain(|n| n.id != id);
        self.shown.retain(|n| n.id != id);
        self.waiting.len() + self.shown.len() != before
    }
}

fn state(host: &ReferenceHost<impl Transport>) -> Outcome {
    record(json!({ "state": host.notifications.permission }))
}

pub(crate) fn permission<T: Transport>(host: &mut ReferenceHost<T>) -> Outcome {
    state(host)
}

pub(crate) fn request_permission<T: Transport>(host: &mut ReferenceHost<T>) -> Outcome {
    let notifications = &mut host.notifications;
    if notifications.permission == "undetermined" {
        // A host with no user to ask answers denied.
        notifications.permission = match notifications.user_decision.as_str() {
            "granted" => "granted",
            _ => "denied",
        }
        .to_string();
    }
    state(host)
}

pub(crate) fn post<T: Transport>(
    host: &mut ReferenceHost<T>,
    id: &str,
    title: &str,
    body: &str,
    at_ms: &str,
) -> Outcome {
    if host.notifications.permission != "granted" {
        return record(json!({ "status": "denied" }));
    }
    let at_ms: i64 = at_ms.parse().unwrap_or(0);
    let Some(now) = utc_now(host) else {
        return failure("sce-host-notify-post", "unavailable");
    };
    let notifications = &mut host.notifications;
    notifications.withdraw(id);
    let notification = Notification {
        id: id.to_string(),
        title: title.to_string(),
        body: body.to_string(),
    };
    if at_ms > now {
        notifications.waiting.push(notification);
        record(json!({ "status": "scheduled" }))
    } else {
        notifications.shown.push(notification);
        record(json!({ "status": "posted" }))
    }
}

pub(crate) fn cancel<T: Transport>(host: &mut ReferenceHost<T>, id: &str) -> Outcome {
    let existed = host.notifications.withdraw(id);
    record(json!({ "existed": existed }))
}
