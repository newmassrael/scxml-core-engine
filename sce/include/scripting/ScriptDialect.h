// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "scripting/ScriptSource.h"
#include <cstdint>
#include <string>

namespace SCE::ScriptDialect {

/**
 * @brief How a COMPOSED question is spelled in each language the seam admits.
 *
 * Some helpers do not only forward the author's expression — having got a
 * value back, they ask the engine a follow-up ABOUT it: is this an array, how
 * long is it, what is element `i`, what type is it, render it as JSON. Those
 * follow-ups are written in the engine's own language, and until the seam
 * existed there was only one language to write them in, so they were spelled
 * inline as ECMAScript.
 *
 * That is the trap this file exists to close. Under build-time lowering the
 * expression handed in is already Lua, and a wrapper composed as ECMAScript
 * around it — `x instanceof Array` — is neither language. It would not be a
 * translation bug the rewriter could catch either, because on a pre-lowered
 * path there is no rewriter left to catch it.
 *
 * Measured 2026-08-28 against the runtime this backend actually loads:
 *
 *   stringify   `JSON.stringify(x)`      the SAME text — `JSON` is a real
 *                                        Lua table (`json_builtins.lua:14`)
 *   isArray     `(x) instanceof Array`   `_isArray(x)`  (ecma_semantics:102)
 *   typeOf      `typeof (x)`             `_typeof(x)`   (ecma_semantics:78)
 *   lengthOf    `(x).length`             `#(x)` — an Array is stored here as
 *                                        a 1-based Lua sequence
 *   elementAt   `(x)[i]`                 `_scxml_index(x, i)` (…:248), which
 *                                        takes the 0-based i and shifts it
 *   temp bind   `var n = (x)`            `n = (x)` — Lua has no `var`, a
 *                                        bare assignment makes the global
 *
 * Only the first is the same in both, and it is the only one the GENERATED
 * C++ path reaches today (`resultToString`). The other five are on
 * `resultToStringArray` / `extractScriptValueArray`, which no template calls —
 * so they are spelled here not because they are urgent but because leaving one
 * ECMAScript-only composition behind is how the next reader concludes the
 * whole file is language-agnostic.
 *
 * ⚠ `elementAt` takes the index the CALLER counts in, which is 0-based in both
 * spellings: `_scxml_index` does the shift to Lua's 1-based storage itself. A
 * caller must not pre-shift, or the element moves twice — the same off-by-one
 * the seam exists to prevent, arriving by a different road.
 */

/// `JSON.stringify(x)` — identical in both languages.
inline ScriptSource stringify(const ScriptSource &value) {
    return ScriptSourceBuilder(value.language()).add("JSON.stringify(").add(value).add(")").build();
}

/// ECMA-262 13.10.2 `x instanceof Array`.
inline ScriptSource isArray(const ScriptSource &value) {
    ScriptSourceBuilder builder(value.language());
    if (value.language() == ScriptLanguage::Lua) {
        return builder.add("_isArray(").add(value).add(")").build();
    }
    return builder.add("(").add(value).add(") instanceof Array").build();
}

/// ECMA-262 13.5.3 `typeof x`.
inline ScriptSource typeOf(const ScriptSource &value) {
    ScriptSourceBuilder builder(value.language());
    if (value.language() == ScriptLanguage::Lua) {
        return builder.add("_typeof(").add(value).add(")").build();
    }
    return builder.add("typeof (").add(value).add(")").build();
}

/// The array's length. `#` is Lua's, and an ECMAScript Array lives here as a
/// 1-based Lua sequence, so the two agree on the answer.
inline ScriptSource lengthOf(const ScriptSource &value) {
    ScriptSourceBuilder builder(value.language());
    if (value.language() == ScriptLanguage::Lua) {
        return builder.add("#(").add(value).add(")").build();
    }
    return builder.add("(").add(value).add(").length").build();
}

/// Element `index`, counted from 0 in BOTH spellings — see the warning above.
inline ScriptSource elementAt(const ScriptSource &value, int64_t index) {
    const std::string subscript = std::to_string(index);
    ScriptSourceBuilder builder(value.language());
    if (value.language() == ScriptLanguage::Lua) {
        return builder.add("_scxml_index(").add(value).add(", ").add(subscript.c_str()).add(")").build();
    }
    return builder.add("(").add(value).add(")[").add(subscript.c_str()).add("]").build();
}

/// Bind `name` to `value` for later statements in the same session. Lua has no
/// `var`; a bare assignment is how a global is made.
inline ScriptSource bindTemporary(const char *name, const ScriptSource &value) {
    ScriptSourceBuilder builder(value.language());
    if (value.language() != ScriptLanguage::Lua) {
        builder.add("var ");
    }
    return builder.add(name).add(" = (").add(value).add(")").build();
}

/// A name previously bound by [bindTemporary], as an expression in `language`.
inline ScriptSource temporary(const char *name, ScriptLanguage language) {
    return ScriptSourceBuilder(language).add(name).build();
}

}  // namespace SCE::ScriptDialect
