// SCE-GENERATED — DO NOT EDIT
// source-hash: 0927eff6973cd55ae798b6a8108a8f69e3c219a339f22cc8bc3e594b2bdd4c1c

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/sync_client.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: sync_client.scxml:35 :: _machine

package com.sce.integration.sync_client

import com.sce.runtime.*
import com.sce.generated.sync_delete_outcome.*
import com.sce.generated.sync_failure.*
import com.sce.generated.sync_retry_at.*
import com.sce.generated.sync_upload_outcome.*


// --- States (W3C SCXML 3.2) ---

sealed interface SyncClientState : State {
    data object Client : SyncClientState
    data object Deleting : SyncClientState
    data object Idle : SyncClientState
    data object Listing : SyncClientState
    data object Uploading : SyncClientState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SyncClientEvent : Event {
    sealed interface Error : SyncClientEvent {
        data object Execution : Error
    }
    sealed interface Sync : SyncClientEvent {
        data object Page : Sync
        sealed interface Phase : Sync {
            data object Done : Phase
        }
        data object Response : Sync
        data object Start : Sync
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `SyncClientPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// SyncClientSyncPagePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sync.page`. Consumers inject it via the `raiseSyncPage` seam
// on the machine — they never name this class directly.
data class SyncClientSyncPagePayload(val more: Boolean)

// SyncClientSyncResponsePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sync.response`. Consumers inject it via the `raiseSyncResponse` seam
// on the machine — they never name this class directly.
data class SyncClientSyncResponsePayload(val kind: UByte, val status: Int, val create: Boolean, val davError: Boolean, val now: Long, val retryAfter: Long)

// SyncClientSyncStartPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sync.start`. Consumers inject it via the `raiseSyncStart` seam
// on the machine — they never name this class directly.
data class SyncClientSyncStartPayload(val byToken: Boolean)


// --- State Machine (W3C SCXML) ---

class SyncClientStateMachine(
) : StateMachineEngine<SyncClientState, SyncClientEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `byToken` datamodel variable, published (`sce:direction="out"`). */
    var byToken: Boolean = false
        private set
    /** W3C SCXML 5.2: the `fullListing` datamodel variable, published (`sce:direction="out"`). */
    var fullListing: Boolean = true
        private set
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    var outcome: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `retryAt` datamodel variable, published (`sce:direction="out"`). */
    var retryAt: Long = 0
        private set
    /** W3C SCXML 5.2: the `deleted` datamodel variable, published (`sce:direction="out"`). */
    var deleted: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `uploaded` datamodel variable, published (`sce:direction="out"`). */
    var uploaded: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `discarded` datamodel variable, published (`sce:direction="out"`). */
    var discarded: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `pages` datamodel variable, published (`sce:direction="out"`). */
    var pages: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val byToken: Boolean,
        val fullListing: Boolean,
        val outcome: UByte,
        val retryAt: Long,
        val deleted: UInt,
        val uploaded: UInt,
        val discarded: UInt,
        val pages: UInt,
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
        val configuration: Set<SyncClientState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        byToken = byToken,
        fullListing = fullListing,
        outcome = outcome,
        retryAt = retryAt,
        deleted = deleted,
        uploaded = uploaded,
        discarded = discarded,
        pages = pages,
        refusals = refusals,
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
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "b07ee998a49953bb6dcbd8a8f1b5201424834bdf432224e225a612af803dcbe2"

    /**
     * This machine's whole state at the macrostep boundary it stands at —
     * every variable, the machine's own included, and where it stands — as
     * the `sce-saved-state` document every backend reads ([SavedState.toJson]).
     *
     * @throws StateRefusal for a machine that is not running, or whose last
     *   macrostep stopped at the microstep ceiling.
     */
    fun save(): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "byToken" to SavedValues.of(byToken),
            "fullListing" to SavedValues.of(fullListing),
            "outcome" to SavedValues.of(outcome),
            "retryAt" to SavedValues.of(retryAt),
            "deleted" to SavedValues.of(deleted),
            "uploaded" to SavedValues.of(uploaded),
            "discarded" to SavedValues.of(discarded),
            "pages" to SavedValues.of(pages),
            "refusals" to SavedValues.of(refusals),
        ),
    )

    /**
     * Stand this machine where [saved] left one, in place of [initialize]: no
     * `<onentry>` runs and no `<data>` is evaluated, since the saved run
     * already did both. Every value is read before any is written, so a
     * refused restore leaves the machine as it was.
     *
     * @throws StateRefusal for a machine that has already started, a state
     *   saved from a document of another shape, a configuration that is not
     *   one of this document, or a value its variable's type cannot hold.
     */
    fun restore(saved: SavedState) {
        beginRestore(saved, savedShape)
        val saved1 = SavedValues.bool(saved.variable("byToken"), "byToken")
        val saved2 = SavedValues.bool(saved.variable("fullListing"), "fullListing")
        val saved3 = SavedValues.uint8(saved.variable("outcome"), "outcome")
        val saved4 = SavedValues.int64(saved.variable("retryAt"), "retryAt")
        val saved5 = SavedValues.uint32(saved.variable("deleted"), "deleted")
        val saved6 = SavedValues.uint32(saved.variable("uploaded"), "uploaded")
        val saved7 = SavedValues.uint32(saved.variable("discarded"), "discarded")
        val saved8 = SavedValues.uint32(saved.variable("pages"), "pages")
        val saved9 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        byToken = saved1
        fullListing = saved2
        outcome = saved3
        retryAt = saved4
        deleted = saved5
        uploaded = saved6
        discarded = saved7
        pages = saved8
        refusals = saved9
        enterSaved(saved)
    }

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingSyncPagePayload: SyncClientSyncPagePayload? = null
    private var pendingSyncResponsePayload: SyncClientSyncResponsePayload? = null
    private var pendingSyncStartPayload: SyncClientSyncStartPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: SyncClientEvent, metadata: EventMetadata) {
        pendingSyncPagePayload = null
        pendingSyncResponsePayload = null
        pendingSyncStartPayload = null
        when (val tp = metadata.typedPayload) {
            is SyncClientSyncPagePayload -> pendingSyncPagePayload = tp
            is SyncClientSyncResponsePayload -> pendingSyncResponsePayload = tp
            is SyncClientSyncStartPayload -> pendingSyncStartPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == SyncClientEvent.Sync.Page) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSyncPagePayload = SyncClientSyncPagePayload(fields.boolean("more"))
                } else if (event == SyncClientEvent.Sync.Response) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSyncResponsePayload = SyncClientSyncResponsePayload(fields.uint8("kind"), fields.int32("status"), fields.boolean("create"), fields.boolean("davError"), fields.int64("now"), fields.int64("retryAfter"))
                } else if (event == SyncClientEvent.Sync.Start) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSyncStartPayload = SyncClientSyncStartPayload(fields.boolean("byToken"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sync.page` — binds the event name and the payload field values in one call.
    fun raiseSyncPage(more: Boolean) {
        send(
            SyncClientEvent.Sync.Page,
            EventMetadata(
                type = "external",
                typedPayload = SyncClientSyncPagePayload(more),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("more" to more))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sync.response` — binds the event name and the payload field values in one call.
    fun raiseSyncResponse(kind: UByte, status: Int, create: Boolean, davError: Boolean, now: Long, retryAfter: Long) {
        send(
            SyncClientEvent.Sync.Response,
            EventMetadata(
                type = "external",
                typedPayload = SyncClientSyncResponsePayload(kind, status, create, davError, now, retryAfter),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("kind" to kind, "status" to status, "create" to create, "davError" to davError, "now" to now, "retryAfter" to retryAfter))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sync.start` — binds the event name and the payload field values in one call.
    fun raiseSyncStart(byToken: Boolean) {
        send(
            SyncClientEvent.Sync.Start,
            EventMetadata(
                type = "external",
                typedPayload = SyncClientSyncStartPayload(byToken),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("byToken" to byToken))
            )
        )
    }


    override val initialState: SyncClientState = SyncClientState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: SyncClientState): SyncClientState? = when (state) {
        is SyncClientState.Deleting -> SyncClientState.Client
        is SyncClientState.Idle -> SyncClientState.Client
        is SyncClientState.Listing -> SyncClientState.Client
        is SyncClientState.Uploading -> SyncClientState.Client
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: SyncClientState): Boolean = when (state) {
        is SyncClientState.Client -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: SyncClientState): List<SyncClientState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: SyncClientState): List<EntryTarget<SyncClientState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<SyncClientState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<SyncClientState, List<SyncClientState>> = mapOf(
            SyncClientState.Client to listOf(SyncClientState.Idle, SyncClientState.Deleting, SyncClientState.Uploading, SyncClientState.Listing),
        )

        val initialTargets: Map<SyncClientState, List<EntryTarget<SyncClientState, HistoryId>>> = mapOf(
            SyncClientState.Client to listOf(StateTarget(SyncClientState.Idle)),
        )

        val documentInitialTargetList: List<EntryTarget<SyncClientState, HistoryId>> =
            listOf(StateTarget(SyncClientState.Client))

        // W3C SCXML 3.13: client's transition 0, as the microstep reads it.
        val transitionClientAt0 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Client,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: deleting's transition 0, as the microstep reads it.
        val transitionDeletingAt0 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Deleting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: deleting's transition 1, as the microstep reads it.
        val transitionDeletingAt1 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Deleting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: deleting's transition 2, as the microstep reads it.
        val transitionDeletingAt2 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Deleting,
            listOf(StateTarget(SyncClientState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: deleting's transition 3, as the microstep reads it.
        val transitionDeletingAt3 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Deleting,
            listOf(StateTarget(SyncClientState.Uploading)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Idle,
            listOf(StateTarget(SyncClientState.Deleting)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: listing's transition 0, as the microstep reads it.
        val transitionListingAt0 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Listing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: listing's transition 1, as the microstep reads it.
        val transitionListingAt1 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Listing,
            listOf(StateTarget(SyncClientState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: listing's transition 2, as the microstep reads it.
        val transitionListingAt2 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Listing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: listing's transition 3, as the microstep reads it.
        val transitionListingAt3 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Listing,
            listOf(StateTarget(SyncClientState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: uploading's transition 0, as the microstep reads it.
        val transitionUploadingAt0 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Uploading,
            listOf(StateTarget(SyncClientState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: uploading's transition 1, as the microstep reads it.
        val transitionUploadingAt1 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Uploading,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: uploading's transition 2, as the microstep reads it.
        val transitionUploadingAt2 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Uploading,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: uploading's transition 3, as the microstep reads it.
        val transitionUploadingAt3 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Uploading,
            listOf(StateTarget(SyncClientState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: uploading's transition 4, as the microstep reads it.
        val transitionUploadingAt4 = EnabledTransition<SyncClientState, HistoryId>(
            SyncClientState.Uploading,
            listOf(StateTarget(SyncClientState.Listing)),
            4,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SyncClientState? = when (stateId) {
        "client" -> SyncClientState.Client
        "deleting" -> SyncClientState.Deleting
        "idle" -> SyncClientState.Idle
        "listing" -> SyncClientState.Listing
        "uploading" -> SyncClientState.Uploading
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SyncClientState): String = when (state) {
        is SyncClientState.Client -> "client"
        is SyncClientState.Deleting -> "deleting"
        is SyncClientState.Idle -> "idle"
        is SyncClientState.Listing -> "listing"
        is SyncClientState.Uploading -> "uploading"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: SyncClientState): Int = when (state) {
        is SyncClientState.Client -> 0
        is SyncClientState.Deleting -> 2
        is SyncClientState.Idle -> 1
        is SyncClientState.Listing -> 4
        is SyncClientState.Uploading -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SyncClientEvent? = when (name) {
        "error.execution" -> SyncClientEvent.Error.Execution
        "sync.page" -> SyncClientEvent.Sync.Page
        "sync.phase.done" -> SyncClientEvent.Sync.Phase.Done
        "sync.response" -> SyncClientEvent.Sync.Response
        "sync.start" -> SyncClientEvent.Sync.Start
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SyncClientEvent): String? = when (event) {
        is SyncClientEvent.Error.Execution -> "error.execution"
        is SyncClientEvent.Sync.Page -> "sync.page"
        is SyncClientEvent.Sync.Phase.Done -> "sync.phase.done"
        is SyncClientEvent.Sync.Response -> "sync.response"
        is SyncClientEvent.Sync.Start -> "sync.start"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: SyncClientState,
        event: SyncClientEvent?
    ): EnabledTransition<SyncClientState, HistoryId>? = when (state) {
        is SyncClientState.Client -> when {
            event is SyncClientEvent.Error.Execution -> transitionClientAt0
            else -> null
        }
        is SyncClientState.Deleting -> when {
            event is SyncClientEvent.Sync.Response && pendingSyncResponsePayload != null && ((try { com.sce.forge.runtime.SceChecked.take(syncDeleteOutcome(pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status)) == 0.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<transition cond='DeleteOutcome(_event.data.kind, _event.data.status) === 0'>: an integer operation overflowed or failed"); false })) -> transitionDeletingAt0
            event is SyncClientEvent.Sync.Response && pendingSyncResponsePayload != null && ((try { com.sce.forge.runtime.SceChecked.take(syncDeleteOutcome(pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status)) == 1.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<transition cond='DeleteOutcome(_event.data.kind, _event.data.status) === 1'>: an integer operation overflowed or failed"); false })) -> transitionDeletingAt1
            event is SyncClientEvent.Sync.Response -> transitionDeletingAt2
            event is SyncClientEvent.Sync.Phase.Done -> transitionDeletingAt3
            else -> null
        }
        is SyncClientState.Idle -> when {
            event is SyncClientEvent.Sync.Start -> transitionIdleAt0
            else -> null
        }
        is SyncClientState.Listing -> when {
            event is SyncClientEvent.Sync.Page && pendingSyncPagePayload != null && (pendingSyncPagePayload!!.more) -> transitionListingAt0
            event is SyncClientEvent.Sync.Page -> transitionListingAt1
            event is SyncClientEvent.Sync.Response && pendingSyncResponsePayload != null && (byToken && pendingSyncResponsePayload!!.kind == 4.toUByte() && pendingSyncResponsePayload!!.davError && (pendingSyncResponsePayload!!.status == 403 || pendingSyncResponsePayload!!.status == 409)) -> transitionListingAt2
            event is SyncClientEvent.Sync.Response -> transitionListingAt3
            else -> null
        }
        is SyncClientState.Uploading -> when {
            event is SyncClientEvent.Sync.Response && pendingSyncResponsePayload != null && (pendingSyncResponsePayload!!.kind != 4.toUByte()) -> transitionUploadingAt0
            event is SyncClientEvent.Sync.Response && pendingSyncResponsePayload != null && ((try { com.sce.forge.runtime.SceChecked.take(syncUploadOutcome(pendingSyncResponsePayload!!.create, pendingSyncResponsePayload!!.status, pendingSyncResponsePayload!!.davError)) == 0.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<transition cond='UploadOutcome(_event.data.create, _event.data.status, _event.data.davError) === 0'>: an integer operation overflowed or failed"); false })) -> transitionUploadingAt1
            event is SyncClientEvent.Sync.Response && pendingSyncResponsePayload != null && ((try { com.sce.forge.runtime.SceChecked.take(syncUploadOutcome(pendingSyncResponsePayload!!.create, pendingSyncResponsePayload!!.status, pendingSyncResponsePayload!!.davError)) == 1.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<transition cond='UploadOutcome(_event.data.create, _event.data.status, _event.data.davError) === 1'>: an integer operation overflowed or failed"); false })) -> transitionUploadingAt2
            event is SyncClientEvent.Sync.Response -> transitionUploadingAt3
            event is SyncClientEvent.Sync.Phase.Done -> transitionUploadingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: sync_client.scxml:35 :: _machine
    override fun onEntry(state: SyncClientState, isDefaultEntry: Boolean) {
        when (state) {
            is SyncClientState.Client -> {
                // SCE-MAP: sync_client.scxml:56 :: client :: _state_body
            }
            is SyncClientState.Deleting -> {
                // SCE-MAP: sync_client.scxml:68 :: deleting :: _state_body
            }
            is SyncClientState.Idle -> {
                // SCE-MAP: sync_client.scxml:57 :: idle :: _state_body
            }
            is SyncClientState.Listing -> {
                // SCE-MAP: sync_client.scxml:116 :: listing :: _state_body
            }
            is SyncClientState.Uploading -> {
                // SCE-MAP: sync_client.scxml:88 :: uploading :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: sync_client.scxml:35 :: _machine
    override fun onExit(state: SyncClientState) {
        when (state) {
            is SyncClientState.Client -> {
                // SCE-MAP: sync_client.scxml:56 :: client :: _state_body
            }
            is SyncClientState.Deleting -> {
                // SCE-MAP: sync_client.scxml:68 :: deleting :: _state_body
            }
            is SyncClientState.Idle -> {
                // SCE-MAP: sync_client.scxml:57 :: idle :: _state_body
            }
            is SyncClientState.Listing -> {
                // SCE-MAP: sync_client.scxml:116 :: listing :: _state_body
            }
            is SyncClientState.Uploading -> {
                // SCE-MAP: sync_client.scxml:88 :: uploading :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: sync_client.scxml:35 :: _machine
    override fun executeTransitionContent(source: SyncClientState, transitionIndex: Int) {
        when (source) {
        is SyncClientState.Client -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: sync_client.scxml:141 :: client :: _transition_0

            try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed") }
            }
            else -> {}
        }
        is SyncClientState.Deleting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: sync_client.scxml:69 :: deleting :: _transition_0

            try { deleted = com.sce.forge.runtime.SceChecked.add(deleted, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='deleted'>: an integer operation overflowed or failed") }
            }
            1 -> {
                // SCE-MAP: sync_client.scxml:75 :: deleting :: _transition_1

            fullListing = true

            byToken = false
            }
            2 -> {
                // SCE-MAP: sync_client.scxml:80 :: deleting :: _transition_2
                if (pendingSyncResponsePayload == null) {
                    return
                }

            try { outcome = com.sce.forge.runtime.SceChecked.take(syncFailure(pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed") }

            try { retryAt = com.sce.forge.runtime.SceChecked.take(syncRetryAt(retryAt, pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status, pendingSyncResponsePayload!!.now, pendingSyncResponsePayload!!.retryAfter)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='retryAt'>: an integer operation overflowed or failed") }
            }
            else -> {}
        }
        is SyncClientState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: sync_client.scxml:58 :: idle :: _transition_0
                if (pendingSyncStartPayload == null) {
                    return
                }

            byToken = pendingSyncStartPayload!!.byToken && !fullListing

            outcome = 0.toUByte()

            deleted = 0.toUInt()

            uploaded = 0.toUInt()

            discarded = 0.toUInt()

            pages = 0.toUInt()
            }
            else -> {}
        }
        is SyncClientState.Listing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: sync_client.scxml:117 :: listing :: _transition_0

            try { pages = com.sce.forge.runtime.SceChecked.add(pages, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='pages'>: an integer operation overflowed or failed") }
            }
            1 -> {
                // SCE-MAP: sync_client.scxml:122 :: listing :: _transition_1

            try { pages = com.sce.forge.runtime.SceChecked.add(pages, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='pages'>: an integer operation overflowed or failed") }

            fullListing = false
            }
            2 -> {
                // SCE-MAP: sync_client.scxml:127 :: listing :: _transition_2

            byToken = false
            }
            3 -> {
                // SCE-MAP: sync_client.scxml:132 :: listing :: _transition_3
                if (pendingSyncResponsePayload == null) {
                    return
                }

            try { outcome = com.sce.forge.runtime.SceChecked.take(syncFailure(pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed") }

            try { retryAt = com.sce.forge.runtime.SceChecked.take(syncRetryAt(retryAt, pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status, pendingSyncResponsePayload!!.now, pendingSyncResponsePayload!!.retryAfter)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='retryAt'>: an integer operation overflowed or failed") }
            }
            else -> {}
        }
        is SyncClientState.Uploading -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: sync_client.scxml:91 :: uploading :: _transition_0
                if (pendingSyncResponsePayload == null) {
                    return
                }

            try { outcome = com.sce.forge.runtime.SceChecked.take(syncFailure(pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed") }

            try { retryAt = com.sce.forge.runtime.SceChecked.take(syncRetryAt(retryAt, pendingSyncResponsePayload!!.kind, pendingSyncResponsePayload!!.status, pendingSyncResponsePayload!!.now, pendingSyncResponsePayload!!.retryAfter)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='retryAt'>: an integer operation overflowed or failed") }
            }
            1 -> {
                // SCE-MAP: sync_client.scxml:96 :: uploading :: _transition_1

            try { uploaded = com.sce.forge.runtime.SceChecked.add(uploaded, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='uploaded'>: an integer operation overflowed or failed") }
            }
            2 -> {
                // SCE-MAP: sync_client.scxml:101 :: uploading :: _transition_2

            try { discarded = com.sce.forge.runtime.SceChecked.add(discarded, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='discarded'>: an integer operation overflowed or failed") }

            fullListing = true

            byToken = false
            }
            3 -> {
                // SCE-MAP: sync_client.scxml:108 :: uploading :: _transition_3
                if (pendingSyncResponsePayload == null) {
                    return
                }

            try { outcome = com.sce.forge.runtime.SceChecked.take(syncFailure(4.toUByte(), pendingSyncResponsePayload!!.status)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed") }

            try { retryAt = com.sce.forge.runtime.SceChecked.take(syncRetryAt(retryAt, 4.toUByte(), pendingSyncResponsePayload!!.status, pendingSyncResponsePayload!!.now, pendingSyncResponsePayload!!.retryAfter)) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SyncClientEvent.Error.Execution, "<assign location='retryAt'>: an integer operation overflowed or failed") }
            }
            else -> {}
        }
        }
    }
}
