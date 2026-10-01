// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * §scxml-3.11: is a set of states a CONFIGURATION of
 * the document, and is `current` its current state?
 *
 * [StateMachineEngine.activeConfiguration] publishes the configuration a
 * machine is in and [StateMachineEngine.enterAt] is the door that takes one
 * back. This file owns the question between them, and it is the only place that
 * asks it.
 *
 * Why this rejects instead of throwing: every other input to this engine is
 * authored — the generator wrote the hierarchy, and one that does not walk is a
 * generator defect. A restored configuration is the single exception. It
 * arrives from OUTSIDE the process, read back by a host from wherever it
 * persisted it, recorded against a document revision that may since have moved.
 * A host holding a stale record has to be able to handle that answer, so this
 * returns a reason and the engine hands it on.
 *
 * The Kotlin twin of the Rust runtime's `helpers::configuration::validate`, the
 * C++ `SCE::Core::validateConfiguration`, the Go `sce.ValidateConfiguration`
 * and the Python `validate_configuration`, asking the same questions of the
 * same static hierarchy — so a configuration one engine accepts is one the
 * others accept.
 */
enum class ConfigurationRejection(val reason: String) {
    /** The accepting answer. */
    NONE("accepted"),

    /** No states at all. A machine is never in nothing. */
    EMPTY("the configuration is empty; a machine is never in nothing"),

    /**
     * A state appears twice. Checked before every arity count below, which
     * would otherwise read a duplicate as a second child and blame the wrong
     * rule.
     */
    DUPLICATE("a state appears twice"),

    /** A state is present whose parent is not — the set is not ancestor-closed. */
    ANCESTOR_MISSING("a state is present whose parent is not, so the set is not ancestor-closed"),

    /** §scxml-3.11: a configuration closes on exactly one root. */
    ROOT_COUNT("a configuration closes on exactly one root (W3C SCXML 3.11)"),

    /** §scxml-3.11: a compound state holds exactly one active child. */
    COMPOUND_CHILD_COUNT("a compound state holds exactly one active child (W3C SCXML 3.11)"),

    /** §scxml-3.11: a `<parallel>` holds EVERY region, and one is missing. */
    PARALLEL_REGION_MISSING("a <parallel> holds every region and one is missing (W3C SCXML 3.11)"),

    /** §scxml-3.11: a `<parallel>` holds every region and nothing else. */
    PARALLEL_CHILD_COUNT("a <parallel> holds every region and nothing else (W3C SCXML 3.11)"),

    /** An atomic state has a child in the set, so it is not atomic here. */
    ATOMIC_HAS_CHILDREN("an atomic state has a child in the set"),

    /** The current state is not in the configuration it is supposed to be in. */
    CURRENT_NOT_ACTIVE("the current state is not in the configuration"),

    /**
     * The current state is compound or parallel. §scxml-3.11 makes the current
     * state the atomic one the engine descended to.
     */
    CURRENT_NOT_ATOMIC("the current state must be the atomic state the engine descended to"),
    ;

    override fun toString(): String = reason
}

/**
 * Whether [configuration] is a configuration of the document whose hierarchy
 * the four accessors describe, with [current] as its current state.
 *
 * Pure: reads the accessors and nothing else. They are passed in rather than
 * read off an engine because the engine's own hierarchy hooks are `protected` —
 * generated code overrides them — and the rules belong somewhere a test can
 * reach without a machine.
 *
 * Cost is quadratic in the set length, which is a handful of states, and this
 * runs once per restore: the shape is chosen for being obviously right rather
 * than for being fast, exactly as its four twins are.
 *
 * What cannot be wrong, and why it is not checked: a member is a value of the
 * generated sealed state interface, one object per state of this document, so
 * "no such state" needs no rejection variant.
 */
fun <S> validateConfiguration(
    configuration: List<S>,
    current: S,
    parentOf: (S) -> S?,
    isAtomic: (S) -> Boolean,
    isParallel: (S) -> Boolean,
    regionsOf: (S) -> List<S>,
): ConfigurationRejection {
    if (configuration.isEmpty()) {
        return ConfigurationRejection.EMPTY
    }

    for (index in configuration.indices) {
        if (configuration.subList(0, index).contains(configuration[index])) {
            return ConfigurationRejection.DUPLICATE
        }
    }

    var roots = 0
    for (state in configuration) {
        val parent = parentOf(state)
        if (parent == null) {
            roots++
        } else if (!configuration.contains(parent)) {
            return ConfigurationRejection.ANCESTOR_MISSING
        }
    }
    if (roots != 1) {
        return ConfigurationRejection.ROOT_COUNT
    }

    for (state in configuration) {
        val arity = childArity(configuration, state, parentOf, isAtomic, isParallel, regionsOf)
        if (arity != ConfigurationRejection.NONE) {
            return arity
        }
    }

    if (!configuration.contains(current)) {
        return ConfigurationRejection.CURRENT_NOT_ACTIVE
    }
    if (!isAtomic(current) || isParallel(current)) {
        return ConfigurationRejection.CURRENT_NOT_ATOMIC
    }

    return ConfigurationRejection.NONE
}

/**
 * The child arity of one member of [configuration] (§scxml-3.4, §scxml-3.11): a
 * `<parallel>` holds every region, a compound state exactly one child, an
 * atomic state none. The one rule both a whole configuration and the part of one
 * a `<history>` records are held to.
 */
private fun <S> childArity(
    configuration: List<S>,
    state: S,
    parentOf: (S) -> S?,
    isAtomic: (S) -> Boolean,
    isParallel: (S) -> Boolean,
    regionsOf: (S) -> List<S>,
): ConfigurationRejection {
    val children = configuration.count { parentOf(it) == state }

    if (isParallel(state)) {
        val regions = regionsOf(state)
        // §scxml-3.4: every region, simultaneously.
        for (region in regions) {
            if (!configuration.contains(region)) {
                return ConfigurationRejection.PARALLEL_REGION_MISSING
            }
        }
        if (children != regions.size) {
            return ConfigurationRejection.PARALLEL_CHILD_COUNT
        }
        return ConfigurationRejection.NONE
    }

    // §scxml-3.11: exactly one. Compound is spelled as "not atomic and not
    // parallel" because that is the pair the generator emits: `isAtomicState`
    // answers false for both shapes, and the parallel arm above has already
    // taken its own.
    if (!isAtomic(state)) {
        if (children != 1) {
            return ConfigurationRejection.COMPOUND_CHILD_COUNT
        }
    } else if (children != 0) {
        return ConfigurationRejection.ATOMIC_HAS_CHILDREN
    }
    return ConfigurationRejection.NONE
}

/**
 * Why a recorded `<history>` value was refused (§scxml-3.10).
 *
 * What a history recorded is part of a configuration: the active children of
 * its parent for a shallow one, the active atomic states below it for a deep
 * one. A value read back from a saved state is held to that, so the restored
 * machine can only be taken, through the history, to a configuration its
 * document could have been in.
 */
enum class HistoryRejection(val reason: String) {
    /** The accepting answer. */
    NONE("accepted"),

    /** The value holds no state. A history records at least one. */
    EMPTY("a history records at least one state"),

    /** A state appears twice. */
    DUPLICATE("a state appears twice"),

    /** A state is not below the history's parent. */
    NOT_BELOW("a state is not below the history's parent"),

    /** A shallow history records the children of its parent, and a state is deeper. */
    NOT_A_CHILD("a shallow history records the children of its parent, and a state is deeper"),

    /** A deep history records atomic states, and one has children. */
    NOT_ATOMIC("a deep history records atomic states, and one has children"),

    /**
     * What the value holds is part of no configuration of the document — a
     * compound state with two active children, a region missing (§scxml-3.11).
     */
    NOT_A_CONFIGURATION("the states are part of no configuration of the document (W3C SCXML 3.11)"),
    ;

    override fun toString(): String = reason
}

/**
 * Whether [recorded] is a value the `<history>` of [parent] could have
 * recorded: its active children for a shallow one, the active atomic states
 * below it for a deep one ([deep]).
 *
 * Read as a configuration of the subtree under [parent]: the recorded states and
 * every ancestor between them and [parent] must satisfy the same child arity a
 * whole configuration does ([validateConfiguration]). A shallow history records
 * only the children, so the arity is judged on [parent] alone. The Kotlin twin
 * of the Rust runtime's `helpers::configuration::validate_history`.
 */
fun <S> validateHistory(
    parent: S,
    deep: Boolean,
    recorded: List<S>,
    parentOf: (S) -> S?,
    isAtomic: (S) -> Boolean,
    isParallel: (S) -> Boolean,
    regionsOf: (S) -> List<S>,
): HistoryRejection {
    if (recorded.isEmpty()) {
        return HistoryRejection.EMPTY
    }
    for (index in recorded.indices) {
        if (recorded.subList(0, index).contains(recorded[index])) {
            return HistoryRejection.DUPLICATE
        }
    }

    // The recorded states, and every state between each of them and the parent.
    val subtree = mutableListOf(parent)
    for (state in recorded) {
        val chain = mutableListOf<S>()
        var at: S? = state
        while (at != parent) {
            if (at == null) {
                return HistoryRejection.NOT_BELOW
            }
            chain.add(at)
            at = parentOf(at)
        }
        // A state that is the parent is below nothing.
        if (chain.isEmpty()) {
            return HistoryRejection.NOT_BELOW
        }
        for (member in chain) {
            if (!subtree.contains(member)) {
                subtree.add(member)
            }
        }
    }

    if (deep) {
        if (recorded.any { !isAtomic(it) || isParallel(it) }) {
            return HistoryRejection.NOT_ATOMIC
        }
        for (state in subtree) {
            if (childArity(subtree, state, parentOf, isAtomic, isParallel, regionsOf) != ConfigurationRejection.NONE) {
                return HistoryRejection.NOT_A_CONFIGURATION
            }
        }
    } else {
        if (recorded.any { parentOf(it) != parent }) {
            return HistoryRejection.NOT_A_CHILD
        }
        if (childArity(subtree, parent, parentOf, isAtomic, isParallel, regionsOf) != ConfigurationRejection.NONE) {
            return HistoryRejection.NOT_A_CONFIGURATION
        }
    }
    return HistoryRejection.NONE
}
