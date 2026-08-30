// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.scripting.lua

/**
 * The names a session holds, as `sce-build`'s ECMAScript frontend sees them.
 *
 * The JVM half of `sce/include/scripting/LoweringScope.h`, deliberately the
 * same class one language over: the two engines reach one frontend through one
 * C surface, so a difference between these two files would be a difference in
 * what `datamodel="ecmascript"` means on two backends.
 *
 * ## What a scope decides
 *
 * The frontend REFUSES any expression naming something its scope does not
 * declare, and that refusal is the whole selector. A scope with nothing in it
 * answers only CLOSED expressions — `1 == '1'`, `-7 % 3`, `Math.round(2.5)` —
 * because those name nothing a session could own. `a && b` names two things,
 * and until the caller says what `a` and `b` are, no parse can be trusted with
 * them.
 *
 * So the scope is not configuration; it is the question. How much a caller
 * declares is exactly how much it gets answered.
 *
 * ## Why a session owns one
 *
 * A run-time scope is the set of names the SESSION holds, which is what
 * `<data id>` and a `<script>` chunk's top level introduce (W3C SCXML
 * §scxml-5.3, §scxml-5.8). It is per-session state with a session's lifetime,
 * not a process-wide constant: two sessions of the same document may
 * legitimately differ, and one session's names must never answer for another's.
 *
 * ## No generation counter here, unlike the C++ scope
 *
 * `SCE::LoweringScope` carries one because `LuaEngine` caches a lowered chunk
 * per session and an answer that depended on the scope goes stale when the
 * scope grows. This engine caches nothing between the lowering and the load —
 * `evaluateLuaBoolean` and `doEvaluateExpr` hand the text to Lua on every call
 * — so there is nothing here for a generation to invalidate. Adding one
 * anyway would be surface no reader could check against a use, which is how a
 * counter comes to be trusted while nothing moves it.
 *
 * ## Refusal is an answer
 *
 * [lowerValue] and its neighbours answer `null` when the frontend will not
 * lower the text, and on this backend that is now FINAL: `LuaScriptEngine`
 * reports the refusal (W3C SCXML §scxml-5.9.1) instead of handing the text to
 * a second translator. It was not final while the text rewriter stood behind
 * it — that is the state the C++ engine passed through too, before its own
 * `retire-rewriter` row closed.
 */
internal class LoweringScope : AutoCloseable {

    private var handle: Long = SceLoweringNative.newScope()

    /** Record one name, as a `<data id>` does. */
    fun declare(name: String) {
        if (handle == 0L || name.isEmpty()) {
            return
        }
        SceLoweringNative.declare(handle, name)
    }

    /**
     * Record what a chunk's top level introduces, as a `<script>` does.
     *
     * Only the top level, because only the top level reaches the datamodel. A
     * chunk the frontend's parser refuses declares nothing — this is a name
     * collector, not a second validator, and the expressions that would have
     * named those variables simply keep the answer they had.
     */
    fun declareChunk(source: String) {
        if (handle == 0L || source.isEmpty()) {
            return
        }
        SceLoweringNative.declareChunk(handle, source)
    }

    /** The frontend's Lua for a value expression, or null if it refuses. */
    fun lowerValue(source: String): String? =
        if (handle == 0L) null else SceLoweringNative.lowerValue(handle, source)

    /**
     * The frontend's Lua for a condition, or null if it refuses.
     *
     * Wrapped in the shared truthiness helper by the frontend itself, because
     * §scxml-5.9's answer for `0`, `''` and `NaN` is false and Lua's is true.
     */
    fun lowerCondition(source: String): String? =
        if (handle == 0L) null else SceLoweringNative.lowerCondition(handle, source)

    /**
     * The frontend's Lua for a statement sequence, or null if it refuses.
     *
     * A chunk brings its own names with it — `var` bindings are hoisted into
     * the chunk's frame before anything resolves — so this asks LESS of the
     * scope than [lowerValue] does. What it still asks for is the names the
     * chunk only READS: a `<data id>` the document declared, or a variable an
     * earlier `<script>` introduced.
     */
    fun lowerScript(source: String): String? =
        if (handle == 0L) null else SceLoweringNative.lowerScript(handle, source)

    /** The frontend's Lua for an assignment target, or null if it refuses. */
    fun lowerLocation(source: String): String? =
        if (handle == 0L) null else SceLoweringNative.lowerLocation(source)

    /**
     * Release the scope. Idempotent, because a session teardown that runs twice
     * is a double free rather than a second cleanup.
     */
    override fun close() {
        if (handle != 0L) {
            SceLoweringNative.freeScope(handle)
            handle = 0L
        }
    }
}
