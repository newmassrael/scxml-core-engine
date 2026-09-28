// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The requester's half of `<invoke type="sce:mesh-rpc">` (SCE_MESH.md
// §mesh-9.5): the correlation table a request is kept in until its reply, its
// deadline or its cancellation retires it, and the `_event.data` the engine is
// handed when it ends in an error. The Kotlin twin of
// backends/rust/mesh/src/rpc.rs, byte for byte.
//
// The engine already carries the SCXML invocation — started once, cancelled
// when its state exits, ended by exactly one `done.invoke.<id>` or
// `error.invoke.<id>` (§mesh-19). What this adds is what only the router
// knows: which wire id answers which invocation, who may answer it
// (§mesh-14.6), and when it stops waiting.

package com.sce.mesh

import com.sce.generated.rpc_status.RpcStatus
import com.sce.runtime.Json

/** The `<param>` a lowered Mesh request carries its event name in. */
const val MESH_EVENT_PARAM = "_mesh_event"

/** The `<param>` a lowered Mesh request carries its own deadline in, when the document gave one (§mesh-9.5). */
const val MESH_DEADLINE_PARAM = "_mesh_deadline_ms"

/** One request waiting for its answer. */
internal data class Pending(
    /** The SCXML invoke id the answer ends (`done.invoke.<id>`). */
    val invokeId: String,
    /** Which start of that invoke this is (the engine's token). */
    val token: Long,
    /** The peer it was sent to, whose link carries its reply. */
    val target: String,
    /** The machines whose reply may answer it (§mesh-14.6). */
    val responders: List<String>,
    /** The monotonic time it stops waiting, if it has a deadline. */
    val expiresMs: Long?,
)

/** How a reply that names a live request stands against it. */
internal enum class Answer {
    /** No request is waiting on that id: answered, cancelled or expired already, so what arrives now is dropped. */
    UNKNOWN,

    /** The request is waiting and the binding the reply arrived on may answer it. */
    ADMITTED,

    /** The request is waiting but that binding is outside its responder set (§mesh-16.7 row 14); it stays answerable. */
    UNDECLARED,
}

/**
 * Every request this router has sent and not yet retired, keyed by the wire
 * `invoke_id` the requester minted for it, as lowercase hex (§mesh-9.5: not
 * the SCXML invoke id, which never crosses the wire).
 */
internal class Correlation {
    private val pending = mutableMapOf<String, Pending>()

    fun register(wireId: ByteArray, request: Pending) {
        pending[hex(wireId)] = request
    }

    /**
     * Whether a reply for [wireId] that arrived on [peer] may retire it. The
     * binding a reply arrived on identifies its responder, never the
     * envelope's `source`, which the sender writes (§mesh-9.5).
     */
    fun check(wireId: ByteArray, peer: String): Answer {
        val request = pending[hex(wireId)] ?: return Answer.UNKNOWN
        return if (peer in request.responders) Answer.ADMITTED else Answer.UNDECLARED
    }

    /** Whether a request is still waiting on [wireId]. */
    fun isWaiting(wireId: ByteArray): Boolean = hex(wireId) in pending

    /** Retire the request [wireId], returning it if it was still waiting. */
    fun retire(wireId: ByteArray): Pending? = pending.remove(hex(wireId))

    /** Forget the request for the start [token] of [invokeId]: its state exited, and nothing goes on the wire (§mesh-9.5). */
    fun cancel(invokeId: String, token: Long) {
        pending.entries.removeAll { it.value.invokeId == invokeId && it.value.token == token }
    }

    /**
     * Forget every request sent to [peer], whose link is gone, in wire-id
     * order: no reply can come back on it (§mesh-16.7 row 5).
     */
    fun forgetSentTo(peer: String): List<String> {
        val lost = pending.entries.filter { it.value.target == peer }.map { it.key }.sorted()
        lost.forEach { pending.remove(it) }
        return lost
    }

    /**
     * Retire every request whose deadline is at or before [nowMs], in wire-id
     * order — the order the Rust core's map keeps, since lowercase hex sorts
     * as the bytes do.
     */
    fun expire(nowMs: Long): List<Pair<String, Pending>> {
        val expired = pending.entries
            .filter { entry -> entry.value.expiresMs?.let { it <= nowMs } == true }
            .map { it.key }
            .sorted()
        return expired.map { it to pending.remove(it)!! }
    }
}

/**
 * The `_event.data` of an `error.invoke.<id>` (§mesh-10.7.1,
 * `errorName: "invoke"`): [status] by the name `rpc_status.scxml` declares for
 * it, the reply's message as `detail`, the machine that answered as `source` —
 * absent for a deadline the requester synthesised — and the wire `invoke_id`.
 */
fun invokeErrorData(status: RpcStatus, detail: String?, source: String?, wireIdHex: String): String =
    ErrorData("invoke", status.declaredName()).apply {
        detail?.let { text("detail", it) }
        source?.let { text("source", it) }
        text("invoke_id", wireIdHex)
    }.finish()

/**
 * The `_event.data` of the `error.execution` a request that cannot reach the
 * wire raises instead of starting (§mesh-9.5's pre-envelope tier).
 */
fun srcNotFoundData(detail: String): String =
    ErrorData("execution", "INVOKE_SRC_NOT_FOUND").apply { text("detail", detail) }.finish()

/** [bytes] as lowercase hex, the form §mesh-10.7 gives an `invoke_id`. */
fun hex(bytes: ByteArray): String {
    val digits = "0123456789abcdef"
    val out = StringBuilder(bytes.size * 2)
    for (byte in bytes) {
        val value = byte.toInt() and 0xFF
        out.append(digits[value ushr 4]).append(digits[value and 0x0F])
    }
    return out.toString()
}

/**
 * The wire invokeid a request's `_event.invokeid` names: [hex] read back. `null`
 * for text that is not the 32 hex digits of one — an event raised by something
 * other than a Mesh request.
 */
fun unhex(text: String): ByteArray? {
    if (text.length != 32) return null
    val out = ByteArray(16)
    for (i in 0 until 16) {
        val hi = text[2 * i].digitToIntOrNull(16) ?: return null
        val lo = text[2 * i + 1].digitToIntOrNull(16) ?: return null
        out[i] = ((hi shl 4) or lo).toByte()
    }
    return out
}

/**
 * A §mesh-10.7.1 `_event.data` object, written in the order its fields are
 * added: `errorName` and `reason` first, then what the caller adds.
 */
internal class ErrorData(errorName: String, reason: String) {
    private val fields = mutableListOf<String>()

    init {
        text("errorName", errorName)
        text("reason", reason)
    }

    fun text(key: String, value: String) {
        fields += "\"$key\":" + Json.quote(value)
    }

    fun number(key: String, value: Any) {
        fields += "\"$key\":$value"
    }

    fun finish(): String = fields.joinToString(",", "{", "}")
}
