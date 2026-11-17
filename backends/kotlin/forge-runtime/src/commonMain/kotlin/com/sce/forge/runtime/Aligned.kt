// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * A list a codec writes the count of: how many entries it holds.
 *
 * The class a printed codec holds a list in, within its bound (docs/adr/0065
 * §6 R2, docs/adr/0081 §2), is one, so two lists of one count can be held
 * together as one [Aligned].
 */
interface Counted {
    /** How many entries the list holds. */
    val size: Int
}

/**
 * Two lists of one count (docs/adr/0065 §6 R5): a frame writes one count, and
 * both lists hold that many entries.
 *
 * The constructor is private and [of] makes a value only of two lists that
 * hold as many entries as each other, so the encode that writes the one count
 * of both refuses nothing (docs/adr/0081 §2): the check runs once, where the
 * value is made. The lists cannot change after, as the classes of their
 * bounds copy what they are made of.
 */
class Aligned<A : Counted, B : Counted> private constructor(
    /** The list the count is written for and every arm holds. */
    val first: A,
    /** The list that shares the first's count. */
    val second: B,
) {
    override fun equals(other: Any?): Boolean =
        other is Aligned<*, *> && other.first == first && other.second == second

    override fun hashCode(): Int = 31 * first.hashCode() + second.hashCode()

    override fun toString(): String = "Aligned(first=$first, second=$second)"

    companion object {
        /**
         * [first] and [second] as one value of one count, or `null` where they
         * hold different counts, which no frame carries.
         */
        fun <A : Counted, B : Counted> of(first: A, second: B): Aligned<A, B>? =
            if (first.size == second.size) Aligned(first, second) else null
    }
}
