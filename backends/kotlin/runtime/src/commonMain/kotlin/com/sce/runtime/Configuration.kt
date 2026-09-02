// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
            continue
        }

        // §scxml-3.11: exactly one. Compound is spelled as "not atomic and not
        // parallel" because that is the pair the generator emits:
        // `isAtomicState` answers false for both shapes, and the parallel arm
        // above has already taken its own.
        if (!isAtomic(state)) {
            if (children != 1) {
                return ConfigurationRejection.COMPOUND_CHILD_COUNT
            }
        } else if (children != 0) {
            return ConfigurationRejection.ATOMIC_HAS_CHILDREN
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
