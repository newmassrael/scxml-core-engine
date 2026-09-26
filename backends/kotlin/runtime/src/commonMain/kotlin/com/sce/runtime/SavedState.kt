// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

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
 * machine's own included — not only the ones a snapshot publishes.
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
 */
class SavedState(
    val shape: String,
    val configuration: List<String>,
    val current: String,
    val variables: Map<String, Any?>,
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
        )
    )

    override fun equals(other: Any?): Boolean =
        other is SavedState && other.shape == shape && other.configuration == configuration &&
            other.current == current && other.variables == variables

    override fun hashCode(): Int = listOf(shape, configuration, current, variables).hashCode()

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
            return SavedState(
                shape = text(field("shape"), "shape"),
                configuration = configuration,
                current = text(field("current"), "current"),
                variables = variables,
            )
        }

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
