// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "scripting/IScriptEngine.h"
#include <functional>
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
     * §scxml-5.2.2: content, src, and expr are mutually exclusive
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
     * @brief Initialize a datamodel variable with external file loading
     *
     * @param jsEngine JSEngine instance
     * @param sessionId Session ID
     * @param varId Variable identifier
     * @param src File URL (e.g., "file:test557.txt")
     * @param errorCallback Error callback
     * @return true if initialization succeeded, false otherwise
     *
     * §scxml-5.2.2: Load content from external source and initialize
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
     * §scxml-5.2.2: Evaluate expr and assign to variable
     */
    static bool initializeVariableFromExpr(IScriptEngine &jsEngine, const std::string &sessionId,
                                           const std::string &varId, const ScriptSource &expr,
                                           std::function<void(const std::string &)> errorCallback);
};

}  // namespace SCE
