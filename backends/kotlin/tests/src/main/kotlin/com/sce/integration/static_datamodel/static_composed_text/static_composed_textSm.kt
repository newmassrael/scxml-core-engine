// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_composed_text.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_composed_text.scxml:31 :: _machine

package com.sce.integration.static_datamodel.static_composed_text

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticComposedTextState : State {
    data object Idle : StaticComposedTextState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticComposedTextEvent : Event {
    sealed interface Batch : StaticComposedTextEvent {
        data object Sent : Batch
    }
    sealed interface Error : StaticComposedTextEvent {
        data object Execution : Error
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticComposedTextPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticComposedTextBatchSentPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `batch.sent`. Consumers inject it via the `raiseBatchSent` seam
// on the machine — they never name this class directly.
data class StaticComposedTextBatchSentPayload(val items: List<StaticComposedTextItemRecord>, val grid: List<List<UByte>>, val levels: List<StaticComposedTextSignalLevelEnum>, val maybe: List<UShort?>, val spare: List<UByte>?, val note: String, val late: List<UByte>) {
    init {
        com.sce.runtime.ValueBounds.list("items", items, 3) { _, _ -> }
        com.sce.runtime.ValueBounds.list("grid", grid, 2) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.list(scePath0, sceValue0, 2) { _, _ -> } }
        com.sce.runtime.ValueBounds.list("levels", levels, 2) { _, _ -> }
        com.sce.runtime.ValueBounds.list("maybe", maybe, 2) { _, _ -> }
        com.sce.runtime.ValueBounds.optional("spare", spare) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.list(scePath0, sceValue0, 2) { _, _ -> } }
        com.sce.runtime.ValueBounds.text("note", note, 4)
        com.sce.runtime.ValueBounds.list("late", late, 2) { _, _ -> }
    }
}


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:SignalLevel` datamodel value. */
enum class StaticComposedTextSignalLevelEnum(val declaredName: String) {
    LOW("low"),
    HIGH("high"),
    CRITICAL("critical");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        LOW -> 0u.toUByte()
        HIGH -> 1u.toUByte()
        CRITICAL -> 2u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticComposedTextSignalLevelEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of SignalLevel")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticComposedTextSignalLevelEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Item` value that holds a composed value (docs/adr/0086). */
data class StaticComposedTextItemRecord(val id: UInt, val amount: Long, val label: String, val level: StaticComposedTextSignalLevelEnum, val due: Long?, val blob: ByteArray, val tags: List<String>) {
    init {
        com.sce.runtime.ValueBounds.text("label", label, 8)
        com.sce.runtime.ValueBounds.bytes("blob", blob, 4)
        com.sce.runtime.ValueBounds.list("tags", tags, 4) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.text(scePath0, sceValue0, 8) }
    }

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
// --- State Machine (W3C SCXML) ---

class StaticComposedTextStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticComposedTextState, StaticComposedTextEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `note_is_ab` datamodel variable, published (`sce:direction="out"`). */
    private var vNoteIsAb: Boolean = false

    /** What the published `note_is_ab` holds. */
    val noteIsAb: Boolean get() = vNoteIsAb
    /** W3C SCXML 5.2: the `note_len` datamodel variable, published (`sce:direction="out"`). */
    private var vNoteLen: UInt = 0.toUInt()

    /** What the published `note_len` holds. */
    val noteLen: UInt get() = vNoteLen
    /** W3C SCXML 5.2: the `first_label_is_ab` datamodel variable, published (`sce:direction="out"`). */
    private var vFirstLabelIsAb: Boolean = false

    /** What the published `first_label_is_ab` holds. */
    val firstLabelIsAb: Boolean get() = vFirstLabelIsAb
    /** W3C SCXML 5.2: the `labels_ab` datamodel variable, published (`sce:direction="out"`). */
    private var vLabelsAb: UInt = 0.toUInt()

    /** What the published `labels_ab` holds. */
    val labelsAb: UInt get() = vLabelsAb
    /** W3C SCXML 5.2: the `first_tags_ab` datamodel variable, published (`sce:direction="out"`). */
    private var vFirstTagsAb: UInt = 0.toUInt()

    /** What the published `first_tags_ab` holds. */
    val firstTagsAb: UInt get() = vFirstTagsAb
    /** W3C SCXML 5.2: the `words_ab` datamodel variable, published (`sce:direction="out"`). */
    private var vWordsAb: UInt = 0.toUInt()

    /** What the published `words_ab` holds. */
    val wordsAb: UInt get() = vWordsAb
    /** W3C SCXML 5.2: the `last_label` datamodel variable, published (`sce:direction="out"`). */
    private var vLastLabel: String = ""

    /** What the published `last_label` holds. */
    val lastLabel: String get() = vLastLabel
    /** W3C SCXML 5.2: the `blob_bytes` datamodel variable, published (`sce:direction="out"`). */
    private var vBlobBytes: UInt = 0.toUInt()

    /** What the published `blob_bytes` holds. */
    val blobBytes: UInt get() = vBlobBytes
    /** W3C SCXML 5.2: the `blobs_ab` datamodel variable, published (`sce:direction="out"`). */
    private var vBlobsAb: UInt = 0.toUInt()

    /** What the published `blobs_ab` holds. */
    val blobsAb: UInt get() = vBlobsAb
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var noteIsAb: Boolean? = null
        var noteLen: UInt? = null
        var firstLabelIsAb: Boolean? = null
        var labelsAb: UInt? = null
        var firstTagsAb: UInt? = null
        var wordsAb: UInt? = null
        var lastLabel: String? = null
        var blobBytes: UInt? = null
        var blobsAb: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.noteIsAb?.let { vNoteIsAb = it }
        params.noteLen?.let { vNoteLen = it }
        params.firstLabelIsAb?.let { vFirstLabelIsAb = it }
        params.labelsAb?.let { vLabelsAb = it }
        params.firstTagsAb?.let { vFirstTagsAb = it }
        params.wordsAb?.let { vWordsAb = it }
        params.lastLabel?.let { vLastLabel = it }
        params.blobBytes?.let { vBlobBytes = it }
        params.blobsAb?.let { vBlobsAb = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val noteIsAb: Boolean,
        val noteLen: UInt,
        val firstLabelIsAb: Boolean,
        val labelsAb: UInt,
        val firstTagsAb: UInt,
        val wordsAb: UInt,
        val lastLabel: String,
        val blobBytes: UInt,
        val blobsAb: UInt,
        val refusals: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticComposedTextState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        noteIsAb = vNoteIsAb,
        noteLen = vNoteLen,
        firstLabelIsAb = vFirstLabelIsAb,
        labelsAb = vLabelsAb,
        firstTagsAb = vFirstTagsAb,
        wordsAb = vWordsAb,
        lastLabel = vLastLabel,
        blobBytes = vBlobBytes,
        blobsAb = vBlobsAb,
        refusals = vRefusals,
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
        first = "6758a021764eaa47cfbd75260447e9720ec8ed89774e3a79594892df39978344",
        revised = "35614bf3d0b07bc0602cff4a8190da638de9c52bc603f97114c39bd906c6b631",
        format2First = "6758a021764eaa47cfbd75260447e9720ec8ed89774e3a79594892df39978344",
        format2Revised = "35614bf3d0b07bc0602cff4a8190da638de9c52bc603f97114c39bd906c6b631",
        records = listOf(
        ),
        enums = listOf(
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
        variables += "note_is_ab" to SavedValues.of(vNoteIsAb)
        variables += "note_len" to SavedValues.of(vNoteLen)
        variables += "first_label_is_ab" to SavedValues.of(vFirstLabelIsAb)
        variables += "labels_ab" to SavedValues.of(vLabelsAb)
        variables += "first_tags_ab" to SavedValues.of(vFirstTagsAb)
        variables += "words_ab" to SavedValues.of(vWordsAb)
        variables += "last_label" to SavedValues.of(vLastLabel)
        variables += "blob_bytes" to SavedValues.of(vBlobBytes)
        variables += "blobs_ab" to SavedValues.of(vBlobsAb)
        variables += "refusals" to SavedValues.of(vRefusals)
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
        val saved1 = SavedValues.bool(savedGrown.variable("note_is_ab"), "note_is_ab")
        val saved2 = SavedValues.uint32(savedGrown.variable("note_len"), "note_len")
        val saved3 = SavedValues.bool(savedGrown.variable("first_label_is_ab"), "first_label_is_ab")
        val saved4 = SavedValues.uint32(savedGrown.variable("labels_ab"), "labels_ab")
        val saved5 = SavedValues.uint32(savedGrown.variable("first_tags_ab"), "first_tags_ab")
        val saved6 = SavedValues.uint32(savedGrown.variable("words_ab"), "words_ab")
        val saved7 = SavedValues.string(savedGrown.variable("last_label"), "last_label", 8)
        val saved8 = SavedValues.uint32(savedGrown.variable("blob_bytes"), "blob_bytes")
        val saved9 = SavedValues.uint32(savedGrown.variable("blobs_ab"), "blobs_ab")
        val saved10 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        return {
            vNoteIsAb = saved1
            vNoteLen = saved2
            vFirstLabelIsAb = saved3
            vLabelsAb = saved4
            vFirstTagsAb = saved5
            vWordsAb = saved6
            vLastLabel = saved7
            vBlobBytes = saved8
            vBlobsAb = saved9
            vRefusals = saved10
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
        SavedValues.bool(savedGrown.variable("note_is_ab"), "note_is_ab")
        SavedValues.uint32(savedGrown.variable("note_len"), "note_len")
        SavedValues.bool(savedGrown.variable("first_label_is_ab"), "first_label_is_ab")
        SavedValues.uint32(savedGrown.variable("labels_ab"), "labels_ab")
        SavedValues.uint32(savedGrown.variable("first_tags_ab"), "first_tags_ab")
        SavedValues.uint32(savedGrown.variable("words_ab"), "words_ab")
        SavedValues.string(savedGrown.variable("last_label"), "last_label", 8)
        SavedValues.uint32(savedGrown.variable("blob_bytes"), "blob_bytes")
        SavedValues.uint32(savedGrown.variable("blobs_ab"), "blobs_ab")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingBatchSentPayload: StaticComposedTextBatchSentPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticComposedTextEvent, metadata: EventMetadata) {
        pendingBatchSentPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticComposedTextBatchSentPayload -> pendingBatchSentPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticComposedTextEvent.Batch.Sent) {
                    val fields = EventPayload.decode(metadata.data, "{\"late\":[]}")
                    pendingBatchSentPayload = StaticComposedTextBatchSentPayload(fields.field("items").let { it.list(3) { it.record(null) { StaticComposedTextItemRecord(id = it.field("id").let { it.uint32() }, amount = it.field("amount").let { it.int64() }, label = it.field("label").let { it.string(8) }, level = it.field("level").let { when (val sceName = it.name()) { "low" -> StaticComposedTextSignalLevelEnum.LOW; "high" -> StaticComposedTextSignalLevelEnum.HIGH; "critical" -> StaticComposedTextSignalLevelEnum.CRITICAL; else -> throw it.unknownVariant(sceName, "SignalLevel") } }, due = it.field("due").let { it.optional { it.int64() } }, blob = it.field("blob").let { it.bytes(4) }, tags = it.field("tags").let { it.list(4) { it.string(8) } }) } } }, fields.field("grid").let { it.list(2) { it.list(2) { it.uint8() } } }, fields.field("levels").let { it.list(2) { when (val sceName = it.name()) { "low" -> StaticComposedTextSignalLevelEnum.LOW; "high" -> StaticComposedTextSignalLevelEnum.HIGH; "critical" -> StaticComposedTextSignalLevelEnum.CRITICAL; else -> throw it.unknownVariant(sceName, "SignalLevel") } } }, fields.field("maybe").let { it.list(2) { it.optional { it.uint16() } } }, fields.field("spare").let { it.optional { it.list(2) { it.uint8() } } }, fields.field("note").let { it.string(4) }, fields.field("late").let { it.list(2) { it.uint8() } })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `batch.sent` — binds the event name and the payload field values in one call.
    fun raiseBatchSent(items: List<StaticComposedTextItemRecord>, grid: List<List<UByte>>, levels: List<StaticComposedTextSignalLevelEnum>, maybe: List<UShort?>, spare: List<UByte>?, note: String, late: List<UByte>) {
        val scePayload = StaticComposedTextBatchSentPayload(items, grid, levels, maybe, spare, note, late)
        send(
            StaticComposedTextEvent.Batch.Sent,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("items" to com.sce.runtime.EventPayload.listJson(scePayload.items) { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("id" to com.sce.runtime.EventPayload.valueJson(it.id), "amount" to com.sce.runtime.EventPayload.valueJson(it.amount), "label" to com.sce.runtime.EventPayload.valueJson(it.label), "level" to com.sce.runtime.EventPayload.valueJson(it.level.declaredName), "due" to ((it.due)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)), "blob" to com.sce.runtime.EventPayload.valueJson(it.blob), "tags" to com.sce.runtime.EventPayload.listJson(it.tags) { com.sce.runtime.EventPayload.valueJson(it) })) }, "grid" to com.sce.runtime.EventPayload.listJson(scePayload.grid) { com.sce.runtime.EventPayload.listJson(it) { com.sce.runtime.EventPayload.valueJson(it) } }, "levels" to com.sce.runtime.EventPayload.listJson(scePayload.levels) { com.sce.runtime.EventPayload.valueJson(it.declaredName) }, "maybe" to com.sce.runtime.EventPayload.listJson(scePayload.maybe) { ((it)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)) }, "spare" to ((scePayload.spare)?.let { com.sce.runtime.EventPayload.listJson(it) { com.sce.runtime.EventPayload.valueJson(it) } } ?: com.sce.runtime.EventPayload.valueJson(null)), "note" to com.sce.runtime.EventPayload.valueJson(scePayload.note), "late" to com.sce.runtime.EventPayload.listJson(scePayload.late) { com.sce.runtime.EventPayload.valueJson(it) })).json
            )
        )
    }


    override val initialState: StaticComposedTextState = StaticComposedTextState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

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
    override val documentInitialTargets: List<EntryTarget<StaticComposedTextState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticComposedTextState, HistoryId>> =
            listOf(StateTarget(StaticComposedTextState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticComposedTextState, HistoryId>(
            StaticComposedTextState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticComposedTextState, HistoryId>(
            StaticComposedTextState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticComposedTextState, HistoryId>(
            StaticComposedTextState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticComposedTextState? = when (stateId) {
        "idle" -> StaticComposedTextState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticComposedTextState): String = when (state) {
        is StaticComposedTextState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticComposedTextState): Int = when (state) {
        is StaticComposedTextState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticComposedTextEvent? = when (name) {
        "batch.sent" -> StaticComposedTextEvent.Batch.Sent
        "error.execution" -> StaticComposedTextEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticComposedTextEvent): String? = when (event) {
        is StaticComposedTextEvent.Batch.Sent -> "batch.sent"
        is StaticComposedTextEvent.Error.Execution -> "error.execution"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticComposedTextState, event: StaticComposedTextEvent?): EnabledTransition<StaticComposedTextState, HistoryId>? = when (state) {
        is StaticComposedTextState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticComposedTextState, event: StaticComposedTextEvent?): EnabledTransition<StaticComposedTextState, HistoryId>? = when {
        event is StaticComposedTextEvent.Batch.Sent && pendingBatchSentPayload != null && (pendingBatchSentPayload!!.note == "ab") -> TransitionsPart0.transitionIdleAt0
        event is StaticComposedTextEvent.Batch.Sent -> TransitionsPart0.transitionIdleAt1
        event is StaticComposedTextEvent.Error.Execution -> TransitionsPart0.transitionIdleAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_composed_text.scxml:31 :: _machine
    override fun onEntry(state: StaticComposedTextState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticComposedTextState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_composed_text.scxml:52 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_composed_text.scxml:31 :: _machine
    override fun onExit(state: StaticComposedTextState) {
        when (state) {
            is StaticComposedTextState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_composed_text.scxml:52 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_composed_text.scxml:31 :: _machine
    override fun executeTransitionContent(source: StaticComposedTextState, transitionIndex: Int) {
        when (source) {
            is StaticComposedTextState.Idle -> transitionContentOfIdle(transitionIndex)
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
            1 -> transitionContentOfIdleAt1()
            2 -> transitionContentOfIdleAt2()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_composed_text.scxml:53 :: idle :: _transition_0
                if (pendingBatchSentPayload == null) {
                    return
                }

            vNoteIsAb = true

            vNoteLen = (pendingBatchSentPayload!!.note).encodeToByteArray().size.toUInt()


            if ((pendingBatchSentPayload!!.items).size.toUInt() > 0.toUInt()) {

            if (try { vFirstLabelIsAb = com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.items, (0).toLong()).label == "ab"; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='first_label_is_ab'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (row in pendingBatchSentPayload!!.items) {


            if (row.label == "ab") {

            if (try { vLabelsAb = com.sce.forge.runtime.SceChecked.add(vLabelsAb, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='labels_ab'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            var ifCondFailed3 = false
            if ((try { (row.tags).size.toUInt() > 0.toUInt() && com.sce.forge.runtime.SceChecked.at(row.tags, (0).toLong()) == "ab" } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<if cond='len(row.tags) > 0 && row.tags[0] === 'ab''>: an integer operation overflowed or failed"); ifCondFailed3 = true; false })) {

            if (try { vFirstTagsAb = com.sce.forge.runtime.SceChecked.add(vFirstTagsAb, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='first_tags_ab'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            if (ifCondFailed3) {
                return
            }


            for (word in row.tags) {


            if (word == "ab") {

            if (try { vWordsAb = com.sce.forge.runtime.SceChecked.add(vWordsAb, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='words_ab'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }

            if (try { vLastLabel = com.sce.forge.runtime.SceChecked.bounded(row.label, 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='last_label'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vBlobBytes = com.sce.forge.runtime.SceChecked.add(vBlobBytes, (row.blob).size.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='blob_bytes'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (row.blob.contentEquals("ab".toByteArray())) {

            if (try { vBlobsAb = com.sce.forge.runtime.SceChecked.add(vBlobsAb, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='blobs_ab'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_composed_text.scxml:78 :: idle :: _transition_1
                if (pendingBatchSentPayload == null) {
                    return
                }

            vNoteLen = (pendingBatchSentPayload!!.note).encodeToByteArray().size.toUInt()
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: static_composed_text.scxml:83 :: idle :: _transition_2

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedTextEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
