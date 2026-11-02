// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the peer core (backends/rust/peer-capi/tests/fixtures) through the
// C ABI, from Kotlin with JNA: the cores of two devices are made, given the administrator's acts, the
// machine's sends, the rounds' answers and a save, with no relay, and every text a core gives (each
// request, each outcome, each standing, the genesis, the save, the reason of a refusal) must be the
// recorded one.
//
// usage: Replay <libsce_peer_capi> <scenario.json>

package com.sce.peer

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.NativeLong
import com.sun.jna.Pointer
import com.sun.jna.Structure
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.LongByReference
import java.io.File
import kotlin.system.exitProcess
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long

private const val ABI_VERSION = 2
private const val OK = 0
private val KEPT = mapOf("taken" to 0, "held" to 1, "not_signed" to 2, "refused" to 3)
private val KINDS = mapOf("sync" to 0, "resync" to 1, "reread" to 2)
private val PROFILES = mapOf("untrusted" to 0, "trusted_hub" to 1)

/** `ScePeerBuf` of `include/sce_peer.h`: a buffer the library made (a `size_t` is a C long here). */
@Structure.FieldOrder("ptr", "len")
open class Buf : Structure() {
    @JvmField var ptr: Pointer? = null
    @JvmField var len: NativeLong = NativeLong(0)
}

/** The same buffer passed by value, as `sce_peer_buf_free` takes it. */
class BufValue : Buf(), Structure.ByValue

/** What `include/sce_peer.h` declares of the calls a replay makes (a `uint8_t *` is a byte array). */
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
    fun sce_peer_open_list(peer: Pointer, profile: Int, genesis: ByteArray): Int
    fun sce_peer_grant(peer: Pointer, node: Long, role: Int, key: ByteArray): Int
    fun sce_peer_remove_member(peer: Pointer, node: Long): Int
    fun sce_peer_standing(peer: Pointer, out: Buf): Int
    fun sce_peer_outbound(peer: Pointer, event: String, data: String, kept: IntByReference, reason: Buf): Int
    fun sce_peer_start(peer: Pointer, kind: Int): Int
    fun sce_peer_poll_request(peer: Pointer, out: Buf): Int
    fun sce_peer_respond(peer: Pointer, response: String): Int
    fun sce_peer_fail(peer: Pointer, reason: String): Int
    fun sce_peer_take_outcome(peer: Pointer, out: Buf): Int
    fun sce_peer_save(peer: Pointer, out: Buf): Int
    fun sce_peer_restore(peer: Pointer, saved: String, kept: LongByReference): Int
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

    // The text of a buffer the library gave, given back; null for an empty buffer.
    fun taken(buf: Buf): String? {
        val ptr = buf.ptr ?: return null
        val bytes = ptr.getByteArray(0, buf.len.toInt())
        val back = BufValue()
        back.ptr = ptr
        back.len = buf.len
        lib.sce_peer_buf_free(back)
        return String(bytes, Charsets.UTF_8)
    }

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

    for (element in scenario.getValue("steps").jsonArray) {
        val step: JsonObject = element.jsonObject
        val at = step.getValue("device").jsonPrimitive.int
        if (at < 0 || at >= devices.size) die("a step names no device")
        val op = step.getValue("op").jsonPrimitive.content
        if (op == "make") {
            val device = devices[at]
            peers[at]?.let { lib.sce_peer_free(it) }
            val genesis = device["genesis"]
            val made = if (genesis == null || genesis is JsonNull) {
                lib.sce_peer_open(list)
            } else {
                lib.sce_peer_invited(list, unhex(genesis.jsonPrimitive.content))
            }
            val core = made ?: die("the core is not made")
            peers[at] = core
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
            "restore" -> {
                val count = LongByReference(-1)
                check(lib.sce_peer_restore(peer, text("text"), count), "restore")
                if (count.value != step.getValue("count").jsonPrimitive.long) {
                    die("the restore kept another number of operations than was recorded")
                }
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
                check(lib.sce_peer_save(peer, out), "save")
                expect(taken(out), text("text"), "the save")
            }
            else -> die("a step the replay does not know: $op")
        }
        steps++
    }
    peers.filterNotNull().forEach { lib.sce_peer_free(it) }
    println("replayed $steps steps")
    exitProcess(0)
}
