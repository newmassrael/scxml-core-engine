// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What `include/sce_peer.h` (backends/rust/peer-capi) declares, for JNA: every function, every
// number and the two kinds of function a host gives the library, once. A `uint8_t *` is a byte
// array, a `size_t` a `NativeLong` (a C long, which is a `size_t` wherever a JVM runs this core), a
// `uint64_t` a `Long` whose bits are the number's (`UINT64_MAX` is -1), a handle a `Pointer`. The
// classes of `PeerCore.kt` are what a host calls; these are what they call.

package com.sce.peer

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.NativeLong
import com.sun.jna.Pointer
import com.sun.jna.Structure
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.LongByReference

/** The numbers of `include/sce_peer.h`, by the names it gives them without `SCE_PEER_`. */
object ScePeerAbi {
    /** `SCE_PEER_ABI_VERSION`: the ABI this binding declares. */
    const val VERSION = 3

    const val OK = 0
    const val ERR_NOTHING_ASKED = 1
    const val ERR_OUTCOME_NOT_TAKEN = 2
    const val ERR_NO_GROUP_DEVICE = 3
    const val ERR_GROUP = 4
    const val ERR_UNREADABLE = 5
    const val ERR_NO_ADMINISTRATOR = 6
    const val ERR_NO_PERSON = 7
    const val ERR_OPENED = 8
    const val ERR_LOG = 9
    const val ERR_NOT_A_MEMBER = 10
    const val ERR_ADMITTED_THROUGH_GROUP = 11
    const val ERR_NO_KEY_PACKAGE = 12
    const val ERR_DRAWN_KEYS = 13
    const val ERR_NOT_OPENED = 14
    const val ERR_NO_RANDOMNESS = 15
    const val ERR_NULL_ARGUMENT = 20
    const val ERR_BAD_TEXT = 21
    const val ERR_PANIC = 22
    const val ERR_BAD_ARGUMENT = 23
    const val ERR_CANCELLED = 24
    const val ERR_DEADLINE = 25

    /** What a host's transport says of one exchange (`ScePeerExchangeFn`). */
    const val TRANSPORT_ANSWERED = 0
    const val TRANSPORT_LINK = 1
    const val TRANSPORT_UNAUTHORIZED = 2
    const val TRANSPORT_DEADLINE = 3
    const val TRANSPORT_CANCELLED = 4

    /** What the core did with a send of the machine (`sce_peer_outbound`). */
    const val KEPT_TAKEN = 0
    const val KEPT_HELD = 1
    const val KEPT_NOT_SIGNED = 2
    const val KEPT_REFUSED = 3

    /** The kinds of round (`sce_peer_start`). */
    const val ROUND_SYNC = 0
    const val ROUND_RESYNC = 1
    const val ROUND_REREAD = 2

    /** Whether the hub of a list `sce_peer_open_list` opens is trusted. */
    const val PROFILE_UNTRUSTED = 0
    const val PROFILE_TRUSTED_HUB = 1

    /** The roles a grant gives. */
    const val ROLE_READER = 1
    const val ROLE_WRITER = 2
    const val ROLE_OWNER = 3

    /** `UINT64_MAX` as `sce_peer_run` takes it: no deadline. */
    const val NO_DEADLINE = -1L

    /**
     * The library at [path] (`libsce_peer_capi.so`, `.dylib` or `sce_peer_capi.dll`), its texts in
     * UTF-8, refused when it is another version of the ABI than this binding declares.
     */
    fun load(path: String): ScePeerLib {
        val lib = Native.load(path, ScePeerLib::class.java, mapOf(Library.OPTION_STRING_ENCODING to "UTF-8"))
        val version = lib.sce_peer_abi_version()
        check(version == VERSION) { "$path is version $version of the peer core's ABI, and this binding declares $VERSION" }
        return lib
    }
}

/** `ScePeerBuf`: a buffer the library made, given back with `sce_peer_buf_free`. */
@Structure.FieldOrder("ptr", "len")
open class Buf : Structure() {
    @JvmField var ptr: Pointer? = null

    @JvmField var len: NativeLong = NativeLong(0)
}

/** The same buffer passed by value, as `sce_peer_buf_free` takes it. */
class BufValue : Buf(), Structure.ByValue

/**
 * `ScePeerExchangeFn`: a host's transport. It carries the `len` bytes at `request` (UTF-8, not
 * NUL-terminated, valid for the call), gives the response with `sce_peer_reply` on `reply`, and
 * returns a `TRANSPORT_*` number; it may take `timeoutMs` (-1: no deadline).
 */
interface ExchangeFn : Callback {
    fun callback(context: Pointer?, request: Pointer?, len: NativeLong, timeoutMs: Long, reply: Pointer?): Int
}

/** `ScePeerReleaseFn`: called once with the host's context when the link that holds it is freed. */
interface ReleaseFn : Callback {
    fun callback(context: Pointer?)
}

/** Every function `include/sce_peer.h` declares. */
@Suppress("FunctionName")
interface ScePeerLib : Library {
    fun sce_peer_abi_version(): Int
    fun sce_peer_last_error(): String?
    fun sce_peer_buf_free(buf: BufValue)
    fun sce_peer_open(list: String): Pointer?
    fun sce_peer_invited(list: String, genesis: ByteArray): Pointer?
    fun sce_peer_free(peer: Pointer)
    fun sce_peer_set_identity(peer: Pointer, node: Long, seed: ByteArray): Int
    fun sce_peer_set_administrator(peer: Pointer, seed: ByteArray): Int
    fun sce_peer_for_a_hub(peer: Pointer): Int
    fun sce_peer_with_group(peer: Pointer, node: Long, seed: ByteArray): Int
    fun sce_peer_open_list(peer: Pointer, profile: Int, genesis: ByteArray): Int
    fun sce_peer_grant(peer: Pointer, node: Long, role: Int, key: ByteArray): Int
    fun sce_peer_add_member(peer: Pointer, node: Long, role: Int, key: ByteArray): Int
    fun sce_peer_remove_member(peer: Pointer, node: Long): Int
    fun sce_peer_standing(peer: Pointer, out: Buf): Int
    fun sce_peer_outbound(peer: Pointer, event: String, data: String, kept: IntByReference, reason: Buf): Int
    fun sce_peer_start(peer: Pointer, kind: Int): Int
    fun sce_peer_publish_checkpoint(peer: Pointer, before: Long, digest: Int): Int
    fun sce_peer_publish_key_package(peer: Pointer): Int
    fun sce_peer_poll_request(peer: Pointer, out: Buf): Int
    fun sce_peer_respond(peer: Pointer, response: String): Int
    fun sce_peer_fail(peer: Pointer, reason: String): Int
    fun sce_peer_take_outcome(peer: Pointer, out: Buf): Int
    fun sce_peer_floor_event(peer: Pointer, before: Long, digest: Int, out: Buf): Int
    fun sce_peer_save_sealed(peer: Pointer, state: Buf, key: ByteArray): Int
    fun sce_peer_save_sealed_under(peer: Pointer, key: ByteArray, state: Buf): Int
    fun sce_peer_restore_sealed(
        peer: Pointer,
        node: Long,
        state: ByteArray,
        len: NativeLong,
        key: ByteArray,
        kept: LongByReference?,
    ): Int
    fun sce_peer_link_wss(base: String, token: String): Pointer?
    fun sce_peer_link_host(exchange: ExchangeFn, release: ReleaseFn?, context: Pointer?): Pointer?
    fun sce_peer_link_free(link: Pointer)
    fun sce_peer_link_network_changed(link: Pointer): Int
    fun sce_peer_link_take_notices(link: Pointer, out: Buf): Int
    fun sce_peer_reply(reply: Pointer, text: ByteArray, len: NativeLong): Int
    fun sce_peer_cancel_new(): Pointer?
    fun sce_peer_cancel_raise(cancel: Pointer): Int
    fun sce_peer_cancel_free(cancel: Pointer)
    fun sce_peer_run(peer: Pointer, link: Pointer, withinMs: Long, cancel: Pointer?): Int
}
