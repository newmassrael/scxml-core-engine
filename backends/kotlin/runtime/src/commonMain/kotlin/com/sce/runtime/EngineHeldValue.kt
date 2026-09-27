// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * A value a script engine hands back without giving it a Kotlin shape — it
 * stays in the engine, and what Kotlin holds is a handle to it.
 *
 * An engine does this for values that would not survive being copied out: a
 * function, a DOM node, an object that is later assigned back and must still
 * be the same object. That is right while the value stays inside the data
 * model, and wrong the moment it leaves: §scxml-B-2-9 says data leaving the
 * ECMAScript data model is serialised to JSON, and only the engine holding the
 * value can say what that JSON is. Asked of the handle rather than of its
 * type, so the writer of `_event.data` needs no knowledge of any engine.
 */
interface EngineHeldValue {
    /**
     * The value as the engine serialises it to JSON (`JSON.stringify`). A
     * value JSON cannot express (a function, `undefined`) is the text `null`,
     * as the C++ writer spells an undefined value.
     */
    fun toJson(): String
}
