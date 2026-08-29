// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/DataModelInitHelper.h"
#include "common/FileLoadingHelper.h"
#include "core/LogMacros.h"
#include "runtime/DataContentHelpers.h"
#include <algorithm>
#include <filesystem>

std::string SCE::DataModelInitHelper::resolveExecutableBasePath(const std::string &relativePath) {
    // ARCHITECTURE.md: Execution location independence for AOT tests
    // Convert relative basePath to absolute based on executable location

    namespace fs = std::filesystem;

    try {
#ifdef __EMSCRIPTEN__
        // WASM: /proc/self/exe points to Node.js binary, not WASM executable
        // W3CTestCLI sets working directory to project root, so resolve from cwd
        fs::path cwd = fs::current_path();
        fs::path absolutePath = (cwd / "build/tests" / relativePath).lexically_normal();

        std::string result = absolutePath.string();
        SCE_LOG_DEBUG("DataModelInitHelper::resolveExecutableBasePath (WASM): '{}' -> '{}'", relativePath, result);
        return result;
#else
        // Native: Get executable path (Linux-specific: /proc/self/exe)
        // For portability, could add platform detection (Mac: _NSGetExecutablePath, Windows: GetModuleFileName)
        fs::path exePath = fs::canonical("/proc/self/exe");

        // Get executable directory
        fs::path exeDir = exePath.parent_path();

        // Resolve relative path from executable directory
        fs::path absolutePath = exeDir / relativePath;

        std::string result = absolutePath.string();
        SCE_LOG_DEBUG("DataModelInitHelper::resolveExecutableBasePath (Native): '{}' -> '{}'", relativePath, result);
        return result;
#endif

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("DataModelInitHelper::resolveExecutableBasePath failed for '{}': {}", relativePath, e.what());
        // Fallback: return original relative path
        return relativePath;
    }
}

bool SCE::DataModelInitHelper::isFunctionExpression(const std::string &expr) {
    // §scxml-B-2: Detect JavaScript function literals
    // Test 453: function() {...} or () => {...} patterns

    if (expr.empty()) {
        return false;
    }

    // Trim leading whitespace
    auto start = std::find_if_not(expr.begin(), expr.end(), [](unsigned char ch) { return std::isspace(ch); });

    if (start == expr.end()) {
        return false;
    }

    std::string trimmed(start, expr.end());

    // Check for "function" keyword
    if (trimmed.find("function") == 0) {
        return true;
    }

    // Check for arrow function: () => or (param) => or param =>
    if (trimmed.find("=>") != std::string::npos) {
        return true;
    }

    return false;
}

bool SCE::DataModelInitHelper::initializeVariable(IScriptEngine &jsEngine, const std::string &sessionId,
                                                  const std::string &varId, const ScriptSource &contentSource,
                                                  std::function<void(const std::string &)> errorCallback) {
    // §scxml-5.2.2 & B.2: Initialize datamodel variable with inline content or expression

    // The AUTHOR's half answers every reading but the expression one: whether
    // the children are XML, and what the string value is when they are not an
    // expression, are questions about the document rather than about the
    // engine's language.
    const std::string &content = contentSource.source();

    if (content.empty()) {
        // §scxml-B-2-2 test 445: Empty content - create variable with undefined value
        // ARCHITECTURE.md Zero Duplication: Matches Interpreter (StateMachine.cpp:1597)
        // setVariable with empty ScriptValue creates undefined variable
        auto result = jsEngine.setVariable(sessionId, varId, ScriptValue{});
        result.wait();
        auto jsResult = result.get();
        if (!jsResult.isSuccess()) {
            errorCallback("Failed to create unbound variable " + varId + ": " + jsResult.getErrorMessage());
            return false;
        }
        SCE_LOG_DEBUG("DataModelInitHelper: Created unbound variable {} (undefined)", varId);
        return true;
    }

    // §scxml-B-2: Detect XML content and create DOM object
    // Match Interpreter behavior (StateMachine.cpp:1756) and JSEngine logic (JSEngineImpl.cpp:359)
    size_t firstNonWhitespace = content.find_first_not_of(" \t\r\n");
    bool isXML = firstNonWhitespace != std::string::npos && content[firstNonWhitespace] == '<';

    if (isXML) {
        // §scxml-B-2: XML content → create DOM object using setVariableAsDOM
        // ARCHITECTURE.MD: Zero Duplication - Matches Interpreter (StateMachine.cpp:1756)
        auto result = jsEngine.setVariableAsDOM(sessionId, varId, content);
        result.wait();
        auto jsResult = result.get();

        if (!jsResult.isSuccess()) {
            errorCallback("Failed to initialize XML DOM for " + varId + ": " + jsResult.getErrorMessage());
            return false;
        }

        SCE_LOG_DEBUG("DataModelInitHelper: Initialized {} with XML DOM content", varId);
        return true;
    }

    // §scxml-B-2: Non-XML content - try evaluating as an expression first
    // ARCHITECTURE.md Zero Duplication: Matches StateMachine.cpp:1772-1778 (try eval first)
    //
    // The LOWERED half goes to the engine. Under the ECMAScript target the two
    // halves are the same string, so this is the author's text as before.
    auto evalResult = jsEngine.evaluateExpression(sessionId, contentSource);
    evalResult.wait();
    auto evalJsResult = evalResult.get();

    if (evalJsResult.isSuccess()) {
        // Successfully evaluated as JavaScript expression
        auto setResult = jsEngine.setVariable(sessionId, varId, evalJsResult.getInternalValue());
        setResult.wait();
        auto setJsResult = setResult.get();

        if (!setJsResult.isSuccess()) {
            errorCallback("Failed to set variable " + varId + ": " + setJsResult.getErrorMessage());
            return false;
        }

        SCE_LOG_DEBUG("DataModelInitHelper: Initialized {} with evaluated content", varId);
        return true;
    }

    // §scxml-B-2 test 558: Evaluation failed - normalize whitespace and store as string
    // ARCHITECTURE.md Zero Duplication: Matches StateMachine.cpp:1793-1811 (fallback to whitespace normalization)
    std::string normalized = normalizeWhitespace(content);

    auto setStrResult = jsEngine.setVariable(sessionId, varId, normalized);
    setStrResult.wait();
    auto setStrJsResult = setStrResult.get();

    if (!setStrJsResult.isSuccess()) {
        errorCallback("Failed to set normalized string for " + varId + ": " + setStrJsResult.getErrorMessage());
        return false;
    }

    SCE_LOG_DEBUG("DataModelInitHelper: Initialized {} with whitespace-normalized string: '{}'", varId, normalized);
    return true;
}

bool SCE::DataModelInitHelper::initializeVariableFromSrc(IScriptEngine &jsEngine, const std::string &sessionId,
                                                         const std::string &varId, const std::string &src,
                                                         const std::string &basePath,
                                                         std::function<void(const std::string &)> errorCallback) {
    // §scxml-5.2.2: Load content from external file
    // ARCHITECTURE.MD: Zero Duplication - Use FileLoadingHelper (Single Source of Truth)

    std::string content;
    std::string errorMsg;
    bool success = FileLoadingHelper::loadExternalScript(src, basePath, content, errorMsg);

    if (!success) {
        errorCallback(errorMsg);
        return false;
    }

    // Initialize with loaded content.
    //
    // `ScriptSource::ecmascript` and not a lowered pair, because the file is
    // read at RUN time: nothing at build time saw this text, so there is no
    // Lua half to pair with it. A Lua engine lowers it through the same
    // frontend at its own boundary — the run-time seat this seam keeps for
    // exactly the text a build cannot see. §scxml-5.2.2.
    bool initSuccess = initializeVariable(jsEngine, sessionId, varId, ScriptSource::ecmascript(content), errorCallback);
    if (initSuccess) {
        SCE_LOG_DEBUG("DataModelInitHelper: Loaded {} from external file: {}", varId, src);
    }
    return initSuccess;
}

bool SCE::DataModelInitHelper::initializeVariableFromExpr(IScriptEngine &jsEngine, const std::string &sessionId,
                                                          const std::string &varId, const ScriptSource &expr,
                                                          std::function<void(const std::string &)> errorCallback) {
    // §scxml-5.3: Evaluate expr attribute and assign to variable
    // Test 277: expr evaluation failure must raise error.execution (no fallback to whitespace normalization)
    // ARCHITECTURE.md Zero Duplication: Matches AOT engine template (scriptengine_helpers.jinja2)

    auto evalResult = jsEngine.evaluateExpression(sessionId, expr);
    evalResult.wait();
    auto evalJsResult = evalResult.get();

    if (!evalJsResult.isSuccess()) {
        // §scxml-5.3: Evaluation failure raises error.execution, variable remains unbound
        // The author's own text: this string reaches `_event.data` on
        // error.execution, where lowered Lua would name a line nobody wrote.
        errorCallback("Failed to evaluate expr for variable " + varId + ": " + expr.source());
        return false;
    }

    // Evaluation succeeded - set variable to evaluated result
    auto setResult = jsEngine.setVariable(sessionId, varId, evalJsResult.getInternalValue());
    setResult.wait();
    auto setJsResult = setResult.get();

    if (!setJsResult.isSuccess()) {
        errorCallback("Failed to set variable " + varId + " after expr evaluation: " + setJsResult.getErrorMessage());
        return false;
    }

    SCE_LOG_DEBUG("DataModelInitHelper: Initialized {} from expr: '{}'", varId, expr.source());
    return true;
}
