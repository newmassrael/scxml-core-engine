// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The handles of the peer core's C ABI as classes a Kotlin host holds: a core (`PeerCore`), a link
// to the relay (`PeerLink`) and the cancellation of a run (`PeerCancel`). Each frees its handle when
// it is closed; a status that is not `OK` is a `ScePeerException` with the library's reason, but for
// the two a run stops with, which `PeerCore.run` returns. What a core gives as text (a standing, an
// outcome, a request) is given as the text, and a sealed state as its bytes, in the forms
// docs/SCE_SHARED_LIST_PEER_ABI.md writes down: the binding reads none of it, so it holds no second
// definition of any of them. Wrapping a sealed state's data key with the platform's key store is
// the host's (the ABI's section 6); the binding hands the key over and keeps no copy.

package com.sce.peer

import com.sun.jna.NativeLong
import com.sun.jna.Pointer
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.LongByReference

/** A status of the library that is not `OK`, with the reason it gave (`sce_peer_last_error`). */
class ScePeerException(val status: Int, message: String) : RuntimeException(message)

/** What the core did with a send of the machine (`SCE_PEER_KEPT_*`). */
enum class Kept(val code: Int) {
    TAKEN(ScePeerAbi.KEPT_TAKEN),
    HELD(ScePeerAbi.KEPT_HELD),
    NOT_SIGNED(ScePeerAbi.KEPT_NOT_SIGNED),
    REFUSED(ScePeerAbi.KEPT_REFUSED),
    ;

    companion object {
        fun of(code: Int): Kept = entries.firstOrNull { it.code == code } ?: throw IllegalStateException("the core kept a send as $code, which is no answer of the ABI")
    }
}

/** What the core did with a send, and why when it refused it. */
data class Sent(val kept: Kept, val reason: String?)

/** A device's sealed state, and the data key it was sealed under, for the host to wrap. */
class Sealed(val state: ByteArray, val key: ByteArray)

/** The text of a buffer the library gave, given back to it; `null` for an empty buffer. */
private fun ScePeerLib.taken(buf: Buf): ByteArray? {
    val ptr = buf.ptr ?: return null
    val bytes = ptr.getByteArray(0, buf.len.toInt())
    val back = BufValue()
    back.ptr = ptr
    back.len = buf.len
    sce_peer_buf_free(back)
    return bytes
}

private fun ScePeerLib.text(buf: Buf): String? = taken(buf)?.toString(Charsets.UTF_8)

private fun ScePeerLib.check(status: Int, what: String) {
    if (status != ScePeerAbi.OK) throw ScePeerException(status, "$what: ${sce_peer_last_error()}")
}

/** A handle the library made, freed once by [free] when it is closed. */
abstract class Handle internal constructor(protected val lib: ScePeerLib, handle: Pointer, private val free: (Pointer) -> Unit) : AutoCloseable {
    private var held: Pointer? = handle

    /** The handle, while it is not freed. */
    val pointer: Pointer
        get() = held ?: throw IllegalStateException("the handle was freed")

    override fun close() {
        held?.let(free)
        held = null
    }
}

/** The peer core of one device of one list (`ScePeer`), used by one thread at a time. */
class PeerCore private constructor(lib: ScePeerLib, handle: Pointer) : Handle(lib, handle, lib::sce_peer_free) {
    companion object {
        /** The core of the device of whoever makes the list `list` (`sce_peer_open`). */
        fun open(lib: ScePeerLib, list: String): PeerCore = PeerCore(lib, made(lib, lib.sce_peer_open(list), "open"))

        /** The core of a device invited to `list` with the hash of its genesis (`sce_peer_invited`). */
        fun invited(lib: ScePeerLib, list: String, genesis: ByteArray): PeerCore {
            require(genesis.size == 32) { "a genesis is 32 bytes" }
            return PeerCore(lib, made(lib, lib.sce_peer_invited(list, genesis), "invited"))
        }

        private fun made(lib: ScePeerLib, handle: Pointer?, what: String): Pointer = handle ?: throw ScePeerException(ScePeerAbi.ERR_NULL_ARGUMENT, "$what: ${lib.sce_peer_last_error()}")
    }

    fun setIdentity(node: Long, seed: ByteArray) = lib.check(lib.sce_peer_set_identity(pointer, node, seed), "set_identity")

    fun setAdministrator(seed: ByteArray) = lib.check(lib.sce_peer_set_administrator(pointer, seed), "set_administrator")

    fun forAHub() = lib.check(lib.sce_peer_for_a_hub(pointer), "for_a_hub")

    /**
     * Before the first exchange of a device with no sealed state yet: the core holds a new device
     * of [node] in the list's group, which the sealed state keeps from then on.
     */
    fun withGroup(node: Long, seed: ByteArray) = lib.check(lib.sce_peer_with_group(pointer, node, seed), "with_group")

    /** Opens a new list; the hash of its genesis, which a person is invited with. */
    fun openList(profile: Int): ByteArray {
        val genesis = ByteArray(32)
        lib.check(lib.sce_peer_open_list(pointer, profile, genesis), "open_list")
        return genesis
    }

    fun grant(node: Long, role: Int, key: ByteArray) = lib.check(lib.sce_peer_grant(pointer, node, role, key), "grant")

    fun addMember(node: Long, role: Int, key: ByteArray) = lib.check(lib.sce_peer_add_member(pointer, node, role, key), "add_member")

    fun removeMember(node: Long) = lib.check(lib.sce_peer_remove_member(pointer, node), "remove_member")

    /** What the device stands at, as the JSON text of the ABI's section 8. */
    fun standing(): String {
        val out = Buf()
        lib.check(lib.sce_peer_standing(pointer, out), "standing")
        return lib.text(out) ?: throw IllegalStateException("the core gave no standing")
    }

    /** What the machine sent, as the event's name and its JSON payload, and what the core did with it. */
    fun outbound(event: String, data: String): Sent {
        val kept = IntByReference(-1)
        val reason = Buf()
        lib.check(lib.sce_peer_outbound(pointer, event, data, kept, reason), "outbound")
        return Sent(Kept.of(kept.value), lib.text(reason))
    }

    /** Begins a round of `kind` (`ScePeerAbi.ROUND_*`). */
    fun start(kind: Int) = lib.check(lib.sce_peer_start(pointer, kind), "start")

    fun publishCheckpoint(before: Long, digest: Int) = lib.check(lib.sce_peer_publish_checkpoint(pointer, before, digest), "publish_checkpoint")

    fun publishKeyPackage() = lib.check(lib.sce_peer_publish_key_package(pointer), "publish_key_package")

    /** The next request text for the relay; `null` once the exchange is over. */
    fun pollRequest(): String? {
        val out = Buf()
        lib.check(lib.sce_peer_poll_request(pointer, out), "poll_request")
        return lib.text(out)
    }

    fun respond(response: String) = lib.check(lib.sce_peer_respond(pointer, response), "respond")

    fun fail(reason: String) = lib.check(lib.sce_peer_fail(pointer, reason), "fail")

    /** What the exchange that ended came to, once, as the JSON text of the ABI's section 5. */
    fun takeOutcome(): String? {
        val out = Buf()
        lib.check(lib.sce_peer_take_outcome(pointer, out), "take_outcome")
        return lib.text(out)
    }

    fun floorEvent(before: Long, digest: Int): String? {
        val out = Buf()
        lib.check(lib.sce_peer_floor_event(pointer, before, digest, out), "floor_event")
        return lib.text(out)
    }

    /**
     * What the device keeps, its secrets with the rest, sealed under a data key the library drew
     * for this save alone (`sce_peer_save_sealed`). The host wraps the key with a key of its
     * platform's key store, keeps the wrapped key beside the state, and zeroes the key it was given.
     */
    fun saveSealed(): Sealed {
        val state = Buf()
        val key = ByteArray(32)
        lib.check(lib.sce_peer_save_sealed(pointer, state, key), "save_sealed")
        return Sealed(lib.taken(state) ?: throw IllegalStateException("the core sealed nothing"), key)
    }

    /** The same, under a 32-byte data key the host gives (`sce_peer_save_sealed_under`). */
    fun saveSealedUnder(key: ByteArray): ByteArray {
        require(key.size == 32) { "a data key is 32 bytes" }
        val state = Buf()
        lib.check(lib.sce_peer_save_sealed_under(pointer, key, state), "save_sealed_under")
        return lib.taken(state) ?: throw IllegalStateException("the core sealed nothing")
    }

    /**
     * Takes a sealed state back for the device of [node], under its unwrapped data key, into a core
     * just made for the list and given nothing else (`sce_peer_restore_sealed`); how many operations
     * and checkpoints the log verified. `ERR_NOT_OPENED` when it does not open under that key for this
     * list and node, `ERR_UNREADABLE` when the bytes are no sealed state.
     */
    fun restoreSealed(node: Long, state: ByteArray, key: ByteArray): Long {
        require(key.size == 32) { "a data key is 32 bytes" }
        val kept = LongByReference(-1)
        lib.check(lib.sce_peer_restore_sealed(pointer, node, state, NativeLong(state.size.toLong()), key, kept), "restore_sealed")
        return kept.value
    }

    /**
     * Runs the exchange begun on the core over [link] on this thread, blocking, until it ends,
     * [withinMs] pass (`ScePeerAbi.NO_DEADLINE`: none) or [cancel] is raised (`sce_peer_run`). The
     * status: `OK` when it ended, `ERR_CANCELLED` or `ERR_DEADLINE` when it stopped; either way the
     * outcome is then taken with [takeOutcome]. Any other status is a [ScePeerException].
     */
    fun run(link: PeerLink, withinMs: Long = ScePeerAbi.NO_DEADLINE, cancel: PeerCancel? = null): Int {
        val status = lib.sce_peer_run(pointer, link.pointer, withinMs, cancel?.pointer)
        return when (status) {
            ScePeerAbi.OK, ScePeerAbi.ERR_CANCELLED, ScePeerAbi.ERR_DEADLINE -> status
            else -> throw ScePeerException(status, "run: ${lib.sce_peer_last_error()}")
        }
    }
}

/** What a host's transport gives back for one exchange. */
sealed class Carried {
    /** The relay's response text. */
    data class Answered(val response: String) : Carried()

    /** Why there is none (`ScePeerAbi.TRANSPORT_*`, not `ANSWERED`), and the reason when there is one. */
    data class Failed(val status: Int, val reason: String?) : Carried()
}

/** A transport a host carries itself: one request text out, and what came of it back. */
fun interface HostTransport {
    /** Carries [request] to the relay, taking at most [timeoutMs] (-1: no deadline). */
    fun exchange(request: String, timeoutMs: Long): Carried
}

/**
 * A link to the relay that the library holds (`ScePeerLink`). A host's transport is held here as
 * long as the link is, since the library calls it until the link is freed and a callback the JVM
 * collected is one JNA can no longer call.
 */
class PeerLink private constructor(lib: ScePeerLib, handle: Pointer, @Suppress("unused") private val exchange: ExchangeFn?) : Handle(lib, handle, lib::sce_peer_link_free) {
    companion object {
        /** The relay's WebSocket binding at [base], presenting [token] (`sce_peer_link_wss`). */
        fun wss(lib: ScePeerLib, base: String, token: String): PeerLink {
            val handle = lib.sce_peer_link_wss(base, token) ?: throw ScePeerException(ScePeerAbi.ERR_BAD_ARGUMENT, "link_wss: ${lib.sce_peer_last_error()}")
            return PeerLink(lib, handle, null)
        }

        /**
         * A link whose every exchange [transport] carries (`sce_peer_link_host`). It is called on the
         * thread that runs, and a failure it throws is the link's failure, with its words.
         */
        fun host(lib: ScePeerLib, transport: HostTransport): PeerLink {
            val exchange = object : ExchangeFn {
                override fun callback(context: Pointer?, request: Pointer?, len: NativeLong, timeoutMs: Long, reply: Pointer?): Int {
                    val text = request?.getByteArray(0, len.toInt())?.toString(Charsets.UTF_8) ?: ""
                    val carried =
                        try {
                            transport.exchange(text, timeoutMs)
                        } catch (failure: Exception) {
                            Carried.Failed(ScePeerAbi.TRANSPORT_LINK, failure.toString())
                        }
                    val (status, given) =
                        when (carried) {
                            is Carried.Answered -> ScePeerAbi.TRANSPORT_ANSWERED to carried.response
                            is Carried.Failed -> carried.status to carried.reason
                        }
                    if (given != null && reply != null) {
                        val bytes = given.toByteArray(Charsets.UTF_8)
                        lib.sce_peer_reply(reply, bytes, NativeLong(bytes.size.toLong()))
                    }
                    return status
                }
            }
            // The callback is held by the link for as long as the library may call it.
            val handle = lib.sce_peer_link_host(exchange, null, null) ?: throw ScePeerException(ScePeerAbi.ERR_NULL_ARGUMENT, "link_host: ${lib.sce_peer_last_error()}")
            return PeerLink(lib, handle, exchange)
        }
    }

    /** The device's network changed; any thread, at any time. */
    fun networkChanged() = lib.check(lib.sce_peer_link_network_changed(pointer), "link_network_changed")

    /** What became of the connection and what the relay pushed, as one JSON array; `null` when nothing. */
    fun takeNotices(): String? {
        val out = Buf()
        lib.check(lib.sce_peer_link_take_notices(pointer, out), "link_take_notices")
        return lib.text(out)
    }
}

/** The cancellation of a run (`ScePeerCancel`): raised from any thread, once, and it stays raised. */
class PeerCancel private constructor(lib: ScePeerLib, handle: Pointer) : Handle(lib, handle, lib::sce_peer_cancel_free) {
    companion object {
        fun create(lib: ScePeerLib): PeerCancel = PeerCancel(lib, lib.sce_peer_cancel_new() ?: throw ScePeerException(ScePeerAbi.ERR_PANIC, "cancel_new: ${lib.sce_peer_last_error()}"))
    }

    /** Cancels the run it was given; any thread, at any time. */
    fun raise() = lib.check(lib.sce_peer_cancel_raise(pointer), "cancel_raise")
}
