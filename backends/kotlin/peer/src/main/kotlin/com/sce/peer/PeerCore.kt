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

/**
 * What the core did with a send of the machine (`SCE_PEER_KEPT_*`). [WAITING] is a send the device
 * may not sign yet, kept with every send after it: the round that reads what it waits for signs it
 * (the outcome's `taken`), or withdraws it with every send after it when it never will be signed
 * (the outcome's `withdrawn`).
 */
enum class Kept(val code: Int) {
    TAKEN(ScePeerAbi.KEPT_TAKEN),
    HELD(ScePeerAbi.KEPT_HELD),
    NOT_SIGNED(ScePeerAbi.KEPT_NOT_SIGNED),
    REFUSED(ScePeerAbi.KEPT_REFUSED),
    WAITING(ScePeerAbi.KEPT_WAITING),
    ;

    companion object {
        fun of(code: Int): Kept = entries.firstOrNull { it.code == code } ?: throw IllegalStateException("the core kept a send as $code, which is no answer of the ABI")
    }
}

/** What the core did with a send, and why when it refused it or keeps it waiting. */
data class Sent(val kept: Kept, val reason: String?)

/** The highest stamp of the device's own changes, and whether its core has read its own chain back. */
data class OwnStamp(val wallTime: Long, val counter: Long, val readBack: Boolean)

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

        /**
         * The core of the device an invitation's text (as its QR code holds it) invites: the one
         * [invited] makes for its list and genesis (`sce_peer_invited_by`).
         */
        fun invitedBy(lib: ScePeerLib, invitation: String): PeerCore {
            val bytes = invitation.toByteArray(Charsets.UTF_8)
            return PeerCore(lib, made(lib, lib.sce_peer_invited_by(bytes, NativeLong(bytes.size.toLong())), "invited_by"))
        }

        /**
         * What an invitation's text says, as the JSON text of the ABI (`sce_peer_invitation_read`):
         * the relay's address and name, the list, the node, the genesis and the expiry; no secret.
         */
        fun readInvitation(lib: ScePeerLib, invitation: String): String {
            val bytes = invitation.toByteArray(Charsets.UTF_8)
            val out = Buf()
            lib.check(lib.sce_peer_invitation_read(bytes, NativeLong(bytes.size.toLong()), out), "invitation_read")
            return lib.text(out) ?: throw IllegalStateException("the core read nothing of the invitation")
        }

        /**
         * The name of a list to open at a relay that takes a creation with no token
         * (`sce_peer_certified_name`, ADR 0080 section 3): `n1.<salt>.<digest>`, which certifies the
         * administrator's key the 32-byte [administrator] seed makes. [salt] is 1 to 32 bytes of the
         * host's, or `null` for 16 the library draws. The host opens the core with this name ([open]),
         * gives it the same seed ([setAdministrator]) and opens the list ([openList]); its [login]
         * creates the list at such a relay.
         */
        fun certifiedName(lib: ScePeerLib, administrator: ByteArray, salt: ByteArray? = null): String {
            require(administrator.size == 32) { "an administrator's seed is 32 bytes" }
            val out = Buf()
            val length = NativeLong((salt?.size ?: 0).toLong())
            lib.check(lib.sce_peer_certified_name(administrator, salt, length, out), "certified_name")
            return lib.text(out) ?: throw IllegalStateException("the core gave no name")
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

    /**
     * Begins a login at the relay named [audience] (`sce_peer_login`): the name the device was
     * configured with, never the one the relay says. It is run as a round is, over a link that
     * presents no token or one that expired, and its outcome holds `token`, which the host gives the
     * link ([PeerLink.setToken]). At a relay that takes a creation with no token, the login of a list
     * the relay does not hold creates it (one just opened under a [certifiedName], or one that lapsed
     * there), and the next round gives the relay back what the device holds. `ERR_NO_PERSON` when the
     * core has no person.
     */
    fun login(audience: String) = lib.check(lib.sce_peer_login(pointer, audience), "login")

    /**
     * An administrator's invitation of the device of [node], which the log never granted, at the relay
     * of address [relay] named [audience], from [nowMs] for [validMs] (at most a day): the text for a
     * QR code (`sce_peer_invite`, or `sce_peer_invite_under` with a 32-byte [secret] the host gives).
     */
    fun invite(relay: String, audience: String, node: Long, nowMs: Long, validMs: Long, secret: ByteArray? = null): String {
        val out = Buf()
        if (secret == null) {
            lib.check(lib.sce_peer_invite(pointer, relay, audience, node, nowMs, validMs, out), "invite")
        } else {
            require(secret.size == 32) { "a secret is 32 bytes" }
            lib.check(lib.sce_peer_invite_under(pointer, relay, audience, node, nowMs, validMs, secret, out), "invite_under")
        }
        return lib.text(out) ?: throw IllegalStateException("the core made no invitation")
    }

    /**
     * Begins the exchange that joins under the invitation's text (`sce_peer_join`), run as a round
     * is; the fingerprint the person compares with the administrator's device's.
     */
    fun join(invitation: String): String {
        val bytes = invitation.toByteArray(Charsets.UTF_8)
        val fingerprint = Buf()
        lib.check(lib.sce_peer_join(pointer, bytes, NativeLong(bytes.size.toLong()), fingerprint), "join")
        return lib.text(fingerprint) ?: throw IllegalStateException("the core gave no fingerprint")
    }

    /**
     * The devices that joined under this core's invitation of [node], as the JSON array of the ABI
     * (`sce_peer_joined`): each key and fingerprint; `null` when none.
     */
    fun joined(node: Long): String? {
        val out = Buf()
        lib.check(lib.sce_peer_joined(pointer, node, out), "joined")
        return lib.text(out)
    }

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

    /**
     * The change a copy that keeps a journal offered (`record_list.made`'s JSON payload), signed in
     * the call (`sce_peer_made`): the verdict the host gives the copy, [Kept.TAKEN] `accepted`,
     * [Kept.HELD] `held`, [Kept.WAITING] `waiting`, [Kept.REFUSED] `refused`, and why for the last two.
     */
    fun made(data: String): Sent {
        val kept = IntByReference(-1)
        val reason = Buf()
        lib.check(lib.sce_peer_made(pointer, data, kept, reason), "made")
        return Sent(Kept.of(kept.value), lib.text(reason))
    }

    /**
     * The records the host has not said it recorded, as the JSON array of the ABI
     * (`sce_peer_take_records`): what the host appends to its journal before [recorded].
     */
    fun takeRecords(): String {
        val out = Buf()
        lib.check(lib.sce_peer_take_records(pointer, out), "take_records")
        return lib.text(out) ?: throw IllegalStateException("the core gave no records")
    }

    /** The host recorded every record up to [through] (`sce_peer_recorded`): the next round gives them the relay. */
    fun recorded(through: Long) = lib.check(lib.sce_peer_recorded(pointer, through), "recorded")

    /**
     * Gives back the records a journal holds past the sealed state restored, in the form [takeRecords]
     * gives (`sce_peer_restore_records`); how many the core did not hold.
     */
    fun restoreRecords(records: String): Long {
        val kept = LongByReference(-1)
        lib.check(lib.sce_peer_restore_records(pointer, records, kept), "restore_records")
        return kept.value
    }

    /**
     * The history a machine built again is given, as the JSON object of the ABI (`sce_peer_history`),
     * from the cut [after] (a JSON array of `[node, seq]` pairs) on, or the whole of it.
     */
    fun history(after: String? = null): String {
        val out = Buf()
        lib.check(lib.sce_peer_history(pointer, after, out), "history")
        return lib.text(out) ?: throw IllegalStateException("the core gave no history")
    }

    /** The stable cut, as the JSON array of `[node, seq]` pairs of the ABI (`sce_peer_stable_cut`). */
    fun stableCut(): String {
        val out = Buf()
        lib.check(lib.sce_peer_stable_cut(pointer, out), "stable_cut")
        return lib.text(out) ?: throw IllegalStateException("the core gave no cut")
    }

    /** Signs a mark when one is owed (`sce_peer_mark`): its number, 0 when none was. */
    fun mark(): Long {
        val seq = LongByReference(-1)
        lib.check(lib.sce_peer_mark(pointer, seq), "mark")
        return seq.value
    }

    /**
     * The highest stamp of the device's own changes (0 and 0 for none), and whether the core has read
     * its own chain back (`sce_peer_own_stamp`): what the host gives its copy as `record_list.raise`.
     */
    fun ownStamp(): OwnStamp {
        val wallTime = LongByReference(-1)
        val counter = IntByReference(-1)
        val readBack = IntByReference(-1)
        lib.check(lib.sce_peer_own_stamp(pointer, wallTime, counter, readBack), "own_stamp")
        return OwnStamp(wallTime.value, counter.value.toLong() and 0xffffffffL, readBack.value != 0)
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
 * A host's wait on its own clock (`ScePeerPauseFn`): the wait a run over the host's transport takes
 * when the relay refused a request for the rate. A host whose time is not the system's (a simulation)
 * gives one; with none, the run's thread sleeps the wait.
 */
fun interface HostPause {
    /**
     * Waits [waitMs] of the host's time: `true` when it waited all of it, `false` when it returned
     * earlier, as it must once the host raises the run's cancellation.
     */
    fun pause(waitMs: Long): Boolean
}

/** The process's sink of what the library logs (`sce_peer_set_log_sink`). */
object PeerLog {
    /** The sink installed, held so that the JVM does not collect a callback the library calls. */
    @Volatile private var installed: LogFn? = null

    /**
     * Gives [sink] each record at [maxLevel] (`ScePeerAbi.LOG_*`) and the levels before it: its level,
     * the module that decided it and the record as one line. It may be called from any thread, and
     * from several at once. A later call replaces it; [sink] `null` or `LOG_OFF` removes it.
     */
    fun install(lib: ScePeerLib, maxLevel: Int, sink: ((Int, String, String) -> Unit)?) {
        val callback =
            sink?.let {
                object : LogFn {
                    override fun callback(level: Int, target: String?, message: String?, user: Pointer?) {
                        it(level, target ?: "", message ?: "")
                    }
                }
            }
        lib.check(lib.sce_peer_set_log_sink(maxLevel, callback, null), "set_log_sink")
        installed = callback
    }
}

/**
 * A link to the relay that the library holds (`ScePeerLink`). A host's transport is held here as
 * long as the link is, since the library calls it until the link is freed and a callback the JVM
 * collected is one JNA can no longer call.
 */
class PeerLink private constructor(
    lib: ScePeerLib,
    handle: Pointer,
    @Suppress("unused") private val exchange: ExchangeFn?,
    @Suppress("unused") private val pause: PauseFn? = null,
) : Handle(lib, handle, lib::sce_peer_link_free) {
    companion object {
        /**
         * The relay's WebSocket binding at [base], presenting [token] (`sce_peer_link_wss`), or none
         * when it is `null`: a link a device logs in over ([PeerCore.login]) and is then given its
         * token ([setToken]).
         */
        fun wss(lib: ScePeerLib, base: String, token: String? = null): PeerLink {
            val handle = lib.sce_peer_link_wss(base, token) ?: throw ScePeerException(ScePeerAbi.ERR_BAD_ARGUMENT, "link_wss: ${lib.sce_peer_last_error()}")
            return PeerLink(lib, handle, null)
        }

        /**
         * The relay's WebSocket binding at [base], presenting [token] (none when `null`), believing the certificate
         * authorities in [authorities] for a `wss://` relay (`sce_peer_link_wss_believing`): the
         * bytes of a file the application ships (an Android raw resource read whole), one or more
         * certificates in PEM or one in DER; with [alsoPublic], the public authorities too.
         */
        fun wssBelieving(lib: ScePeerLib, base: String, token: String?, authorities: ByteArray, alsoPublic: Boolean = false): PeerLink {
            val roots = if (alsoPublic) ScePeerAbi.ROOTS_AND_PUBLIC else ScePeerAbi.ROOTS_ONLY
            val handle =
                lib.sce_peer_link_wss_believing(base, token, authorities, NativeLong(authorities.size.toLong()), roots)
                    ?: throw ScePeerException(ScePeerAbi.ERR_BAD_ARGUMENT, "link_wss_believing: ${lib.sce_peer_last_error()}")
            return PeerLink(lib, handle, null)
        }

        /**
         * A link whose every exchange [transport] carries (`sce_peer_link_host`). It is called on the
         * thread that runs, and a failure it throws is the link's failure, with its words. A run waits
         * out the relay's refusal for the rate with [pause], on the host's clock
         * (`sce_peer_link_set_pause`), or, with none, by sleeping its thread.
         */
        fun host(lib: ScePeerLib, transport: HostTransport, pause: HostPause? = null): PeerLink {
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
            // The callbacks are held by the link for as long as the library may call them.
            val handle = lib.sce_peer_link_host(exchange, null, null) ?: throw ScePeerException(ScePeerAbi.ERR_NULL_ARGUMENT, "link_host: ${lib.sce_peer_last_error()}")
            val paused =
                pause?.let {
                    object : PauseFn {
                        override fun callback(context: Pointer?, waitMs: Long): Int =
                            try {
                                if (it.pause(waitMs)) ScePeerAbi.PAUSE_WAITED else ScePeerAbi.PAUSE_WOKEN
                            } catch (failure: Exception) {
                                ScePeerAbi.PAUSE_WOKEN
                            }
                    }
                }
            if (paused != null) {
                val status = lib.sce_peer_link_set_pause(handle, paused)
                if (status != ScePeerAbi.OK) {
                    lib.sce_peer_link_free(handle)
                    throw ScePeerException(status, "link_set_pause: ${lib.sce_peer_last_error()}")
                }
            }
            return PeerLink(lib, handle, exchange, paused)
        }
    }

    /** The device's network changed; any thread, at any time. */
    fun networkChanged() = lib.check(lib.sce_peer_link_network_changed(pointer), "link_network_changed")

    /**
     * The token the link presents from its next dial on, `null` for none (`sce_peer_link_set_token`):
     * the one a login's outcome holds. Not while a run uses the link; `ERR_BAD_ARGUMENT` for a link
     * whose transport the host carries itself, whose credentials are the host's.
     */
    fun setToken(token: String?) = lib.check(lib.sce_peer_link_set_token(pointer, token), "link_set_token")

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
