// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_model_harness.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_model_harness.scxml:67 :: _machine

package com.sce.integration.static_model_harness

import com.sce.runtime.*
import com.sce.generated.harness_ask.*
import com.sce.generated.harness_repair.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticModelHarnessState : State {
    data object Accepted : StaticModelHarnessState
    data object Check : StaticModelHarnessState
    data object Compose : StaticModelHarnessState
    data object GaveUp : StaticModelHarnessState
    data object Generate : StaticModelHarnessState
    data object Harness : StaticModelHarnessState
    data object Repair : StaticModelHarnessState
    data object Retry : StaticModelHarnessState
    data object Waiting : StaticModelHarnessState
    data object Work : StaticModelHarnessState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticModelHarnessEvent : Event {
    sealed interface Done : StaticModelHarnessEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Gen : Invoke
        }
        sealed interface State : Done {
            data object Harness : State
        }
    }
    sealed interface Error : StaticModelHarnessEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Self : Invoke
            data object Gen : Invoke
        }
    }
    sealed interface Harness : StaticModelHarnessEvent {
        data object Offer : Harness
        data object Start : Harness
        data object Timeout : Harness
    }
}
/**
 * The answer schemas this machine's model generations name with `sce:answer`
 * (docs/adr/0087, section 4.1): each schema's id, which is the `answer_schema`
 * its request carries, with the JSON Schema and the GBNF grammar a host hands
 * its provider. A host resolves a request's id here and nowhere else. A GBNF
 * grammar is as long as its largest text bound, so each text is declared by
 * the generator's one door for a text constant of any length.
 */
object StaticModelHarnessAnswerSchemas {
    const val ID_0: String = "urn:sce:schema:lift_answer:2"
    const val JSON_SCHEMA_0: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:lift_answer:2\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"kind\":{\"enum\":[\"meeting\",\"task\"]},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]},\"tag\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*\$\"},\"priority\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"id\",\"amount\",\"title\",\"kind\",\"note\",\"tag\",\"priority\"],\"additionalProperties\":false}"
    const val GBNF_0: String = "root ::= \"{\\\"id\\\":\" uint32 \",\\\"amount\\\":\" int64 \",\\\"title\\\":\" \"\\\"\" t16 \"\\\"\" \",\\\"kind\\\":\" ( \"\\\"meeting\\\"\" | \"\\\"task\\\"\" ) \",\\\"note\\\":\" ( \"null\" | \"\\\"\" t8 \"\\\"\" ) \",\\\"tag\\\":\" \"\\\"\" b4 \"\\\"\" \",\\\"priority\\\":\" uint8 \"}\"\nint64 ::= \"\\\"\" ( \"-\" ( [1-9] [0-9]{0,17} | [1-8] [0-9]{18} | \"9\" [0-1] [0-9]{17} | \"92\" [0-1] [0-9]{16} | \"922\" [0-2] [0-9]{15} | \"9223\" [0-2] [0-9]{14} | \"92233\" [0-6] [0-9]{13} | \"922337\" [0-1] [0-9]{12} | \"92233720\" [0-2] [0-9]{10} | \"922337203\" [0-5] [0-9]{9} | \"9223372036\" [0-7] [0-9]{8} | \"92233720368\" [0-4] [0-9]{7} | \"922337203685\" [0-3] [0-9]{6} | \"9223372036854\" [0-6] [0-9]{5} | \"92233720368547\" [0-6] [0-9]{4} | \"922337203685477\" [0-4] [0-9]{3} | \"9223372036854775\" [0-7] [0-9]{2} | \"922337203685477580\" [0-8] ) | \"0\" | [1-9] [0-9]{0,17} | [1-8] [0-9]{18} | \"9\" [0-1] [0-9]{17} | \"92\" [0-1] [0-9]{16} | \"922\" [0-2] [0-9]{15} | \"9223\" [0-2] [0-9]{14} | \"92233\" [0-6] [0-9]{13} | \"922337\" [0-1] [0-9]{12} | \"92233720\" [0-2] [0-9]{10} | \"922337203\" [0-5] [0-9]{9} | \"9223372036\" [0-7] [0-9]{8} | \"92233720368\" [0-4] [0-9]{7} | \"922337203685\" [0-3] [0-9]{6} | \"9223372036854\" [0-6] [0-9]{5} | \"92233720368547\" [0-6] [0-9]{4} | \"922337203685477\" [0-4] [0-9]{3} | \"9223372036854775\" [0-7] [0-9]{2} | \"922337203685477580\" [0-7] ) \"\\\"\"\nuint32 ::= \"0\" | [1-9] [0-9]{0,8} | [1-3] [0-9]{9} | \"4\" [0-1] [0-9]{8} | \"42\" [0-8] [0-9]{7} | \"429\" [0-3] [0-9]{6} | \"4294\" [0-8] [0-9]{5} | \"42949\" [0-5] [0-9]{4} | \"429496\" [0-6] [0-9]{3} | \"4294967\" [0-1] [0-9]{2} | \"42949672\" [0-8] [0-9] | \"429496729\" [0-5]\nuint8 ::= \"0\" | [1-9] [0-9]{0,1} | \"1\" [0-9]{2} | \"2\" [0-4] [0-9] | \"25\" [0-5]\nbyte ::= [\\x20-\\x21\\x23-\\x5b\\x5d-\\u00ff] | \"\\\\\" [\\\"\\\\bfnrt] | \"\\\\u000\" [0-7bef] | \"\\\\u001\" [0-9a-f]\nb0 ::= \"\"\nb1 ::= \"\" | byte b0\nb2 ::= \"\" | byte b1\nb3 ::= \"\" | byte b2\nb4 ::= \"\" | byte b3\nc1 ::= [\\x20-\\x21\\x23-\\x5b\\x5d-\\x7f] | \"\\\\\" [\\\"\\\\bfnrt] | \"\\\\u000\" [1-7bef] | \"\\\\u001\" [0-9a-f]\nc2 ::= [\\u0080-\\u07ff]\nc3 ::= [\\u0800-\\ud7ff\\ue000-\\uffff]\nc4 ::= [\\U00010000-\\U0010ffff]\nt0 ::= \"\"\nt1 ::= \"\" | c1 t0\nt2 ::= \"\" | c1 t1 | c2 t0\nt3 ::= \"\" | c1 t2 | c2 t1 | c3 t0\nt4 ::= \"\" | c1 t3 | c2 t2 | c3 t1 | c4 t0\nt5 ::= \"\" | c1 t4 | c2 t3 | c3 t2 | c4 t1\nt6 ::= \"\" | c1 t5 | c2 t4 | c3 t3 | c4 t2\nt7 ::= \"\" | c1 t6 | c2 t5 | c3 t4 | c4 t3\nt8 ::= \"\" | c1 t7 | c2 t6 | c3 t5 | c4 t4\nt9 ::= \"\" | c1 t8 | c2 t7 | c3 t6 | c4 t5\nt10 ::= \"\" | c1 t9 | c2 t8 | c3 t7 | c4 t6\nt11 ::= \"\" | c1 t10 | c2 t9 | c3 t8 | c4 t7\nt12 ::= \"\" | c1 t11 | c2 t10 | c3 t9 | c4 t8\nt13 ::= \"\" | c1 t12 | c2 t11 | c3 t10 | c4 t9\nt14 ::= \"\" | c1 t13 | c2 t12 | c3 t11 | c4 t10\nt15 ::= \"\" | c1 t14 | c2 t13 | c3 t12 | c4 t11\nt16 ::= \"\" | c1 t15 | c2 t14 | c3 t13 | c4 t12\n"

    /** Every answer schema of the machine, in the order the document first names it. */
    val all: List<AnswerSchema> = listOf(
        AnswerSchema(ID_0, JSON_SCHEMA_0, GBNF_0),
    )
}

// --- Transition table (§synth-6.2.6), because the document declares it ---

/**
 * One transition of this machine, as constant data. Nothing in the machine
 * reads it: a host that draws the machine, or checks it against its own
 * design, does. `from`, `to` and `event` are this file's own
 * [StaticModelHarnessState] and [StaticModelHarnessEvent]; `index` is the
 * transition's position within its state in document order; `annotation` is
 * the `sce:req` tokens as written, space-separated and uninterpreted. `to` and
 * `event` are null where a transition has no single id in those hierarchies: a
 * targetless transition, a first target that is a `<history>`, an eventless
 * transition, a wildcard or prefix descriptor, or several descriptors.
 * `targets` and `events` are the complete answer there: the whole target set
 * and every descriptor as written, space-separated (`*`, a prefix such as
 * `error.*` and a literal name alike, none resolved), empty when the
 * transition has none. `sce-codegen transition-table` lists the same
 * transitions in the same order.
 */
data class StaticModelHarnessTransition(
    val from: StaticModelHarnessState,
    val to: StaticModelHarnessState?,
    val event: StaticModelHarnessEvent?,
    val targets: String,
    val events: String,
    val index: Int,
    val annotation: String,
)

object StaticModelHarnessTransitions {
    /** The digest of the source set this machine was generated from: the `source-hash` of the file's own header, as a value. */
    const val SOURCE_HASH: String = "a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22"

    /** Every transition of the machine. */
    val all: List<StaticModelHarnessTransition> = listOf(
        StaticModelHarnessTransition(StaticModelHarnessState.Waiting, null, StaticModelHarnessEvent.Harness.Offer, "", "harness.offer", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Waiting, StaticModelHarnessState.Work, StaticModelHarnessEvent.Harness.Start, "work", "harness.start", 1, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Work, StaticModelHarnessState.GaveUp, StaticModelHarnessEvent.Harness.Timeout, "gave_up", "harness.timeout", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Compose, StaticModelHarnessState.Generate, null, "generate", "", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Generate, StaticModelHarnessState.Check, StaticModelHarnessEvent.Done.Invoke.Gen, "check", "done.invoke.gen", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Generate, StaticModelHarnessState.GaveUp, StaticModelHarnessEvent.Done.Invoke.Gen, "gave_up", "done.invoke.gen", 1, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Generate, StaticModelHarnessState.Retry, StaticModelHarnessEvent.Error.Invoke.Gen, "retry", "error.invoke.gen", 2, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Generate, StaticModelHarnessState.GaveUp, StaticModelHarnessEvent.Error.Execution, "gave_up", "error.execution", 3, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Retry, StaticModelHarnessState.GaveUp, null, "gave_up", "", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Retry, StaticModelHarnessState.Generate, null, "generate", "", 1, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Check, StaticModelHarnessState.Repair, null, "repair", "", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Check, StaticModelHarnessState.Repair, null, "repair", "", 1, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Check, StaticModelHarnessState.Accepted, null, "accepted", "", 2, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Repair, StaticModelHarnessState.GaveUp, null, "gave_up", "", 0, ""),
        StaticModelHarnessTransition(StaticModelHarnessState.Repair, StaticModelHarnessState.Generate, null, "generate", "", 1, ""),
    )
}

// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticModelHarnessPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticModelHarnessDoneInvokeGenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.gen`. Consumers inject it via the `raiseDoneInvokeGen` seam
// on the machine — they never name this class directly.
data class StaticModelHarnessDoneInvokeGenPayload(val error: String, val text: String, val constrained: Boolean, val model: String, val prompt_tokens: UInt, val answer_tokens: UInt, val finish: String)

// StaticModelHarnessHarnessOfferPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `harness.offer`. Consumers inject it via the `raiseHarnessOffer` seam
// on the machine — they never name this class directly.
data class StaticModelHarnessHarnessOfferPayload(val id: UInt)

// StaticModelHarnessHarnessStartPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `harness.start`. Consumers inject it via the `raiseHarnessStart` seam
// on the machine — they never name this class directly.
data class StaticModelHarnessHarnessStartPayload(val request: String)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:HarnessCode` datamodel value. */
enum class StaticModelHarnessHarnessCodeEnum(val declaredName: String) {
    NOT_JSON("notJson"),
    NOT_OBJECT("notObject"),
    TOO_DEEP("tooDeep"),
    MISSING("missing"),
    NULL_NOT_OPTIONAL("nullNotOptional"),
    WRONG_KIND("wrongKind"),
    BAD_NUMBER("badNumber"),
    OUT_OF_RANGE("outOfRange"),
    TOO_LONG("tooLong"),
    TOO_MANY("tooMany"),
    UNKNOWN_VARIANT("unknownVariant"),
    NOT_BYTE_TEXT("notByteText"),
    NUL_IN_TEXT("nulInText"),
    TOO_FEW("tooFew"),
    NOT_OFFERED("notOffered");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        NOT_JSON -> 1u.toUByte()
        NOT_OBJECT -> 2u.toUByte()
        TOO_DEEP -> 3u.toUByte()
        MISSING -> 4u.toUByte()
        NULL_NOT_OPTIONAL -> 5u.toUByte()
        WRONG_KIND -> 6u.toUByte()
        BAD_NUMBER -> 7u.toUByte()
        OUT_OF_RANGE -> 8u.toUByte()
        TOO_LONG -> 9u.toUByte()
        TOO_MANY -> 10u.toUByte()
        UNKNOWN_VARIANT -> 11u.toUByte()
        NOT_BYTE_TEXT -> 12u.toUByte()
        NUL_IN_TEXT -> 13u.toUByte()
        TOO_FEW -> 14u.toUByte()
        NOT_OFFERED -> 15u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticModelHarnessHarnessCodeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of HarnessCode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticModelHarnessHarnessCodeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Refusal` datamodel value. */
data class StaticModelHarnessRefusalRecord(val code: StaticModelHarnessHarnessCodeEnum, val path: String) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("code" to code.toSaved(), "path" to SavedValues.of(path))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticModelHarnessRefusalRecord = StaticModelHarnessRefusalRecord(code = StaticModelHarnessHarnessCodeEnum.fromSaved(SavedValues.field(value, what, "code"), "$what.code"), path = SavedValues.string(SavedValues.field(value, what, "path"), "$what.path", 256))
    }
}
/** SCE Accepted Subset §2.15: an `enum:enum_lift_kind` datamodel value. */
enum class StaticModelHarnessEnumLiftKindEnum(val declaredName: String) {
    MEETING("meeting"),
    TASK("task");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        MEETING -> 0u.toUByte()
        TASK -> 1u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticModelHarnessEnumLiftKindEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of enum_lift_kind")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticModelHarnessEnumLiftKindEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Answer` datamodel value. */
data class StaticModelHarnessAnswerRecord(val id: UInt, val amount: Long, val title: String, val kind: StaticModelHarnessEnumLiftKindEnum, val note: String?, val tag: ByteArray, val priority: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "amount" to SavedValues.of(amount), "title" to SavedValues.of(title), "kind" to kind.toSaved(), "note" to SavedValues.optional(note) { SavedValues.of(it) }, "tag" to SavedValues.of(tag), "priority" to SavedValues.of(priority))

    override fun equals(other: Any?): Boolean =
        other is StaticModelHarnessAnswerRecord && id == other.id && amount == other.amount && title == other.title && kind == other.kind && note == other.note && tag.contentEquals(other.tag) && priority == other.priority

    override fun hashCode(): Int = 31 * (31 * (31 * (31 * (31 * (31 * (31 * (0) + id.hashCode()) + amount.hashCode()) + title.hashCode()) + kind.hashCode()) + note.hashCode()) + tag.contentHashCode()) + priority.hashCode()

    /** This value with a copy of each byte string, which a host may write into. */
    fun detached(): StaticModelHarnessAnswerRecord = copy(tag = tag.copyOf())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticModelHarnessAnswerRecord = StaticModelHarnessAnswerRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), amount = SavedValues.int64(SavedValues.field(value, what, "amount"), "$what.amount"), title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), kind = StaticModelHarnessEnumLiftKindEnum.fromSaved(SavedValues.field(value, what, "kind"), "$what.kind"), note = SavedValues.optional(SavedValues.field(value, what, "note"), "$what.note") { e, w -> SavedValues.string(e, w, 8) }, tag = SavedValues.bytes(SavedValues.field(value, what, "tag"), "$what.tag", 4), priority = SavedValues.uint8(SavedValues.field(value, what, "priority"), "$what.priority"))
    }
}
/** SCE Accepted Subset §2.15: an `enum:LiftRefusalCode` datamodel value. */
enum class StaticModelHarnessLiftRefusalCodeEnum(val declaredName: String) {
    NOT_JSON("notJson"),
    NOT_OBJECT("notObject"),
    TOO_DEEP("tooDeep"),
    MISSING("missing"),
    NULL_NOT_OPTIONAL("nullNotOptional"),
    WRONG_KIND("wrongKind"),
    BAD_NUMBER("badNumber"),
    OUT_OF_RANGE("outOfRange"),
    TOO_LONG("tooLong"),
    TOO_MANY("tooMany"),
    UNKNOWN_VARIANT("unknownVariant"),
    NOT_BYTE_TEXT("notByteText"),
    NUL_IN_TEXT("nulInText"),
    TOO_FEW("tooFew");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        NOT_JSON -> 1u.toUByte()
        NOT_OBJECT -> 2u.toUByte()
        TOO_DEEP -> 3u.toUByte()
        MISSING -> 4u.toUByte()
        NULL_NOT_OPTIONAL -> 5u.toUByte()
        WRONG_KIND -> 6u.toUByte()
        BAD_NUMBER -> 7u.toUByte()
        OUT_OF_RANGE -> 8u.toUByte()
        TOO_LONG -> 9u.toUByte()
        TOO_MANY -> 10u.toUByte()
        UNKNOWN_VARIANT -> 11u.toUByte()
        NOT_BYTE_TEXT -> 12u.toUByte()
        NUL_IN_TEXT -> 13u.toUByte()
        TOO_FEW -> 14u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticModelHarnessLiftRefusalCodeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of LiftRefusalCode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticModelHarnessLiftRefusalCodeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:LiftRefusal` datamodel value. */
data class StaticModelHarnessLiftRefusalRecord(val code: StaticModelHarnessLiftRefusalCodeEnum, val path: String, val bound: UInt?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("code" to code.toSaved(), "path" to SavedValues.of(path), "bound" to SavedValues.optional(bound) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticModelHarnessLiftRefusalRecord = StaticModelHarnessLiftRefusalRecord(code = StaticModelHarnessLiftRefusalCodeEnum.fromSaved(SavedValues.field(value, what, "code"), "$what.code"), path = SavedValues.string(SavedValues.field(value, what, "path"), "$what.path", 256), bound = SavedValues.optional(SavedValues.field(value, what, "bound"), "$what.bound") { e, w -> SavedValues.uint32(e, w) })
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object StaticModelHarnessLimits {
    /** The bound `sce:limit="harness_attempts"` names, as this build resolved it. */
    const val HARNESS_ATTEMPTS: UInt = 3u
    /** The bound `sce:limit="lift_path"` names, as this build resolved it. */
    const val LIFT_PATH: UInt = 256u
    /** The bound `sce:limit="model_answer"` names, as this build resolved it. */
    const val MODEL_ANSWER: UInt = 4096u
    /** The bound `sce:limit="model_prompt"` names, as this build resolved it. */
    const val MODEL_PROMPT: UInt = 8192u
}
// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:request`: the record the host is asked to start `<invoke id="gen">` with (SCE Accepted Subset §2.12). */
data class StaticModelHarnessGenRequest(
    val prompt: String,
    val answer_schema: String,
    val max_tokens: UInt,
    val temperature_milli: UShort,
)

/** `sce:result`: the record the host completes `<invoke id="gen">` with (SCE Accepted Subset §2.12). */
data class StaticModelHarnessGenResult(
    val error: String,
    val text: String,
    val constrained: Boolean,
    val model: String,
    val prompt_tokens: UInt,
    val answer_tokens: UInt,
    val finish: String,
) {
    /** The JSON `done.invoke.gen` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "text" to text, "constrained" to constrained, "model" to model, "prompt_tokens" to prompt_tokens, "answer_tokens" to answer_tokens, "finish" to finish))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-model-generate">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostModelGenerateInvoker`.
 */
interface StaticModelHarnessSceHostModelGenerateInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="gen">`. [token] names this start;
     * a host that finishes later hands it back to `completeGen` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startGen(request: StaticModelHarnessGenRequest, token: Long): StaticModelHarnessGenResult?

    /** §scxml-6.4: `<invoke id="gen">`'s state exited while the start [token] names was still running. */
    fun cancelGen(token: Long)
}


class StaticModelHarnessStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticModelHarnessState, StaticModelHarnessEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `offered` datamodel variable, published (`sce:direction="out"`). */
    private var vOffered: List<UInt> = emptyList()

    /** What the published `offered` holds. */
    val offered: List<UInt> get() = vOffered
    /** W3C SCXML 5.2: the `request` datamodel variable, the machine's own. */
    private var vRequest: String = ""
    /** W3C SCXML 5.2: the `prompt` datamodel variable, the machine's own. */
    private var vPrompt: String = ""
    /** W3C SCXML 5.2: the `said` datamodel variable, the machine's own. */
    private var vSaid: String = ""
    /** W3C SCXML 5.2: the `offered_ok` datamodel variable, the machine's own. */
    private var vOfferedOk: Boolean = false
    /** W3C SCXML 5.2: the `refused` datamodel variable, the machine's own. */
    private var vRefused: StaticModelHarnessRefusalRecord = StaticModelHarnessRefusalRecord(code = StaticModelHarnessHarnessCodeEnum.NOT_JSON, path = "")
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: List<StaticModelHarnessRefusalRecord> = emptyList()

    /** What the published `refusals` holds. */
    val refusals: List<StaticModelHarnessRefusalRecord> get() = vRefusals
    /** W3C SCXML 5.2: the `attempts` datamodel variable, published (`sce:direction="out"`). */
    private var vAttempts: UInt = 0.toUInt()

    /** What the published `attempts` holds. */
    val attempts: UInt get() = vAttempts
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    private var vOutcome: String = ""

    /** What the published `outcome` holds. */
    val outcome: String get() = vOutcome
    /** W3C SCXML 5.2: the `answer` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswer: StaticModelHarnessAnswerRecord? = null

    /** What the published `answer` holds. */
    val answer: StaticModelHarnessAnswerRecord? get() = vAnswer?.detached()
    /** W3C SCXML 5.2: the `why` datamodel variable, published (`sce:direction="out"`). */
    private var vWhy: StaticModelHarnessLiftRefusalRecord? = null

    /** What the published `why` holds. */
    val why: StaticModelHarnessLiftRefusalRecord? get() = vWhy

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var request: String? = null
        var prompt: String? = null
        var said: String? = null
        var offeredOk: Boolean? = null
        var attempts: UInt? = null
        var outcome: String? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.request?.let { vRequest = it }
        params.prompt?.let { vPrompt = it }
        params.said?.let { vSaid = it }
        params.offeredOk?.let { vOfferedOk = it }
        params.attempts?.let { vAttempts = it }
        params.outcome?.let { vOutcome = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val offered: List<UInt>,
        val refusals: List<StaticModelHarnessRefusalRecord>,
        val attempts: UInt,
        val outcome: String,
        val answer: StaticModelHarnessAnswerRecord?,
        val why: StaticModelHarnessLiftRefusalRecord?,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticModelHarnessState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        offered = vOffered,
        refusals = vRefusals,
        attempts = vAttempts,
        outcome = vOutcome,
        answer = vAnswer?.detached(),
        why = vWhy,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), currentData(), false)
    )

    /**
     * The machine as the host sees it: one [Snapshot] per completed macrostep
     * (W3C SCXML Appendix D), never a state between two microsteps.
     *
     * Compose integration: `val s by sm.snapshot.collectAsState()`
     */
    val snapshot: kotlinx.coroutines.flow.StateFlow<Snapshot>
        get() = _snapshot

    override fun onMacrostepComplete(truncated: Boolean) {
        _snapshot.value = Snapshot(activeConfiguration, currentData(), truncated)
    }

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "37befbc8acd00383a5b5c22dccd834a45417865a992fce63e4fff9ffba57eea3",
        revised = "71d8d448c8aef08c94b4c434c99fe8b570a2053ef5f397824e9bd8dd1f950a3a",
        format2First = "3b077e2ef9a79afe1e0bdd2b4b4bef2b710cf1f1d0ff92e34196fe799ea64bda",
        format2Revised = "4e941d5a22fb0a84a8f3a01e5bf4846243f1bd294e9e6b5169a55cbd776d73d8",
        records = listOf(
            RecordLineage(
                event = "harness.refusal",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("refused", false),
                    RecordHolder("refusals", true),
                ),
            ),
            RecordLineage(
                event = "lift.answer",
                revision = 2,
                revisions = listOf("346ba2c78ac1d5e5d878a2f364be668e2f2503a0f555b1d323584dc30cd26483"),
                added = listOf(
                    AddedField("priority", 2, "3"),
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("answer", false),
                ),
            ),
            RecordLineage(
                event = "lift.refusal",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("why", false),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_harness_code.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("refused", "code", false),
                    EnumHolder("refusals", "code", true),
                ),
            ),
            EnumLineage(
                key = "enum:enum_lift_kind.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("answer", "kind", false),
                ),
            ),
            EnumLineage(
                key = "enum:sce:std/lift/refusal_code.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("why", "code", false),
                ),
            ),
        ),
    )

    /**
     * This machine's whole state at the macrostep boundary it stands at —
     * every variable, the machine's own included, and where it stands — as
     * the `sce-saved-state` document every backend reads ([SavedState.toJson]).
     * Each delayed `<send>` still waiting is written as the moment it comes due
     * on the wall clock whose reading now is [wallNowMs], in milliseconds since
     * the Unix epoch.
     *
     * @throws StateRefusal for a machine that is not running, or whose last
     *   macrostep stopped at the microstep ceiling.
     */
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "offered" to SavedValues.list(vOffered) { SavedValues.of(it) }
        variables += "request" to SavedValues.of(vRequest)
        variables += "prompt" to SavedValues.of(vPrompt)
        variables += "said" to SavedValues.of(vSaid)
        variables += "offered_ok" to SavedValues.of(vOfferedOk)
        variables += "refused" to vRefused.toSaved()
        variables += "refusals" to SavedValues.list(vRefusals) { it.toSaved() }
        variables += "attempts" to SavedValues.of(vAttempts)
        variables += "outcome" to SavedValues.of(vOutcome)
        variables += "answer" to SavedValues.optional(vAnswer) { it.toSaved() }
        variables += "why" to SavedValues.optional(vWhy) { it.toSaved() }
    }

    /** [save] at the host's wall clock now. */
    fun save(): SavedState = save(SavedState.wallClockMs())

    /**
     * Stand this machine where [saved] left one, in place of [initialize]: no
     * `<onentry>` runs and no `<data>` is evaluated, since the saved run
     * already did both. Every value is read before any is written, so a
     * refused restore leaves the machine as it was.
     *
     * The delayed sends [saved] holds are armed against this machine's `clock`,
     * which is installed before a restore as before [initialize]; [wallNowMs]
     * is what time it is on the wall clock the saved `due`s were written
     * against. A send comes due when its saved moment does, and one already due
     * comes due now.
     *
     * @throws StateRefusal for a machine that has already started, a state
     *   saved from a document of another shape, a configuration that is not
     *   one of this document, or a value its variable's type cannot hold.
     */
    fun restore(saved: SavedState, wallNowMs: Long) {
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val writeVariablesPart0 = restoreVariablesPart0(savedGrown)
        writeVariablesPart0()
        enterSaved(savedGrown, wallNowMs)
    }

    // Reads this part's variables from [savedGrown], refusing a value its
    // variable's type cannot hold, and answers what writes them: nothing is
    // written until every part has read.
    private fun restoreVariablesPart0(savedGrown: SavedState): () -> Unit {
        val saved1 = SavedValues.list(savedGrown.variable("offered"), "offered", 8) { e, w -> SavedValues.uint32(e, w) }
        val saved2 = SavedValues.string(savedGrown.variable("request"), "request", 512)
        val saved3 = SavedValues.string(savedGrown.variable("prompt"), "prompt", 8192)
        val saved4 = SavedValues.string(savedGrown.variable("said"), "said", 4096)
        val saved5 = SavedValues.bool(savedGrown.variable("offered_ok"), "offered_ok")
        val saved6 = StaticModelHarnessRefusalRecord.fromSaved(savedGrown.variable("refused"), "refused")
        val saved7 = SavedValues.list(savedGrown.variable("refusals"), "refusals", 3) { e, w -> StaticModelHarnessRefusalRecord.fromSaved(e, w) }
        val saved8 = SavedValues.uint32(savedGrown.variable("attempts"), "attempts")
        val saved9 = SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        val saved10 = SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticModelHarnessAnswerRecord.fromSaved(e, w) }
        val saved11 = SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticModelHarnessLiftRefusalRecord.fromSaved(e, w) }
        return {
            vOffered = saved1
            vRequest = saved2
            vPrompt = saved3
            vSaid = saved4
            vOfferedOk = saved5
            vRefused = saved6
            vRefusals = saved7
            vAttempts = saved8
            vOutcome = saved9
            vAnswer = saved10
            vWhy = saved11
        }
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        judgeVariablesPart0(savedGrown)
    }

    private fun judgeVariablesPart0(savedGrown: SavedState) {
        SavedValues.list(savedGrown.variable("offered"), "offered", 8) { e, w -> SavedValues.uint32(e, w) }
        SavedValues.string(savedGrown.variable("request"), "request", 512)
        SavedValues.string(savedGrown.variable("prompt"), "prompt", 8192)
        SavedValues.string(savedGrown.variable("said"), "said", 4096)
        SavedValues.bool(savedGrown.variable("offered_ok"), "offered_ok")
        StaticModelHarnessRefusalRecord.fromSaved(savedGrown.variable("refused"), "refused")
        SavedValues.list(savedGrown.variable("refusals"), "refusals", 3) { e, w -> StaticModelHarnessRefusalRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("attempts"), "attempts")
        SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticModelHarnessAnswerRecord.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticModelHarnessLiftRefusalRecord.fromSaved(e, w) }
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticModelHarnessState>> = listOf(
        Triple("sce-host-model-generate", "gen", StaticModelHarnessState.Generate),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeGenPayload: StaticModelHarnessDoneInvokeGenPayload? = null
    private var pendingHarnessOfferPayload: StaticModelHarnessHarnessOfferPayload? = null
    private var pendingHarnessStartPayload: StaticModelHarnessHarnessStartPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticModelHarnessEvent, metadata: EventMetadata) {
        pendingDoneInvokeGenPayload = null
        pendingHarnessOfferPayload = null
        pendingHarnessStartPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticModelHarnessDoneInvokeGenPayload -> pendingDoneInvokeGenPayload = tp
            is StaticModelHarnessHarnessOfferPayload -> pendingHarnessOfferPayload = tp
            is StaticModelHarnessHarnessStartPayload -> pendingHarnessStartPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticModelHarnessEvent.Done.Invoke.Gen) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeGenPayload = StaticModelHarnessDoneInvokeGenPayload(fields.string("error"), fields.string("text"), fields.boolean("constrained"), fields.string("model"), fields.uint32("prompt_tokens"), fields.uint32("answer_tokens"), fields.string("finish"))
                } else if (event == StaticModelHarnessEvent.Harness.Offer) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHarnessOfferPayload = StaticModelHarnessHarnessOfferPayload(fields.uint32("id"))
                } else if (event == StaticModelHarnessEvent.Harness.Start) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHarnessStartPayload = StaticModelHarnessHarnessStartPayload(fields.string("request"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `harness.offer` — binds the event name and the payload field values in one call.
    fun raiseHarnessOffer(id: UInt) {
        send(
            StaticModelHarnessEvent.Harness.Offer,
            EventMetadata(
                type = "external",
                typedPayload = StaticModelHarnessHarnessOfferPayload(id),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `harness.start` — binds the event name and the payload field values in one call.
    fun raiseHarnessStart(request: String) {
        send(
            StaticModelHarnessEvent.Harness.Start,
            EventMetadata(
                type = "external",
                typedPayload = StaticModelHarnessHarnessStartPayload(request),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("request" to request))
            )
        )
    }


    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="gen">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`). An owner-thread operation; from another
     * thread, [postCompleteGen].
     */
    fun completeGen(token: Long, result: StaticModelHarnessGenResult): Boolean =
        completeHostInvoke("sce-host-model-generate", "gen", token, result.wire())

    /**
     * [completeGen] from any thread: the completion is posted to this
     * machine's `inbox`, and its owner applies it with `completeHostInvoke` at
     * its next macrostep boundary, where the token is checked. `true` when the
     * inbox took it.
     */
    fun postCompleteGen(token: Long, result: StaticModelHarnessGenResult): Boolean =
        inbox.post(
            com.sce.runtime.InboxInput.HostInvokeEnding(
                sessionId = null,
                processorType = "sce-host-model-generate",
                invokeId = "gen",
                token = token,
                done = true,
                data = result.wire(),
            ),
        )

    /**
     * Register [invoker] as the handler for `type="sce-host-model-generate"`.
     */
    fun registerSceHostModelGenerateInvoker(invoker: StaticModelHarnessSceHostModelGenerateInvoker) {
        registerInvoker("sce-host-model-generate") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "gen" -> {
                        val typed = StaticModelHarnessGenRequest(
                            prompt = TypedRequest.string(start.params, start.invokeId, "prompt"),
                            answer_schema = TypedRequest.string(start.params, start.invokeId, "answer_schema"),
                            max_tokens = TypedRequest.uint32(start.params, start.invokeId, "max_tokens"),
                            temperature_milli = TypedRequest.uint16(start.params, start.invokeId, "temperature_milli"),
                        )
                        invoker.startGen(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "gen" -> {
                        invoker.cancelGen(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticModelHarnessState = StaticModelHarnessState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: StaticModelHarnessState): StaticModelHarnessState? = when (state) {
        is StaticModelHarnessState.Accepted -> StaticModelHarnessState.Harness
        is StaticModelHarnessState.Check -> StaticModelHarnessState.Work
        is StaticModelHarnessState.Compose -> StaticModelHarnessState.Work
        is StaticModelHarnessState.GaveUp -> StaticModelHarnessState.Harness
        is StaticModelHarnessState.Generate -> StaticModelHarnessState.Work
        is StaticModelHarnessState.Repair -> StaticModelHarnessState.Work
        is StaticModelHarnessState.Retry -> StaticModelHarnessState.Work
        is StaticModelHarnessState.Waiting -> StaticModelHarnessState.Harness
        is StaticModelHarnessState.Work -> StaticModelHarnessState.Harness
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: StaticModelHarnessState): Boolean = when (state) {
        is StaticModelHarnessState.Harness -> true
        is StaticModelHarnessState.Work -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticModelHarnessState): Boolean = when (state) {
        is StaticModelHarnessState.Accepted -> true
        is StaticModelHarnessState.GaveUp -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: StaticModelHarnessState): List<StaticModelHarnessState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: StaticModelHarnessState): List<EntryTarget<StaticModelHarnessState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticModelHarnessState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<StaticModelHarnessState, List<StaticModelHarnessState>> = LinkedHashMap<StaticModelHarnessState, List<StaticModelHarnessState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<StaticModelHarnessState, List<StaticModelHarnessState>>) {
            table[StaticModelHarnessState.Harness] = listOf(StaticModelHarnessState.Waiting, StaticModelHarnessState.Work, StaticModelHarnessState.Accepted, StaticModelHarnessState.GaveUp)
            table[StaticModelHarnessState.Work] = listOf(StaticModelHarnessState.Compose, StaticModelHarnessState.Generate, StaticModelHarnessState.Retry, StaticModelHarnessState.Check, StaticModelHarnessState.Repair)
        }


        val initialTargets: Map<StaticModelHarnessState, List<EntryTarget<StaticModelHarnessState, HistoryId>>> = LinkedHashMap<StaticModelHarnessState, List<EntryTarget<StaticModelHarnessState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<StaticModelHarnessState, List<EntryTarget<StaticModelHarnessState, HistoryId>>>) {
            table[StaticModelHarnessState.Harness] = listOf(StateTarget(StaticModelHarnessState.Waiting))
            table[StaticModelHarnessState.Work] = listOf(StateTarget(StaticModelHarnessState.Compose))
        }


        val documentInitialTargetList: List<EntryTarget<StaticModelHarnessState, HistoryId>> =
            listOf(StateTarget(StaticModelHarnessState.Harness))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: check's transition 0, as the microstep reads it.
        val transitionCheckAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Check,
            listOf(StateTarget(StaticModelHarnessState.Repair)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 1, as the microstep reads it.
        val transitionCheckAt1 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Check,
            listOf(StateTarget(StaticModelHarnessState.Repair)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 2, as the microstep reads it.
        val transitionCheckAt2 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Check,
            listOf(StateTarget(StaticModelHarnessState.Accepted)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: compose's transition 0, as the microstep reads it.
        val transitionComposeAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Compose,
            listOf(StateTarget(StaticModelHarnessState.Generate)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: generate's transition 0, as the microstep reads it.
        val transitionGenerateAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Generate,
            listOf(StateTarget(StaticModelHarnessState.Check)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: generate's transition 1, as the microstep reads it.
        val transitionGenerateAt1 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Generate,
            listOf(StateTarget(StaticModelHarnessState.GaveUp)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: generate's transition 2, as the microstep reads it.
        val transitionGenerateAt2 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Generate,
            listOf(StateTarget(StaticModelHarnessState.Retry)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: generate's transition 3, as the microstep reads it.
        val transitionGenerateAt3 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Generate,
            listOf(StateTarget(StaticModelHarnessState.GaveUp)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: repair's transition 0, as the microstep reads it.
        val transitionRepairAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Repair,
            listOf(StateTarget(StaticModelHarnessState.GaveUp)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: repair's transition 1, as the microstep reads it.
        val transitionRepairAt1 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Repair,
            listOf(StateTarget(StaticModelHarnessState.Generate)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: retry's transition 0, as the microstep reads it.
        val transitionRetryAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Retry,
            listOf(StateTarget(StaticModelHarnessState.GaveUp)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: retry's transition 1, as the microstep reads it.
        val transitionRetryAt1 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Retry,
            listOf(StateTarget(StaticModelHarnessState.Generate)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Waiting,
            listOf(StateTarget(StaticModelHarnessState.Work)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: work's transition 0, as the microstep reads it.
        val transitionWorkAt0 = EnabledTransition<StaticModelHarnessState, HistoryId>(
            StaticModelHarnessState.Work,
            listOf(StateTarget(StaticModelHarnessState.GaveUp)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticModelHarnessState? = when (stateId) {
        "accepted" -> StaticModelHarnessState.Accepted
        "check" -> StaticModelHarnessState.Check
        "compose" -> StaticModelHarnessState.Compose
        "gave_up" -> StaticModelHarnessState.GaveUp
        "generate" -> StaticModelHarnessState.Generate
        "harness" -> StaticModelHarnessState.Harness
        "repair" -> StaticModelHarnessState.Repair
        "retry" -> StaticModelHarnessState.Retry
        "waiting" -> StaticModelHarnessState.Waiting
        "work" -> StaticModelHarnessState.Work
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticModelHarnessState): String = when (state) {
        is StaticModelHarnessState.Accepted -> "accepted"
        is StaticModelHarnessState.Check -> "check"
        is StaticModelHarnessState.Compose -> "compose"
        is StaticModelHarnessState.GaveUp -> "gave_up"
        is StaticModelHarnessState.Generate -> "generate"
        is StaticModelHarnessState.Harness -> "harness"
        is StaticModelHarnessState.Repair -> "repair"
        is StaticModelHarnessState.Retry -> "retry"
        is StaticModelHarnessState.Waiting -> "waiting"
        is StaticModelHarnessState.Work -> "work"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticModelHarnessState): Int = when (state) {
        is StaticModelHarnessState.Accepted -> 8
        is StaticModelHarnessState.Check -> 6
        is StaticModelHarnessState.Compose -> 3
        is StaticModelHarnessState.GaveUp -> 9
        is StaticModelHarnessState.Generate -> 4
        is StaticModelHarnessState.Harness -> 0
        is StaticModelHarnessState.Repair -> 7
        is StaticModelHarnessState.Retry -> 5
        is StaticModelHarnessState.Waiting -> 1
        is StaticModelHarnessState.Work -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticModelHarnessEvent? = when (name) {
        "done.invoke" -> StaticModelHarnessEvent.Done.Invoke.Self
        "done.invoke.gen" -> StaticModelHarnessEvent.Done.Invoke.Gen
        "done.state.harness" -> StaticModelHarnessEvent.Done.State.Harness
        "error.execution" -> StaticModelHarnessEvent.Error.Execution
        "error.invoke" -> StaticModelHarnessEvent.Error.Invoke.Self
        "error.invoke.gen" -> StaticModelHarnessEvent.Error.Invoke.Gen
        "harness.offer" -> StaticModelHarnessEvent.Harness.Offer
        "harness.start" -> StaticModelHarnessEvent.Harness.Start
        "harness.timeout" -> StaticModelHarnessEvent.Harness.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticModelHarnessEvent): String? = when (event) {
        is StaticModelHarnessEvent.Done.Invoke.Self -> "done.invoke"
        is StaticModelHarnessEvent.Done.Invoke.Gen -> "done.invoke.gen"
        is StaticModelHarnessEvent.Done.State.Harness -> "done.state.harness"
        is StaticModelHarnessEvent.Error.Execution -> "error.execution"
        is StaticModelHarnessEvent.Error.Invoke.Self -> "error.invoke"
        is StaticModelHarnessEvent.Error.Invoke.Gen -> "error.invoke.gen"
        is StaticModelHarnessEvent.Harness.Offer -> "harness.offer"
        is StaticModelHarnessEvent.Harness.Start -> "harness.start"
        is StaticModelHarnessEvent.Harness.Timeout -> "harness.timeout"
    }


    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "gen",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when (state) {
        is StaticModelHarnessState.Check -> firstEnabledTransitionOfCheck(state, event)
        is StaticModelHarnessState.Compose -> firstEnabledTransitionOfCompose(state, event)
        is StaticModelHarnessState.Generate -> firstEnabledTransitionOfGenerate(state, event)
        is StaticModelHarnessState.Repair -> firstEnabledTransitionOfRepair(state, event)
        is StaticModelHarnessState.Retry -> firstEnabledTransitionOfRetry(state, event)
        is StaticModelHarnessState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        is StaticModelHarnessState.Work -> firstEnabledTransitionOfWork(state, event)
        else -> null
    }


    // check's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfCheck(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event == null && vWhy != null -> TransitionsPart0.transitionCheckAt0
        event == null && !vOfferedOk -> TransitionsPart0.transitionCheckAt1
        event == null -> TransitionsPart0.transitionCheckAt2
        else -> null
    }


    // compose's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfCompose(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionComposeAt0
        else -> null
    }


    // generate's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGenerate(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event is StaticModelHarnessEvent.Done.Invoke.Gen && pendingDoneInvokeGenPayload != null && (pendingDoneInvokeGenPayload!!.error == "") -> TransitionsPart0.transitionGenerateAt0
        event is StaticModelHarnessEvent.Done.Invoke.Gen -> TransitionsPart0.transitionGenerateAt1
        event is StaticModelHarnessEvent.Error.Invoke.Gen -> TransitionsPart0.transitionGenerateAt2
        event is StaticModelHarnessEvent.Error.Execution -> TransitionsPart0.transitionGenerateAt3
        else -> null
    }


    // repair's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRepair(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event == null && vAttempts >= 3.toUInt() -> TransitionsPart0.transitionRepairAt0
        event == null -> TransitionsPart0.transitionRepairAt1
        else -> null
    }


    // retry's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRetry(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event == null && vAttempts >= 3.toUInt() -> TransitionsPart0.transitionRetryAt0
        event == null -> TransitionsPart0.transitionRetryAt1
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event is StaticModelHarnessEvent.Harness.Offer -> TransitionsPart0.transitionWaitingAt0
        event is StaticModelHarnessEvent.Harness.Start -> TransitionsPart0.transitionWaitingAt1
        else -> null
    }


    // work's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWork(state: StaticModelHarnessState, event: StaticModelHarnessEvent?): EnabledTransition<StaticModelHarnessState, HistoryId>? = when {
        event is StaticModelHarnessEvent.Harness.Timeout -> TransitionsPart0.transitionWorkAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_model_harness.scxml:67 :: _machine
    override fun onEntry(state: StaticModelHarnessState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticModelHarnessState.Accepted -> onEntryOfAccepted()
            is StaticModelHarnessState.Check -> onEntryOfCheck()
            is StaticModelHarnessState.Compose -> onEntryOfCompose()
            is StaticModelHarnessState.GaveUp -> onEntryOfGaveUp()
            is StaticModelHarnessState.Generate -> onEntryOfGenerate()
            is StaticModelHarnessState.Harness -> onEntryOfHarness()
            is StaticModelHarnessState.Repair -> onEntryOfRepair()
            is StaticModelHarnessState.Retry -> onEntryOfRetry()
            is StaticModelHarnessState.Waiting -> onEntryOfWaiting()
            is StaticModelHarnessState.Work -> onEntryOfWork()
        }
    }


    private fun onEntryOfAccepted() {
                // SCE-MAP: static_model_harness.scxml:208 :: accepted :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(StaticModelHarnessEvent.Done.State.Harness, EventMetadata.platform())
    }

    private fun onEntryOfCheck() {
                // SCE-MAP: static_model_harness.scxml:161 :: check :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            vOfferedOk = false


            if (vAnswer != null) {


            for (offer in vOffered) {


            if (offer == com.sce.forge.runtime.SceOptional.proven(vAnswer).id) {

            vOfferedOk = true
            }
            }
            }
                }
    }

    private fun onEntryOfCompose() {
                // SCE-MAP: static_model_harness.scxml:119 :: compose :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { vPrompt = com.sce.forge.runtime.SceChecked.bounded(HarnessAsk.render(vRequest), 8192); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<sce:render template='Ask'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
    }

    private fun onEntryOfGaveUp() {
                // SCE-MAP: static_model_harness.scxml:209 :: gave_up :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(StaticModelHarnessEvent.Done.State.Harness, EventMetadata.platform())
    }

    private fun onEntryOfGenerate() {
                // SCE-MAP: static_model_harness.scxml:128 :: generate :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "generate.${System.identityHashCode(this)}.gen"
                    deferInvoke(StaticModelHarnessState.Generate, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        // SCE Accepted Subset 2.12: `sce:request` makes the
                        // params a record, so each value is held to its field
                        // here, where the invocation starts; one that cannot
                        // be evaluated or does not fit starts nothing.
                        run {
                            val v: Any? = vPrompt
                            hostInvokeParams["prompt"] =
                                (hostInvokeParams["prompt"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "prompt", v)
                        }
                        run {
                            val v: Any? = (256).toLong()
                            hostInvokeParams["max_tokens"] =
                                (hostInvokeParams["max_tokens"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "max_tokens", v)
                        }
                        run {
                            val v: Any? = (0).toLong()
                            hostInvokeParams["temperature_milli"] =
                                (hostInvokeParams["temperature_milli"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "temperature_milli", v)
                        }
                        run {
                            val v: Any? = (8000).toLong()
                            hostInvokeParams["_sce_deadline_ms"] =
                                (hostInvokeParams["_sce_deadline_ms"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "_sce_deadline_ms", v)
                        }
                        hostInvokeParams["answer_schema"] =
                            (hostInvokeParams["answer_schema"] ?: emptyList()) + "urn:sce:schema:lift_answer:2"
                        putParam(hostInvokePayload, "answer_schema", "urn:sce:schema:lift_answer:2")
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-model-generate",
                                invokeId = "gen",
                                src = "",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
    }

    private fun onEntryOfHarness() {
                // SCE-MAP: static_model_harness.scxml:98 :: harness :: _state_body
    }

    private fun onEntryOfRepair() {
                // SCE-MAP: static_model_harness.scxml:191 :: repair :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { vAttempts = com.sce.forge.runtime.SceChecked.add(vAttempts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='attempts'>: an integer operation overflowed or failed"); true }) {
                return@run
            }

            if (if (vRefusals.size < 3) { vRefusals = vRefusals + (vRefused); false } else { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<sce:append target='refusals'>: the list already holds its capacity of 3"); true }) {
                return@run
            }

            if (try { vPrompt = com.sce.forge.runtime.SceChecked.bounded(HarnessRepair.render((vRefused.code).declaredName, vRefused.path, vSaid), 8192); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<sce:render template='Repair'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
    }

    private fun onEntryOfRetry() {
                // SCE-MAP: static_model_harness.scxml:151 :: retry :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { vAttempts = com.sce.forge.runtime.SceChecked.add(vAttempts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='attempts'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: static_model_harness.scxml:99 :: waiting :: _state_body
    }

    private fun onEntryOfWork() {
                // SCE-MAP: static_model_harness.scxml:108 :: work :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("overall", 20000L, StaticModelHarnessEvent.Harness.Timeout, EventMetadata.sentBy(sessionId, sendId = "overall", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_model_harness.scxml:67 :: _machine
    override fun onExit(state: StaticModelHarnessState) {
        when (state) {
            is StaticModelHarnessState.Accepted -> onExitOfAccepted()
            is StaticModelHarnessState.Check -> onExitOfCheck()
            is StaticModelHarnessState.Compose -> onExitOfCompose()
            is StaticModelHarnessState.GaveUp -> onExitOfGaveUp()
            is StaticModelHarnessState.Generate -> onExitOfGenerate()
            is StaticModelHarnessState.Harness -> onExitOfHarness()
            is StaticModelHarnessState.Repair -> onExitOfRepair()
            is StaticModelHarnessState.Retry -> onExitOfRetry()
            is StaticModelHarnessState.Waiting -> onExitOfWaiting()
            is StaticModelHarnessState.Work -> onExitOfWork()
        }
    }


    private fun onExitOfAccepted() {
                // SCE-MAP: static_model_harness.scxml:208 :: accepted :: _state_body
    }

    private fun onExitOfCheck() {
                // SCE-MAP: static_model_harness.scxml:161 :: check :: _state_body
    }

    private fun onExitOfCompose() {
                // SCE-MAP: static_model_harness.scxml:119 :: compose :: _state_body
    }

    private fun onExitOfGaveUp() {
                // SCE-MAP: static_model_harness.scxml:209 :: gave_up :: _state_body
    }

    private fun onExitOfGenerate() {
                // SCE-MAP: static_model_harness.scxml:128 :: generate :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(StaticModelHarnessState.Generate)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-model-generate", "gen")
    }

    private fun onExitOfHarness() {
                // SCE-MAP: static_model_harness.scxml:98 :: harness :: _state_body
    }

    private fun onExitOfRepair() {
                // SCE-MAP: static_model_harness.scxml:191 :: repair :: _state_body
    }

    private fun onExitOfRetry() {
                // SCE-MAP: static_model_harness.scxml:151 :: retry :: _state_body
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: static_model_harness.scxml:99 :: waiting :: _state_body
    }

    private fun onExitOfWork() {
                // SCE-MAP: static_model_harness.scxml:108 :: work :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            cancelSend("overall")
                }
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_model_harness.scxml:67 :: _machine
    override fun executeTransitionContent(source: StaticModelHarnessState, transitionIndex: Int) {
        when (source) {
            is StaticModelHarnessState.Check -> transitionContentOfCheck(transitionIndex)
            is StaticModelHarnessState.Generate -> transitionContentOfGenerate(transitionIndex)
            is StaticModelHarnessState.Repair -> transitionContentOfRepair(transitionIndex)
            is StaticModelHarnessState.Retry -> transitionContentOfRetry(transitionIndex)
            is StaticModelHarnessState.Waiting -> transitionContentOfWaiting(transitionIndex)
            is StaticModelHarnessState.Work -> transitionContentOfWork(transitionIndex)
            else -> {}
        }
    }


    // The content of `check`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfCheck(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfCheckAt0()
            1 -> transitionContentOfCheckAt1()
            2 -> transitionContentOfCheckAt2()
            else -> {}
        }
    }


    private fun transitionContentOfCheckAt0() {
                // SCE-MAP: static_model_harness.scxml:175 :: check :: _transition_0


            if (vWhy != null) {

            if (try { vRefused = vRefused.copy(code = com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(StaticModelHarnessHarnessCodeEnum.fromUnderlying((StaticModelHarnessLiftRefusalCodeEnum::toUnderlying)(com.sce.forge.runtime.SceOptional.proven(vWhy).code))))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='refused.code'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vRefused = vRefused.copy(path = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceOptional.proven(vWhy).path, 256)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='refused.path'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfCheckAt1() {
                // SCE-MAP: static_model_harness.scxml:182 :: check :: _transition_1

            vRefused = vRefused.copy(code = StaticModelHarnessHarnessCodeEnum.NOT_OFFERED)

            if (try { vRefused = vRefused.copy(path = com.sce.forge.runtime.SceChecked.bounded("/id", 256)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='refused.path'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfCheckAt2() {
                // SCE-MAP: static_model_harness.scxml:186 :: check :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("accepted", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    // The content of `generate`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfGenerate(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfGenerateAt0()
            1 -> transitionContentOfGenerateAt1()
            3 -> transitionContentOfGenerateAt3()
            else -> {}
        }
    }


    private fun transitionContentOfGenerateAt0() {
                // SCE-MAP: static_model_harness.scxml:136 :: generate :: _transition_0
                if (pendingDoneInvokeGenPayload == null) {
                    return
                }

            if (try { vSaid = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGenPayload!!.text, 4096); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='said'>: an integer operation overflowed or failed"); true }) {
                return
            }

            when (val sceLift = com.sce.runtime.EventPayload.liftRecord(vSaid, 1, 256, "{\"priority\":3}") { r -> StaticModelHarnessAnswerRecord(id = r.field("id").let { v -> v.uint32() }, amount = r.field("amount").let { v -> v.int64() }, title = r.field("title").let { v -> v.string(16) }, kind = r.field("kind").let { v -> v.name().let { name -> when (name) { "meeting" -> StaticModelHarnessEnumLiftKindEnum.MEETING; "task" -> StaticModelHarnessEnumLiftKindEnum.TASK; else -> throw v.unknownVariant(name, "enum_lift_kind") } } }, note = r.field("note").let { v -> v.optional { present -> present.string(8) } }, tag = r.field("tag").let { v -> v.bytes(4) }, priority = r.field("priority").let { v -> v.uint8() }) }) { is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vAnswer = sceLift.value; vWhy = null }; is com.sce.runtime.EventPayload.LiftOutcome.Refused -> { vWhy = StaticModelHarnessLiftRefusalRecord(code = when (sceLift.refusal.code) { com.sce.runtime.EventPayload.RefusalCode.NOT_JSON -> StaticModelHarnessLiftRefusalCodeEnum.NOT_JSON; com.sce.runtime.EventPayload.RefusalCode.NOT_OBJECT -> StaticModelHarnessLiftRefusalCodeEnum.NOT_OBJECT; com.sce.runtime.EventPayload.RefusalCode.TOO_DEEP -> StaticModelHarnessLiftRefusalCodeEnum.TOO_DEEP; com.sce.runtime.EventPayload.RefusalCode.MISSING -> StaticModelHarnessLiftRefusalCodeEnum.MISSING; com.sce.runtime.EventPayload.RefusalCode.NULL_NOT_OPTIONAL -> StaticModelHarnessLiftRefusalCodeEnum.NULL_NOT_OPTIONAL; com.sce.runtime.EventPayload.RefusalCode.WRONG_KIND -> StaticModelHarnessLiftRefusalCodeEnum.WRONG_KIND; com.sce.runtime.EventPayload.RefusalCode.BAD_NUMBER -> StaticModelHarnessLiftRefusalCodeEnum.BAD_NUMBER; com.sce.runtime.EventPayload.RefusalCode.OUT_OF_RANGE -> StaticModelHarnessLiftRefusalCodeEnum.OUT_OF_RANGE; com.sce.runtime.EventPayload.RefusalCode.TOO_LONG -> StaticModelHarnessLiftRefusalCodeEnum.TOO_LONG; com.sce.runtime.EventPayload.RefusalCode.TOO_MANY -> StaticModelHarnessLiftRefusalCodeEnum.TOO_MANY; com.sce.runtime.EventPayload.RefusalCode.UNKNOWN_VARIANT -> StaticModelHarnessLiftRefusalCodeEnum.UNKNOWN_VARIANT; com.sce.runtime.EventPayload.RefusalCode.NOT_BYTE_TEXT -> StaticModelHarnessLiftRefusalCodeEnum.NOT_BYTE_TEXT; com.sce.runtime.EventPayload.RefusalCode.NUL_IN_TEXT -> StaticModelHarnessLiftRefusalCodeEnum.NUL_IN_TEXT; com.sce.runtime.EventPayload.RefusalCode.TOO_FEW -> StaticModelHarnessLiftRefusalCodeEnum.TOO_FEW; }, path = sceLift.refusal.pointer, bound = sceLift.refusal.bound?.toUInt()) } }
    }

    private fun transitionContentOfGenerateAt1() {
                // SCE-MAP: static_model_harness.scxml:140 :: generate :: _transition_1
                if (pendingDoneInvokeGenPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGenPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfGenerateAt3() {
                // SCE-MAP: static_model_harness.scxml:146 :: generate :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("invalid_argument", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    // The content of `repair`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRepair(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRepairAt0()
            else -> {}
        }
    }


    private fun transitionContentOfRepairAt0() {
                // SCE-MAP: static_model_harness.scxml:201 :: repair :: _transition_0

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("exhausted", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    // The content of `retry`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRetry(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRetryAt0()
            else -> {}
        }
    }


    private fun transitionContentOfRetryAt0() {
                // SCE-MAP: static_model_harness.scxml:155 :: retry :: _transition_0

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("exhausted", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    // The content of `waiting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWaiting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWaitingAt0()
            1 -> transitionContentOfWaitingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfWaitingAt0() {
                // SCE-MAP: static_model_harness.scxml:100 :: waiting :: _transition_0
                if (pendingHarnessOfferPayload == null) {
                    return
                }

            if (if (vOffered.size < 8) { vOffered = vOffered + (pendingHarnessOfferPayload!!.id); false } else { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<sce:append target='offered'>: the list already holds its capacity of 8"); true }) {
                return
            }
    }

    private fun transitionContentOfWaitingAt1() {
                // SCE-MAP: static_model_harness.scxml:103 :: waiting :: _transition_1
                if (pendingHarnessStartPayload == null) {
                    return
                }

            if (try { vRequest = com.sce.forge.runtime.SceChecked.bounded(pendingHarnessStartPayload!!.request, 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='request'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    // The content of `work`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWork(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWorkAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWorkAt0() {
                // SCE-MAP: static_model_harness.scxml:115 :: work :: _transition_0

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("timeout", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticModelHarnessEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
