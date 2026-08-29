// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <cstddef>

/**
 * @file SceLowering.h
 * @brief The C surface over `sce-build`'s ECMAScript frontend.
 *
 * The other half of this contract is `sce-build/src/ffi.rs`. The two are
 * hand-written and therefore able to drift, which is why
 * `ffi_header_matches_surface` compares them: a header that promises an
 * entry point the library no longer exports is a clean compile and a
 * link error, and one whose signature has silently changed is worse than
 * that.
 *
 * ## Why the engine calls this at all
 *
 * `EcmaScriptToLuaTransformer` rewrites the author's ECMAScript into Lua
 * as TEXT, without parsing it, so it cannot say where an operand ends.
 * `docs/SCE_LUA_TRANSLATION_SEAM.md` and
 * `tests/ecmascript/lua_engine_divergences.json` list what that costs —
 * 23 entries, every one of them on the `runtime-rewriter` path. The
 * frontend behind this header parses, and it answers them.
 *
 * The owner's decision on 2026-08-29 was to link it and retire the
 * rewriter. This header is the first half of that: the rewriter is still
 * here and still the fallback.
 *
 * ## Refusal is an answer
 *
 * Every `sce_lower_*` returns NULL when the frontend will not lower the
 * text — it did not parse, or it names something the scope does not
 * declare. A caller is expected to have a path for that; `LuaEngine`
 * falls back to the rewriter, which is what lets this be adopted one
 * class of expression at a time instead of all at once.
 *
 * ## Ownership
 *
 * A non-NULL result is the caller's, and is released exactly once with
 * `sce_lower_free`. A scope handle is the caller's from `sce_scope_new`
 * to `sce_scope_free`.
 */

extern "C" {

/// An opaque handle to the set of names the frontend may resolve.
///
/// An EMPTY scope is a meaningful question, not a degenerate one: it
/// asks whether the expression can be answered without the caller
/// naming anything. 11 of the 23 tracked divergences are closed
/// expressions and are answered that way.
struct SceLoweringScope;

/// Open a scope with nothing declared.
SceLoweringScope *sce_scope_new(void);

/// Declare one name, as a `<data id>` does.
void sce_scope_declare(SceLoweringScope *scope, const char *name);

/// Declare whatever a chunk's top level introduces, as a document-level
/// `<script>` does at load time (W3C SCXML, §scxml-5.8).
void sce_scope_declare_chunk(SceLoweringScope *scope, const char *source);

/// Release a scope. NULL is accepted.
void sce_scope_free(SceLoweringScope *scope);

/// Lower a value expression. NULL if the frontend refuses.
char *sce_lower_value(const char *source, const SceLoweringScope *scope);

/// Lower a condition — the result is a Lua boolean.
char *sce_lower_condition(const char *source, const SceLoweringScope *scope);

/// Lower a statement sequence.
char *sce_lower_script(const char *source, const SceLoweringScope *scope);

/// Lower an assignment target. No scope: a location names what it writes.
char *sce_lower_location(const char *source);

/// Release what an `sce_lower_*` returned. NULL is accepted.
void sce_lower_free(char *text);

}  // extern "C"
