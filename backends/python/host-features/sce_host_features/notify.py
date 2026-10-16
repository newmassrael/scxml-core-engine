# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Notifications (docs/SCE_HOST_FEATURES.md, section 4), kept in memory: the
platform's tray is two lists."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, List

from .records import Outcome, failure, record

if TYPE_CHECKING:
    from .host import ReferenceHost


@dataclass
class Notification:
    """One notification the host holds. What was posted is held as it was given:
    the host shows text as written."""

    id: str
    title: str
    body: str


@dataclass
class Notifications:
    permission: str = "undetermined"
    user_decision: str = "none"
    #: Kept to be shown at their instant, in the order posted.
    waiting: List[Notification] = field(default_factory=list)
    #: Handed to the platform, in the order posted.
    shown: List[Notification] = field(default_factory=list)

    def withdraw(self, notification_id: str) -> bool:
        """Remove what the id names from both lists; whether it named anything."""
        before = len(self.waiting) + len(self.shown)
        self.waiting = [n for n in self.waiting if n.id != notification_id]
        self.shown = [n for n in self.shown if n.id != notification_id]
        return len(self.waiting) + len(self.shown) != before


def permission(host: "ReferenceHost") -> Outcome:
    return record({"state": host.notifications.permission})


def request_permission(host: "ReferenceHost") -> Outcome:
    notifications = host.notifications
    if notifications.permission == "undetermined":
        # A host with no user to ask answers denied.
        notifications.permission = "granted" if notifications.user_decision == "granted" else "denied"
    return permission(host)


def post(host: "ReferenceHost", notification_id: str, title: str, body: str, at_ms: str) -> Outcome:
    notifications = host.notifications
    if notifications.permission != "granted":
        return record({"status": "denied"})
    try:
        at = int(at_ms)
    except ValueError:
        at = 0
    reading = host.clock.read()
    if reading is None:
        return failure("sce-host-notify-post", "unavailable")
    notifications.withdraw(notification_id)
    notification = Notification(notification_id, title, body)
    if at > reading.utc_ms:
        notifications.waiting.append(notification)
        return record({"status": "scheduled"})
    notifications.shown.append(notification)
    return record({"status": "posted"})


def cancel(host: "ReferenceHost", notification_id: str) -> Outcome:
    return record({"existed": host.notifications.withdraw(notification_id)})
