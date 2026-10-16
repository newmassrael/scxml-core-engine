// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

/** One notification the host holds. What was posted is held as it was given: the host shows text as written. */
internal data class Notification(val id: String, val title: String, val body: String)

/** The platform's tray, as two lists. */
internal class Notifications {
    var permission: String = "undetermined"
    var userDecision: String = "none"

    /** Kept to be shown at their instant, in the order posted. */
    var waiting: MutableList<Notification> = mutableListOf()

    /** Handed to the platform, in the order posted. */
    var shown: MutableList<Notification> = mutableListOf()

    /** Remove what [id] names from both lists; whether it named anything. */
    fun withdraw(id: String): Boolean {
        val before = waiting.size + shown.size
        waiting.removeAll { it.id == id }
        shown.removeAll { it.id == id }
        return waiting.size + shown.size != before
    }
}

internal fun ReferenceHost.notifyPermission(): Outcome = record("state" to notifications.permission)

internal fun ReferenceHost.notifyRequestPermission(): Outcome {
    if (notifications.permission == "undetermined") {
        // A host with no user to ask answers denied.
        notifications.permission = if (notifications.userDecision == "granted") "granted" else "denied"
    }
    return notifyPermission()
}

internal fun ReferenceHost.notifyPost(id: String, title: String, body: String, atMs: String): Outcome {
    if (notifications.permission != "granted") return record("status" to "denied")
    val at = atMs.toLongOrNull() ?: 0L
    val reading = clock.read() ?: return failure("sce-host-notify-post", "unavailable")
    notifications.withdraw(id)
    val notification = Notification(id, title, body)
    return if (at > reading.utcMs) {
        notifications.waiting.add(notification)
        record("status" to "scheduled")
    } else {
        notifications.shown.add(notification)
        record("status" to "posted")
    }
}

internal fun ReferenceHost.notifyCancel(id: String): Outcome = record("existed" to notifications.withdraw(id))
