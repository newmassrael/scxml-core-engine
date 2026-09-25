// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import kotlin.jvm.JvmInline

// W3C SCXML Appendix D's microstep — which transitions an event selects, which
// of them survive preemption, which states they exit and in which order, the
// order their content runs in, which states they enter — written once for every
// machine this runtime runs.
//
// The procedures are transcribed rather than paraphrased, one function per
// procedure and under the appendix's names, so a reader can hold this file
// against the specification line by line — and against the other engines'
// transcriptions of it: the C++ engines' sce/include/core/MicrostepAlgorithms.h,
// the Rust runtime's backends/rust/runtime/src/helpers/microstep.rs, the Python
// runtime's backends/python/runtime/sce_runtime/microstep.py and the Go
// runtime's backends/go/runtime/microstep.go, which this file follows function
// for function.
//
// What differs between machines is injected. A [Document] describes the
// document — its structure, and what each `<history>` recorded. A [Run] is the
// running machine: its configuration, which of a state's transitions an event
// enables, and what exiting a state, running a transition's content and
// entering a state DO. Nothing here knows how a machine stores either, so a
// table written by hand answers them as well as a generated machine does.
//
// A document whose `<history>` defaults name one another in a cycle has no
// entry set — dereferencing never reaches a state — and a hierarchy whose parent
// links cycle has no domain; both are generator defects. The procedures assume a
// legal document, and the parent walks below throw rather than spin when handed
// one that is not.

/**
 * The maximum supported state hierarchy depth. It bounds every walk up the
 * parent links here, so a cyclic parent relationship — a generator defect — is
 * reported rather than walked forever. Matches the Rust and Go runtimes' 16.
 *
 * W3C SCXML has no normative depth limit; 16 covers every real-world document
 * (typical: 1-5, complex: up to 10).
 */
const val MAX_HIERARCHY_DEPTH = 16

/**
 * Names one `<history>` pseudo-state of a generated document.
 *
 * A history is not a state — it is never in a configuration — so a generated
 * machine numbers its histories apart from its states, and a target list names
 * one through [HistoryTarget]. The number means nothing outside the machine
 * that issued it. A value class rather than a bare `Int`, so a state's position
 * can never be handed over where a history is meant.
 */
@JvmInline
value class HistoryId(val index: Int)

/**
 * One token of a target list, as the document wrote it.
 *
 * A transition's target, a state's initial transition and a `<history>`'s
 * default transition all name either states or `<history>` pseudo-states. The
 * two kinds are kept apart rather than folded into one identifier: the entry
 * procedures dereference a history (to what it recorded, or to its default) and
 * add a state. Exactly one of [state] and [history] is non-null.
 */
sealed interface EntryTarget<out S : Any, out H : Any> {
    /** The state this token names, or `null` when it names a history. */
    val state: S?

    /** The history this token names, or `null` when it names a state. */
    val history: H?
}

/** The target token that names a `<state>`, `<parallel>` or `<final>`. */
data class StateTarget<out S : Any>(override val state: S) : EntryTarget<S, Nothing> {
    override val history: Nothing? get() = null
}

/** The target token that names a `<history>` pseudo-state. */
data class HistoryTarget<out H : Any>(override val history: H) : EntryTarget<Nothing, H> {
    override val state: Nothing? get() = null
}

/**
 * A transition selection enabled: one member of Appendix D's
 * `enabledTransitions`.
 *
 * What the microstep reads of it — its source, its target list as written,
 * whether it is internal — plus the index its source state knows it by, so the
 * machine that owns its executable content can run it. A transition with no
 * targets exits and enters nothing and only runs its content.
 *
 * [targets] is the document's own table: a generated machine hands out lists
 * it built once, because a transition is a fact about the document, not about
 * the run. Nothing here writes to it.
 *
 * @property source the state whose transition this is
 * @property targets the target list as written — empty for a targetless one
 * @property transitionIndex the position of this transition among its source's
 *   own transitions
 * @property hasActions whether the transition has executable content to run
 * @property isInternal whether the transition was written `type="internal"`
 */
data class EnabledTransition<S : Any, H : Any>(
    val source: S,
    val targets: List<EntryTarget<S, H>>,
    val transitionIndex: Int,
    val hasActions: Boolean,
    val isInternal: Boolean,
) {
    /**
     * Whether the transition has no targets. §scxml-D-computeExitSet guards the
     * whole computation with `if t.target`: a transition without targets exits
     * and enters nothing.
     */
    val isTargetless: Boolean get() = targets.isEmpty()

    /** The same transition as the entry procedures read it. */
    fun entryTransition(): EntryTransition<S, H> = EntryTransition(source, targets, isInternal)
}

/**
 * The part of a transition the entry procedures read.
 *
 * [source] is `null` for the document's own initial transition, whose source is
 * the `<scxml>` element — which has no state identifier here, and whose domain
 * is the whole document.
 */
data class EntryTransition<S : Any, H : Any>(
    val source: S?,
    val targets: List<EntryTarget<S, H>>,
    val isInternal: Boolean = false,
)

/**
 * What a microstep enters: the appendix's three out-parameters.
 *
 * [statesToEnter] is already sorted into entry order, so a machine enters it
 * front to back. [statesForDefaultEntry] answers whether a state's initial
 * transition content runs; [defaultHistoryContent] whether, and whose,
 * `<history>` default transition content runs after that state's own entry —
 * keyed by the history's parent, as the appendix keys it.
 */
class EntrySet<S : Any, H : Any> internal constructor() {
    private val entering = mutableListOf<S>()
    private val enteredByDefault = mutableListOf<S>()
    private val owedHistoryContent = mutableMapOf<S, H>()

    /** The states to enter, in entry order. */
    val statesToEnter: List<S> get() = entering

    /** The compound states whose initial state is entered by default. */
    val statesForDefaultEntry: List<S> get() = enteredByDefault

    /** A history's parent, mapped to the history whose default content it is owed. */
    val defaultHistoryContent: Map<S, H> get() = owedHistoryContent

    /** Whether [state] is entered by this microstep. */
    operator fun contains(state: S): Boolean = state in entering

    /**
     * Whether [state]'s initial state is entered by default, so its initial
     * transition's executable content runs.
     */
    fun isDefaultEntry(state: S): Boolean = state in enteredByDefault

    /**
     * The `<history>` whose default transition content runs after [state] is
     * entered, if a history of [state] was taken with nothing recorded.
     */
    fun defaultHistoryContentOf(state: S): H? = owedHistoryContent[state]

    internal fun enter(state: S) = addOnce(entering, state)

    internal fun enterByDefault(state: S) = addOnce(enteredByDefault, state)

    internal fun oweHistoryContent(parent: S, history: H) {
        owedHistoryContent[parent] = history
    }

    internal fun sortIntoEntryOrder(documentOrder: (S) -> Int) {
        entering.sortBy(documentOrder)
    }
}

/**
 * The document, as the entry and exit procedures read it.
 *
 * Structure only, plus the one piece of run-time state the entry procedures
 * need: what each `<history>` recorded.
 */
interface Document<S : Any, H : Any> {
    /** The state's parent, or `null` when its parent is the `<scxml>` element. */
    fun parentOf(state: S): S?

    /**
     * Whether the state is a `<state>` with child states. A `<parallel>`
     * answers false: this is the appendix's isCompoundState, and with the
     * `<scxml>` element it is the set findLCCA chooses a domain from.
     */
    fun isCompound(state: S): Boolean

    /** Whether the state is a `<parallel>`. */
    fun isParallel(state: S): Boolean

    /**
     * Whether the state is a `<final>` element — not whether it is IN a final
     * state; that is [Microstep.isInFinalState].
     */
    fun isFinal(state: S): Boolean

    /**
     * §scxml-D-getChildStates: the state's `<state>`, `<parallel>` and
     * `<final>` children, in document order.
     */
    fun childStates(state: S): List<S>

    /**
     * A compound state's initial transition target, as written — the first
     * child state when the document names none.
     */
    fun initialTargets(state: S): List<EntryTarget<S, H>>

    /** The state a `<history>` is declared in. */
    fun historyParent(history: H): S

    /**
     * What the history recorded when its parent was last exited, or `null`
     * before that ever happened.
     */
    fun historyValue(history: H): List<S>?

    /** A `<history>`'s default transition target, as written. */
    fun historyDefaultTargets(history: H): List<EntryTarget<S, H>>

    /** The state's position in document order, which is also entry order. */
    fun documentOrder(state: S): Int
}

/**
 * The running machine, as the microstep drives it. An event of `null` is the
 * machine's "no event": the eventless selection.
 */
interface Run<S : Any, H : Any, E : Any> : Document<S, H> {
    /**
     * The active states, in any order. The procedures hold the answer across
     * the exits and entries they drive, so it must not alias storage those
     * mutate.
     */
    fun configuration(): List<S>

    /**
     * This state's first transition, in document order, that [event] enables
     * and whose guard holds; for `null`, its first eventless transition whose
     * guard holds. The only place a guard is evaluated.
     */
    fun firstEnabledTransition(state: S, event: E?): EnabledTransition<S, H>?

    /**
     * Records this state's histories from the configuration as it stood before
     * the microstep's first exit, runs its onexit, cancels its invocations and
     * removes it from the configuration.
     */
    fun exitState(state: S, configurationBeforeExit: List<S>)

    /** Runs one transition's executable content. */
    fun executeTransitionContent(transition: EnabledTransition<S, H>)

    /**
     * Adds the state to the configuration and schedules its invocations, runs
     * its onentry, then its initial transition's content when [isDefaultEntry];
     * for a `<final>`, what the appendix does on entering one.
     */
    fun enterState(state: S, isDefaultEntry: Boolean)

    /** Runs a `<history>`'s default transition content. */
    fun executeHistoryDefaultContent(history: H)
}

/** Appendix D's procedures, over a [Document] and a [Run]. */
object Microstep {

    // ════════════════════════════════════════════════════════════════════════
    // Selection
    // ════════════════════════════════════════════════════════════════════════

    /**
     * Appendix D's selectTransitions, or its selectEventlessTransitions when
     * [event] is `null`.
     *
     * @return the optimal enabled transition set, in selection order
     */
    fun <S : Any, H : Any, E : Any> selectTransitions(run: Run<S, H, E>, event: E?): List<EnabledTransition<S, H>> {
        val configuration = run.configuration()
        val atomicStates = configuration
            .filter { !run.isCompound(it) && !run.isParallel(it) }
            .sortedBy { run.documentOrder(it) }

        val enabled = mutableListOf<EnabledTransition<S, H>>()
        for (atomic in atomicStates) {
            // §scxml-D-selectTransitions: the atomic state first, then its
            // proper ancestors, and the first enabled transition in document
            // order ends the walk for this atomic state. The set is ORDERED and
            // a set: two atomic states under one ancestor both reach its
            // transition, and it is one transition, taken once. With no event
            // this is §scxml-D-selectEventlessTransitions, the same walk over
            // transitions that have no event.
            var current: S? = atomic
            var depth = 0
            while (current != null) {
                if (depth == MAX_HIERARCHY_DEPTH) {
                    error("selectTransitions: cyclic parent relationship detected walking from $atomic")
                }
                val found = run.firstEnabledTransition(current, event)
                if (found != null) {
                    val seen = enabled.any {
                        it.source == found.source && it.transitionIndex == found.transitionIndex
                    }
                    if (!seen) enabled.add(found)
                    break
                }
                current = run.parentOf(current)
                depth++
            }
        }
        return removeConflictingTransitions(run, enabled, configuration)
    }

    /**
     * Appendix D's removeConflictingTransitions.
     *
     * Two transitions conflict when their exit sets intersect, and the one whose
     * source is a descendant of the other's wins; otherwise the one selected
     * first does.
     */
    fun <S : Any, H : Any> removeConflictingTransitions(
        doc: Document<S, H>,
        enabled: List<EnabledTransition<S, H>>,
        configuration: List<S>,
    ): List<EnabledTransition<S, H>> {
        var filtered = listOf<EnabledTransition<S, H>>()
        for (t1 in enabled) {
            val t1Exit = computeExitSet(doc, t1, configuration)
            val conflicts = { t2: EnabledTransition<S, H> ->
                intersects(t1Exit, computeExitSet(doc, t2, configuration))
            }

            // §scxml-D-removeConflictingTransitions: t1 is preempted by any kept
            // transition it conflicts with whose source it does not descend
            // from. A transition that exits nothing — a targetless one —
            // conflicts with nothing and can never be preempted.
            val preempted = filtered.any { t2 -> conflicts(t2) && !isDescendant(doc, t1.source, t2.source) }
            if (preempted) continue
            // Not preempted means every kept transition t1 conflicts with has a
            // source t1 descends from: the appendix removes exactly those.
            filtered = filtered.filterNot(conflicts) + t1
        }
        return filtered
    }

    // ════════════════════════════════════════════════════════════════════════
    // Domains and exit sets
    // ════════════════════════════════════════════════════════════════════════

    /**
     * Appendix D's getEffectiveTargetStates, over a target list.
     *
     * @return the states the list stands for, histories dereferenced — to what
     *   each recorded or, before its parent was ever exited, to its default —
     *   each state once and in the order first named. The domain, and so the
     *   exit set, is a question about these.
     */
    fun <S : Any, H : Any> effectiveTargetStates(doc: Document<S, H>, targets: List<EntryTarget<S, H>>): List<S> {
        val effective = mutableListOf<S>()
        addEffectiveTargetStates(doc, targets, effective)
        return effective
    }

    private fun <S : Any, H : Any> addEffectiveTargetStates(
        doc: Document<S, H>,
        targets: List<EntryTarget<S, H>>,
        effective: MutableList<S>,
    ) {
        for (target in targets) {
            val state = target.state
            if (state != null) {
                addOnce(effective, state)
                continue
            }
            // §scxml-D-getEffectiveTargetStates: a history stands for the
            // configuration it recorded, and before its parent was ever exited
            // for the targets of its default transition — which may name
            // histories themselves, hence the recursion.
            val history = target.history ?: continue
            val recorded = doc.historyValue(history)
            if (!recorded.isNullOrEmpty()) {
                for (s in recorded) addOnce(effective, s)
            } else {
                addEffectiveTargetStates(doc, doc.historyDefaultTargets(history), effective)
            }
        }
    }

    /**
     * Appendix D's getTransitionDomain, for a transition that has targets.
     *
     * [effectiveTargets] are the transition's EFFECTIVE targets: a history that
     * recorded a state deep inside its parent is a target that deep.
     *
     * @return the domain, or `null` when it is the `<scxml>` element — always so
     *   for the document's initial transition
     */
    fun <S : Any, H : Any> transitionDomain(
        doc: Document<S, H>,
        transition: EntryTransition<S, H>,
        effectiveTargets: List<S>,
    ): S? {
        val source = transition.source ?: return null

        // §scxml-D-getTransitionDomain: an internal transition whose targets all
        // lie below a compound source has the SOURCE as its domain, so the
        // source stays active and only its active descendants are exited. That
        // is not the same as exiting nothing: a transition rooted at one of
        // those descendants exits it too, and the appendix expects the two to be
        // found in conflict.
        if (transition.isInternal && doc.isCompound(source) && allDescend(doc, effectiveTargets, source)) {
            return source
        }
        return findLCCA(doc, source, effectiveTargets)
    }

    /**
     * Appendix D's findLCCA over `[head] + tail`: the first proper ancestor of
     * [head] that is a compound state — or the `<scxml>` element, reported as
     * `null` — and contains every state of [tail].
     *
     * Asked of the source and EVERY target at once: combining pairwise answers
     * can only widen the domain.
     */
    fun <S : Any, H : Any> findLCCA(doc: Document<S, H>, head: S, tail: List<S>): S? {
        var current = head
        repeat(MAX_HIERARCHY_DEPTH) {
            val ancestor = doc.parentOf(current) ?: return null
            // §scxml-D-findLCCA filters the ancestors with
            // isCompoundStateOrScxmlElement: a <parallel> is never a domain.
            if (doc.isCompound(ancestor) && allDescend(doc, tail, ancestor)) return ancestor
            current = ancestor
        }
        error("findLCCA: cyclic parent relationship detected walking from $head")
    }

    /**
     * Appendix D's computeExitSet, for one transition: the active proper
     * descendants of its domain, in the configuration's own order.
     */
    fun <S : Any, H : Any> computeExitSet(
        doc: Document<S, H>,
        transition: EnabledTransition<S, H>,
        configuration: List<S>,
    ): List<S> {
        // §scxml-D-computeExitSet: the appendix guards the whole computation
        // with `if t.target`, so a transition without one exits nothing at all
        // and can therefore never be preempted.
        if (transition.isTargetless) return emptyList()

        val targets = effectiveTargetStates(doc, transition.targets)
        val domain = transitionDomain(doc, transition.entryTransition(), targets)
        // The domain itself is not exited; everything active below it is. When
        // the domain is the <scxml> element every active state is a descendant
        // of it — the sibling regions of an enclosing <parallel> included.
        return configuration.filter { domain == null || isDescendant(doc, it, domain) }
    }

    /**
     * Appendix D's computeExitSet over the microstep's transitions, in
     * exitOrder — the states exitStates exits.
     */
    fun <S : Any, H : Any> computeStatesToExit(
        doc: Document<S, H>,
        transitions: List<EnabledTransition<S, H>>,
        configuration: List<S>,
    ): List<S> {
        val statesToExit = mutableListOf<S>()
        // §scxml-D-computeExitSet takes the microstep's whole transition list
        // and unions the exit sets; §scxml-D-exitStates exits that union in
        // exitOrder — descendants before their ancestors and reverse document
        // order among the rest, which together are exactly reverse document
        // order.
        for (transition in transitions) {
            for (state in computeExitSet(doc, transition, configuration)) addOnce(statesToExit, state)
        }
        return statesToExit.sortedByDescending { doc.documentOrder(it) }
    }

    // ════════════════════════════════════════════════════════════════════════
    // The microstep
    // ════════════════════════════════════════════════════════════════════════

    /**
     * Appendix D's microstep: exit, run the transitions' content, enter.
     *
     * @return what was entered — the entry set, its states in entry order
     */
    fun <S : Any, H : Any, E : Any> microstep(
        run: Run<S, H, E>,
        transitions: List<EnabledTransition<S, H>>,
    ): EntrySet<S, H> {
        // §scxml-D-microstepProcedure: every exit, then every transition's
        // content, then every entry — for the whole set at once, which is what
        // lets the regions of a <parallel> each take their own transition in
        // one step.
        exitStates(run, transitions)
        executeTransitionContent(run, transitions)
        return enterStates(run, transitions.map { it.entryTransition() })
    }

    /** Appendix D's exitStates. */
    fun <S : Any, H : Any, E : Any> exitStates(run: Run<S, H, E>, transitions: List<EnabledTransition<S, H>>) {
        // §scxml-D-exitStates: the union of the transitions' exit sets, exited
        // in exitOrder. Every history is recorded from the configuration as it
        // stood BEFORE the first exit, which is the snapshot each exit is
        // handed.
        val configuration = run.configuration()
        for (state in computeStatesToExit(run, transitions, configuration)) {
            run.exitState(state, configuration)
        }
    }

    /** Appendix D's executeTransitionContent. */
    fun <S : Any, H : Any, E : Any> executeTransitionContent(
        run: Run<S, H, E>,
        transitions: List<EnabledTransition<S, H>>,
    ) {
        // §scxml-D-executeTransitionContent: in the order the transitions were
        // selected, which is not their sources' document order once an
        // ancestor's transition is reached from a later region.
        for (transition in transitions) {
            if (transition.hasActions) run.executeTransitionContent(transition)
        }
    }

    /**
     * Appendix D's enterStates.
     *
     * Also the whole of the appendix's entry into the initial configuration
     * (§scxml-D-interpret): hand it the document's initial transition, whose
     * source is the `<scxml>` element (`source == null`).
     */
    fun <S : Any, H : Any, E : Any> enterStates(
        run: Run<S, H, E>,
        transitions: List<EntryTransition<S, H>>,
    ): EntrySet<S, H> {
        val entry = computeEntrySet(run, transitions)
        for (state in entry.statesToEnter) {
            // §scxml-D-enterStates: onentry, then the initial transition's
            // content if and only if this state's initial state is being entered
            // by default, then a history's default content owed to it.
            run.enterState(state, entry.isDefaultEntry(state))
            entry.defaultHistoryContentOf(state)?.let { run.executeHistoryDefaultContent(it) }
        }
        return entry
    }

    // ════════════════════════════════════════════════════════════════════════
    // The entry set
    // ════════════════════════════════════════════════════════════════════════

    /**
     * Appendix D's computeEntrySet.
     *
     * [transitions] are the microstep's, in the order they were selected; a
     * transition without targets enters nothing.
     *
     * @return the entry set, its states sorted into entry order
     */
    fun <S : Any, H : Any> computeEntrySet(
        doc: Document<S, H>,
        transitions: List<EntryTransition<S, H>>,
    ): EntrySet<S, H> {
        val entry = EntrySet<S, H>()
        for (transition in transitions) {
            // §scxml-D-computeEntrySet: first every target with its default
            // descendants, then the ancestors that are entered inside the domain
            // — ancestors outside it were never exited. The order matters for a
            // target SET: the regions of a <parallel> that another target
            // already descends into must be seen as taken before the ancestor
            // walk fills the rest with defaults.
            for (target in transition.targets) addDescendantStatesToEnter(doc, target, entry)
            val effective = effectiveTargetStates(doc, transition.targets)
            if (effective.isEmpty()) continue
            val domain = transitionDomain(doc, transition, effective)
            for (state in effective) addAncestorStatesToEnter(doc, StateTarget(state), domain, entry)
        }

        // §scxml-D-enterStates: states are entered in entryOrder — ancestors
        // before descendants, document order between the rest, which is exactly
        // document order, because that is a pre-order walk.
        entry.sortIntoEntryOrder(doc::documentOrder)
        return entry
    }

    /**
     * Appendix D's addDescendantStatesToEnter.
     *
     * Adds [target] and every descendant entering it enters: a history's
     * recorded or default configuration, a compound state's initial state(s),
     * every region of a `<parallel>` not already on the set.
     */
    fun <S : Any, H : Any> addDescendantStatesToEnter(
        doc: Document<S, H>,
        target: EntryTarget<S, H>,
        entry: EntrySet<S, H>,
    ) {
        val history = target.history
        if (history != null) {
            val parent = doc.historyParent(history)
            // §scxml-3.10: a transition to a history behaves as a transition to
            // the configuration it stored, or — before its parent was ever
            // visited — to its default stored configuration.
            val recorded = doc.historyValue(history)
            if (!recorded.isNullOrEmpty()) {
                // §scxml-D-addDescendantStatesToEnter: a history that has
                // recorded a configuration enters it, and the ancestors between
                // it and the history's parent — a deep history records atomic
                // states, which need not be children.
                for (state in recorded) addDescendantStatesToEnter(doc, StateTarget(state), entry)
                for (state in recorded) addAncestorStatesToEnter(doc, StateTarget(state), parent, entry)
                return
            }
            // §scxml-D-addDescendantStatesToEnter: nothing recorded yet, so the
            // default transition is taken, and its content is owed once the
            // parent has been entered — keyed by the parent, a later history of
            // the same parent replacing it.
            entry.oweHistoryContent(parent, history)
            val defaults = doc.historyDefaultTargets(history)
            for (default in defaults) addDescendantStatesToEnter(doc, default, entry)
            for (default in defaults) addAncestorStatesToEnter(doc, default, parent, entry)
            return
        }

        val state = target.state ?: return
        entry.enter(state)
        if (doc.isCompound(state)) {
            // §scxml-D-addDescendantStatesToEnter: a compound state entered as a
            // target is entered by DEFAULT — the one condition under which its
            // initial transition's content runs. Its initial transition names
            // the child or children it enters (§scxml-3.3), possibly several and
            // possibly deep (§scxml-3.6).
            entry.enterByDefault(state)
            val initial = doc.initialTargets(state)
            for (child in initial) addDescendantStatesToEnter(doc, child, entry)
            for (child in initial) addAncestorStatesToEnter(doc, child, state, entry)
        } else if (doc.isParallel(state)) {
            addRegionDefaults(doc, state, entry)
        }
    }

    /**
     * Appendix D's addAncestorStatesToEnter.
     *
     * Adds the proper ancestors of [target] up to, not including, [ancestor]
     * (`null`: up to the `<scxml>` element), filling in the regions of every
     * `<parallel>` among them.
     */
    fun <S : Any, H : Any> addAncestorStatesToEnter(
        doc: Document<S, H>,
        target: EntryTarget<S, H>,
        ancestor: S?,
        entry: EntrySet<S, H>,
    ) {
        for (anc in properAncestors(doc, target, ancestor)) {
            // §scxml-D-addAncestorStatesToEnter: an ancestor is entered WITHOUT
            // its default initial state — the set already holds the descendant
            // it leads to. A <parallel> still gives its other regions their
            // defaults, because all of them are entered.
            entry.enter(anc)
            if (doc.isParallel(anc)) addRegionDefaults(doc, anc, entry)
        }
    }

    /**
     * Appendix D's getProperAncestors: the ancestors of [target] in ancestry
     * order up to, not including, [ancestor] — and nothing at all when
     * [ancestor] is not above it.
     *
     * A `<history>` sits in its parent, so the parent is its first proper
     * ancestor. A `null` [ancestor] is the `<scxml>` element, which is above
     * every state and is never entered, so it is not in the list.
     */
    fun <S : Any, H : Any> properAncestors(doc: Document<S, H>, target: EntryTarget<S, H>, ancestor: S?): List<S> {
        val chain = mutableListOf<S>()
        val history = target.history
        var current: S? = if (history != null) doc.historyParent(history) else target.state?.let(doc::parentOf)
        repeat(MAX_HIERARCHY_DEPTH) {
            // §scxml-D-getProperAncestors: when ancestor is the state's parent,
            // the state itself, or one of its descendants — none of which the
            // walk can meet — the answer is the empty set.
            val reached = current ?: return if (ancestor != null) emptyList() else chain
            if (ancestor != null && reached == ancestor) return chain
            chain.add(reached)
            current = doc.parentOf(reached)
        }
        error("properAncestors: cyclic parent relationship detected walking from $target")
    }

    private fun <S : Any, H : Any> addRegionDefaults(doc: Document<S, H>, parallel: S, entry: EntrySet<S, H>) {
        // §scxml-3.4: every child of an active <parallel> is active, so a region
        // nothing on the set descends into is entered by default.
        for (child in doc.childStates(parallel)) {
            val taken = entry.statesToEnter.any { isDescendant(doc, it, child) }
            if (!taken) addDescendantStatesToEnter(doc, StateTarget(child), entry)
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // History
    // ════════════════════════════════════════════════════════════════════════

    /**
     * Appendix D's exitStates, its history half: what a `<history>` of [parent]
     * records as [parent] is exited — for a deep history the active atomic
     * states below [parent], for a shallow one its active children.
     *
     * Read off [configurationBeforeExit], the configuration as it stood before
     * the microstep's first exit: the appendix records every history before it
     * exits any state, so every history of one microstep reads the same
     * configuration. Returned in that configuration's order.
     */
    fun <S : Any, H : Any> recordedHistory(
        doc: Document<S, H>,
        parent: S,
        deep: Boolean,
        configurationBeforeExit: List<S>,
    ): List<S> = configurationBeforeExit.filter { state ->
        if (deep) {
            // §scxml-D-exitStates: `isAtomicState(s0) and isDescendant(s0, s)`.
            !doc.isCompound(state) && !doc.isParallel(state) && isDescendant(doc, state, parent)
        } else {
            // §scxml-D-exitStates: `s0.parent == s`.
            doc.parentOf(state) == parent
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // Predicates
    // ════════════════════════════════════════════════════════════════════════

    /**
     * Appendix D's isInFinalState: a compound state is in a final state when
     * one of its `<final>` children is active; a `<parallel>` when EVERY one of
     * its child states is — asked recursively, so a region that is itself a
     * `<parallel>` counts only once all of ITS regions do. Nothing else ever
     * is.
     */
    fun <S : Any, H : Any> isInFinalState(doc: Document<S, H>, state: S, configuration: Collection<S>): Boolean =
        when {
            doc.isCompound(state) -> doc.childStates(state).any { doc.isFinal(it) && it in configuration }
            doc.isParallel(state) -> doc.childStates(state).all { isInFinalState(doc, it, configuration) }
            else -> false
        }

    /** Appendix D's isDescendant: whether [state] lies strictly below [ancestor]. */
    fun <S : Any, H : Any> isDescendant(doc: Document<S, H>, state: S, ancestor: S): Boolean {
        var current = state
        repeat(MAX_HIERARCHY_DEPTH) {
            val parent = doc.parentOf(current) ?: return false
            if (parent == ancestor) return true
            current = parent
        }
        error("isDescendant: cyclic parent relationship detected walking from $state")
    }

    private fun <S : Any, H : Any> allDescend(doc: Document<S, H>, states: List<S>, ancestor: S): Boolean =
        states.all { isDescendant(doc, it, ancestor) }

    private fun <S : Any> intersects(a: List<S>, b: List<S>): Boolean = a.any { it in b }
}

private fun <S> addOnce(set: MutableList<S>, state: S) {
    if (state !in set) set.add(state)
}
