// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_model.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_model.scxml:33 :: _machine

package com.sce.integration.static_host_model

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostModelState : State {
    data object Generating : StaticHostModelState
    data object Idle : StaticHostModelState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostModelEvent : Event {
    sealed interface Done : StaticHostModelEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Gen : Invoke
        }
    }
    sealed interface Error : StaticHostModelEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Self : Invoke
            data object Gen : Invoke
        }
    }
    sealed interface Host : StaticHostModelEvent {
        sealed interface Model : Host {
            sealed interface Generate : Model {
                data object Request : Generate
            }
        }
    }
    sealed interface Model : StaticHostModelEvent {
        data object Leave : Model
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
object StaticHostModelAnswerSchemas {
    const val ID_0: String = "urn:sce:schema:lift_answer:2"
    const val JSON_SCHEMA_0: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:lift_answer:2\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])\$\"},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"kind\":{\"enum\":[\"meeting\",\"task\"]},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]},\"tag\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*\$\"},\"priority\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"id\",\"amount\",\"title\",\"kind\",\"note\",\"tag\",\"priority\"],\"additionalProperties\":false}"
    const val GBNF_0: String = "root ::= \"{\\\"id\\\":\" uint32 \",\\\"amount\\\":\" int64 \",\\\"title\\\":\" \"\\\"\" t16 \"\\\"\" \",\\\"kind\\\":\" ( \"\\\"meeting\\\"\" | \"\\\"task\\\"\" ) \",\\\"note\\\":\" ( \"null\" | \"\\\"\" t8 \"\\\"\" ) \",\\\"tag\\\":\" \"\\\"\" b4 \"\\\"\" \",\\\"priority\\\":\" uint8 \"}\"\nint64 ::= \"\\\"\" ( \"-\" ( [1-9] [0-9]{0,17} | [1-8] [0-9]{18} | \"9\" [0-1] [0-9]{17} | \"92\" [0-1] [0-9]{16} | \"922\" [0-2] [0-9]{15} | \"9223\" [0-2] [0-9]{14} | \"92233\" [0-6] [0-9]{13} | \"922337\" [0-1] [0-9]{12} | \"92233720\" [0-2] [0-9]{10} | \"922337203\" [0-5] [0-9]{9} | \"9223372036\" [0-7] [0-9]{8} | \"92233720368\" [0-4] [0-9]{7} | \"922337203685\" [0-3] [0-9]{6} | \"9223372036854\" [0-6] [0-9]{5} | \"92233720368547\" [0-6] [0-9]{4} | \"922337203685477\" [0-4] [0-9]{3} | \"9223372036854775\" [0-7] [0-9]{2} | \"922337203685477580\" [0-8] ) | \"0\" | [1-9] [0-9]{0,17} | [1-8] [0-9]{18} | \"9\" [0-1] [0-9]{17} | \"92\" [0-1] [0-9]{16} | \"922\" [0-2] [0-9]{15} | \"9223\" [0-2] [0-9]{14} | \"92233\" [0-6] [0-9]{13} | \"922337\" [0-1] [0-9]{12} | \"92233720\" [0-2] [0-9]{10} | \"922337203\" [0-5] [0-9]{9} | \"9223372036\" [0-7] [0-9]{8} | \"92233720368\" [0-4] [0-9]{7} | \"922337203685\" [0-3] [0-9]{6} | \"9223372036854\" [0-6] [0-9]{5} | \"92233720368547\" [0-6] [0-9]{4} | \"922337203685477\" [0-4] [0-9]{3} | \"9223372036854775\" [0-7] [0-9]{2} | \"922337203685477580\" [0-7] ) \"\\\"\"\nuint32 ::= \"0\" | [1-9] [0-9]{0,8} | [1-3] [0-9]{9} | \"4\" [0-1] [0-9]{8} | \"42\" [0-8] [0-9]{7} | \"429\" [0-3] [0-9]{6} | \"4294\" [0-8] [0-9]{5} | \"42949\" [0-5] [0-9]{4} | \"429496\" [0-6] [0-9]{3} | \"4294967\" [0-1] [0-9]{2} | \"42949672\" [0-8] [0-9] | \"429496729\" [0-5]\nuint8 ::= \"0\" | [1-9] [0-9]{0,1} | \"1\" [0-9]{2} | \"2\" [0-4] [0-9] | \"25\" [0-5]\nbyte ::= [\\x20-\\x21\\x23-\\x5b\\x5d-\\u00ff] | \"\\\\\" [\\\"\\\\bfnrt] | \"\\\\u000\" [0-7bef] | \"\\\\u001\" [0-9a-f]\nb0 ::= \"\"\nb1 ::= \"\" | byte b0\nb2 ::= \"\" | byte b1\nb3 ::= \"\" | byte b2\nb4 ::= \"\" | byte b3\nc1 ::= [\\x20-\\x21\\x23-\\x5b\\x5d-\\x7f] | \"\\\\\" [\\\"\\\\bfnrt] | \"\\\\u000\" [1-7bef] | \"\\\\u001\" [0-9a-f]\nc2 ::= [\\u0080-\\u07ff]\nc3 ::= [\\u0800-\\ud7ff\\ue000-\\uffff]\nc4 ::= [\\U00010000-\\U0010ffff]\nt0 ::= \"\"\nt1 ::= \"\" | c1 t0\nt2 ::= \"\" | c1 t1 | c2 t0\nt3 ::= \"\" | c1 t2 | c2 t1 | c3 t0\nt4 ::= \"\" | c1 t3 | c2 t2 | c3 t1 | c4 t0\nt5 ::= \"\" | c1 t4 | c2 t3 | c3 t2 | c4 t1\nt6 ::= \"\" | c1 t5 | c2 t4 | c3 t3 | c4 t2\nt7 ::= \"\" | c1 t6 | c2 t5 | c3 t4 | c4 t3\nt8 ::= \"\" | c1 t7 | c2 t6 | c3 t5 | c4 t4\nt9 ::= \"\" | c1 t8 | c2 t7 | c3 t6 | c4 t5\nt10 ::= \"\" | c1 t9 | c2 t8 | c3 t7 | c4 t6\nt11 ::= \"\" | c1 t10 | c2 t9 | c3 t8 | c4 t7\nt12 ::= \"\" | c1 t11 | c2 t10 | c3 t9 | c4 t8\nt13 ::= \"\" | c1 t12 | c2 t11 | c3 t10 | c4 t9\nt14 ::= \"\" | c1 t13 | c2 t12 | c3 t11 | c4 t10\nt15 ::= \"\" | c1 t14 | c2 t13 | c3 t12 | c4 t11\nt16 ::= \"\" | c1 t15 | c2 t14 | c3 t13 | c4 t12\n"

    /** Every answer schema of the machine, in the order the document first names it. */
    val all: List<AnswerSchema> = listOf(
        AnswerSchema(ID_0, JSON_SCHEMA_0, GBNF_0),
    )
}

// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticHostModelPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticHostModelDoneInvokeGenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.gen`. Consumers inject it via the `raiseDoneInvokeGen` seam
// on the machine — they never name this class directly.
data class StaticHostModelDoneInvokeGenPayload(val error: String, val text: String, val constrained: Boolean, val model: String, val prompt_tokens: UInt, val answer_tokens: UInt, val finish: String)

// StaticHostModelHostModelGenerateRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.model.generate.request`. Consumers inject it via the `raiseHostModelGenerateRequest` seam
// on the machine — they never name this class directly.
data class StaticHostModelHostModelGenerateRequestPayload(val prompt: String, val answer_schema: String, val max_tokens: UInt, val temperature_milli: UShort)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:enum_lift_kind` datamodel value. */
enum class StaticHostModelEnumLiftKindEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticHostModelEnumLiftKindEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of enum_lift_kind")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticHostModelEnumLiftKindEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Answer` datamodel value. */
data class StaticHostModelAnswerRecord(val id: UInt, val amount: Long, val title: String, val kind: StaticHostModelEnumLiftKindEnum, val note: String?, val tag: ByteArray, val priority: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "amount" to SavedValues.of(amount), "title" to SavedValues.of(title), "kind" to kind.toSaved(), "note" to SavedValues.optional(note) { SavedValues.of(it) }, "tag" to SavedValues.of(tag), "priority" to SavedValues.of(priority))

    override fun equals(other: Any?): Boolean =
        other is StaticHostModelAnswerRecord && id == other.id && amount == other.amount && title == other.title && kind == other.kind && note == other.note && tag.contentEquals(other.tag) && priority == other.priority

    override fun hashCode(): Int = 31 * (31 * (31 * (31 * (31 * (31 * (31 * (0) + id.hashCode()) + amount.hashCode()) + title.hashCode()) + kind.hashCode()) + note.hashCode()) + tag.contentHashCode()) + priority.hashCode()

    /** This value with a copy of each byte string, which a host may write into. */
    fun detached(): StaticHostModelAnswerRecord = copy(tag = tag.copyOf())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticHostModelAnswerRecord = StaticHostModelAnswerRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), amount = SavedValues.int64(SavedValues.field(value, what, "amount"), "$what.amount"), title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), kind = StaticHostModelEnumLiftKindEnum.fromSaved(SavedValues.field(value, what, "kind"), "$what.kind"), note = SavedValues.optional(SavedValues.field(value, what, "note"), "$what.note") { e, w -> SavedValues.string(e, w, 8) }, tag = SavedValues.bytes(SavedValues.field(value, what, "tag"), "$what.tag", 4), priority = SavedValues.uint8(SavedValues.field(value, what, "priority"), "$what.priority"))
    }
}
/** SCE Accepted Subset §2.15: an `enum:refusal_code` datamodel value. */
enum class StaticHostModelRefusalCodeEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticHostModelRefusalCodeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of refusal_code")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticHostModelRefusalCodeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:LiftRefusal` datamodel value. */
data class StaticHostModelLiftRefusalRecord(val code: StaticHostModelRefusalCodeEnum, val path: String, val bound: UInt?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("code" to code.toSaved(), "path" to SavedValues.of(path), "bound" to SavedValues.optional(bound) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticHostModelLiftRefusalRecord = StaticHostModelLiftRefusalRecord(code = StaticHostModelRefusalCodeEnum.fromSaved(SavedValues.field(value, what, "code"), "$what.code"), path = SavedValues.string(SavedValues.field(value, what, "path"), "$what.path", 256), bound = SavedValues.optional(SavedValues.field(value, what, "bound"), "$what.bound") { e, w -> SavedValues.uint32(e, w) })
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object StaticHostModelLimits {
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
data class StaticHostModelGenRequest(
    val prompt: String,
    val answer_schema: String,
    val max_tokens: UInt,
    val temperature_milli: UShort,
)

/** `sce:result`: the record the host completes `<invoke id="gen">` with (SCE Accepted Subset §2.12). */
data class StaticHostModelGenResult(
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
interface StaticHostModelSceHostModelGenerateInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="gen">`. [token] names this start;
     * a host that finishes later hands it back to `completeGen` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startGen(request: StaticHostModelGenRequest, token: Long): StaticHostModelGenResult?

    /** §scxml-6.4: `<invoke id="gen">`'s state exited while the start [token] names was still running. */
    fun cancelGen(token: Long)
}


class StaticHostModelStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticHostModelState, StaticHostModelEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `prompt` datamodel variable, the machine's own. */
    private var vPrompt: String = ""
    /** W3C SCXML 5.2: the `max_tokens` datamodel variable, the machine's own. */
    private var vMaxTokens: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `temperature_milli` datamodel variable, the machine's own. */
    private var vTemperatureMilli: UShort = 0.toUShort()
    /** W3C SCXML 5.2: the `calls` datamodel variable, published (`sce:direction="out"`). */
    private var vCalls: UInt = 0.toUInt()

    /** What the published `calls` holds. */
    val calls: UInt get() = vCalls
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    private var vOutcome: String = ""

    /** What the published `outcome` holds. */
    val outcome: String get() = vOutcome
    /** W3C SCXML 5.2: the `finish` datamodel variable, published (`sce:direction="out"`). */
    private var vFinish: String = ""

    /** What the published `finish` holds. */
    val finish: String get() = vFinish
    /** W3C SCXML 5.2: the `constrained` datamodel variable, published (`sce:direction="out"`). */
    private var vConstrained: Boolean = false

    /** What the published `constrained` holds. */
    val constrained: Boolean get() = vConstrained
    /** W3C SCXML 5.2: the `answered_by` datamodel variable, published (`sce:direction="out"`). */
    private var vAnsweredBy: String = ""

    /** What the published `answered_by` holds. */
    val answeredBy: String get() = vAnsweredBy
    /** W3C SCXML 5.2: the `prompt_tokens` datamodel variable, published (`sce:direction="out"`). */
    private var vPromptTokens: UInt = 0.toUInt()

    /** What the published `prompt_tokens` holds. */
    val promptTokens: UInt get() = vPromptTokens
    /** W3C SCXML 5.2: the `answer_tokens` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswerTokens: UInt = 0.toUInt()

    /** What the published `answer_tokens` holds. */
    val answerTokens: UInt get() = vAnswerTokens
    /** W3C SCXML 5.2: the `answer` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswer: StaticHostModelAnswerRecord? = null

    /** What the published `answer` holds. */
    val answer: StaticHostModelAnswerRecord? get() = vAnswer?.detached()
    /** W3C SCXML 5.2: the `why` datamodel variable, published (`sce:direction="out"`). */
    private var vWhy: StaticHostModelLiftRefusalRecord? = null

    /** What the published `why` holds. */
    val why: StaticHostModelLiftRefusalRecord? get() = vWhy

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var prompt: String? = null
        var maxTokens: UInt? = null
        var temperatureMilli: UShort? = null
        var calls: UInt? = null
        var outcome: String? = null
        var finish: String? = null
        var constrained: Boolean? = null
        var answeredBy: String? = null
        var promptTokens: UInt? = null
        var answerTokens: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.prompt?.let { vPrompt = it }
        params.maxTokens?.let { vMaxTokens = it }
        params.temperatureMilli?.let { vTemperatureMilli = it }
        params.calls?.let { vCalls = it }
        params.outcome?.let { vOutcome = it }
        params.finish?.let { vFinish = it }
        params.constrained?.let { vConstrained = it }
        params.answeredBy?.let { vAnsweredBy = it }
        params.promptTokens?.let { vPromptTokens = it }
        params.answerTokens?.let { vAnswerTokens = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val calls: UInt,
        val outcome: String,
        val finish: String,
        val constrained: Boolean,
        val answeredBy: String,
        val promptTokens: UInt,
        val answerTokens: UInt,
        val answer: StaticHostModelAnswerRecord?,
        val why: StaticHostModelLiftRefusalRecord?,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostModelState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        calls = vCalls,
        outcome = vOutcome,
        finish = vFinish,
        constrained = vConstrained,
        answeredBy = vAnsweredBy,
        promptTokens = vPromptTokens,
        answerTokens = vAnswerTokens,
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
        first = "71b7e2f6efe67b97423eada78502bf45f45666ca41178d66f083b83af856f487",
        revised = "b9e0109fe02a6b471d723b9067e6b78c6feb6b5a578e316efd93015a1531bdb2",
        format2First = "529ec77bbc60bbfcdb3dfbe1ae5f8ce2e34deec20ed0ed495741a6252344b702",
        format2Revised = "73a09bac29295fb1f7d5d1dcbe20adf2371d7e658b9347ead7caad59e9f1d1ef",
        records = listOf(
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
        variables += "prompt" to SavedValues.of(vPrompt)
        variables += "max_tokens" to SavedValues.of(vMaxTokens)
        variables += "temperature_milli" to SavedValues.of(vTemperatureMilli)
        variables += "calls" to SavedValues.of(vCalls)
        variables += "outcome" to SavedValues.of(vOutcome)
        variables += "finish" to SavedValues.of(vFinish)
        variables += "constrained" to SavedValues.of(vConstrained)
        variables += "answered_by" to SavedValues.of(vAnsweredBy)
        variables += "prompt_tokens" to SavedValues.of(vPromptTokens)
        variables += "answer_tokens" to SavedValues.of(vAnswerTokens)
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
        val saved1 = SavedValues.string(savedGrown.variable("prompt"), "prompt", 8192)
        val saved2 = SavedValues.uint32(savedGrown.variable("max_tokens"), "max_tokens")
        val saved3 = SavedValues.uint16(savedGrown.variable("temperature_milli"), "temperature_milli")
        val saved4 = SavedValues.uint32(savedGrown.variable("calls"), "calls")
        val saved5 = SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        val saved6 = SavedValues.string(savedGrown.variable("finish"), "finish", 16)
        val saved7 = SavedValues.bool(savedGrown.variable("constrained"), "constrained")
        val saved8 = SavedValues.string(savedGrown.variable("answered_by"), "answered_by", 128)
        val saved9 = SavedValues.uint32(savedGrown.variable("prompt_tokens"), "prompt_tokens")
        val saved10 = SavedValues.uint32(savedGrown.variable("answer_tokens"), "answer_tokens")
        val saved11 = SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticHostModelAnswerRecord.fromSaved(e, w) }
        val saved12 = SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticHostModelLiftRefusalRecord.fromSaved(e, w) }
        return {
            vPrompt = saved1
            vMaxTokens = saved2
            vTemperatureMilli = saved3
            vCalls = saved4
            vOutcome = saved5
            vFinish = saved6
            vConstrained = saved7
            vAnsweredBy = saved8
            vPromptTokens = saved9
            vAnswerTokens = saved10
            vAnswer = saved11
            vWhy = saved12
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
        SavedValues.string(savedGrown.variable("prompt"), "prompt", 8192)
        SavedValues.uint32(savedGrown.variable("max_tokens"), "max_tokens")
        SavedValues.uint16(savedGrown.variable("temperature_milli"), "temperature_milli")
        SavedValues.uint32(savedGrown.variable("calls"), "calls")
        SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        SavedValues.string(savedGrown.variable("finish"), "finish", 16)
        SavedValues.bool(savedGrown.variable("constrained"), "constrained")
        SavedValues.string(savedGrown.variable("answered_by"), "answered_by", 128)
        SavedValues.uint32(savedGrown.variable("prompt_tokens"), "prompt_tokens")
        SavedValues.uint32(savedGrown.variable("answer_tokens"), "answer_tokens")
        SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticHostModelAnswerRecord.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticHostModelLiftRefusalRecord.fromSaved(e, w) }
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticHostModelState>> = listOf(
        Triple("sce-host-model-generate", "gen", StaticHostModelState.Generating),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeGenPayload: StaticHostModelDoneInvokeGenPayload? = null
    private var pendingHostModelGenerateRequestPayload: StaticHostModelHostModelGenerateRequestPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticHostModelEvent, metadata: EventMetadata) {
        pendingDoneInvokeGenPayload = null
        pendingHostModelGenerateRequestPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticHostModelDoneInvokeGenPayload -> pendingDoneInvokeGenPayload = tp
            is StaticHostModelHostModelGenerateRequestPayload -> pendingHostModelGenerateRequestPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticHostModelEvent.Done.Invoke.Gen) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeGenPayload = StaticHostModelDoneInvokeGenPayload(fields.string("error"), fields.string("text"), fields.boolean("constrained"), fields.string("model"), fields.uint32("prompt_tokens"), fields.uint32("answer_tokens"), fields.string("finish"))
                } else if (event == StaticHostModelEvent.Host.Model.Generate.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostModelGenerateRequestPayload = StaticHostModelHostModelGenerateRequestPayload(fields.string("prompt"), fields.string("answer_schema"), fields.uint32("max_tokens"), fields.uint16("temperature_milli"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.model.generate.request` — binds the event name and the payload field values in one call.
    fun raiseHostModelGenerateRequest(prompt: String, answer_schema: String, max_tokens: UInt, temperature_milli: UShort) {
        send(
            StaticHostModelEvent.Host.Model.Generate.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostModelHostModelGenerateRequestPayload(prompt, answer_schema, max_tokens, temperature_milli),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("prompt" to prompt, "answer_schema" to answer_schema, "max_tokens" to max_tokens, "temperature_milli" to temperature_milli))
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
    fun completeGen(token: Long, result: StaticHostModelGenResult): Boolean =
        completeHostInvoke("sce-host-model-generate", "gen", token, result.wire())

    /**
     * [completeGen] from any thread: the completion is posted to this
     * machine's `inbox`, and its owner applies it with `completeHostInvoke` at
     * its next macrostep boundary, where the token is checked. `true` when the
     * inbox took it.
     */
    fun postCompleteGen(token: Long, result: StaticHostModelGenResult): Boolean =
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
    fun registerSceHostModelGenerateInvoker(invoker: StaticHostModelSceHostModelGenerateInvoker) {
        registerInvoker("sce-host-model-generate") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "gen" -> {
                        val typed = StaticHostModelGenRequest(
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


    override val initialState: StaticHostModelState = StaticHostModelState.Idle

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticHostModelState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostModelState, HistoryId>> =
            listOf(StateTarget(StaticHostModelState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: generating's transition 0, as the microstep reads it.
        val transitionGeneratingAt0 = EnabledTransition<StaticHostModelState, HistoryId>(
            StaticHostModelState.Generating,
            listOf(StateTarget(StaticHostModelState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: generating's transition 1, as the microstep reads it.
        val transitionGeneratingAt1 = EnabledTransition<StaticHostModelState, HistoryId>(
            StaticHostModelState.Generating,
            listOf(StateTarget(StaticHostModelState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: generating's transition 2, as the microstep reads it.
        val transitionGeneratingAt2 = EnabledTransition<StaticHostModelState, HistoryId>(
            StaticHostModelState.Generating,
            listOf(StateTarget(StaticHostModelState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: generating's transition 3, as the microstep reads it.
        val transitionGeneratingAt3 = EnabledTransition<StaticHostModelState, HistoryId>(
            StaticHostModelState.Generating,
            listOf(StateTarget(StaticHostModelState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: generating's transition 4, as the microstep reads it.
        val transitionGeneratingAt4 = EnabledTransition<StaticHostModelState, HistoryId>(
            StaticHostModelState.Generating,
            listOf(StateTarget(StaticHostModelState.Idle)),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostModelState, HistoryId>(
            StaticHostModelState.Idle,
            listOf(StateTarget(StaticHostModelState.Generating)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostModelState? = when (stateId) {
        "generating" -> StaticHostModelState.Generating
        "idle" -> StaticHostModelState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostModelState): String = when (state) {
        is StaticHostModelState.Generating -> "generating"
        is StaticHostModelState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostModelState): Int = when (state) {
        is StaticHostModelState.Generating -> 1
        is StaticHostModelState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostModelEvent? = when (name) {
        "done.invoke" -> StaticHostModelEvent.Done.Invoke.Self
        "done.invoke.gen" -> StaticHostModelEvent.Done.Invoke.Gen
        "error.execution" -> StaticHostModelEvent.Error.Execution
        "error.invoke" -> StaticHostModelEvent.Error.Invoke.Self
        "error.invoke.gen" -> StaticHostModelEvent.Error.Invoke.Gen
        "host.model.generate.request" -> StaticHostModelEvent.Host.Model.Generate.Request
        "model.leave" -> StaticHostModelEvent.Model.Leave
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostModelEvent): String? = when (event) {
        is StaticHostModelEvent.Done.Invoke.Self -> "done.invoke"
        is StaticHostModelEvent.Done.Invoke.Gen -> "done.invoke.gen"
        is StaticHostModelEvent.Error.Execution -> "error.execution"
        is StaticHostModelEvent.Error.Invoke.Self -> "error.invoke"
        is StaticHostModelEvent.Error.Invoke.Gen -> "error.invoke.gen"
        is StaticHostModelEvent.Host.Model.Generate.Request -> "host.model.generate.request"
        is StaticHostModelEvent.Model.Leave -> "model.leave"
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
    override fun firstEnabledTransition(state: StaticHostModelState, event: StaticHostModelEvent?): EnabledTransition<StaticHostModelState, HistoryId>? = when (state) {
        is StaticHostModelState.Generating -> firstEnabledTransitionOfGenerating(state, event)
        is StaticHostModelState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // generating's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGenerating(state: StaticHostModelState, event: StaticHostModelEvent?): EnabledTransition<StaticHostModelState, HistoryId>? = when {
        event is StaticHostModelEvent.Done.Invoke.Gen && pendingDoneInvokeGenPayload != null && (pendingDoneInvokeGenPayload!!.error == "") -> TransitionsPart0.transitionGeneratingAt0
        event is StaticHostModelEvent.Done.Invoke.Gen -> TransitionsPart0.transitionGeneratingAt1
        event is StaticHostModelEvent.Error.Invoke.Gen -> TransitionsPart0.transitionGeneratingAt2
        event is StaticHostModelEvent.Error.Execution -> TransitionsPart0.transitionGeneratingAt3
        event is StaticHostModelEvent.Model.Leave -> TransitionsPart0.transitionGeneratingAt4
        else -> null
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticHostModelState, event: StaticHostModelEvent?): EnabledTransition<StaticHostModelState, HistoryId>? = when {
        event is StaticHostModelEvent.Host.Model.Generate.Request -> TransitionsPart0.transitionIdleAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_model.scxml:33 :: _machine
    override fun onEntry(state: StaticHostModelState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostModelState.Generating -> onEntryOfGenerating()
            is StaticHostModelState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfGenerating() {
                // SCE-MAP: static_host_model.scxml:70 :: generating :: _state_body
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
                    val generatedInvokeId = "generating.${System.identityHashCode(this)}.gen"
                    deferInvoke(StaticHostModelState.Generating, generatedInvokeId) {
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
                            val v: Any? = (vMaxTokens).toLong()
                            hostInvokeParams["max_tokens"] =
                                (hostInvokeParams["max_tokens"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "max_tokens", v)
                        }
                        run {
                            val v: Any? = (vTemperatureMilli).toLong()
                            hostInvokeParams["temperature_milli"] =
                                (hostInvokeParams["temperature_milli"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "temperature_milli", v)
                        }
                        run {
                            val v: Any? = (1000).toLong()
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
                            raisePlatformError(StaticHostModelEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
    }

    private fun onEntryOfIdle() {
                // SCE-MAP: static_host_model.scxml:54 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_model.scxml:33 :: _machine
    override fun onExit(state: StaticHostModelState) {
        when (state) {
            is StaticHostModelState.Generating -> onExitOfGenerating()
            is StaticHostModelState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfGenerating() {
                // SCE-MAP: static_host_model.scxml:70 :: generating :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(StaticHostModelState.Generating)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-model-generate", "gen")
    }

    private fun onExitOfIdle() {
                // SCE-MAP: static_host_model.scxml:54 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_model.scxml:33 :: _machine
    override fun executeTransitionContent(source: StaticHostModelState, transitionIndex: Int) {
        when (source) {
            is StaticHostModelState.Generating -> transitionContentOfGenerating(transitionIndex)
            is StaticHostModelState.Idle -> transitionContentOfIdle(transitionIndex)
        }
    }


    // The content of `generating`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfGenerating(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfGeneratingAt0()
            1 -> transitionContentOfGeneratingAt1()
            2 -> transitionContentOfGeneratingAt2()
            3 -> transitionContentOfGeneratingAt3()
            4 -> transitionContentOfGeneratingAt4()
            else -> {}
        }
    }


    private fun transitionContentOfGeneratingAt0() {
                // SCE-MAP: static_host_model.scxml:78 :: generating :: _transition_0
                if (pendingDoneInvokeGenPayload == null) {
                    return
                }

            if (try { vFinish = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGenPayload!!.finish, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='finish'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vConstrained = pendingDoneInvokeGenPayload!!.constrained

            if (try { vAnsweredBy = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGenPayload!!.model, 128); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='answered_by'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vPromptTokens = pendingDoneInvokeGenPayload!!.prompt_tokens

            vAnswerTokens = pendingDoneInvokeGenPayload!!.answer_tokens

            when (val sceLift = com.sce.runtime.EventPayload.liftRecord(pendingDoneInvokeGenPayload!!.text, 1, 256, "{\"priority\":3}") { r -> StaticHostModelAnswerRecord(id = r.field("id").let { v -> v.uint32() }, amount = r.field("amount").let { v -> v.int64() }, title = r.field("title").let { v -> v.string(16) }, kind = r.field("kind").let { v -> v.name().let { name -> when (name) { "meeting" -> StaticHostModelEnumLiftKindEnum.MEETING; "task" -> StaticHostModelEnumLiftKindEnum.TASK; else -> throw v.unknownVariant(name, "enum_lift_kind") } } }, note = r.field("note").let { v -> v.optional { present -> present.string(8) } }, tag = r.field("tag").let { v -> v.bytes(4) }, priority = r.field("priority").let { v -> v.uint8() }) }) { is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vAnswer = sceLift.value; vWhy = null }; is com.sce.runtime.EventPayload.LiftOutcome.Refused -> { vWhy = StaticHostModelLiftRefusalRecord(code = when (sceLift.refusal.code) { com.sce.runtime.EventPayload.RefusalCode.NOT_JSON -> StaticHostModelRefusalCodeEnum.NOT_JSON; com.sce.runtime.EventPayload.RefusalCode.NOT_OBJECT -> StaticHostModelRefusalCodeEnum.NOT_OBJECT; com.sce.runtime.EventPayload.RefusalCode.TOO_DEEP -> StaticHostModelRefusalCodeEnum.TOO_DEEP; com.sce.runtime.EventPayload.RefusalCode.MISSING -> StaticHostModelRefusalCodeEnum.MISSING; com.sce.runtime.EventPayload.RefusalCode.NULL_NOT_OPTIONAL -> StaticHostModelRefusalCodeEnum.NULL_NOT_OPTIONAL; com.sce.runtime.EventPayload.RefusalCode.WRONG_KIND -> StaticHostModelRefusalCodeEnum.WRONG_KIND; com.sce.runtime.EventPayload.RefusalCode.BAD_NUMBER -> StaticHostModelRefusalCodeEnum.BAD_NUMBER; com.sce.runtime.EventPayload.RefusalCode.OUT_OF_RANGE -> StaticHostModelRefusalCodeEnum.OUT_OF_RANGE; com.sce.runtime.EventPayload.RefusalCode.TOO_LONG -> StaticHostModelRefusalCodeEnum.TOO_LONG; com.sce.runtime.EventPayload.RefusalCode.TOO_MANY -> StaticHostModelRefusalCodeEnum.TOO_MANY; com.sce.runtime.EventPayload.RefusalCode.UNKNOWN_VARIANT -> StaticHostModelRefusalCodeEnum.UNKNOWN_VARIANT; com.sce.runtime.EventPayload.RefusalCode.NOT_BYTE_TEXT -> StaticHostModelRefusalCodeEnum.NOT_BYTE_TEXT; com.sce.runtime.EventPayload.RefusalCode.NUL_IN_TEXT -> StaticHostModelRefusalCodeEnum.NUL_IN_TEXT; com.sce.runtime.EventPayload.RefusalCode.TOO_FEW -> StaticHostModelRefusalCodeEnum.TOO_FEW; }, path = sceLift.refusal.pointer, bound = sceLift.refusal.bound?.toUInt()) } }


            if (vWhy == null) {

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("read", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            } else {

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfGeneratingAt1() {
                // SCE-MAP: static_host_model.scxml:91 :: generating :: _transition_1
                if (pendingDoneInvokeGenPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGenPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfGeneratingAt2() {
                // SCE-MAP: static_host_model.scxml:94 :: generating :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfGeneratingAt3() {
                // SCE-MAP: static_host_model.scxml:97 :: generating :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("invalid_argument", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfGeneratingAt4() {
                // SCE-MAP: static_host_model.scxml:100 :: generating :: _transition_4

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("left", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_host_model.scxml:55 :: idle :: _transition_0
                if (pendingHostModelGenerateRequestPayload == null) {
                    return
                }

            if (try { vPrompt = com.sce.forge.runtime.SceChecked.bounded(pendingHostModelGenerateRequestPayload!!.prompt, 8192); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='prompt'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vMaxTokens = pendingHostModelGenerateRequestPayload!!.max_tokens

            vTemperatureMilli = pendingHostModelGenerateRequestPayload!!.temperature_milli

            if (try { vCalls = com.sce.forge.runtime.SceChecked.add(vCalls, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='calls'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vFinish = com.sce.forge.runtime.SceChecked.bounded("", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='finish'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vConstrained = false

            if (try { vAnsweredBy = com.sce.forge.runtime.SceChecked.bounded("", 128); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostModelEvent.Error.Execution, "<assign location='answered_by'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vPromptTokens = 0.toUInt()

            vAnswerTokens = 0.toUInt()

            vWhy = null
    }
}
