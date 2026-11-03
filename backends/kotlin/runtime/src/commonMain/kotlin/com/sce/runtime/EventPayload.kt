// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Lifting an event's typed `_event.data` view out of the data it carries.
 *
 * NL→IR Item C1 Path A gives a schema'd event a typed payload that the natively
 * lowered guards read. One producer fills it: the generated `raise<Event>`
 * inject seam. Every other producer — `<send>` with `<param>`, namelist or
 * `<content>`, an invoke forwarding an event either way, autoforward,
 * BasicHTTP, mesh — fills [EventMetadata.data], the wire §scxml-5.10
 * describes and §scxml-B-2-8-1 reads.
 *
 * Until this file the two never met, so the same guard answered differently
 * depending on where its event came from, and a typed payload could not cross
 * an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
 * already refuses for the same guard, measured on the same document: no data, a
 * missing field, or a value of another type each give `error.execution` and a
 * guard that does not fire. A native lowering that answered differently would
 * make the optimisation observable, which is the one thing it may not be.
 *
 * A field may be of any type of the language (docs/adr/0086): a list of any
 * type, an optional value, a record, nested to any depth. Each is read as one
 * [Value] at the path that names it, and three combinators walk the
 * constructors that hold other values, each calling the reader of its part:
 * [Value.list], [Value.optional] and [Value.record]. A value outside its type is
 * refused before anything of it is kept, and the refusal names the path
 * through every depth as a JSON Pointer (RFC 6901): `'items/2/tags/0' is 9
 * bytes, past the 8 its type declares`. A key a record's schema does not
 * declare is stepped over.
 *
 * The JSON is read through the runtime's one reader ([Json]); what the inject
 * seam writes goes through its [Json.quote].
 *
 * Cross-language siblings: `sce_runtime.event_payload` (Python),
 * `sce.LiftPayload` (Go), `SCE::EventPayload` (C++).
 */
object EventPayload {

    /** Why an event's data cannot be read as its schema's fields. */
    class Refusal(message: String) : Exception(message)

    /**
     * The largest whole number every engine holds exactly as a number (`2^53 - 1`,
     * ECMAScript's `Number.MAX_SAFE_INTEGER`): the largest a JSON number may carry
     * for a 64-bit field.
     */
    private const val EXACT_IN_A_DOUBLE: Long = (1L shl 53) - 1

    /**
     * Whether [text] is a whole number written as digits and nothing else: no sign
     * but `-` (and only where [signed]), no leading zero, no `-0`.
     */
    private fun isDecimal(text: String, signed: Boolean): Boolean {
        val digits = if (text.startsWith("-")) {
            if (!signed) return false
            text.substring(1)
        } else {
            text
        }
        if (digits.isEmpty() || !digits.all { it in '0'..'9' }) return false
        if (digits != "0" && digits.startsWith("0")) return false
        return !(digits == "0" && text.startsWith("-"))
    }

    /** [segment] below [prefix], as a JSON Pointer's reference tokens are joined. */
    private fun join(prefix: String, segment: String): String =
        if (prefix.isEmpty()) segment else "$prefix/$segment"

    /**
     * One value of an event's data, at the [path] through every depth that names
     * it: a field's name, then each index and field name below it, joined by `/`.
     */
    class Value internal constructor(private val raw: Any?, val path: String) {

        /** Whether the value is `null`, the one spelling of an absent value. */
        val isNull: Boolean get() = raw == null

        /**
         * A refusal of this value, saying [what] of it after its path:
         * `'items/1/level' (x) is not a variant of Level`.
         */
        fun refuse(what: String): Refusal = Refusal("'$path' $what")

        /** The refusal of `null` where the type is not optional, or [otherwise]. */
        private fun wrongKind(otherwise: String): Refusal =
            if (isNull) refuse("is null, and its type is not optional") else refuse(otherwise)

        /**
         * The value as the number it was written as.
         *
         * ⚠ A truth value is not a number here. JSON spells both, and a schema
         * that declared `uint32` and received `true` has been handed something
         * its own type says cannot occur.
         */
        private fun number(): String {
            if (raw !is Json.Number) {
                throw wrongKind("is not a number ($raw)")
            }
            return raw.text
        }

        private fun whole(): Long {
            val text = number()
            return text.toLongOrNull() ?: throw refuse("is not a whole number ($text)")
        }

        private fun <T> narrowed(value: Long, min: Long, max: Long, of: (Long) -> T): T {
            if (value < min || value > max) {
                throw refuse("does not fit the width its schema declares ($value)")
            }
            return of(value)
        }

        fun int8(): Byte = narrowed(whole(), Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong()) { it.toByte() }

        fun int16(): Short = narrowed(whole(), Short.MIN_VALUE.toLong(), Short.MAX_VALUE.toLong()) { it.toShort() }

        fun int32(): Int = narrowed(whole(), Int.MIN_VALUE.toLong(), Int.MAX_VALUE.toLong()) { it.toInt() }

        /**
         * A 64-bit whole number's digits: written as a text of them, or as a JSON
         * number when it is exact in a double (`|n| <= 2^53 - 1`). A 64-bit integer
         * crosses as a text because a reader that takes a JSON number through a
         * double carries one past 2^53 with its low bits wrong and no error; a
         * number past that is refused here on every engine, so the same payload
         * gives the same field wherever it is read.
         */
        private fun wide(signed: Boolean): String =
            when (raw) {
                is String ->
                    if (isDecimal(raw, signed)) raw
                    else throw refuse("is not a whole number written as digits ($raw)")
                is Json.Number -> {
                    val n = raw.text.toLongOrNull() ?: throw refuse("is not a whole number (${raw.text})")
                    if (n > EXACT_IN_A_DOUBLE || n < -EXACT_IN_A_DOUBLE) {
                        throw refuse(
                            "is a number past 2^53 - 1, which is not exact everywhere " +
                                "(${raw.text}); a 64-bit integer that large is written as a " +
                                "text of its digits",
                        )
                    }
                    raw.text
                }
                else -> throw wrongKind("is not a number ($raw)")
            }

        fun int64(): Long {
            val text = wide(true)
            return text.toLongOrNull() ?: throw refuse("does not fit the width its schema declares ($text)")
        }

        fun uint8(): UByte = narrowed(whole(), 0L, UByte.MAX_VALUE.toLong()) { it.toUByte() }

        fun uint16(): UShort = narrowed(whole(), 0L, UShort.MAX_VALUE.toLong()) { it.toUShort() }

        fun uint32(): UInt = narrowed(whole(), 0L, UInt.MAX_VALUE.toLong()) { it.toUInt() }

        /**
         * An unsigned 64-bit value. Its top half has no signed spelling, so the
         * text is read as unsigned rather than through [whole].
         */
        fun uint64(): ULong {
            val text = wide(false)
            return text.toULongOrNull() ?: throw refuse("does not fit the width its schema declares ($text)")
        }

        /**
         * A 32-bit fractional value: the binary32 nearest the number the payload
         * carries. A JSON number is a double wherever it is read, so the single is
         * the one nearest that double, rounded once, and a number past the largest
         * single does not fit, as a whole number past its width does not. Parsing
         * the text to a `Float` directly would round from the text instead, and
         * give infinity for `1e39`.
         */
        fun float32(): Float {
            val text = number()
            val wide = text.toDoubleOrNull() ?: throw refuse("is not a number ($text)")
            if (kotlin.math.abs(wide) > Float.MAX_VALUE.toDouble()) {
                throw refuse("does not fit the width its schema declares ($text)")
            }
            return wide.toFloat()
        }

        fun float64(): Double {
            val text = number()
            return text.toDoubleOrNull() ?: throw refuse("is not a number ($text)")
        }

        fun boolean(): Boolean {
            if (raw !is Boolean) {
                throw wrongKind("is not a truth value ($raw)")
            }
            return raw
        }

        /**
         * The text the value carries, held to no bound: what an enum's variant is
         * named by, which its reader matches against the names its enum declares.
         */
        fun name(): String {
            if (raw !is String) {
                throw wrongKind("is not a text ($raw)")
            }
            return raw
        }

        /**
         * A text of at most [bound] UTF-8 bytes (docs/adr/0086, section 4.4): one
         * past the bound is refused, at whatever depth it stands, and never cut.
         */
        fun string(bound: Int): String {
            val text = name()
            val size = text.encodeToByteArray().size
            if (size > bound) {
                throw pastBytes(size, bound)
            }
            return text
        }

        /**
         * A byte string of at most [bound] bytes.
         *
         * JSON has no byte string, so the wire carries the byte-exact Latin-1 text
         * the inject seam writes, and this reads it back the same way: every one
         * of the 256 values is one character and back. Printable ASCII — what a
         * bytes guard compares — is the same bytes either way.
         */
        fun bytes(bound: Int): ByteArray {
            if (raw !is String) {
                throw wrongKind("is not a byte string ($raw)")
            }
            val out = ByteArray(raw.length)
            for (i in raw.indices) {
                val c = raw[i].code
                if (c > 0xFF) {
                    throw refuse("carries a character above U+00FF, which no single byte spells")
                }
                out[i] = c.toByte()
            }
            if (out.size > bound) {
                throw pastBytes(out.size, bound)
            }
            return out
        }

        private fun pastBytes(held: Int, bound: Int): Refusal =
            refuse("is $held bytes, past the $bound its type declares")

        /**
         * A value of `optional<T>` (docs/adr/0055): absent where the value is
         * `null`, and otherwise what [read] reads of it, held to `T`.
         */
        fun <T : Any> optional(read: (Value) -> T): T? = if (isNull) null else read(this)

        /**
         * A value of `list<T>[bound]`: an array of at most [bound] elements, each
         * read by [read] at the path of its index, in order.
         *
         * The bound is held before any element is read: an array longer than
         * [bound] is refused whole, and no element of it is decoded (ADR 0077,
         * section 2).
         */
        fun <T> list(bound: Int, read: (Value) -> T): List<T> {
            if (raw !is List<*>) {
                throw wrongKind("is not a list")
            }
            if (raw.size > bound) {
                throw refuse("holds ${raw.size} elements, past the $bound its type declares")
            }
            return raw.mapIndexed { index, item -> read(Value(item, join(path, index.toString()))) }
        }

        /**
         * A value of a record type: an object whose fields [read] reads through
         * the [Fields] it is handed. [defaults], for a record whose schema a later
         * revision added fields to, is the JSON object of their defaults, which
         * the generator spells from that schema, and a field the value lacks is
         * read out of it (docs/adr/0086, section 10.3).
         */
        fun <T> record(defaults: String? = null, read: (Fields) -> T): T {
            if (raw !is Map<*, *>) {
                throw wrongKind("is not a record")
            }
            @Suppress("UNCHECKED_CAST")
            val members = raw as Map<String, Any?>
            val fallback = if (defaults == null) emptyMap() else defaultsOf(defaults)
            return read(Fields(fallback + members, path))
        }
    }

    /**
     * The fields an event carries, each value still in the spelling it was
     * written in, so a whole number stays exact until it is read at the width
     * its schema declares. The fields of a record value are read the same way,
     * at the record's path.
     */
    class Fields internal constructor(internal val values: Map<String, Any?>, private val prefix: String = "") {

        /**
         * The value of the field [name], at its path: the delivered one, else
         * the default a later revision of the schema gave it, else a refusal
         * naming the path. A member the schema does not declare is never asked
         * for, so it is stepped over.
         */
        fun field(name: String): Value {
            val path = join(prefix, name)
            if (!values.containsKey(name)) {
                throw Refusal("the event's data has no '$path'")
            }
            return Value(values[name], path)
        }

        fun int8(name: String): Byte = field(name).int8()

        fun int16(name: String): Short = field(name).int16()

        fun int32(name: String): Int = field(name).int32()

        fun int64(name: String): Long = field(name).int64()

        fun uint8(name: String): UByte = field(name).uint8()

        fun uint16(name: String): UShort = field(name).uint16()

        fun uint32(name: String): UInt = field(name).uint32()

        fun uint64(name: String): ULong = field(name).uint64()

        fun float32(name: String): Float = field(name).float32()

        fun float64(name: String): Double = field(name).float64()

        fun boolean(name: String): Boolean = field(name).boolean()

        /** A text field, as the text it carries; [Value.string] holds it to its bound. */
        fun string(name: String): String = field(name).name()

        /** A byte-string field, held to no bound; [Value.bytes] holds it to its own. */
        fun bytes(name: String): ByteArray = field(name).bytes(Int.MAX_VALUE)

        /**
         * A field of `optional<T>` (docs/adr/0055, section 4): absent where the
         * delivery carries `null`, and otherwise the value [read] takes out of the
         * field, held to its type as any field's is. A delivery that lacks the key
         * is refused, as for every field: a missing key says the sender's revision
         * has no such field, and `null` is the one spelling of absence. Where a
         * later revision added the field, [decode] with its defaults already put
         * the default (`null` when it declares none) in the key's place.
         */
        fun <T : Any> optional(name: String, read: (String) -> T): T? =
            if (field(name).isNull) null else read(name)
    }

    /**
     * The fields the event's data names.
     *
     * The JSON read is §scxml-B-2-8-1's second rung, the same one the script
     * engine takes for the same string — through the runtime's one reader.
     */
    fun decode(data: String): Fields {
        val trimmed = data.trim()
        if (trimmed.isEmpty()) {
            throw Refusal("the event carries no data")
        }
        if (!trimmed.startsWith('{')) {
            throw Refusal(
                "the event's data is a bare value, and this event's schema declares named fields")
        }
        val value = try {
            Json.parse(trimmed)
        } catch (e: Json.Error.Trailing) {
            throw Refusal("the event's data carries more than one JSON value")
        } catch (e: Json.Error.Malformed) {
            throw Refusal("the event's data is not JSON (${e.what})")
        }
        @Suppress("UNCHECKED_CAST")
        return Fields(value as Map<String, Any?>)
    }

    /**
     * [decode] for a schema a later revision of which added fields: [defaults]
     * is the JSON object of those fields' defaults, which the generator spells
     * from the schema, and a field the delivery lacks is read out of it instead
     * (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").
     *
     * A delivery with no data at all lacks every field, so it is read as one
     * that names none: a field of the first revision is then refused by name,
     * and a schema all of whose fields came later reads its defaults. A field
     * the delivery does carry is read as it came, and held to its type as any
     * other value is.
     */
    fun decode(data: String, defaults: String): Fields {
        val delivered = if (data.isBlank()) emptyMap() else decode(data).values
        return Fields(defaultsOf(defaults) + delivered)
    }

    /** The members of a schema's defaults, which the generator spells as one JSON object. */
    private fun defaultsOf(defaults: String): Map<String, Any?> {
        val parsed = try {
            Json.parse(defaults.trim())
        } catch (e: Json.Error) {
            null
        }
        if (parsed !is Map<*, *>) {
            throw Refusal("the schema's defaults are not a JSON object, which the generator never writes")
        }
        @Suppress("UNCHECKED_CAST")
        return parsed as Map<String, Any?>
    }

    /** A value already spelled as JSON text, which [encode] writes as it is. */
    class Raw(val json: String) {
        override fun toString(): String = json
        override fun equals(other: Any?): Boolean = other is Raw && other.json == json
        override fun hashCode(): Int = json.hashCode()
    }

    /**
     * The wire spelling of the fields an inject seam was given, so the script
     * engine binds `_event.data` to the same values the typed carrier holds.
     */
    fun encode(fields: Map<String, Any?>): String {
        val parts = fields.entries.joinToString(",") { (key, value) ->
            "${Json.quote(key)}:${literal(value)}"
        }
        return "{$parts}"
    }

    /**
     * The JSON spelling of one record (docs/adr/0086, section 3.1): each member
     * is a field's id and its value, and the object writes them ascending by the
     * id's UTF-8 bytes, whatever order they are given in (ARCHITECTURE.md, "JSON
     * Object Key Order", at every depth; [Json.compareKeys]).
     */
    fun recordJson(members: Map<String, Any?>): Raw {
        val sorted = members.entries.sortedWith { l, r -> Json.compareKeys(l.key, r.key) }
        return Raw(sorted.joinToString(",", "{", "}") { (key, value) -> "${Json.quote(key)}:${literal(value)}" })
    }

    /** The JSON spelling of one list: an array of what [write] spells each element as, in order. */
    fun <T> listJson(items: List<T>, write: (T) -> Raw): Raw =
        Raw(items.joinToString(",", "[", "]") { write(it).json })

    /** The JSON spelling [encode] gives one value: what a list's element of a scalar type is written as. */
    fun valueJson(value: Any?): Raw = Raw(literal(value))

    /** A byte string as the Latin-1 text the wire carries (see [Value.bytes]). */
    fun bytesAsText(bytes: ByteArray): String {
        val chars = CharArray(bytes.size) { (bytes[it].toInt() and 0xFF).toChar() }
        return chars.concatToString()
    }

    private fun literal(value: Any?): String = when (value) {
        null -> "null"
        is Raw -> value.json
        is Boolean -> value.toString()
        is UByte, is UShort, is UInt -> value.toString()
        is Byte, is Short, is Int -> value.toString()
        // A 64-bit integer crosses as the text of its digits.
        is ULong, is Long -> Json.quote(value.toString())
        // JSON has no spelling for a double that is not finite. A float32 is
        // still written as the platform spells it: the contract pins the
        // 64-bit spelling (ARCHITECTURE.md, "JSON Number Text").
        is Double -> if (value.isFinite()) Json.numberText(value) else "null"
        is Float -> value.toString()
        is ByteArray -> Json.quote(bytesAsText(value))
        else -> Json.quote(value.toString())
    }
}

/** A value a host built is past a bound its type declares; thrown where the value is made. */
class BoundExceeded(message: String) : IllegalArgumentException(message)

/**
 * The checks a generated record's `init` runs over a value a host builds
 * (docs/adr/0086, section 5.4), one call per constructor of the field's type,
 * so a value that exists is one of its type at every depth. Each names the
 * path, as the payload reader does.
 */
object ValueBounds {

    /** A text of at most [bound] UTF-8 bytes. */
    fun text(path: String, value: String, bound: Int) {
        val size = value.encodeToByteArray().size
        if (size > bound) {
            throw BoundExceeded("'$path' is $size bytes, past the $bound its type declares")
        }
    }

    /** A byte string of at most [bound] bytes. */
    fun bytes(path: String, value: ByteArray, bound: Int) {
        if (value.size > bound) {
            throw BoundExceeded("'$path' is ${value.size} bytes, past the $bound its type declares")
        }
    }

    /** A list of at most [bound] elements, each held to its own type by [each] at the path of its index. */
    fun <T> list(path: String, value: List<T>, bound: Int, each: (String, T) -> Unit) {
        if (value.size > bound) {
            throw BoundExceeded("'$path' holds ${value.size} elements, past the $bound its type declares")
        }
        value.forEachIndexed { index, item -> each("$path/$index", item) }
    }

    /** An optional value, held to its type by [present] where it is present. */
    fun <T : Any> optional(path: String, value: T?, present: (String, T) -> Unit) {
        if (value != null) {
            present(path, value)
        }
    }
}
