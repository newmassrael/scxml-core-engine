#pragma once

#include "scripting/IJSExecutionEngine.h"
#include <functional>
#include <string>

/**
 * @brief Helper for initializing datamodel variables with XML DOM support
 *
 * W3C SCXML B.2: ECMAScript datamodel must convert XML content to DOM structures
 * ARCHITECTURE.MD: Zero Duplication - Shared by Interpreter and AOT engines
 *
 * This helper provides a unified way to initialize datamodel variables:
 * - Inline XML content → DOM object with getElementsByTagName(), getAttribute()
 * - External file (src attribute) → Load file content and convert to DOM
 * - expr attribute → Evaluate expression directly
 */

namespace RSM {

// Forward declaration
class JSEngine;

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
     * W3C SCXML B.2: Function expressions must preserve function type
     * Test 453: ECMAScript function literals stored as functions, not converted
     */
    static bool isFunctionExpression(const std::string &expr);

    /**
     * @brief Initialize a datamodel variable in JSEngine
     *
     * @param jsEngine JSEngine instance for variable storage and expression evaluation
     * @param sessionId Session ID for JSEngine context
     * @param varId Variable identifier (e.g., "var1")
     * @param content Inline XML content or empty if using src/expr
     * @param errorCallback Function to call on error (receives error message)
     * @return true if initialization succeeded, false otherwise
     *
     * W3C SCXML 5.2.2: content, src, and expr are mutually exclusive
     * - If content is non-empty and starts with '<', create DOM object
     * - Otherwise, evaluate content as JavaScript expression
     *
     * W3C SCXML 5.3: Raises error.execution if initialization fails
     */
    static bool initializeVariable(JSEngine &jsEngine, const std::string &sessionId, const std::string &varId,
                                   const std::string &content, std::function<void(const std::string &)> errorCallback);

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
     * W3C SCXML 5.2.2: Load content from external source and initialize
     */
    static bool initializeVariableFromSrc(JSEngine &jsEngine, const std::string &sessionId, const std::string &varId,
                                          const std::string &src, const std::string &basePath,
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
     * W3C SCXML 5.2.2: Evaluate expr and assign to variable
     */
    static bool initializeVariableFromExpr(JSEngine &jsEngine, const std::string &sessionId, const std::string &varId,
                                           const std::string &expr,
                                           std::function<void(const std::string &)> errorCallback);
};

}  // namespace RSM
