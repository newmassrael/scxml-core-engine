// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import kotlin.time.Clock
import kotlin.time.ExperimentalTime

/** Why a saved state cannot be restored, or a machine cannot be saved. */
class StateRefusal(message: String) : Exception(message) {
    /**
     * This refusal, of the child session at [path] of the session that reads it
     * (`invokes[0].session`): a tree is refused as a whole, and the reason names
     * the session it is about by its path from the root (docs/adr/0045, section
     * 2), `invokes[0].session.invokes[1].session` for a grandchild.
     *
     * Cross-language sibling: `sce_rust_runtime::saved_state::StateRefusal::within`.
     */
    fun within(path: String): StateRefusal {
        val reason = message ?: ""
        return if (reason.startsWith(SESSION_AT)) {
            StateRefusal("$SESSION_AT$path.${reason.removePrefix(SESSION_AT)}")
        } else {
            StateRefusal("$SESSION_AT$path: $reason")
        }
    }

    private companion object {
        /** What a refusal inside a child session of a tree begins with, before the session's path. */
        const val SESSION_AT = "the session at "
    }
}

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
 *   (§scxml-6.4), in document order: in format 2 each with the child's whole
 *   state, which a restore resumes where it stood; in format 1 by the id alone,
 *   which a restore starts again from its beginning ([SavedInvokes]). One whose
 *   child has ended is absent — its `done.invoke` is in the external queue, or
 *   already taken — so it is not started twice.
 * @property sessionId the session's own id (`_sessionid`), which events held in
 *   the tree name sessions by and a restore binds again (docs/adr/0045). An XML
 *   Nmtoken in format 2; empty in a format-1 state, which holds none.
 * @property invokeStarts how many times each `<invoke>` of the document has
 *   started in this session, by its id, for each started at least once, in
 *   document order: a start after a restore is numbered on from it. Empty in a
 *   format-1 state, which holds none.
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
 * @property revisions the revision each record schema past its first was saved
 *   at, by the event the schema declares, ordered by it (docs/adr/0044). Empty
 *   when every record schema the machine holds is in its first revision, which
 *   is every state saved before schemas had revisions; [upgrade] reads a state
 *   saved at an earlier revision as the machine's own.
 */
class SavedState(
    val shape: String,
    val configuration: List<String>,
    val current: String,
    val variables: Map<String, Any?>,
    val history: Map<String, List<String>> = emptyMap(),
    val pending: List<SavedSend> = emptyList(),
    val invokes: SavedInvokes = SavedInvokes.Restarted(emptyList()),
    val hostInvokes: List<SavedHostInvoke> = emptyList(),
    val hostInvokeToken: Long = 0L,
    val autoSendSeq: Long = 0L,
    val external: List<SavedEvent> = emptyList(),
    val revisions: Map<String, SavedRevision> = emptyMap(),
    val sessionId: String = "",
    val invokeStarts: Map<String, Long> = emptyMap(),
) {
    /** The variable [id], or a refusal naming it. */
    fun variable(id: String): Any? {
        if (!variables.containsKey(id)) throw StateRefusal("the saved state has no variable '$id'")
        return variables[id]
    }

    /**
     * The format this state is of: [FORMAT_2] for a tree of sessions, [FORMAT_1]
     * for a state that names its running children by id alone.
     */
    val format: Int
        get() = when (invokes) {
            is SavedInvokes.Restarted -> FORMAT_1
            is SavedInvokes.Resumed -> FORMAT_2
        }

    /** The id the document gives each running `<invoke>`, in order. */
    val invokeIds: List<String> get() = invokes.ids

    /**
     * The id a restore binds the session to: the saved one, for a format-2 state,
     * since the session carries on and every child's id and a peer's address
     * name it (docs/adr/0045); `null` for a format-1 state, which holds none, so
     * the restored machine keeps the id it was created with.
     */
    val boundSessionId: String? get() = if (format == FORMAT_2) sessionId else null

    /**
     * How many times the `<invoke>` [id] had started in this session when it was
     * saved: 0 for one that had not, and for every one of a format-1 state, which
     * holds no count.
     */
    fun invokeStart(id: String): Long = invokeStarts[id] ?: 0L

    /** The state as `sce-saved-state` JSON, in the format it is of: the root of the text, with its `format`. */
    fun toJson(): String {
        val members = linkedMapOf<String, Any?>("format" to Json.Number(format.toString()))
        members.putAll(sessionMembers())
        return Json.write(members)
    }

    /**
     * The members of this session, in the order every backend writes them:
     * format 1's, and in format 2 `sessionid` after `shape` and `invokestarts`
     * after `invokes`, each running child's session written by the same rule.
     */
    internal fun sessionMembers(): LinkedHashMap<String, Any?> {
        val members = linkedMapOf<String, Any?>("shape" to shape)
        val resumed = invokes as? SavedInvokes.Resumed
        if (resumed != null) members["sessionid"] = sessionId
        members["configuration"] = configuration
        members["current"] = current
        members["variables"] = variables
        members["history"] = history
        members["pending"] = pending.map { it.toJsonValue() }
        if (resumed != null) {
            members["invokes"] = resumed.children.map { it.toJsonValue() }
            members["invokestarts"] = invokeStarts.mapValuesTo(LinkedHashMap()) { (_, count) -> count.toString() }
        } else {
            members["invokes"] = invokes.ids
        }
        members["hostinvokes"] = hostInvokes.map { it.toJsonValue() }
        members["hostinvoketoken"] = hostInvokeToken.toString()
        members["sendseq"] = autoSendSeq.toString()
        members["external"] = external.map { it.toJsonValue() }
        // Absent, not empty, when no record schema is past its first revision:
        // the text of such a state is the text it was before revisions existed.
        if (revisions.isNotEmpty()) {
            members["revisions"] = revisions.mapValuesTo(LinkedHashMap()) { (_, saved) -> saved.toJsonValue() }
        }
        return members
    }

    override fun equals(other: Any?): Boolean =
        other is SavedState && other.shape == shape && other.configuration == configuration &&
            other.current == current && other.variables == variables && other.history == history &&
            other.pending == pending && other.invokes == invokes && other.hostInvokes == hostInvokes &&
            other.hostInvokeToken == hostInvokeToken && other.autoSendSeq == autoSendSeq &&
            other.external == external && other.revisions == revisions && other.sessionId == sessionId &&
            other.invokeStarts == invokeStarts

    override fun hashCode(): Int =
        listOf(
            shape, configuration, current, variables, history, pending, invokes, hostInvokes, hostInvokeToken,
            autoSendSeq, external, revisions, sessionId, invokeStarts,
        ).hashCode()

    companion object {
        /** Format 1: a state that names its running children by id alone. */
        const val FORMAT_1: Int = 1

        /** Format 2: a tree of sessions, each running child saved inside its parent (docs/adr/0045). */
        const val FORMAT_2: Int = 2

        /** The newest format version this runtime writes; it reads this one and [FORMAT_1]. */
        const val FORMAT: Int = FORMAT_2

        /**
         * The stability status of the saved-state wire surface, held in
         * lockstep with `x-sce-schema-status` in
         * `schemas/sce-saved-state.v1.schema.json` and
         * `schemas/sce-saved-state.v2.schema.json`, the surface's two formats
         * (`SCE_WIRE_CONTRACTS.md`). A flip to `"stable"` changes all three in
         * one commit.
         */
        const val SCHEMA_STATUS: String = "pre-release"

        /**
         * Read `sce-saved-state` JSON, of format 1 or 2. The shape is not judged
         * here — the machine that restores it knows its own.
         */
        fun fromJson(text: String): SavedState {
            val value = try {
                Json.parse(text)
            } catch (e: Json.Error) {
                throw StateRefusal("the saved state is ${e.message}")
            }
            val members = value as? Map<*, *> ?: throw StateRefusal("the saved state is not an object")
            if (!members.containsKey("format")) throw StateRefusal("the saved state has no 'format'")
            val written = members["format"]
            val format = when (written) {
                Json.Number(FORMAT_1.toString()) -> FORMAT_1
                Json.Number(FORMAT_2.toString()) -> FORMAT_2
                else -> throw StateRefusal(
                    "the saved state is format ${Json.write(written)}, and this runtime reads formats " +
                        "$FORMAT_1 and $FORMAT_2")
            }
            return fromSession(members, format)
        }

        /**
         * The session [members] hold, of a text of [format]: the root, or a child's
         * `session`, which states no `format` of its own.
         */
        internal fun fromSession(members: Map<*, *>, format: Int): SavedState {
            fun field(name: String): Any? {
                if (!members.containsKey(name)) throw StateRefusal("the saved state has no '$name'")
                return members[name]
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
                .mapIndexed { i, item -> SavedSend.fromJsonValue(item, "pending[$i]", format) }
            val listed = field("invokes") as? List<*> ?: throw StateRefusal("'invokes' is not an array")
            val invokes = if (format == FORMAT_1) {
                SavedInvokes.Restarted(listed.map { text(it, "invokes") })
            } else {
                SavedInvokes.Resumed(listed.mapIndexed { i, item -> SavedChild.fromJsonValue(item, i) })
            }
            // Format 2's own members: the session's id and its count of starts.
            var sessionId = ""
            val invokeStarts = LinkedHashMap<String, Long>()
            if (format == FORMAT_2) {
                sessionId = text(field("sessionid"), "sessionid")
                SessionId.refusalOf(sessionId)?.let {
                    throw StateRefusal("'sessionid' ($sessionId) is not an XML Nmtoken, which a session id is")
                }
                val counted = field("invokestarts") as? Map<*, *> ?: throw StateRefusal("'invokestarts' is not an object")
                for ((id, count) in counted) {
                    invokeStarts[id as String] = readCount(
                        count as? String ?: throw StateRefusal("'invokestarts.$id' is not a text"),
                        "invokestarts.$id",
                    )
                }
            }
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
            // Optional, unlike every member above: a state whose record schemas are
            // all in their first revision leaves it out, as every state saved before
            // schemas had revisions does.
            val revisions = LinkedHashMap<String, SavedRevision>()
            if (members.containsKey("revisions")) {
                val named = members["revisions"] as? Map<*, *> ?: throw StateRefusal("'revisions' is not an object")
                if (named.isEmpty()) {
                    throw StateRefusal(
                        "'revisions' names no schema; a state whose record schemas are all in their first " +
                            "revision leaves it out")
                }
                for ((event, value) in named) {
                    revisions[event as String] = SavedRevision.fromJsonValue(value, "revisions.$event")
                }
            }
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
                revisions = revisions,
                sessionId = sessionId,
                invokeStarts = invokeStarts,
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

        /**
         * [saved] as a state of the machine whose saved state is bound to [shape],
         * or the refusal that says why it is not one (docs/adr/0044) — judged
         * before any variable is read, as [checkShape] is.
         *
         * A state that names no revision is of the first shape; one that does is
         * of the revised shape, and each revision it names is of a schema a record
         * variable of this machine is of, no later than this machine's revision of
         * it, and declares what this machine's revision of that number declares
         * (the digests agree). Such a state is then read as one of this machine:
         * each record of a schema saved at an earlier revision — a record
         * variable, or each element of a list of records — takes the default of
         * every field a later revision added. A record that already holds such a
         * field is refused, since the revision it was saved at does not declare
         * it, and so is one that holds `null` in a field a later revision widened
         * to optional, since at the revision it was saved at the field was a value
         * of its present type and no build wrote it absent (docs/adr/0055, section
         * 6). What is not a record where a record is held is left for the
         * variable's own read to refuse.
         *
         * The events of the external queue and of the waiting sends hold their
         * data as the wire carried it, and the lift reads it with the defaults
         * when each is delivered, so nothing of them is grown here.
         *
         * Cross-language sibling: `sce_rust_runtime::saved_state::upgrade`.
         */
        fun upgrade(saved: SavedState, shape: SavedShape): SavedState {
            // A format-2 state names an enum by its document and a format-1 state by
            // the alias it is imported under, so each is judged against its own
            // shape (docs/adr/0045, section 2).
            val first = if (saved.format == FORMAT_2) shape.format2First else shape.first
            val revised = if (saved.format == FORMAT_2) shape.format2Revised else shape.revised
            when {
                saved.revisions.isEmpty() -> checkShape(saved, first)
                saved.shape == first -> throw StateRefusal(
                    "the saved state names revisions of its record schemas and has the shape of a state " +
                        "that names none")
                else -> checkShape(saved, revised)
            }
            for ((key, named) in saved.revisions) {
                // A record schema by the event it declares, an enum by its key.
                val (revision, digests) = shape.records.firstOrNull { it.event == key }
                    ?.let { it.revision to it.revisions }
                    ?: shape.enums.firstOrNull { it.key == key }?.let { it.revision to it.revisions }
                    ?: throw StateRefusal("'revisions' names '$key', which no record or enum of this machine is")
                if (named.revision > revision) {
                    throw StateRefusal(
                        "the saved state was saved at revision ${named.revision} of '$key', later than this " +
                            "machine's $revision: a machine does not restore what it does not know")
                }
                // The digest of revision r is the (r - 2)th: the first has none.
                if (digests.getOrNull(named.revision - 2) != named.fields) {
                    throw StateRefusal(
                        "revision ${named.revision} of '$key' declared other members where the state was " +
                            "saved than it does in this machine")
                }
            }
            // An enum value saved at an earlier revision of its enum is one of that
            // revision's variants. Judged on the values as saved, before a record is
            // grown by defaults, which are this machine's.
            for (lineage in shape.enums) {
                val at = saved.revisions[lineage.key]?.revision ?: 1
                val later = lineage.added.filter { it.since > at }
                if (later.isEmpty()) continue
                for (holder in lineage.holders) checkEnumHolder(saved, lineage, holder, at, later)
            }
            // A record's field saved before the revision that widened it holds a
            // value of `T`, so a `null` there is no absence and the state is not
            // one any build wrote. Judged on the values as saved, before a record
            // is grown by defaults, which are this machine's and may be `null` for
            // a field added absent.
            for (record in shape.records) {
                val at = saved.revisions[record.event]?.revision ?: 1
                val held = record.widened.filter { it.since <= at && at < it.optionalSince }
                if (held.isEmpty()) continue
                for (holder in record.holders) checkWidenedHolder(saved, record, holder, at, held)
            }
            val variables = LinkedHashMap(saved.variables)
            for (record in shape.records) {
                val at = saved.revisions[record.event]?.revision ?: 1
                val added = record.added.filter { it.since > at }
                if (added.isEmpty()) continue
                for (holder in record.holders) {
                    if (!variables.containsKey(holder.variable)) continue
                    val value = variables[holder.variable]
                    variables[holder.variable] = if (holder.list) {
                        (value as? List<*>)?.mapIndexed { i, item ->
                            grownRecord(item, "${holder.variable}[$i]", record, at, added)
                        } ?: value
                    } else {
                        grownRecord(value, holder.variable, record, at, added)
                    }
                }
            }
            return SavedState(
                shape = saved.shape,
                configuration = saved.configuration,
                current = saved.current,
                variables = variables,
                history = saved.history,
                pending = saved.pending,
                invokes = saved.invokes,
                hostInvokes = saved.hostInvokes,
                hostInvokeToken = saved.hostInvokeToken,
                autoSendSeq = saved.autoSendSeq,
                external = saved.external,
                revisions = saved.revisions,
                sessionId = saved.sessionId,
                invokeStarts = saved.invokeStarts,
            )
        }

        /**
         * Refuse a value [holder] holds, in [saved], that names one of [later], the
         * variants of [lineage] added after [at], the revision it was saved at. What
         * is not a text where one is held is left for the variable's own read to
         * refuse.
         */
        private fun checkEnumHolder(
            saved: SavedState,
            lineage: EnumLineage,
            holder: EnumHolder,
            at: Int,
            later: List<AddedVariant>,
        ) {
            if (!saved.variables.containsKey(holder.variable)) return
            val value = saved.variables[holder.variable]
            val field = holder.field
            // Each value the holder holds, with what names it in a refusal.
            val held: List<Pair<String, Any?>> = when {
                field == null -> listOf(holder.variable to value)
                !holder.list -> (value as? Map<*, *>)
                    ?.takeIf { it.containsKey(field) }
                    ?.let { listOf("${holder.variable}.$field" to it[field]) }
                    ?: emptyList()
                else -> (value as? List<*>).orEmpty().mapIndexedNotNull { i, record ->
                    (record as? Map<*, *>)?.takeIf { it.containsKey(field) }
                        ?.let { "${holder.variable}[$i].$field" to it[field] }
                }
            }
            for ((what, name) in held) {
                val variant = later.firstOrNull { it.name == name } ?: continue
                throw StateRefusal(
                    "'$what' holds '$name', which revision $at of '${lineage.key}' does not declare: " +
                        "revision ${variant.since} added it")
            }
        }

        /**
         * Refuse a `null` the record, or each record of the list, [holder] holds in
         * [saved] has in a field of [widened], the fields of [record] that a revision
         * after [at], the one it was saved at, widened to optional: at [at] each was
         * a value of `T`. What is not a record where one is held, and a member the
         * record lacks, is left for the variable's own read to refuse.
         */
        private fun checkWidenedHolder(
            saved: SavedState,
            record: RecordLineage,
            holder: RecordHolder,
            at: Int,
            widened: List<WidenedField>,
        ) {
            if (!saved.variables.containsKey(holder.variable)) return
            val value = saved.variables[holder.variable]
            if (holder.list) {
                (value as? List<*>)?.forEachIndexed { i, item ->
                    checkWidenedRecord(item, "${holder.variable}[$i]", record, at, widened)
                }
            } else {
                checkWidenedRecord(value, holder.variable, record, at, widened)
            }
        }

        /**
         * Refuse the saved record [value], of [record] saved at revision [at], if it
         * holds `null` in a field of [widened]; [what] names it in the refusal.
         */
        private fun checkWidenedRecord(
            value: Any?,
            what: String,
            record: RecordLineage,
            at: Int,
            widened: List<WidenedField>,
        ) {
            val members = value as? Map<*, *> ?: return
            for (field in widened) {
                if (members.containsKey(field.id) && members[field.id] == null) {
                    throw StateRefusal(
                        "'$what.${field.id}' is null, which revision $at of '${record.event}' holds as a " +
                            "value: revision ${field.optionalSince} widened it")
                }
            }
        }

        /**
         * The saved record [value], of [record] saved at revision [at], with the
         * default of each of [added], the fields later revisions added; [what]
         * names it in a refusal. A value that is not an object is returned as it
         * is, for the record's own read to refuse.
         */
        private fun grownRecord(
            value: Any?,
            what: String,
            record: RecordLineage,
            at: Int,
            added: List<AddedField>,
        ): Any? {
            val members = value as? Map<*, *> ?: return value
            val grown = LinkedHashMap<String, Any?>()
            for ((name, member) in members) grown[name as String] = member
            for (field in added) {
                if (grown.containsKey(field.id)) {
                    throw StateRefusal(
                        "'$what' holds '${field.id}', which revision $at of '${record.event}' does not declare: " +
                            "revision ${field.since} added it")
                }
                grown[field.id] = try {
                    Json.parse(field.default)
                } catch (e: Json.Error) {
                    throw StateRefusal("the default of '${field.id}' of '${record.event}' is ${e.message}")
                }
            }
            return grown
        }
    }
}

/**
 * The running child sessions of a saved session, as the format it was written
 * in holds them.
 */
sealed interface SavedInvokes {
    /** The id the document gives each running `<invoke>`, in order. */
    val ids: List<String>

    /**
     * Format 1: the id of each `<invoke>` whose child is running. The text holds
     * nothing of a child, so a restore starts each again from its beginning,
     * under the same id: all that state promised.
     */
    data class Restarted(override val ids: List<String>) : SavedInvokes

    /**
     * Format 2: each running child with its whole state, resumed by a restore
     * where it stood (docs/adr/0045).
     */
    data class Resumed(val children: List<SavedChild>) : SavedInvokes {
        override val ids: List<String> get() = children.map { it.id }
    }
}

/**
 * A running child session, saved inside the session that started it
 * (docs/adr/0045, section 1).
 *
 * @property id the id the document gives the `<invoke>` that started it.
 * @property start which start of that `<invoke>` this is in its parent, counted
 *   from 1: the count the parent's [SavedState.invokeStarts] holds for it.
 * @property invokeId the id the child was started under, as `_event.invokeid`
 *   and an `idlocation` variable hold it: carried, not computed again.
 * @property document the document the child runs: the child's name for a static
 *   `<invoke>`, the candidate's for a hybrid one.
 * @property session the child's own state, its running children included.
 */
data class SavedChild(
    val id: String,
    val start: Long,
    val invokeId: String,
    val document: String,
    val session: SavedState,
) {
    internal fun toJsonValue(): Map<String, Any?> = linkedMapOf(
        "id" to id,
        "start" to start.toString(),
        "invokeid" to invokeId,
        "document" to document,
        "session" to session.sessionMembers(),
    )

    internal companion object {
        /**
         * The [index]th running child of a format-2 session. A refusal inside the
         * child's own session names it by its path ([StateRefusal.within]).
         */
        fun fromJsonValue(value: Any?, index: Int): SavedChild {
            val what = "invokes[$index]"
            val members = value as? Map<*, *> ?: throw StateRefusal("'$what' is not an object")
            fun text(key: String): String {
                if (!members.containsKey(key)) throw StateRefusal("'$what' has no '$key'")
                val written = members[key] as? String ?: throw StateRefusal("'$what.$key' is not a text")
                if (written.isEmpty()) throw StateRefusal("'$what.$key' is empty")
                return written
            }
            val start = readCount(text("start"), "$what.start")
            if (!members.containsKey("session")) throw StateRefusal("'$what' has no 'session'")
            val sessionMembers = members["session"] as? Map<*, *>
                ?: throw StateRefusal("'$what.session' is not an object")
            if (sessionMembers.containsKey("format")) {
                throw StateRefusal("'$what.session' states a format, which only the root of a tree does")
            }
            val session = try {
                SavedState.fromSession(sessionMembers, SavedState.FORMAT_2)
            } catch (refusal: StateRefusal) {
                throw refusal.within("$what.session")
            }
            return SavedChild(
                id = text("id"),
                start = start,
                invokeId = text("invokeid"),
                document = text("document"),
                session = session,
            )
        }
    }
}

/**
 * A count of starts, as the text a saved state writes it as: a whole number
 * from 1 with no leading zero, within a signed 64-bit count, as every backend
 * reads one.
 */
private fun readCount(written: String, what: String): Long =
    written.toLongOrNull()?.takeIf { it >= 1 && !written.startsWith('0') && written.all(Char::isDigit) }
        ?: throw StateRefusal("'$what' ($written) is not a count of starts, a whole number from 1")

/**
 * The revision of one record schema a state was saved at.
 *
 * @property revision the schema's `sce:revision` where the state was saved, 2 or
 *   later.
 * @property fields the SHA-256, as lowercase hex, of what that revision declares:
 *   each field it has, with its type and its enum's variant names. A restoring
 *   machine holds the same digest of each of its own schema's revisions, so two
 *   builds that disagree about what one revision declares are told apart.
 */
data class SavedRevision(val revision: Int, val fields: String) {
    internal fun toJsonValue(): Map<String, Any?> = linkedMapOf(
        "revision" to Json.Number(revision.toString()),
        "fields" to fields,
    )

    internal companion object {
        fun fromJsonValue(value: Any?, what: String): SavedRevision {
            val members = value as? Map<*, *> ?: throw StateRefusal("'$what' is not an object")
            if (!members.containsKey("revision")) throw StateRefusal("'$what' has no 'revision'")
            if (!members.containsKey("fields")) throw StateRefusal("'$what' has no 'fields'")
            val written = (members["revision"] as? Json.Number)?.text
                ?: throw StateRefusal("'$what.revision' is not a number")
            val revision = written.toIntOrNull()?.takeIf { it >= 2 && written.all(Char::isDigit) }
                ?: throw StateRefusal(
                    "'$what.revision' ($written) is not a revision past the first; a schema in its first " +
                        "revision is left out")
            val fields = (members["fields"] as? String)
                ?.takeIf { digest -> digest.length == 64 && digest.all { it in '0'..'9' || it in 'a'..'f' } }
                ?: throw StateRefusal("'$what.fields' is not a SHA-256 written as lowercase hex")
            return SavedRevision(revision, fields)
        }
    }
}

/**
 * What a machine's saved state is bound to: the document's shape, and the
 * revisions of each schema a record variable of it is of (docs/adr/0044). The
 * generator computes it, and every saved state is written and judged against it.
 *
 * @property first the shape a state names when every record schema it holds is
 *   in its first revision: the states, histories, invocations and variables,
 *   each record variable with the fields of its schema's first revision.
 * @property revised the shape a state names when it names a later revision of
 *   any record schema. A reader that predates revisions knows only the first
 *   shape, so it refuses such a state rather than read it without the fields it
 *   does not know.
 * @property format2First [first] as a format-2 state names it (docs/adr/0045,
 *   section 2): each enum-typed variable and record field spelled by its enum's
 *   document ([EnumLineage.key]) rather than by the alias it is imported under,
 *   so renaming an alias leaves it as it was. The same as [first] for a
 *   document that holds no enum.
 * @property format2Revised [revised] as a format-2 state names it.
 * @property records each record schema a variable is of, by the event it
 *   declares, ordered by it.
 * @property enums each enum a value of the state is of, by its key, ordered by
 *   it.
 */
class SavedShape(
    val first: String,
    val revised: String,
    val format2First: String,
    val format2Revised: String,
    val records: List<RecordLineage>,
    val enums: List<EnumLineage> = emptyList(),
)

/**
 * An enum a value of a machine's state is of, as a saved state names it
 * (docs/adr/0044).
 *
 * @property key `enum:` and the enum's document, which names it in
 *   `revisions`: a standard document by its `sce:std/...` name, any other by
 *   its path relative to the machine's own document, `/`-separated. The
 *   document, not an alias it is imported under, so renaming an alias leaves
 *   it as it was.
 * @property revision the enum's revision in this machine.
 * @property revisions the digest of what each revision from the second to
 *   [revision] declares, in order ([SavedRevision.fields]).
 * @property added each variant a revision past the first added.
 * @property holders where a value of it is held.
 */
class EnumLineage(
    val key: String,
    val revision: Int,
    val revisions: List<String>,
    val added: List<AddedVariant>,
    val holders: List<EnumHolder>,
)

/**
 * A variant a revision past the first added to an enum.
 *
 * @property name the variant's declared name, which a saved state holds it by.
 * @property since the revision that added it.
 */
class AddedVariant(val name: String, val since: Int)

/**
 * Where a value of an [EnumLineage] is held.
 *
 * @property variable the variable's document id.
 * @property field the field of the record the variable holds, for a value held
 *   in a record; `null` for a variable of the enum.
 * @property list whether the variable holds a list of such records rather than
 *   one.
 */
class EnumHolder(val variable: String, val field: String?, val list: Boolean)

/**
 * A record schema a machine's variables are of, as a saved state names it.
 *
 * @property event the event the schema declares, which names it in `revisions`.
 * @property revision the schema's revision in this machine.
 * @property revisions the digest of what each revision from the second to
 *   [revision] declares, in order ([SavedRevision.fields]).
 * @property added each field a revision past the first added, in the schema's
 *   order.
 * @property widened each field a revision widened from `T` to `optional<T>`, in
 *   the schema's order (docs/adr/0055, section 6).
 * @property holders the variables that hold a record of it, or a list of them.
 */
class RecordLineage(
    val event: String,
    val revision: Int,
    val revisions: List<String>,
    val added: List<AddedField>,
    val widened: List<WidenedField>,
    val holders: List<RecordHolder>,
)

/**
 * A field a revision widened from `T` to `optional<T>` (`sce:optional-since`).
 * A record saved before that revision holds it as a value of `T`, so a `null`
 * there is no absence: [SavedState.upgrade] refuses it.
 *
 * @property id the field's id, the member a saved record holds it under.
 * @property since the revision that added the field (1 for a field of the first
 *   revision): a record saved before it holds no member for it.
 * @property optionalSince the revision that widened it.
 */
class WidenedField(val id: String, val since: Int, val optionalSince: Int)

/**
 * A field a revision past the first added to a record schema.
 *
 * @property id the field's id, the member a saved record holds it under.
 * @property since the revision that added it.
 * @property default its `sce:default` as saved-state JSON: the JSON a delivery
 *   carries, which a saved state writes the same way for every type a field can
 *   have.
 */
class AddedField(val id: String, val since: Int, val default: String)

/**
 * A variable that holds a record of a [RecordLineage].
 *
 * @property variable the variable's document id.
 * @property list whether it holds a list of records rather than one.
 */
class RecordHolder(val variable: String, val list: Boolean)

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
            is SavedAct.Parent -> members.putAll(
                listOf(
                    "act" to "parent",
                    "event" to act.event,
                    "data" to act.data,
                    "sendid" to act.sendId,
                    "origin" to act.origin,
                )
            )
            is SavedAct.Child -> members.putAll(
                listOf(
                    "act" to "child",
                    "invokeid" to act.invokeId,
                    "event" to act.event,
                    "data" to act.data,
                    "sendid" to act.sendId,
                    "origin" to act.origin,
                )
            )
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
        /** The waiting send [value], of a state of [format]; a send to the parent or to a child is an act of format 2 only. */
        fun fromJsonValue(value: Any?, what: String, format: Int): SavedSend {
            val members = value as? Map<*, *> ?: throw StateRefusal("'$what' is not an object")
            fun text(key: String): String {
                if (!members.containsKey(key)) throw StateRefusal("'$what' has no '$key'")
                return members[key] as? String ?: throw StateRefusal("'$what.$key' is not a text")
            }
            val due = readMoment(text("due"), "$what.due")
            val act = when (val name = text("act")) {
                "parent" -> if (format == SavedState.FORMAT_2) {
                    SavedAct.Parent(text("event"), text("data"), text("sendid"), text("origin"))
                } else {
                    throw StateRefusal(
                        "'$what.act' is 'parent', which is not raise, internal or host, the acts of format 1")
                }
                "child" -> if (format == SavedState.FORMAT_2) {
                    SavedAct.Child(text("invokeid"), text("event"), text("data"), text("sendid"), text("origin"))
                } else {
                    throw StateRefusal(
                        "'$what.act' is 'child', which is not raise, internal or host, the acts of format 1")
                }
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
                else -> throw StateRefusal(
                    if (format == SavedState.FORMAT_1) {
                        "'$what.act' is '$name', which is not raise, internal or host, the acts of format 1"
                    } else {
                        "'$what.act' is '$name', which is not raise, internal, host, parent or child"
                    }
                )
            }
            return SavedSend(due, act)
        }
    }
}

/**
 * What a waiting `<send>` does when it comes due. The five the delayed sends
 * of a document that saves can be: its own event on this session's external
 * queue or on its internal queue (`#_internal`), an act a host-served
 * processor performs (§scxml-6.2.5), or an event for the session that started
 * this one or for a running child of it, which the tree holds (docs/adr/0045).
 *
 * A delayed send to any other session is none of them, and a document that
 * makes one is generated without the save API (§2.15): such a send is
 * delivered through a session this state does not carry.
 */
sealed interface SavedAct {
    /** An event for the session that started this one (`#_parent`). */
    data class Parent(
        /** The event's name, as the parent resolves it. */
        val event: String,
        /** `_event.data`, as the wire carries it. */
        val data: String,
        /** The `<send>`'s id, which a `<cancel>` names. */
        val sendId: String,
        /** `_event.origin`: this session's id. */
        val origin: String,
    ) : SavedAct

    /** An event for a running child of this session (`#_<invokeid>`). */
    data class Child(
        /** The id the target names the child's `<invoke>` by. */
        val invokeId: String,
        /** The event's name, as the child resolves it. */
        val event: String,
        /** `_event.data`, as the wire carries it. */
        val data: String,
        /** The `<send>`'s id, which a `<cancel>` names. */
        val sendId: String,
        /** `_event.origin`: this session's id. */
        val origin: String,
    ) : SavedAct

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
    //
    // A 32-bit real is written as the 64-bit real it widens to, which is exact:
    // the number a reader that holds JSON numbers as doubles finds is the number
    // the machine holds, `0.30000001192092896` and not the shorter `0.3` a
    // single prints for it, which is another number to that reader.
    fun of(value: Float): Any = of(value.toDouble())
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

    /**
     * A byte string as its byte-exact Latin-1 text, each byte the character of that
     * code point (docs/adr/0005, decision 2): the spelling it takes on the wire, so
     * one value is one text on every backend, and a JSON reader that has no byte
     * string reads it as a string.
     */
    fun of(value: ByteArray): Any = CharArray(value.size) { (value[it].toInt() and 0xFF).toChar() }.concatToString()

    /** A list, each element written by [element]. */
    fun <T> list(value: List<T>, element: (T) -> Any?): Any = value.map(element)

    /**
     * A value that may be absent (docs/adr/0055, section 4): `null` where it is
     * absent, and otherwise the value [present] writes, as a value of its type
     * is written anywhere else.
     */
    fun <T : Any> optional(value: T?, present: (T) -> Any): Any? = value?.let(present)

    /**
     * A value that may be absent read back: absent where the saved state holds
     * `null`, and otherwise the value [present] reads, held to its type as any
     * value is. Only a place the machine declares optional reads through this, so
     * a `null` saved where the machine holds a value of `T` is refused by the
     * reader of `T`, naming what holds it ([notA]).
     */
    fun <T : Any> optional(value: Any?, what: String, present: (Any?, String) -> T): T? =
        if (value == null) null else present(value, what)

    /**
     * The refusal of [value], saved for [what], which is not [kind] (`a number`,
     * `a text`, ...). A `null` is named apart: it is an absent value
     * (docs/adr/0055), which only a place the restoring build declares optional
     * reads, so a state saved after a widening is refused at its first `null` by
     * a build from before it.
     */
    private fun notA(value: Any?, what: String, kind: String): StateRefusal =
        if (value == null) {
            StateRefusal("'$what' is absent (null), and its type is not optional")
        } else {
            StateRefusal("'$what' is not $kind")
        }

    private fun number(value: Any?, what: String): String =
        (value as? Json.Number)?.text ?: throw notA(value, what, "a number")

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
        value as? String ?: throw notA(value, what, "a 64-bit whole number written as a text")

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
        value as? Boolean ?: throw notA(value, what, "a truth value")

    fun string(value: Any?, what: String): String = value as? String ?: throw notA(value, what, "a text")

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
        val items = value as? List<*> ?: throw notA(value, what, "an array")
        if (items.size > capacity) {
            throw StateRefusal("'$what' holds ${items.size} elements, past the $capacity it is bounded by")
        }
        return items.mapIndexed { i, item -> element(item, "$what[$i]") }
    }

    /**
     * A byte string read back from its Latin-1 text ([of]), only if it holds no more
     * than the [capacity] bytes the machine bounds it by — a machine never holds
     * more, and a restored one must not be the first to. A character past U+00FF is
     * no byte, and is refused rather than cut.
     */
    fun bytes(value: Any?, what: String, capacity: Int): ByteArray {
        val text = string(value, what)
        val bytes = ByteArray(text.length) {
            val unit = text[it].code
            if (unit > 0xFF) {
                val hex = unit.toString(16).uppercase().padStart(4, '0')
                throw StateRefusal("'$what' holds U+$hex, which is past U+00FF and so no byte")
            }
            unit.toByte()
        }
        if (bytes.size > capacity) {
            throw StateRefusal("'$what' holds ${bytes.size} bytes, past the $capacity it is bounded by")
        }
        return bytes
    }

    /** The field [name] of a saved record [value] — what a generated record reads each field with. */
    fun field(value: Any?, record: String, name: String): Any? {
        val members = value as? Map<*, *> ?: throw StateRefusal("'$record' is not an object")
        if (!members.containsKey(name)) throw StateRefusal("'$record' has no field '$name'")
        return members[name]
    }
}
