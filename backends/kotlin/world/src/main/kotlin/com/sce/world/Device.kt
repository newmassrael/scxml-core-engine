// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The device a hosted copy of a shared list is on, when it holds the list through the peer core
// (ADR 0079): the core is the Rust library, reached through its C ABI with the Kotlin binding
// (backends/kotlin/peer), and a round is the library's own driver, `sce_peer_run`, over a link
// whose transport this process carries. This file writes no protocol of the relay, no rule of
// the trust and no reading of what the core gives: it hands the core what the world asks it to
// and hands back the core's texts as they are.
//
// The world owns the network and the relay, so the transport's every exchange is the world's
// to carry: in the middle of a `peer_round`, each request the core puts is said to the world as
// `{"ok":true,"exchange":<request text>}`, and the next line heard is the world's `peer_carried`,
// which says what became of it. The run then goes on, and the round's reply is said when it ends.
// It is still one line for one line, and nothing here reads a clock or waits on anything but the
// next line, so what the device does is a function of the requests, as the machine's steps are.
// When the relay refuses a request for the rate, the run waits on the world's clock, not this
// process's (the link's pause, `sce_peer_link_set_pause`): it says `{"ok":true,"pause":<ms>}`, and
// the world's next line is `peer_paused` once its time reached the end of the wait.
//
// What the world can say of an exchange is what a transport can: the response, the link's
// failure, the deadline passing (the world measures it on its own clock), or the job's time
// ending, which is the cancellation a host raises from the operating system's word that its
// background work must stop (WorkManager's `onStopped`, iOS's expiration handler), followed by
// the transport giving the exchange up, as the ABI's section 11 says a run is stopped.

package com.sce.world

import com.sce.peer.Carried
import com.sce.peer.PeerCancel
import com.sce.peer.PeerCore
import com.sce.peer.PeerLink
import com.sce.peer.ScePeerAbi
import com.sce.peer.ScePeerLib
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonObjectBuilder
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

/** The core of the device a hosted copy is on, made and driven by the `peer_*` requests. */
class HostedDevice(private val lines: Lines) : AutoCloseable {
    companion object {
        /** Whether [op] is a request of the device rather than of the machine. */
        fun serves(op: String): Boolean = op.startsWith("peer_")
    }

    private var lib: ScePeerLib? = null
    private var loadedFrom: String? = null
    private var core: PeerCore? = null
    private var link: PeerLink? = null

    /** The cancellation of the run under way, while one runs. */
    private var running: PeerCancel? = null

    fun reply(op: String, request: JsonObject): JsonObject =
        when (op) {
            "peer_make" -> make(request)
            "peer_outbound" -> outbound(request)
            "peer_made" -> made(request)
            "peer_record" -> record()
            "peer_round" -> round(request)
            "peer_standing" -> ok { put("standing", core().standing()) }
            "peer_carried" -> throw ProtocolError("no exchange of the device's core waits for what was carried")
            "peer_paused" -> throw ProtocolError("no run of the device's core waits on the world's clock")
            else -> throw ProtocolError("unknown op `$op`")
        }

    override fun close() {
        link?.close()
        link = null
        core?.close()
        core = null
    }

    private fun core(): PeerCore = core ?: throw ProtocolError("the device has no core: `peer_make` first")

    private fun ok(fill: JsonObjectBuilder.() -> Unit = {}): JsonObject =
        buildJsonObject {
            put("ok", true)
            fill()
        }

    /**
     * A new core in place of the one held: the library at `library`, the core of a device that
     * makes the list `list` or is invited to it with the hash `genesis` (hex), its person the
     * node `node` with the key the hex `seed` makes, the administrator's key `administrator`
     * (hex) when it holds one, and a new device of the list's group when `group` is true.
     */
    private fun make(request: JsonObject): JsonObject {
        close()
        val path = text(request, "library")
        val loaded = lib ?: ScePeerAbi.load(path).also { lib = it; loadedFrom = path }
        if (loadedFrom != path) throw ProtocolError("the library is loaded from $loadedFrom, not $path")
        val list = text(request, "list")
        val genesis = optionalText(request, "genesis")
        val made = if (genesis == null) PeerCore.open(loaded, list) else PeerCore.invited(loaded, list, unhex(genesis))
        core = made
        val node = number(request, "node")
        val seed = unhex(text(request, "seed"))
        made.setIdentity(node, seed)
        optionalText(request, "administrator")?.let { made.setAdministrator(unhex(it)) }
        if (request["group"]?.jsonPrimitive?.booleanOrNull == true) made.withGroup(node, seed)
        // A run waits out the relay's refusal for the rate on the world's clock, not this process's:
        // the world's relay counts the world's time.
        link = PeerLink.host(loaded, { exchange, _ -> carry(exchange) }) { waitMs -> pause(waitMs) }
        return ok()
    }

    /** What the machine sent, given to the core: what it did with it, and why when it refused. */
    private fun outbound(request: JsonObject): JsonObject {
        val sent = core().outbound(text(request, "event"), text(request, "data"))
        return ok {
            put("kept", sent.kept.name.lowercase())
            put("reason", sent.reason)
        }
    }

    /**
     * The change the copy offered (`data`, `record_list.made`'s payload), given to the core: the
     * verdict it gave, as `kept` names it, and why when it refused it or keeps it waiting.
     */
    private fun made(request: JsonObject): JsonObject {
        val sent = core().made(text(request, "data"))
        return ok {
            put("kept", sent.kept.name.lowercase())
            put("reason", sent.reason)
        }
    }

    /**
     * The host's journal takes every record the core gives, at once, and says so: the records'
     * text, which the world's host keeps nowhere else.
     */
    private fun record(): JsonObject {
        val held = core()
        val records = held.takeRecords()
        val numbers = Json.parseToJsonElement(records).jsonArray.map { it.jsonObject.getValue("seq").jsonPrimitive.content.toLong() }
        numbers.maxOrNull()?.let { held.recorded(it) }
        return ok { put("records", records) }
    }

    /**
     * An exchange of the core begun (`begin`: `sync`, `resync`, `reread`, or `key_package` for
     * the publication of a key package of the group's device) and run over the link by the
     * library, to its end: the status the run returned and the outcome's text.
     */
    private fun round(request: JsonObject): JsonObject {
        val held = core()
        val carrier = link ?: throw ProtocolError("the device has no link")
        val loaded = lib ?: throw ProtocolError("the library is not loaded")
        when (val begin = text(request, "begin")) {
            "sync" -> held.start(ScePeerAbi.ROUND_SYNC)
            "resync" -> held.start(ScePeerAbi.ROUND_RESYNC)
            "reread" -> held.start(ScePeerAbi.ROUND_REREAD)
            "key_package" -> held.publishKeyPackage()
            else -> throw ProtocolError("no exchange begins as `$begin`")
        }
        val status =
            PeerCancel.create(loaded).use { cancel ->
                running = cancel
                try {
                    held.run(carrier, ScePeerAbi.NO_DEADLINE, cancel)
                } finally {
                    running = null
                }
            }
        val outcome = held.takeOutcome() ?: throw ProtocolError("the run ended and no outcome waits")
        return ok {
            put("ran", status)
            put("outcome", outcome)
        }
    }

    /** One exchange of the run, carried by the world: what it said became of [exchange]. */
    private fun carry(exchange: String): Carried {
        lines.say(ok { put("exchange", exchange) }.toString())
        val line = lines.hear() ?: return Carried.Failed(ScePeerAbi.TRANSPORT_LINK, "the world went away in the middle of an exchange")
        val answer =
            try {
                Json.parseToJsonElement(line).jsonObject
            } catch (failure: IllegalArgumentException) {
                return refused("what the world carried is not a JSON object: ${failure.message}")
            }
        if (answer["op"]?.jsonPrimitive?.content != "peer_carried") {
            return refused("an exchange of the device's core waits for `peer_carried`, and the world asked another")
        }
        if (answer.containsKey("response")) return Carried.Answered(text(answer, "response"))
        return when (val failed = text(answer, "failed")) {
            "link" -> Carried.Failed(ScePeerAbi.TRANSPORT_LINK, optionalText(answer, "reason"))
            "deadline" -> Carried.Failed(ScePeerAbi.TRANSPORT_DEADLINE, null)
            "cancelled" -> {
                // The job's time ended: the host raises the run's cancellation, and its
                // transport gives the exchange up.
                running?.raise()
                Carried.Failed(ScePeerAbi.TRANSPORT_CANCELLED, null)
            }
            else -> refused("no exchange is carried as `$failed`")
        }
    }

    /**
     * One wait of the run, on the world's clock: the relay refused a request for the rate, and the
     * run asks again once [waitMs] of the world's time have passed. Said to the world as
     * `{"ok":true,"pause":<ms>}`, and over when it says `peer_paused`; `false` when the world said
     * anything else, or went away, which the run reads as a wait cut short.
     */
    private fun pause(waitMs: Long): Boolean {
        lines.say(ok { put("pause", waitMs) }.toString())
        val line = lines.hear() ?: return false
        val answer =
            try {
                Json.parseToJsonElement(line).jsonObject
            } catch (failure: IllegalArgumentException) {
                refused("what the world said of a wait is not a JSON object: ${failure.message}")
                return false
            }
        if (answer["op"]?.jsonPrimitive?.content != "peer_paused") {
            refused("a wait of the device's core waits for `peer_paused`, and the world asked another")
            return false
        }
        return true
    }

    /**
     * A line in the middle of an exchange that is not what the world owes it: refused, as any
     * request this server cannot honour is, and the exchange failed as the link's. The world
     * that said it ends the run on the refusal.
     */
    private fun refused(why: String): Carried {
        lines.say(buildJsonObject { put("ok", false); put("error", why) }.toString())
        return Carried.Failed(ScePeerAbi.TRANSPORT_LINK, why)
    }

    private fun optionalText(request: JsonObject, name: String): String? = request[name]?.takeUnless { it is JsonNull }?.let { text(request, name) }

    private fun unhex(hex: String): ByteArray {
        if (hex.length % 2 != 0) throw ProtocolError("`$hex` is not hexadecimal bytes")
        return ByteArray(hex.length / 2) { hex.substring(2 * it, 2 * it + 2).toInt(16).toByte() }
    }
}
