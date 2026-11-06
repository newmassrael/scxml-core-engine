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
 * through every depth: `'items/2/tags/0' is 9 bytes, past the 8 its type
 * declares`. A key a record's schema does not declare is stepped over.
 *
 * A refusal is a [Refusal]: the sentence the delivery's `error.execution`
 * carries, and beside it what a statechart reads as data (docs/adr/0087,
 * section 2.4): a [RefusalCode], the place as a strict RFC 6901 pointer
 * (`/items/2/tags/0`, `~` written `~0` and `/` written `~1`, empty for the
 * whole text) and the bound the value broke. A repeated name anywhere in the
 * text, and a text holding U+0000, are refused, so every runtime's reader
 * holds one set of texts (tests/json_text/payload_refusal.json).
 *
 * The JSON is read through the runtime's one reader ([Json]); what the inject
 * seam writes goes through its [Json.quote].
 *
 * Cross-language siblings: `sce_runtime.event_payload` (Python),
 * `sce.LiftPayload` (Go), `SCE::EventPayload` (C++).
 */
object EventPayload {

    /**
     * The class of a refusal: the enum `sce:std/lift/refusal_code.scxml`
     * declares, each code by its [declaredName] at its [number]
     * (docs/adr/0087, section 2.3).
     */
    enum class RefusalCode(val number: Int, val declaredName: String) {
        /** The text is empty, is not JSON, carries more than one JSON value, or repeats a name. */
        NOT_JSON(1, "notJson"),

        /** The text is one JSON value that is not an object. */
        NOT_OBJECT(2, "notObject"),

        /** A container is opened deeper than any a value of the schema holds. */
        TOO_DEEP(3, "tooDeep"),

        /** A field of a record is absent and no revision default fills it. */
        MISSING(4, "missing"),

        /** `null` stands where the type is not optional. */
        NULL_NOT_OPTIONAL(5, "nullNotOptional"),

        /** A value of another JSON kind. */
        WRONG_KIND(6, "wrongKind"),

        /** A number not in its type's spelling. */
        BAD_NUMBER(7, "badNumber"),

        /** A whole number past its type's width, or a real past a `float32`'s range. */
        OUT_OF_RANGE(8, "outOfRange"),

        /** A text or byte string past its bound. */
        TOO_LONG(9, "tooLong"),

        /** A list past its bound. */
        TOO_MANY(10, "tooMany"),

        /** A name the enum does not declare. */
        UNKNOWN_VARIANT(11, "unknownVariant"),

        /** A byte string's text holding a character above U+00FF. */
        NOT_BYTE_TEXT(12, "notByteText"),

        /** A text holding U+0000. */
        NUL_IN_TEXT(13, "nulInText"),

        /** A list short of the lower count its type states (docs/adr/0086, section 15). */
        TOO_FEW(14, "tooFew"),
        ;

        companion object {
            /** The code the enum declares by [name], or null when it declares none. */
            fun named(name: String): RefusalCode? = entries.firstOrNull { it.declaredName == name }
        }
    }

    /**
     * Why an event's data cannot be read as its schema's fields: the sentence
     * the delivery's `error.execution` carries, and beside it its [code], the
     * place as a strict RFC 6901 [pointer] (empty for the whole text), and the
     * [bound] the value broke, for a text, a byte string or a list past it.
     */
    class Refusal(
        message: String,
        val code: RefusalCode,
        val pointer: String,
        val bound: Long? = null,
    ) : Exception(message)

    /**
     * [token] below the strict RFC 6901 pointer [prefix] (empty for the whole
     * text): `/` and the token with `~` written `~0` and `/` written `~1`
     * (RFC 6901, section 3).
     */
    fun pointerBelow(prefix: String, token: String): String =
        "$prefix/" + token.replace("~", "~0").replace("/", "~1")

    /** Whether [text] spells a whole number: digits, after a `-` or not. */
    private fun spellsWhole(text: String): Boolean {
        val digits = text.removePrefix("-")
        return digits.isNotEmpty() && digits.all { it in '0'..'9' }
    }

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
     * it: a field's name, then each index and field name below it, joined by `/`;
     * and at the strict RFC 6901 [pointer] to the same place, which a refusal
     * carries beside its sentence.
     */
    class Value internal constructor(private val raw: Any?, val path: String, val pointer: String) {

        /** Whether the value is `null`, the one spelling of an absent value. */
        val isNull: Boolean get() = raw == null

        /**
         * A refusal of this value, of class [code], saying [what] of it after its
         * path: `'items/1/level' (x) is not a variant of Level`.
         */
        fun refuse(code: RefusalCode, what: String, bound: Int? = null): Refusal =
            Refusal("'$path' $what", code, pointer, bound?.toLong())

        /** The refusal of the text [name] this value carries, which the enum [alias] does not declare. */
        fun unknownVariant(name: String, alias: String): Refusal =
            refuse(RefusalCode.UNKNOWN_VARIANT, "($name) is not a variant of $alias")

        /** The refusal of `null` where the type is not optional, or [otherwise]. */
        private fun wrongKind(otherwise: String): Refusal =
            if (isNull) {
                refuse(RefusalCode.NULL_NOT_OPTIONAL, "is null, and its type is not optional")
            } else {
                refuse(RefusalCode.WRONG_KIND, otherwise)
            }

        /** The refusal of a number past the width its type declares. */
        private fun pastWidth(text: String): Refusal =
            refuse(RefusalCode.OUT_OF_RANGE, "does not fit the width its schema declares ($text)")

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
            // `-0` is a JSON number and no spelling of a whole number: every
            // runtime refuses it, as it refuses `"-0"` for a 64-bit integer.
            if (text == "-0") throw refuse(RefusalCode.BAD_NUMBER, "is not a whole number ($text)")
            // A whole number no Long holds is past every width this reads; any
            // other spelling is no whole number at all.
            return text.toLongOrNull() ?: throw refuse(
                if (spellsWhole(text)) RefusalCode.OUT_OF_RANGE else RefusalCode.BAD_NUMBER,
                "is not a whole number ($text)",
            )
        }

        private fun <T> narrowed(value: Long, min: Long, max: Long, of: (Long) -> T): T {
            if (value < min || value > max) {
                // Below zero is no spelling of an unsigned number.
                val code = if (min == 0L && value < 0L) RefusalCode.BAD_NUMBER else RefusalCode.OUT_OF_RANGE
                throw refuse(code, "does not fit the width its schema declares ($value)")
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
                    else throw refuse(RefusalCode.BAD_NUMBER, "is not a whole number written as digits ($raw)")
                // `-0` is no whole number, and a number below zero no spelling of
                // an unsigned one, as for every narrower width.
                is Json.Number -> {
                    if (raw.text == "-0" || (!signed && raw.text.startsWith("-"))) {
                        throw refuse(RefusalCode.BAD_NUMBER, "is not a whole number of its type (${raw.text})")
                    }
                    val n = raw.text.toLongOrNull()
                        ?: throw refuse(RefusalCode.BAD_NUMBER, "is not a whole number (${raw.text})")
                    if (n > EXACT_IN_A_DOUBLE || n < -EXACT_IN_A_DOUBLE) {
                        throw refuse(
                            RefusalCode.BAD_NUMBER,
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
            return text.toLongOrNull() ?: throw pastWidth(text)
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
            return text.toULongOrNull() ?: throw pastWidth(text)
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
            val wide = text.toDoubleOrNull() ?: throw refuse(RefusalCode.BAD_NUMBER, "is not a number ($text)")
            if (kotlin.math.abs(wide) > Float.MAX_VALUE.toDouble()) {
                throw pastWidth(text)
            }
            return wide.toFloat()
        }

        /**
         * A 64-bit fractional value. A number past the largest double (`1e999`)
         * does not fit it, as one past the largest single does not fit a
         * `float32`: no runtime holds the infinity a reader would round it to.
         */
        fun float64(): Double {
            val text = number()
            val real = text.toDoubleOrNull() ?: throw refuse(RefusalCode.BAD_NUMBER, "is not a number ($text)")
            if (!real.isFinite()) throw pastWidth(text)
            return real
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
         *
         * A text holding U+0000 is refused: a runtime that holds a text
         * NUL-terminated would end it there, so no runtime holds one, and every
         * reader refuses the same texts (docs/adr/0087, section 2.3). A byte
         * string may hold the byte zero ([bytes]).
         */
        fun name(): String {
            if (raw !is String) {
                throw wrongKind("is not a text ($raw)")
            }
            if (raw.indexOf('\u0000') >= 0) {
                throw refuse(RefusalCode.NUL_IN_TEXT, "carries U+0000, which no text holds")
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
                    throw refuse(
                        RefusalCode.NOT_BYTE_TEXT,
                        "carries a character above U+00FF, which no single byte spells",
                    )
                }
                out[i] = c.toByte()
            }
            if (out.size > bound) {
                throw pastBytes(out.size, bound)
            }
            return out
        }

        private fun pastBytes(held: Int, bound: Int): Refusal =
            refuse(RefusalCode.TOO_LONG, "is $held bytes, past the $bound its type declares", bound)

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
        fun <T> list(bound: Int, read: (Value) -> T): List<T> = listWithin(0, bound, read)

        /**
         * A value of `list<T>[least..bound]`: an array of at least [least] and at
         * most [bound] elements (docs/adr/0086, section 15), each read by [read]
         * at the path of its index, in order.
         *
         * Both counts are held before any element is read: an array longer than
         * [bound] is refused as [RefusalCode.TOO_MANY], one shorter than [least]
         * as [RefusalCode.TOO_FEW] naming [least], and no element of either is
         * decoded.
         */
        fun <T> listWithin(least: Int, bound: Int, read: (Value) -> T): List<T> {
            if (raw !is List<*>) {
                throw wrongKind("is not a list")
            }
            if (raw.size > bound) {
                throw refuse(
                    RefusalCode.TOO_MANY,
                    "holds ${raw.size} elements, past the $bound its type declares",
                    bound,
                )
            }
            if (raw.size < least) {
                throw refuse(RefusalCode.TOO_FEW, ValueBounds.countShort(raw.size, least), least)
            }
            return raw.mapIndexed { index, item ->
                val token = index.toString()
                read(Value(item, join(path, token), pointerBelow(pointer, token)))
            }
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
            return read(Fields(fallback + members, path, pointer))
        }

        /**
         * A value of a union type (docs/adr/0092, section 4): one object holding
         * its tag under the key [tag] and the fields of the case the tag names
         * beside it.
         *
         * The tag is read first, by [readTag], the reader of the tag's enum,
         * wherever it stands in the object; [readCase] is then handed the case
         * the tag named and this same value, and reads the case's record from it
         * with [record] (an empty case reads nothing). The refusals come in one
         * order on every runtime: a value that is not an object; the tag absent
         * ([RefusalCode.MISSING] at the tag's pointer), then whatever the enum's
         * reader refuses of it (`null`, not a text, a name the enum does not
         * declare); then the case's record. A key the case does not declare is
         * never asked for, so it is stepped over, as a record steps over one, and
         * a text that carries another case's keys, or `null` under them, reads as
         * the case its tag names.
         */
        fun <E, T> union(tag: String, readTag: (Value) -> E, readCase: (E, Value) -> T): T {
            if (raw !is Map<*, *>) {
                throw wrongKind("is not a union")
            }
            @Suppress("UNCHECKED_CAST")
            val members = raw as Map<String, Any?>
            return readCase(readTag(Fields(members, path, pointer).field(tag)), this)
        }
    }

    /**
     * The fields an event carries, each value still in the spelling it was
     * written in, so a whole number stays exact until it is read at the width
     * its schema declares. The fields of a record value are read the same way,
     * at the record's path.
     */
    class Fields internal constructor(
        internal val values: Map<String, Any?>,
        private val prefix: String = "",
        private val pointerPrefix: String = "",
    ) {

        /**
         * The value of the field [name], at its path: the delivered one, else
         * the default a later revision of the schema gave it, else a refusal
         * naming the path. A member the schema does not declare is never asked
         * for, so it is stepped over.
         */
        fun field(name: String): Value {
            val path = join(prefix, name)
            val pointer = pointerBelow(pointerPrefix, name)
            if (!values.containsKey(name)) {
                throw Refusal("the event's data has no '$path'", RefusalCode.MISSING, pointer)
            }
            return Value(values[name], path, pointer)
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
        // Only the whitespace RFC 8259 has: any other is a text that is not JSON.
        val trimmed = Json.trimWhitespace(data)
        if (trimmed.isEmpty()) {
            throw Refusal("the event carries no data", RefusalCode.NOT_JSON, "")
        }
        if (!trimmed.startsWith('{')) {
            // One sentence for a text that is no object, as it has always been;
            // the code tells one JSON value that is not an object from a text
            // that is not JSON at all (docs/adr/0087, section 2.3).
            val oneValue = try {
                Json.parse(trimmed)
                true
            } catch (e: Json.Error) {
                false
            }
            throw Refusal(
                "the event's data is a bare value, and this event's schema declares named fields",
                if (oneValue) RefusalCode.NOT_OBJECT else RefusalCode.NOT_JSON,
                "",
            )
        }
        val value = try {
            Json.parse(trimmed)
        } catch (e: Json.Error.Trailing) {
            throw Refusal("the event's data carries more than one JSON value", RefusalCode.NOT_JSON, "")
        } catch (e: Json.Error.Malformed) {
            throw Refusal("the event's data is not JSON (${e.what})", RefusalCode.NOT_JSON, "")
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
        val delivered = if (Json.trimWhitespace(data).isEmpty()) emptyMap() else decode(data).values
        return Fields(defaultsOf(defaults) + delivered)
    }

    /**
     * Read the text a `<sce:lift>` reads as the fields of a record of a schema
     * (docs/adr/0087, section 2.1): [depth] is the deepest container a value of
     * the schema holds, [pathBound] the room of the refusal record's `path` a
     * `tooDeep` pointer is held to ([liftTooDeep]), and [defaults], for a
     * schema a later revision of which added fields, the JSON object of their
     * defaults, which a text that lacks one reads in its place.
     *
     * Three refusals come before the fields are read, in this order: a text
     * that is empty or only whitespace (`notJson`); a container opened deeper
     * than [depth], decided over the text's brackets alone before it is read as
     * JSON ([liftTooDeep], `tooDeep`); and a text that is not one JSON object,
     * as a delivery is refused ([decode]).
     */
    fun lift(text: String, depth: Int, pathBound: Int, defaults: String?): Fields {
        if (Json.trimWhitespace(text).isEmpty()) {
            throw Refusal("the text is empty", RefusalCode.NOT_JSON, "")
        }
        liftTooDeep(text, depth, pathBound)?.let { throw it }
        val delivered = decode(text).values
        return Fields(if (defaults == null) delivered else defaultsOf(defaults) + delivered)
    }

    /**
     * The refusal of a container [text] opens deeper than [depth], the deepest a
     * value of a lift's schema holds (docs/adr/0087, section 2.1, item 4), or
     * null when it opens none.
     *
     * Decided over the brackets alone, before the text is read as JSON, so that
     * every runtime decides it alike and no reader descends past the type: a
     * text is skipped from its opening quote to the next quote no backslash
     * escapes, each `{` and `[` outside one opens a level and each `}` and `]`
     * closes one, and the first opened past [depth] is refused, whatever the
     * text holds after it. A text that never closes ends the walk with nothing
     * refused, and the reading of the text as JSON refuses it.
     *
     * The pointer names the container refused (docs/adr/0087, section 2.4):
     * for each container it stands in, outermost first, the element's index in
     * an array (the `,` the scan met at that level since the array opened) or
     * the member's name in an object (the last text closed at that level before
     * a `:`, since the object opened or its last `,`), each an RFC 6901 token.
     * It stops before the first place it cannot write: a member with no name
     * before its colon, a name that is no text (no JSON string, or one holding
     * U+0000), or a token that would take the pointer past [pathBound] UTF-8
     * bytes, the room of the refusal record's `path`. The bound is [depth]. The
     * scan keeps one entry per open container, at most [depth] of them.
     */
    fun liftTooDeep(text: String, depth: Int, pathBound: Int): Refusal? {
        val open = ArrayList<LiftScanLevel>()
        var at = 0
        while (at < text.length) {
            val top = open.lastOrNull()
            when (text[at]) {
                '"' -> {
                    val start = at
                    at++
                    while (at < text.length && text[at] != '"') {
                        at += if (text[at] == '\\') 2 else 1
                    }
                    if (at >= text.length) return null
                    top?.lastText = text.substring(start, at + 1)
                }
                ':' -> if (top != null) {
                    top.member = top.lastText
                    top.lastText = null
                }
                ',' -> if (top != null) {
                    top.member = null
                    top.lastText = null
                    top.index++
                }
                '{', '[' -> {
                    if (open.size >= depth) {
                        return Refusal(
                            "the text opens a container deeper than the $depth levels a value of its schema holds",
                            RefusalCode.TOO_DEEP,
                            liftScanPointer(open, pathBound),
                            depth.toLong(),
                        )
                    }
                    open.add(LiftScanLevel(array = text[at] == '['))
                }
                '}', ']' -> if (top != null) open.removeAt(open.size - 1)
            }
            at++
        }
        return null
    }

    /**
     * One container [liftTooDeep] stands in: whether it is an array, the
     * elements it has passed, the last text closed in it, and the member a `:`
     * after that text named.
     */
    private class LiftScanLevel(val array: Boolean) {
        var index = 0L
        var lastText: String? = null
        var member: String? = null
    }

    /** The pointer to the place [open] stands at, as [liftTooDeep] writes it. */
    private fun liftScanPointer(open: List<LiftScanLevel>, pathBound: Int): String {
        var pointer = ""
        for (level in open) {
            val token = if (level.array) {
                level.index.toString()
            } else {
                val name = level.member?.let {
                    try {
                        Json.parse(it) as? String
                    } catch (e: Json.Error) {
                        null
                    }
                }
                if (name == null || name.contains('\u0000')) break
                name
            }
            val below = pointerBelow(pointer, token)
            if (below.encodeToByteArray().size > pathBound) break
            pointer = below
        }
        return pointer
    }

    /** What a `<sce:lift>` answers: a record of its schema, or the refusal of the text. */
    sealed class LiftOutcome<out T> {
        /** The text is a record of the schema. */
        class Lifted<out T>(val value: T) : LiftOutcome<T>()

        /** The text is not: the first place refused, its code, pointer and bound. */
        class Refused(val refusal: Refusal) : LiftOutcome<Nothing>()
    }

    /**
     * What a `<sce:lift>` reads (docs/adr/0087, section 2.1): [text] as a record
     * of a schema, whose fields [read] reads out of the [Fields] it is handed, as
     * a delivery of the schema is read; or the refusal of the first place that
     * is not one, its code, pointer and bound the statechart keeps as a record.
     * [lift] says what is refused before the fields are read. This runtime keeps
     * no log, so neither outcome is written to one.
     */
    fun <T> liftRecord(
        text: String,
        depth: Int,
        pathBound: Int,
        defaults: String?,
        read: (Fields) -> T,
    ): LiftOutcome<T> =
        try {
            LiftOutcome.Lifted(read(lift(text, depth, pathBound, defaults)))
        } catch (refusal: Refusal) {
            LiftOutcome.Refused(refusal)
        }

    /** The members of a schema's defaults, which the generator spells as one JSON object. */
    private fun defaultsOf(defaults: String): Map<String, Any?> {
        val parsed = try {
            Json.parse(defaults.trim())
        } catch (e: Json.Error) {
            null
        }
        if (parsed !is Map<*, *>) {
            // A text that is none is the generator's, not a sender's, and is
            // refused as no JSON object of the whole.
            throw Refusal(
                "the schema's defaults are not a JSON object, which the generator never writes",
                RefusalCode.NOT_JSON,
                "",
            )
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

    /**
     * The JSON spelling of one union value (docs/adr/0092, section 2.1): the tag
     * under the key [tag], its value [variant], the case's name as its enum spells
     * it, beside [members], the fields of the case's record (none for an empty
     * case). The object writes the tag among the fields, every key ascending by
     * its UTF-8 bytes, as [recordJson] writes a record.
     */
    fun unionJson(tag: String, variant: String, members: Map<String, Any?>): Raw =
        recordJson(members + (tag to variant))

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
        listWithin(path, value, 0, bound, each)
    }

    /**
     * A list of at least [least] and at most [bound] elements (docs/adr/0086, section 15.6),
     * each held to its own type by [each] at the path of its index: a list is built from empty
     * an element at a time, so its lower count is held where it is written.
     */
    fun <T> listWithin(path: String, value: List<T>, least: Int, bound: Int, each: (String, T) -> Unit) {
        if (value.size > bound) {
            throw BoundExceeded("'$path' holds ${value.size} elements, past the $bound its type declares")
        }
        if (value.size < least) {
            throw BoundExceeded("'$path' ${countShort(value.size, least)}")
        }
        value.forEachIndexed { index, item -> each("$path/$index", item) }
    }

    /** Why a list of [held] elements is no value of a type whose lower count is [least]. */
    internal fun countShort(held: Int, least: Int): String =
        "holds $held elements, fewer than the $least its type declares"

    /** An optional value, held to its type by [present] where it is present. */
    fun <T : Any> optional(path: String, value: T?, present: (String, T) -> Unit) {
        if (value != null) {
            present(path, value)
        }
    }
}
