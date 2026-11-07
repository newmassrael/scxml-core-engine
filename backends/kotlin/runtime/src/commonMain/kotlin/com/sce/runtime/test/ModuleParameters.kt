// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5, §11.8;
// ETSI ES 201 873-1 §8.2): the value a suite gives each, which every component
// that declares a module parameter of that id reads into its variable before
// its machine starts.
//
// A value is held as the one JSON value model a saved state is made of
// (`com.sce.runtime.Json`: a map, a list, a text, a truth value, a
// `Json.Number` or null). The generated suite writes each default through
// `SavedValues.of` of its declared type, and the generated component reads it
// back through the `SavedValues` reader of the type it declares, which are the
// reads a restored variable is held by: a value of another type, or past its
// bound, is refused by name rather than converted. The runtime knows the type
// of no module parameter, so it is written once for every suite.
//
// Cross-language siblings: `sce_rust_runtime::test::ModuleParameters` (Rust),
// `SCE::Test::ModuleParameters` (C++).

package com.sce.runtime.test

import com.sce.runtime.StateRefusal

/** A module parameter's value: the saved form of a value of its declared type. */
typealias ModuleParameterValue = Any?

/**
 * The module parameters a run hands its components, by id.
 *
 * Generated from a `test-suite` with each parameter at its default, and given
 * other values by the caller of a run where it overrides one (-1 §8.2.1).
 */
class ModuleParameters {
    private val byId = mutableMapOf<String, ModuleParameterValue>()

    /** Gives the module parameter [id] the value [value], in place of any it had. */
    fun set(id: String, value: ModuleParameterValue) {
        byId[id] = value
    }

    /** Whether the module parameter [id] has a value; null is a value. */
    fun has(id: String): Boolean = byId.containsKey(id)

    /**
     * The value of the module parameter [id].
     *
     * @throws StateRefusal naming it, when it has none, since no value is made
     *   up in its place.
     */
    fun of(id: String): ModuleParameterValue {
        if (!byId.containsKey(id)) {
            throw StateRefusal("no value is given for the module parameter '$id'")
        }
        return byId[id]
    }

    /** Every module parameter that has a value, by id, in ascending order of id. */
    val values: Map<String, ModuleParameterValue>
        get() = byId.keys.sorted().associateWith { byId[it] }

    /** Another set of module parameters with the same values, which a change to either leaves apart. */
    fun copy(): ModuleParameters = ModuleParameters().also { it.byId.putAll(byId) }

    override fun equals(other: Any?): Boolean = other is ModuleParameters && other.byId == byId

    override fun hashCode(): Int = byId.hashCode()

    override fun toString(): String = "ModuleParameters($values)"
}
