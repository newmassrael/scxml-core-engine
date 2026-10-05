// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import kotlin.time.Clock
import kotlin.time.ExperimentalTime

/** Why a saved state cannot be restored, or a machine cannot be saved. */
class StateRefusal(message: String) : Exception(message)

/**
 * A `datamodel="sce-static"` machine's whole state, saved at a macrostep
 * boundary and restored into a new process (SCE Accepted Subset §2.15, E17).
 *
 * A host whose process can be killed — an Android app the system reclaims —
 * saves the machine and gives it back later. What it saves is everything a
 * macrostep boundary holds that the document cannot recompute: where the
 * machine is (its configuration and current leaf) and every variable, the
 * machine's own included — not only the ones a snapshot publishes — what each
 * `<history>` recorded, the delayed `<send>`s still waiting, and the external
 * queue, in order.
 *
 * A delayed send is saved as the moment it comes due on the host's WALL clock
 * ([SavedSend.due], milliseconds since the Unix epoch), not as a wait. A wait
 * would start again when the process came back, and a timer that ran out while
 * it was dead would be late by exactly as long as it was dead. The engine's own
 * [SceClock] is monotonic and has no epoch, so the host says what time it is on
 * the wall when it saves and when it restores; an entry already due when the
 * machine comes back is armed as due now, in the order it would have fired.
 *
 * The format is one JSON document (`sce-saved-state`, version [FORMAT]), the
 * same on every backend, so what one backend saved another can read. A
 * variable is keyed by its document id and written as its `sce:type` says: a
 * number, except a 64-bit integer, written as a text so no reader that holds
 * numbers as doubles loses its low bits; a byte string as an array of its
 * bytes; a record as an object of its fields; a list as an array.
 *
 * A saved state is bound to the SHAPE of the document it was saved from — a
 * digest the generator computes of every state with its kind and parent and
 * every variable with its type — not to the document's text. A later build
 * whose guards, actions or comments changed restores it; one that renamed,
 * re-typed or moved something a saved state names refuses it.
 *
 * Cross-language sibling: `sce_rust_runtime::saved_state` (Rust).
 *
 * @property shape the shape of the document the machine was generated from.
 * @property configuration the active configuration, as state ids.
 * @property current the current leaf, as a state id — which of a
 *   `<parallel>`'s regions the machine last stood in, which the configuration
 *   alone cannot say.
 * @property variables every variable, by document id, in declaration order;
 *   each value as [Json] holds it.
 * @property history what each `<history>` recorded when its parent was last
 *   exited (§scxml-3.10), as state ids in document order, keyed by the
 *   history's id and ordered by it. A history that has recorded nothing is
 *   absent: a resumed machine takes its default transition, as the saved one
 *   would have.
 * @property pending the delayed `<send>`s still waiting (§scxml-6.2), in the
 *   order they would be delivered: earliest first, entries due at the same
 *   moment in the order they were sent.
 * @property invokes the `<invoke>`s whose child session is running
 *   (§scxml-6.4), by the id the document gives each, in document order. A child
 *   is not saved: a restored machine starts each of these again from its
 *   beginning, under the same id. One whose child has ended is absent — its
 *   `done.invoke` is in the external queue, or already taken — so it is not
 *   started twice.
 * @property hostInvokes the `<invoke>`s a declared host invoker is running
 *   (§scxml-6.4.1), by `(type, id)`. A restored machine starts each again from
 *   the request it was started with, with the deadline it had left; the host
 *   is told it is a restart (`HostInvokeRequest.restarted`).
 * @property hostInvokeToken the token the next host-run start receives. Carried
 *   on, so that a start a restored machine makes is never given a token an
 *   earlier run already handed to a host that may still answer with it.
 * @property autoSendSeq how many ids the machine has generated for a
 *   `<send idlocation>`: the number the last one carried. Carried on, so that an
 *   id a restored machine generates is never one an earlier run already handed
 *   to the document, which may still hold it in a variable.
 * @property external the external queue, front first: events raised to the
 *   machine that it has not yet been driven through. Only the internal queue
 *   is empty at a macrostep boundary, so a state that left these out would
 *   lose them.
 */
class SavedState(
    val shape: String,
    val configuration: List<String>,
    val current: String,
    val variables: Map<String, Any?>,
    val history: Map<String, List<String>> = emptyMap(),
    val pending: List<SavedSend> = emptyList(),
    val invokes: List<String> = emptyList(),
    val hostInvokes: List<SavedHostInvoke> = emptyList(),
    val hostInvokeToken: Long = 0L,
    val autoSendSeq: Long = 0L,
    val external: List<SavedEvent> = emptyList(),
) {
    /** The variable [id], or a refusal naming it. */
    fun variable(id: String): Any? {
        if (!variables.containsKey(id)) throw StateRefusal("the saved state has no variable '$id'")
        return variables[id]
    }

    /** The state as `sce-saved-state` JSON. */
    fun toJson(): String = Json.write(
        linkedMapOf(
            "format" to Json.Number(FORMAT.toString()),
            "shape" to shape,
            "configuration" to configuration,
            "current" to current,
            "variables" to variables,
            "history" to history,
            "pending" to pending.map { it.toJsonValue() },
            "invokes" to invokes,
            "hostinvokes" to hostInvokes.map { it.toJsonValue() },
            "hostinvoketoken" to hostInvokeToken.toString(),
            "sendseq" to autoSendSeq.toString(),
            "external" to external.map { it.toJsonValue() },
        )
    )

    override fun equals(other: Any?): Boolean =
        other is SavedState && other.shape == shape && other.configuration == configuration &&
            other.current == current && other.variables == variables && other.history == history &&
            other.pending == pending && other.invokes == invokes && other.hostInvokes == hostInvokes &&
            other.hostInvokeToken == hostInvokeToken && other.autoSendSeq == autoSendSeq &&
            other.external == external

    override fun hashCode(): Int =
        listOf(
            shape, configuration, current, variables, history, pending, invokes, hostInvokes, hostInvokeToken,
            autoSendSeq, external,
        ).hashCode()

    companion object {
        /** The format version this runtime writes and reads. */
        const val FORMAT: Int = 1

        /**
         * The stability status of the saved-state wire surface, held in
         * lockstep with `x-sce-schema-status` in
         * `schemas/sce-saved-state.v1.schema.json` (`SCE_WIRE_CONTRACTS.md`).
         * A flip to `"stable"` changes both in one commit.
         */
        const val SCHEMA_STATUS: String = "pre-release"

        /**
         * Read `sce-saved-state` JSON. The shape is not judged here — the
         * machine that restores it knows its own.
         */
        fun fromJson(text: String): SavedState {
            val value = try {
                Json.parse(text)
            } catch (e: Json.Error) {
                throw StateRefusal("the saved state is ${e.message}")
            }
            val members = value as? Map<*, *> ?: throw StateRefusal("the saved state is not an object")
            fun field(name: String): Any? {
                if (!members.containsKey(name)) throw StateRefusal("the saved state has no '$name'")
                return members[name]
            }
            val format = field("format")
            if (format != Json.Number(FORMAT.toString())) {
                throw StateRefusal(
                    "the saved state is format ${Json.write(format)}, and this runtime reads format $FORMAT")
            }
            fun text(v: Any?, what: String): String = v as? String ?: throw StateRefusal("'$what' is not a text")
            val configuration = (field("configuration") as? List<*>
                ?: throw StateRefusal("'configuration' is not an array")).map { text(it, "configuration") }
            @Suppress("UNCHECKED_CAST")
            val variables = field("variables") as? Map<String, Any?>
                ?: throw StateRefusal("'variables' is not an object")
            val history = LinkedHashMap<String, List<String>>()
            for ((id, states) in field("history") as? Map<*, *> ?: throw StateRefusal("'history' is not an object")) {
                val list = states as? List<*> ?: throw StateRefusal("'history.$id' is not an array")
                history[id as String] = list.map { text(it, "history.$id") }
            }
            val pending = (field("pending") as? List<*> ?: throw StateRefusal("'pending' is not an array"))
                .mapIndexed { i, item -> SavedSend.fromJsonValue(item, "pending[$i]") }
            val invokes = (field("invokes") as? List<*> ?: throw StateRefusal("'invokes' is not an array"))
                .map { text(it, "invokes") }
            val hostInvokes = (field("hostinvokes") as? List<*> ?: throw StateRefusal("'hostinvokes' is not an array"))
                .mapIndexed { i, item -> SavedHostInvoke.fromJsonValue(item, "hostinvokes[$i]") }
            val hostInvokeToken = readToken(
                field("hostinvoketoken") as? String ?: throw StateRefusal("'hostinvoketoken' is not a text"),
                "hostinvoketoken",
            )
            val autoSendSeq = readToken(
                field("sendseq") as? String ?: throw StateRefusal("'sendseq' is not a text"),
                "sendseq",
            )
            val external = (field("external") as? List<*> ?: throw StateRefusal("'external' is not an array"))
                .mapIndexed { i, item -> SavedEvent.fromJsonValue(item, "external[$i]") }
            return SavedState(
                shape = text(field("shape"), "shape"),
                configuration = configuration,
                current = text(field("current"), "current"),
                variables = variables,
                history = history,
                pending = pending,
                invokes = invokes,
                hostInvokes = hostInvokes,
                hostInvokeToken = hostInvokeToken,
                autoSendSeq = autoSendSeq,
                external = external,
            )
        }

        /**
         * The wall clock now, in milliseconds since the Unix epoch: what a host
         * that has no clock of its own to give `save` and `restore` gives them.
         */
        @OptIn(ExperimentalTime::class)
        fun wallClockMs(): Long = Clock.System.now().toEpochMilliseconds()

        /**
         * Refuse [saved] unless it was saved from a document of this [shape] —
         * judged before any variable is read, so a state saved from another
         * document is refused for that and not for whichever variable it
         * happens to lack.
         */
        fun checkShape(saved: SavedState, shape: String) {
            if (saved.shape != shape) {
                throw StateRefusal(
                    "the saved state is of a document of shape ${saved.shape}, and this machine's is $shape")
            }
        }
    }
}

/**
 * One event of a saved external queue: its name and the `_event` fields a
 * document can read (§scxml-5.10.1). A typed payload is not saved — it is
 * lifted again from [data] when the event is delivered, the one path every
 * other delivery takes.
 */
data class SavedEvent(
    val name: String,
    val data: String = "",
    val type: String = "external",
    val sendId: String = "",
    val origin: String = "",
    val originType: String = "",
    val invokeId: String = "",
    /**
     * The token a host-run invocation's completion or failure was stamped with
     * when `completeHostInvoke` accepted it, `null` for any other event.
     *
     * The engine refuses a host `done.invoke` that carries none when it is
     * dequeued, because it may be a cancelled run's late reply. A completion
     * that was accepted and is still queued has to keep the stamp across a
     * save, or the restored machine would refuse the one answer it was waiting
     * for. Written as it was, not as a guess from the event's name: an event a
     * host raised through the ordinary door and the engine had yet to refuse is
     * one the restored machine must still refuse.
     */
    val hostInvokeToken: Long? = null,
) {
    internal fun toJsonValue(): Map<String, Any?> = linkedMapOf(
        "name" to name,
        "data" to data,
        "type" to type,
        "sendid" to sendId,
        "origin" to origin,
        "origintype" to originType,
        "invokeid" to invokeId,
        "hostinvoketoken" to hostInvokeToken?.toString(),
    )

    internal companion object {
        fun fromJsonValue(value: Any?, what: String): SavedEvent {
            val members = value as? Map<*, *> ?: throw StateRefusal("'$what' is not an object")
            fun text(key: String): String {
                if (!members.containsKey(key)) throw StateRefusal("'$what' has no '$key'")
                return members[key] as? String ?: throw StateRefusal("'$what.$key' is not a text")
            }
            if (!members.containsKey("hostinvoketoken")) throw StateRefusal("'$what' has no 'hostinvoketoken'")
            val hostInvokeToken = when (val written = members["hostinvoketoken"]) {
                null -> null
                is String -> readToken(written, "$what.hostinvoketoken")
                else -> throw StateRefusal("'$what.hostinvoketoken' is neither a text nor null")
            }
            return SavedEvent(
                name = text("name"),
                data = text("data"),
                type = text("type"),
                sendId = text("sendid"),
                origin = text("origin"),
                originType = text("origintype"),
                invokeId = text("invokeid"),
                hostInvokeToken = hostInvokeToken,
            )
        }
    }
}

/**
 * A host-run invocation's token, as the text a saved state writes it as: digits,
 * within a signed 64-bit count, as every backend reads one.
 */
private fun readToken(written: String, what: String): Long =
    written.toLongOrNull()?.takeIf { it >= 0 && written.all(Char::isDigit) }
        ?: throw StateRefusal("'$what' ($written) is not a whole number a token can be")

/**
 * A moment, as the text a saved state writes it as — milliseconds since the Unix
 * epoch — which [what] (the member, named in a refusal) holds. A text of digits,
 * as every 64-bit integer is, and one the other backends hold too.
 */
private fun readMoment(written: String, what: String): Long =
    written.toLongOrNull()?.takeIf { it >= 0 && written.all(Char::isDigit) }
        ?: throw StateRefusal("'$what' ($written) is not a whole number of milliseconds")

/**
 * The member `params` of [members], read as an object of arrays of texts, by
 * name; [what] names [members] in a refusal.
 */
private fun readParams(members: Map<*, *>, what: String): Map<String, List<String>> {
    if (!members.containsKey("params")) throw StateRefusal("'$what' has no 'params'")
    val params = members["params"] as? Map<*, *> ?: throw StateRefusal("'$what.params' is not an object")
    val read = LinkedHashMap<String, List<String>>()
    for ((name, values) in params) {
        val list = values as? List<*> ?: throw StateRefusal("'$what.params.$name' is not an array")
        read[name as String] = list.map {
            it as? String ?: throw StateRefusal("'$what.params.$name' holds a value that is not a text")
        }
    }
    return read
}

/** `params` as a saved state writes them: by name, so the text one machine writes is every backend's. */
private fun paramsToJson(params: Map<String, List<String>>): Map<String, Any?> =
    params.entries.sortedBy { it.key }.associateTo(LinkedHashMap()) { it.key to it.value }

/**
 * An `<invoke>` a declared host invoker is running, as it is started again
 * (§scxml-6.4.1): every field is what the request the host was handed carried, so
 * the restarted invocation reads as the one the document began.
 *
 * @property processorType the `type` the `<invoke>` named.
 * @property invokeId the invoke's id, which `done.invoke.<id>` names.
 * @property src `<invoke src>`, empty when the document named none.
 * @property params `<param>` values by name; a repeated name keeps every value in
 *   document order. Without the engine's own deadline parameter, which [due] holds.
 * @property data the namelist and `<param>` pairs as JSON, as the request carried them.
 * @property content inline `<content>`, empty when the document carried none.
 * @property due when its deadline comes due, in milliseconds since the Unix epoch
 *   on the wall clock of the host that saved it; `null` for an invocation that has
 *   none. Already past for one that came due while the machine waited to be
 *   restarted.
 */
data class SavedHostInvoke(
    val processorType: String,
    val invokeId: String,
    val src: String = "",
    val params: Map<String, List<String>> = emptyMap(),
    val data: String = "",
    val content: String = "",
    val due: Long? = null,
) {
    internal fun toJsonValue(): Map<String, Any?> = linkedMapOf(
        "type" to processorType,
        "id" to invokeId,
        "src" to src,
        "params" to paramsToJson(params),
        "data" to data,
        "content" to content,
        "due" to due?.toString(),
    )

    internal companion object {
        fun fromJsonValue(value: Any?, what: String): SavedHostInvoke {
            val members = value as? Map<*, *> ?: throw StateRefusal("'$what' is not an object")
            fun text(key: String): String {
                if (!members.containsKey(key)) throw StateRefusal("'$what' has no '$key'")
                return members[key] as? String ?: throw StateRefusal("'$what.$key' is not a text")
            }
            if (!members.containsKey("due")) throw StateRefusal("'$what' has no 'due'")
            val due = when (val written = members["due"]) {
                null -> null
                is String -> readMoment(written, "$what.due")
                else -> throw StateRefusal("'$what.due' is neither a text nor null")
            }
            return SavedHostInvoke(
                processorType = text("type"),
                invokeId = text("id"),
                src = text("src"),
                params = readParams(members, what),
                data = text("data"),
                content = text("content"),
                due = due,
            )
        }
    }
}

/**
 * One delayed `<send>` that has not been delivered yet.
 *
 * @property due when it comes due, in milliseconds since the Unix epoch on the
 *   wall clock of the host that saved it (see [SavedState]).
 * @property act what it does when it comes due.
 */
data class SavedSend(val due: Long, val act: SavedAct) {
    internal fun toJsonValue(): Map<String, Any?> {
        val members = linkedMapOf<String, Any?>("due" to due.toString())
        when (act) {
            is SavedAct.Raise -> members.putAll(
                listOf(
                    "act" to "raise",
                    "event" to act.event,
                    "data" to act.data,
                    "sendid" to act.sendId,
                    "origin" to act.origin,
                )
            )
            is SavedAct.Internal -> members.putAll(
                listOf(
                    "act" to "internal",
                    "event" to act.event,
                    "data" to act.data,
                    "sendid" to act.sendId,
                    "origin" to act.origin,
                )
            )
            is SavedAct.Host -> members.putAll(
                listOf(
                    "act" to "host",
                    "type" to act.processorType,
                    "event" to act.event,
                    "target" to act.target,
                    "content" to act.content,
                    "params" to paramsToJson(act.params),
                    "sendid" to act.sendId,
                    "data" to act.data,
                    "invokeid" to act.invokeId,
                )
            )
        }
        return members
    }

    internal companion object {
        fun fromJsonValue(value: Any?, what: String): SavedSend {
            val members = value as? Map<*, *> ?: throw StateRefusal("'$what' is not an object")
            fun text(key: String): String {
                if (!members.containsKey(key)) throw StateRefusal("'$what' has no '$key'")
                return members[key] as? String ?: throw StateRefusal("'$what.$key' is not a text")
            }
            val due = readMoment(text("due"), "$what.due")
            val act = when (val name = text("act")) {
                "raise" -> SavedAct.Raise(text("event"), text("data"), text("sendid"), text("origin"))
                "internal" -> SavedAct.Internal(text("event"), text("data"), text("sendid"), text("origin"))
                "host" -> SavedAct.Host(
                    processorType = text("type"),
                    event = text("event"),
                    target = text("target"),
                    content = text("content"),
                    params = readParams(members, what),
                    sendId = text("sendid"),
                    data = text("data"),
                    invokeId = text("invokeid"),
                )
                else -> throw StateRefusal("'$what.act' is '$name', which is not raise, internal or host")
            }
            return SavedSend(due, act)
        }
    }
}

/**
 * What a waiting `<send>` does when it comes due. The three the delayed sends
 * of a document that saves can be: its own event on this session's external
 * queue or on its internal queue (`#_internal`), or an act a host-served
 * processor performs (§scxml-6.2.5).
 *
 * A delayed send to a parent, a child or another session is none of them, and
 * a document that makes one is generated without the save API (§2.15): such a
 * send is delivered through a session this state does not carry.
 */
sealed interface SavedAct {
    /** An event for this session's external queue. */
    data class Raise(
        /** The event's name, as the document spells it. */
        val event: String,
        /** `_event.data`, as the wire carries it. */
        val data: String,
        /** The `<send>`'s id, which `_event.sendid` carries and a `<cancel>` names. */
        val sendId: String,
        /** `_event.origin`: the session that sent it. */
        val origin: String,
    ) : SavedAct

    /** An event for this session's internal queue. */
    data class Internal(
        /** The event's name, as the document spells it. */
        val event: String,
        /** `_event.data`, as the wire carries it. */
        val data: String,
        /** The `<send>`'s id, which a `<cancel>` names. */
        val sendId: String,
        /** `_event.origin`; empty on a backend whose internal events carry none. */
        val origin: String,
    ) : SavedAct

    /**
     * A `<send>` a host-served processor performs, as it is performed when it
     * comes due (§scxml-6.2.5): every field is what the document wrote, so a
     * handler sees the request it would have seen had there been no delay.
     */
    data class Host(
        /** The `type` the send named. */
        val processorType: String,
        /** `<send event>`. */
        val event: String,
        /** `<send target>`, empty when the document named none. */
        val target: String,
        /** Inline `<content>`, empty when the document carried none. */
        val content: String,
        /** `<param>` values by name; a repeated name keeps every value in document order. */
        val params: Map<String, List<String>>,
        /** The send's id. */
        val sendId: String,
        /** The event's `_event.data` as a local delivery would carry it. */
        val data: String,
        /** `_event.invokeid` of the event being processed when the `<send>` executed. */
        val invokeId: String,
    ) : SavedAct
}

/**
 * How each `sce:type` is written into a saved state and read back: only if it
 * is one — a value of another kind, or one its width cannot hold, is refused
 * rather than converted. The functions a generated machine's `save` and
 * `restore` call, one per type, named as the type is.
 */
object SavedValues {

    fun of(value: UByte): Any = Json.Number(value.toString())
    fun of(value: UShort): Any = Json.Number(value.toString())
    fun of(value: UInt): Any = Json.Number(value.toString())
    fun of(value: Byte): Any = Json.Number(value.toString())
    fun of(value: Short): Any = Json.Number(value.toString())
    fun of(value: Int): Any = Json.Number(value.toString())

    // A 64-bit integer is written as a text: a reader that holds JSON numbers
    // as doubles would lose its low bits, and a saved state outlives the
    // backend that wrote it.
    fun of(value: ULong): Any = value.toString()
    fun of(value: Long): Any = value.toString()

    // JSON has no spelling for a value that is not finite, so it is written
    // as the text every backend reads it back from.
    fun of(value: Float): Any = if (value.isFinite()) Json.Number(value.toString()) else nonFinite(value.toDouble())
    fun of(value: Double): Any = if (value.isFinite()) Json.Number(value.toString()) else nonFinite(value)

    private fun nonFinite(value: Double): String = when {
        value.isNaN() -> "NaN"
        value > 0 -> "Infinity"
        else -> "-Infinity"
    }

    private fun real(value: Any?, what: String): Double = when (value) {
        "NaN" -> Double.NaN
        "Infinity" -> Double.POSITIVE_INFINITY
        "-Infinity" -> Double.NEGATIVE_INFINITY
        else -> {
            val text = number(value, what)
            text.toDoubleOrNull() ?: throw StateRefusal("'$what' ($text) is not a number")
        }
    }
    fun of(value: Boolean): Any = value
    fun of(value: String): Any = value

    /** A byte string as the array of its bytes, each 0 to 255. */
    fun of(value: ByteArray): Any = value.map { Json.Number((it.toInt() and 0xFF).toString()) }

    /** A list, each element written by [element]. */
    fun <T> list(value: List<T>, element: (T) -> Any?): Any = value.map(element)

    private fun number(value: Any?, what: String): String =
        (value as? Json.Number)?.text ?: throw StateRefusal("'$what' is not a number")

    private fun whole(value: Any?, what: String, min: Long, max: Long): Long {
        val text = number(value, what)
        val n = text.toLongOrNull()
        if (n == null || n < min || n > max) {
            throw StateRefusal("'$what' ($text) is not a whole number its type can hold")
        }
        return n
    }

    fun uint8(value: Any?, what: String): UByte = whole(value, what, 0, UByte.MAX_VALUE.toLong()).toUByte()
    fun uint16(value: Any?, what: String): UShort = whole(value, what, 0, UShort.MAX_VALUE.toLong()).toUShort()
    fun uint32(value: Any?, what: String): UInt = whole(value, what, 0, UInt.MAX_VALUE.toLong()).toUInt()
    fun int8(value: Any?, what: String): Byte =
        whole(value, what, Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong()).toByte()
    fun int16(value: Any?, what: String): Short =
        whole(value, what, Short.MIN_VALUE.toLong(), Short.MAX_VALUE.toLong()).toShort()
    fun int32(value: Any?, what: String): Int =
        whole(value, what, Int.MIN_VALUE.toLong(), Int.MAX_VALUE.toLong()).toInt()

    private fun wideText(value: Any?, what: String): String =
        value as? String ?: throw StateRefusal("'$what' is not a 64-bit whole number written as a text")

    fun uint64(value: Any?, what: String): ULong {
        val text = wideText(value, what)
        return text.toULongOrNull() ?: throw StateRefusal("'$what' ($text) is not a whole number its type can hold")
    }

    fun int64(value: Any?, what: String): Long {
        val text = wideText(value, what)
        return text.toLongOrNull() ?: throw StateRefusal("'$what' ($text) is not a whole number its type can hold")
    }

    fun float32(value: Any?, what: String): Float = real(value, what).toFloat()

    fun float64(value: Any?, what: String): Double = real(value, what)

    fun bool(value: Any?, what: String): Boolean =
        value as? Boolean ?: throw StateRefusal("'$what' is not a truth value")

    fun string(value: Any?, what: String): String = value as? String ?: throw StateRefusal("'$what' is not a text")

    /**
     * A text read back only if it holds no more than the [capacity] UTF-8 bytes
     * the machine bounds it by — a machine never holds more, and a restored one
     * must not be the first to.
     */
    fun string(value: Any?, what: String, capacity: Int): String {
        val text = string(value, what)
        val bytes = text.encodeToByteArray().size
        if (bytes > capacity) {
            throw StateRefusal("'$what' holds $bytes UTF-8 bytes, past the $capacity it is bounded by")
        }
        return text
    }

    /**
     * A list read back element by element, only if it holds no more than the
     * [capacity] the machine bounds it by — a machine never holds more, and a
     * restored one must not be the first to.
     */
    fun <T> list(value: Any?, what: String, capacity: Int, element: (Any?, String) -> T): List<T> {
        val items = value as? List<*> ?: throw StateRefusal("'$what' is not an array")
        if (items.size > capacity) {
            throw StateRefusal("'$what' holds ${items.size} elements, past the $capacity it is bounded by")
        }
        return items.mapIndexed { i, item -> element(item, "$what[$i]") }
    }

    /** A byte string read back from the array of its bytes, held to its [capacity]. */
    fun bytes(value: Any?, what: String, capacity: Int): ByteArray =
        list(value, what, capacity) { item, w -> uint8(item, w).toByte() }.toByteArray()

    /** The field [name] of a saved record [value] — what a generated record reads each field with. */
    fun field(value: Any?, record: String, name: String): Any? {
        val members = value as? Map<*, *> ?: throw StateRefusal("'$record' is not an object")
        if (!members.containsKey(name)) throw StateRefusal("'$record' has no field '$name'")
        return members[name]
    }
}
