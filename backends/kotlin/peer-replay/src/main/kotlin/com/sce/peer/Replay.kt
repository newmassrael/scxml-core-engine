// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the peer core (backends/rust/peer-capi/tests/fixtures) through the
// C ABI, from Kotlin with JNA: the cores of two devices are made, given the administrator's acts, the
// machine's sends and the rounds' answers, with no relay, and one is sealed and made again from its
// sealed state with no key given again; every text a core gives (each request, each outcome, each
// standing, the genesis, the sealed state, the reason of a refusal) must be the recorded one. The
// data key of a sealed state is wrapped as a host wraps it, with a key of its own (`Wrapper`), and
// unwrapped to restore.
//
// usage: Replay <libsce_peer_capi> <scenario.json>

package com.sce.peer

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.NativeLong
import com.sun.jna.Pointer
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.LongByReference
import java.io.File
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.system.exitProcess
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long

private const val ABI_VERSION = 8
private const val OK = 0
private val KEPT = mapOf("taken" to 0, "held" to 1, "not_signed" to 2, "refused" to 3, "waiting" to 4)
private val KINDS = mapOf("sync" to 0, "resync" to 1, "reread" to 2)
private val PROFILES = mapOf("untrusted" to 0, "trusted_hub" to 1)

// The ABI's declarations (`ScePeerLib`, `Buf`, `BufValue`) are the binding's, `:sce-kotlin-peer`,
// in this package: the replay calls the functions as the header names them, and declares none.

/**
 * How a host keeps the data key of a sealed state: wrapped with a key it holds, beside the state.
 * The wrapping key here is a software AES key; on Android it is a key of the Keystore
 * (`KeyGenParameterSpec` under the `AndroidKeyStore` provider, in StrongBox when the device has
 * one), reached through these same `Cipher` calls, and it never leaves the store. The cipher draws
 * the IV of each wrapping, which is kept with the wrapped key.
 */
private class Wrapper {
    private val key: SecretKey = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()

    /** The data key wrapped: the IV and the wrapped bytes. */
    fun wrap(dataKey: ByteArray): Pair<ByteArray, ByteArray> {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key)
        return cipher.iv to cipher.doFinal(dataKey)
    }

    /** The data key a wrapping holds. */
    fun unwrap(wrapped: Pair<ByteArray, ByteArray>): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, wrapped.first))
        return cipher.doFinal(wrapped.second)
    }
}

private fun unhex(text: String): ByteArray =
    ByteArray(text.length / 2) { text.substring(2 * it, 2 * it + 2).toInt(16).toByte() }

private fun hex(bytes: ByteArray): String =
    bytes.joinToString("") { "%02x".format(it.toInt() and 0xff) }

fun main(args: Array<String>) {
    if (args.size != 2) {
        System.err.println("usage: Replay <libsce_peer_capi> <scenario.json>")
        exitProcess(2)
    }
    val lib = Native.load(args[0], ScePeerLib::class.java, mapOf(Library.OPTION_STRING_ENCODING to "UTF-8"))
    val scenario = Json.parseToJsonElement(File(args[1]).readText()).jsonObject
    var steps = 0

    fun die(what: String): Nothing {
        System.err.println("replay: step $steps: $what (${lib.sce_peer_last_error()})")
        exitProcess(1)
    }

    fun check(status: Int, what: String) {
        if (status != OK) die("$what failed with $status")
    }

    // The bytes of a buffer the library gave, given back; null for an empty buffer.
    fun takenBytes(buf: Buf): ByteArray? {
        val ptr = buf.ptr ?: return null
        val bytes = ptr.getByteArray(0, buf.len.toInt())
        val back = BufValue()
        back.ptr = ptr
        back.len = buf.len
        lib.sce_peer_buf_free(back)
        return bytes
    }

    // The text of a buffer the library gave, given back; null for an empty buffer.
    fun taken(buf: Buf): String? = takenBytes(buf)?.let { String(it, Charsets.UTF_8) }

    fun expect(got: String?, want: String, what: String) {
        val have = got ?: ""
        if (have != want) {
            System.err.println("replay: step $steps: $what is not the recorded one\n  got:  $have\n  want: $want")
            exitProcess(1)
        }
    }

    if (lib.sce_peer_abi_version() != ABI_VERSION) die("the library is another ABI version")
    val list = scenario.getValue("list").jsonPrimitive.content
    val devices = scenario.getValue("devices").jsonArray.map { it.jsonObject }
    val peers = arrayOfNulls<Pointer>(devices.size)
    val wrapper = Wrapper()
    // The data key of each device's last sealed state, as its host keeps it: wrapped.
    val wrapped = arrayOfNulls<Pair<ByteArray, ByteArray>>(devices.size)

    // The core of the device at `at`, given nothing but the list and the genesis.
    fun bare(at: Int): Pointer {
        peers[at]?.let { lib.sce_peer_free(it) }
        val genesis = devices[at]["genesis"]
        val made = if (genesis == null || genesis is JsonNull) {
            lib.sce_peer_open(list)
        } else {
            lib.sce_peer_invited(list, unhex(genesis.jsonPrimitive.content))
        }
        val core = made ?: die("the core is not made")
        peers[at] = core
        return core
    }

    for (element in scenario.getValue("steps").jsonArray) {
        val step: JsonObject = element.jsonObject
        val at = step.getValue("device").jsonPrimitive.int
        if (at < 0 || at >= devices.size) die("a step names no device")
        val op = step.getValue("op").jsonPrimitive.content
        if (op == "restore") {
            // The device started again: a core given nothing but the list and the genesis, and the
            // data key its host unwraps.
            val kept = wrapped[at] ?: die("no sealed state of the device was kept")
            val key = wrapper.unwrap(kept)
            if (hex(key) != step.getValue("key").jsonPrimitive.content) {
                die("the key unwrapped is not the recorded one")
            }
            val state = unhex(step.getValue("state").jsonPrimitive.content)
            val core = bare(at)
            val node = devices[at].getValue("node").jsonPrimitive.long
            val count = LongByReference(-1)
            check(
                lib.sce_peer_restore_sealed(core, node, state, NativeLong(state.size.toLong()), key, count),
                "restore_sealed",
            )
            key.fill(0)
            if (count.value != step.getValue("count").jsonPrimitive.long) {
                die("the restore kept another number of operations than was recorded")
            }
            steps++
            continue
        }
        if (op == "make" || op == "make_invited") {
            val device = devices[at]
            val core = if (op == "make") {
                bare(at)
            } else {
                // The device started from the text of its invitation alone.
                peers[at]?.let { lib.sce_peer_free(it) }
                val invitation = step.getValue("invitation").jsonPrimitive.content.toByteArray(Charsets.UTF_8)
                val made = lib.sce_peer_invited_by(invitation, NativeLong(invitation.size.toLong()))
                    ?: die("the core is not made from the invitation")
                peers[at] = made
                made
            }
            val node = device.getValue("node").jsonPrimitive.long
            val seed = unhex(device.getValue("seed").jsonPrimitive.content)
            check(lib.sce_peer_set_identity(core, node, seed), "set_identity")
            val administrator = device["administrator"]
            if (administrator != null && administrator !is JsonNull) {
                val key = unhex(administrator.jsonPrimitive.content)
                check(lib.sce_peer_set_administrator(core, key), "set_administrator")
            }
            steps++
            continue
        }
        val peer = peers[at] ?: die("the device's core is not made")
        val text = { name: String -> step.getValue(name).jsonPrimitive.content }
        val out = Buf()
        when (op) {
            "open" -> {
                val profile = PROFILES[text("profile")] ?: die("a profile the replay does not know")
                val genesis = ByteArray(32)
                check(lib.sce_peer_open_list(peer, profile, genesis), "open_list")
                expect(hex(genesis), text("genesis"), "the genesis")
            }
            "grant" -> {
                val node = step.getValue("node").jsonPrimitive.long
                val role = step.getValue("role").jsonPrimitive.int
                check(lib.sce_peer_grant(peer, node, role, unhex(text("key"))), "grant")
            }
            "remove" -> {
                val node = step.getValue("node").jsonPrimitive.long
                check(lib.sce_peer_remove_member(peer, node), "remove_member")
            }
            "standing" -> {
                check(lib.sce_peer_standing(peer, out), "standing")
                expect(taken(out), text("text"), "the standing")
            }
            "outbound" -> {
                val kept = IntByReference(-1)
                check(lib.sce_peer_outbound(peer, text("event"), text("data"), kept, out), "outbound")
                if (kept.value != KEPT[text("kept")]) die("the core did not keep the send as recorded")
                expect(taken(out), text("reason"), "the reason of the refusal")
            }
            "start" -> {
                val kind = KINDS[text("kind")] ?: die("a kind of round the replay does not know")
                check(lib.sce_peer_start(peer, kind), "start")
            }
            "login" -> check(lib.sce_peer_login(peer, text("audience")), "login")
            "request" -> {
                check(lib.sce_peer_poll_request(peer, out), "poll_request")
                val request = taken(out) ?: die("the core asks nothing where a request was recorded")
                expect(request, text("text"), "the request")
            }
            "respond" -> check(lib.sce_peer_respond(peer, text("text")), "respond")
            "fail" -> check(lib.sce_peer_fail(peer, text("reason")), "fail")
            "outcome" -> {
                check(lib.sce_peer_poll_request(peer, out), "poll_request")
                if (taken(out) != null) die("the core still asks where the exchange was over")
                val ended = Buf()
                check(lib.sce_peer_take_outcome(peer, ended), "take_outcome")
                val outcome = taken(ended) ?: die("no exchange ended where an outcome was recorded")
                expect(outcome, text("text"), "the outcome")
            }
            "save" -> {
                val key = unhex(text("key"))
                check(lib.sce_peer_save_sealed_under(peer, key, out), "save_sealed_under")
                val state = takenBytes(out) ?: die("the sealed state is empty")
                expect(hex(state), text("state"), "the sealed state")
                wrapped[at] = wrapper.wrap(key)
                key.fill(0)
            }
            "invite" -> {
                val node = step.getValue("node").jsonPrimitive.long
                val nowMs = step.getValue("now_ms").jsonPrimitive.long
                val validMs = step.getValue("valid_ms").jsonPrimitive.long
                val secret = unhex(text("secret"))
                check(
                    lib.sce_peer_invite_under(peer, text("relay"), text("audience"), node, nowMs, validMs, secret, out),
                    "invite_under",
                )
                expect(taken(out), text("text"), "the invitation")
            }
            "invitation" -> {
                val invitation = text("invitation").toByteArray(Charsets.UTF_8)
                check(lib.sce_peer_invitation_read(invitation, NativeLong(invitation.size.toLong()), out), "invitation_read")
                expect(taken(out), text("text"), "what the invitation says")
            }
            "join" -> {
                val invitation = text("invitation").toByteArray(Charsets.UTF_8)
                check(lib.sce_peer_join(peer, invitation, NativeLong(invitation.size.toLong()), out), "join")
                expect(taken(out), text("fingerprint"), "the fingerprint")
            }
            "joined" -> {
                val node = step.getValue("node").jsonPrimitive.long
                check(lib.sce_peer_joined(peer, node, out), "joined")
                expect(taken(out), text("text"), "the devices that joined")
            }
            "made" -> {
                val kept = IntByReference(-1)
                check(lib.sce_peer_made(peer, text("data"), kept, out), "made")
                if (kept.value != KEPT[text("kept")]) die("the core did not give the verdict recorded")
                expect(taken(out), text("reason"), "the reason of the refusal or of the wait")
            }
            "records" -> {
                check(lib.sce_peer_take_records(peer, out), "take_records")
                expect(taken(out), text("text"), "the records")
            }
            "recorded" -> check(lib.sce_peer_recorded(peer, step.getValue("through").jsonPrimitive.long), "recorded")
            "restore_records" -> {
                val count = LongByReference(-1)
                check(lib.sce_peer_restore_records(peer, text("records"), count), "restore_records")
                if (count.value != step.getValue("count").jsonPrimitive.long) {
                    die("the records given back held another number of new ones than was recorded")
                }
            }
            "history" -> {
                check(lib.sce_peer_history(peer, null, out), "history")
                expect(taken(out), text("text"), "the history")
            }
            "stable_cut" -> {
                check(lib.sce_peer_stable_cut(peer, out), "stable_cut")
                expect(taken(out), text("text"), "the stable cut")
            }
            "fold" -> {
                val parts = step.getValue("parts").jsonPrimitive.int
                check(lib.sce_peer_fold(peer, parts, out), "fold")
                expect(taken(out), text("text"), "the fold")
                // The binding's own call, on a core of the list that holds no part: the fold the
                // ABI gives for none, no restore and no cut.
                val none = PeerCore.open(lib, list).use { core -> Json.parseToJsonElement(core.fold(parts)).jsonObject }
                if (none.getValue("restores").jsonArray.isNotEmpty() || none.getValue("cut").jsonArray.isNotEmpty()) {
                    die("the binding's fold of a core that holds no part is not empty: $none")
                }
            }
            "mark" -> {
                val seq = LongByReference(-1)
                check(lib.sce_peer_mark(peer, seq), "mark")
                if (seq.value != step.getValue("seq").jsonPrimitive.long) die("the core signed another mark than was recorded")
            }
            "own_stamp" -> {
                val wallTime = LongByReference(-1)
                val counter = IntByReference(-1)
                val readBack = IntByReference(-1)
                check(lib.sce_peer_own_stamp(peer, wallTime, counter, readBack), "own_stamp")
                val got = Triple(wallTime.value, counter.value.toLong() and 0xffffffffL, readBack.value)
                val want = Triple(
                    step.getValue("wall_time").jsonPrimitive.long,
                    step.getValue("counter").jsonPrimitive.long,
                    step.getValue("read_back").jsonPrimitive.int,
                )
                if (got != want) die("the core's own stamp $got is not the recorded one")
            }
            else -> die("a step the replay does not know: $op")
        }
        steps++
    }
    peers.filterNotNull().forEach { lib.sce_peer_free(it) }
    println("replayed $steps steps")
    exitProcess(0)
}
