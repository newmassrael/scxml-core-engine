// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * A value that may be absent, `optional<T>` (docs/adr/0055), as generated Kotlin
 * reads it. The value is a `T?`: absent is `null`, a present value is the `T`
 * itself, `x ?? d` is `x ?: d` and equality is `==`. What this object adds is
 * the one read that is not an operator.
 */
public object SceOptional {
    /**
     * [value], which the build has proven present: a `.value` is generated only
     * where a test (`x !== null`) proves it (docs/adr/0055, section 3), so no
     * run reaches the absent arm, and it is no failure a statement receives.
     *
     * A function rather than `!!` at the read: a test of a stable value, such as
     * a field of a loop's item, makes Kotlin read it as `T` already, and `!!` on
     * a `T` is a warning, which the generated code is compiled to refuse. Handed
     * a `T` or a `T?`, this reads the same.
     */
    public fun <T : Any> proven(value: T?): T =
        checkNotNull(value) { "an optional value read where the build proved it present was absent" }
}
