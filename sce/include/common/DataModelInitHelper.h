// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "scripting/IScriptEngine.h"
#include <functional>
#include <optional>
#include <string>

/**
 * @brief Helper for initializing datamodel variables with XML DOM support
 *
 * §scxml-B-2: ECMAScript datamodel must convert XML content to DOM structures
 * ARCHITECTURE.MD: Zero Duplication - Shared by Interpreter and AOT engines
 *
 * This helper provides a unified way to initialize datamodel variables:
 * - Inline XML content → DOM object with getElementsByTagName(), getAttribute()
 * - External file (src attribute) → Load file content and convert to DOM
 * - expr attribute → Evaluate expression directly
 */

namespace SCE {

class DataModelInitHelper {
public:
    /**
     * @brief Resolve basePath relative to executable location
     *
     * AOT tests need location-independent basePath resolution.
     * Converts relative basePath to absolute based on executable location.
     *
     * @param relativePath Relative path from executable directory (e.g., "w3c_static_generated")
     * @return Absolute basePath for FileLoadingHelper
     *
     * Example:
     * - Executable: /home/user/project/build/tests/w3c_test_cli
     * - relativePath: "w3c_static_generated"
     * - Returns: "/home/user/project/build/tests/w3c_static_generated"
     *
     * Under WASM the executable is the program's path as its host declared it
     * (Emscripten's `thisProgram`, argv[0]): under Node, the script Node runs.
     *
     * ARCHITECTURE.md: Execution location independence for AOT tests
     */
    static std::string resolveExecutableBasePath(const std::string &relativePath);

    /**
     * @brief Check if expression is a JavaScript function literal
     *
     * @param expr Expression to check
     * @return true if expr is function literal (function() {...} or () => ...)
     *
     * §scxml-B-2: Function expressions must preserve function type
     * Test 453: ECMAScript function literals stored as functions, not converted
     */
    static bool isFunctionExpression(const std::string &expr);

    /**
     * @brief Initialize a datamodel variable in JSEngine
     *
     * @param jsEngine JSEngine instance for variable storage and expression evaluation
     * @param sessionId Session ID for JSEngine context
     * @param varId Variable identifier (e.g., "var1")
     * @param content Inline content, as a `ScriptSource` — see below
     * @param errorCallback Function to call on error (receives error message)
     * @return true if initialization succeeded, false otherwise
     *
     * §scxml-5.3.2: content, src, and expr are mutually exclusive
     * - If content is non-empty and starts with '<', create DOM object
     * - Otherwise, evaluate content as an expression
     *
     * §scxml-5.3: Raises error.execution if initialization fails
     *
     * A `ScriptSource` and not a `std::string`, because this evaluates: the
     * non-XML arm reaches `evaluateExpression`, and a parameter that could
     * only be the author's text meant a `--script-engine lua` artifact
     * evaluated ECMAScript here while the `expr` arm beside it carried Lua the
     * build-time frontend had produced. Measured 2026-08-29 — the site was
     * invisible to the migration scan because a C++ RAW string literal made
     * it read as a log line.
     *
     * §scxml-B-2's first reading is answered from `content.source()`, the
     * author's own text: whether the children are an XML document is a
     * question about the document and not about which language the engine
     * speaks. The DOM node and the whitespace-normalized string are built
     * from that same half, and only the expression reading uses the lowered
     * one.
     */
    static bool initializeVariable(IScriptEngine &jsEngine, const std::string &sessionId, const std::string &varId,
                                   const ScriptSource &content, std::function<void(const std::string &)> errorCallback);

    /**
     * @brief The value inline children that are not an XML document hold
     *
     * The reading `initializeVariable` gives the children of a `<data>`: the
     * value of the expression they are, when they evaluate as one, and the
     * space-normalized string of the author's text otherwise. Never fails —
     * text that is no expression is a string.
     *
     * @param jsEngine Engine the expression is evaluated by
     * @param sessionId Session it is evaluated in
     * @param content The children, as a `ScriptSource` (see `initializeVariable`)
     */
    static ScriptValue inlineContentValue(IScriptEngine &jsEngine, const std::string &sessionId,
                                          const ScriptSource &content);

    /**
     * @brief The event data an inline `<content>` carries, as JSON text
     *
     * The one reading inline content has under a data model, which a
     * generated machine's send decides when it is built
     * (`to_author_data_content`, sce-build): children that are an XML
     * document are their own text, whose DOM node is built by whatever binds
     * `_event.data`; any other children are `inlineContentValue`, written as
     * the JSON text every delivery carries. So `<content>n + 1</content>` is
     * the number the expression gives, `<content>'v'</content>` the string
     * `v`, and `<content>a  b</content>` the string `a b`, on both engines.
     *
     * Every inline `<content>` that becomes event data takes it: a `<send>`'s
     * and a `<donedata>`'s alike, so a done event cannot read the same
     * children differently from a message.
     *
     * @param jsEngine Engine the expression is evaluated by
     * @param sessionId Session it is evaluated in
     * @param content The children, as a `ScriptSource` (see `initializeVariable`)
     * @param outTypedData When given, the value the JSON text writes, for the
     *        typed event pipeline; reset when the children are XML, whose
     *        value is the DOM node the binding builds from the text
     */
    static std::string inlineContentEventData(IScriptEngine &jsEngine, const std::string &sessionId,
                                              const ScriptSource &content,
                                              std::optional<ScriptValue> *outTypedData = nullptr);

    /**
     * @brief Initialize a datamodel variable with external file loading
     *
     * @param jsEngine JSEngine instance
     * @param sessionId Session ID
     * @param varId Variable identifier
     * @param src File URL (e.g., "file:test557.txt")
     * @param errorCallback Error callback
     * @return true if initialization succeeded, false otherwise
     *
     * §scxml-5.3.1: Load content from external source and initialize
     */
    static bool initializeVariableFromSrc(IScriptEngine &jsEngine, const std::string &sessionId,
                                          const std::string &varId, const std::string &src, const std::string &basePath,
                                          std::function<void(const std::string &)> errorCallback);

    /**
     * @brief Initialize a datamodel variable with expression
     *
     * @param jsEngine JSEngine instance
     * @param sessionId Session ID
     * @param varId Variable identifier
     * @param expr JavaScript expression to evaluate
     * @param errorCallback Error callback
     * @return true if initialization succeeded, false otherwise
     *
     * §scxml-5.3.1: Evaluate expr and assign to variable
     */
    static bool initializeVariableFromExpr(IScriptEngine &jsEngine, const std::string &sessionId,
                                           const std::string &varId, const ScriptSource &expr,
                                           std::function<void(const std::string &)> errorCallback);
};

}  // namespace SCE
