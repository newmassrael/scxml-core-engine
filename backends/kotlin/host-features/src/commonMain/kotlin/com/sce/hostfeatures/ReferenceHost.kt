// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

/** Every operation type of the standard host features, in the order the specification lists them. */
val TYPES: List<String> = listOf(
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
)

/** What a host answers a start with. */
sealed class Outcome {
    /**
     * A completion: [data] is the result record, as the JSON a completion carries. A
     * failure is such a record whose `error` is a code of the operation's closed set.
     */
    data class Done(val data: String) : Outcome()

    /** A host that would not start the operation; [data] is the reason, as the JSON `error.execution` carries. */
    data class Refuse(val data: String) : Outcome()

    /** A host that has started the operation and answers later, or never. */
    object Hold : Outcome()
}

/** A start the host was given: the operation, the invocation's id and the request's parameters. */
data class Start(val operation: String, val id: String, val params: Map<String, String>)

/** The refusal the specification gives an argument the operation does not accept. */
internal const val INVALID_ARGUMENT = """{"reason":"invalid_argument"}"""

/**
 * The standard host features' semantics, over a clock and a transport.
 *
 * It is told the start of an operation (its type and the request's text parameters)
 * and answers with an [Outcome]. It knows no engine, so the same object serves a
 * generated machine through [serve], a test, or another embedding. Time comes from a
 * [ClockSource] and the network from a [Transport], both the embedding's to supply.
 */
class ReferenceHost(internal val clock: ClockSource, internal val transport: Transport) {
    internal val store = Store()
    internal val notifications = Notifications()

    /** Failures to inject: the operation type to the code it fails with. */
    private var faults: Map<String, String> = emptyMap()

    /** The invocations started and not yet answered, by id. */
    internal val held = mutableListOf<String>()
    private val cancelledIds = mutableListOf<String>()

    /** The highest monotonic reading answered, which no answer goes below. */
    internal var monotonicFloor: Long? = null
    private var starts = mutableListOf<Start>()

    /** Inject failures: each operation type named fails with the code it maps to, once the request has been accepted. */
    fun setFaults(faults: Map<String, String>) {
        this.faults = faults
    }

    /** Bound the store to [keys] keys; a write of a new key past it fails `quota_exceeded`. */
    fun setQuota(keys: Int) {
        store.quota = keys
    }

    /** The notification permission, as the platform says it: `granted`, `denied` or `undetermined`. */
    fun setPermission(state: String) {
        notifications.permission = state
    }

    /** What the user chooses when asked: `granted`, `denied`, or `none` for a host with no user to ask. */
    fun setUserDecision(decision: String) {
        notifications.userDecision = decision
    }

    /** The store's keys and values, in key order. */
    fun storeEntries(): List<Pair<String, String>> = store.entries()

    /** The ids of the waiting notifications, in the order posted. */
    fun waiting(): List<String> = notifications.waiting.map { it.id }

    /** The ids of the shown notifications, in the order posted. */
    fun shown(): List<String> = notifications.shown.map { it.id }

    /** The invocations the host was told to stop, in the order told. */
    fun cancelled(): List<String> = cancelledIds.toList()

    /** The starts the host was given since this was last called, oldest first. */
    fun takeStarts(): List<Start> {
        val taken = starts
        starts = mutableListOf()
        return taken
    }

    /**
     * Start the operation of invocation [id] with [params].
     *
     * `null` for a type that is not one of the standard host features: the host serves
     * none of it. Otherwise the arguments are judged first (a refusal starts nothing),
     * then a failure the embedding injected, then the operation itself.
     */
    fun start(operation: String, id: String, params: Map<String, String>): Outcome? {
        if (operation !in TYPES) return null
        starts.add(Start(operation, id, params.toMap()))
        fun argument(name: String): String = params[name] ?: ""
        val refused = when (operation) {
            "sce-host-kv-get", "sce-host-kv-delete", "sce-host-kv-put" -> !validKey(argument("key"))
            "sce-host-notify-post", "sce-host-notify-cancel" -> argument("id").isEmpty()
            "sce-host-http-fetch" -> !acceptable(argument("method"), argument("url"), argument("headers"))
            else -> false
        }
        if (refused) return Outcome.Refuse(INVALID_ARGUMENT)
        faults[operation]?.let { return failure(operation, it) }
        return when (operation) {
            "sce-host-clock-now" -> clockNow()
            "sce-host-clock-zone" -> clockZone()
            "sce-host-kv-get" -> kvGet(argument("key"))
            "sce-host-kv-put" -> kvPut(argument("key"), argument("value"))
            "sce-host-kv-delete" -> kvDelete(argument("key"))
            "sce-host-kv-next-key" -> kvNextKey(argument("after"), argument("prefix"))
            "sce-host-notify-permission" -> notifyPermission()
            "sce-host-notify-request-permission" -> notifyRequestPermission()
            "sce-host-notify-post" ->
                notifyPost(argument("id"), argument("title"), argument("body"), argument("at_ms"))
            "sce-host-notify-cancel" -> notifyCancel(argument("id"))
            else -> httpFetch(id, argument("method"), argument("url"), argument("headers"), argument("body"))
        }
    }

    /**
     * The host is told to stop invocation [id], because its state exited or its deadline
     * passed. What it held for it is dropped.
     */
    fun cancel(id: String) {
        held.removeAll { it == id }
        cancelledIds.add(id)
    }
}

/** A JSON string: quotes, backslashes and control characters escaped, the rest as it is. */
internal fun jsonString(text: String): String {
    val out = StringBuilder("\"")
    for (c in text) {
        when {
            c == '"' -> out.append("\\\"")
            c == '\\' -> out.append("\\\\")
            c == '\n' -> out.append("\\n")
            c == '\r' -> out.append("\\r")
            c == '\t' -> out.append("\\t")
            c < ' ' -> out.append("\\u").append(c.code.toString(16).padStart(4, '0'))
            else -> out.append(c)
        }
    }
    return out.append('"').toString()
}

private fun json(fields: List<Pair<String, Any>>): String =
    fields.joinToString(",", "{", "}") { (name, value) ->
        val written = if (value is String) jsonString(value) else value.toString()
        "${jsonString(name)}:$written"
    }

/**
 * A result record as the JSON a completion carries, its `error` empty. A 64-bit integer
 * is written as the text of its digits, as a payload's reader takes it.
 */
internal fun record(vararg fields: Pair<String, Any>): Outcome =
    Outcome.Done(json(listOf<Pair<String, Any>>("error" to "") + fields))

/**
 * The failure [code] of [operation]: its result record with the fields at their zero and
 * `error` the code, so a machine reads the failure as a typed field.
 */
internal fun failure(operation: String, code: String): Outcome {
    val zero: List<Pair<String, Any>> = when (operation) {
        "sce-host-clock-now" -> listOf("utc_ms" to "0", "monotonic_ms" to "0")
        "sce-host-clock-zone" -> listOf("zone_id" to "", "utc_offset_s" to 0)
        "sce-host-kv-get" -> listOf("found" to false, "value" to "")
        "sce-host-kv-put" -> listOf("replaced" to false)
        "sce-host-kv-delete", "sce-host-notify-cancel" -> listOf("existed" to false)
        "sce-host-kv-next-key" -> listOf("found" to false, "key" to "")
        "sce-host-notify-permission", "sce-host-notify-request-permission" -> listOf("state" to "")
        "sce-host-notify-post" -> listOf("status" to "")
        "sce-host-http-fetch" -> listOf("status" to 0, "headers" to "", "body" to "")
        else -> error("`$operation` is not a standard host operation")
    }
    return Outcome.Done(json(zero + ("error" to code)))
}
