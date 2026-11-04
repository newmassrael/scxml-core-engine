// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/DataModelInitHelper.h"
#include "common/EventDataHelper.h"
#include "common/FileLoadingHelper.h"
#include "common/LogToLogger.h"
#include "runtime/DataContentHelpers.h"
#include <algorithm>
#include <filesystem>

#ifdef __EMSCRIPTEN__
#include <cstdlib>
#include <emscripten.h>

// WASM: the program's path as its host declared it, which Emscripten keeps as
// `thisProgram` and hands to main() as argv[0]. Under Node it is the script
// Node runs; in a browser, where no file is the program, `./this.program`.
EM_JS_DEPS(sce_this_program_deps, "$stringToNewUTF8");
// clang-format off
EM_JS(char *, sce_this_program, (), {
    return stringToNewUTF8(thisProgram);
});
// clang-format on
#endif

std::string SCE::DataModelInitHelper::resolveExecutableBasePath(const std::string &relativePath) {
    // ARCHITECTURE.md: Execution location independence for AOT tests
    // Convert relative basePath to absolute based on executable location

    namespace fs = std::filesystem;

    try {
#ifdef __EMSCRIPTEN__
        // WASM: /proc/self/exe names the Node binary, not this program. The
        // program's own path is the one its host declared; resolved against
        // its directory, as natively, a host that mounts its directories at
        // their own paths (tests/w3c/wasm_polyfill.js) reads the same file.
        char *program = sce_this_program();
        const fs::path programPath(program);
        std::free(program);
        fs::path absolutePath = (programPath.parent_path() / relativePath).lexically_normal();

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
    // §scxml-5.3.2 & B.2: Initialize datamodel variable with inline content or expression

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
    if (isXMLContent(content)) {
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

    // §scxml-B-2: Non-XML content - the value of the expression it is, or
    // its space-normalized text (test 558)
    auto setResult = jsEngine.setVariable(sessionId, varId, inlineContentValue(jsEngine, sessionId, contentSource));
    setResult.wait();
    auto setJsResult = setResult.get();

    if (!setJsResult.isSuccess()) {
        errorCallback("Failed to set variable " + varId + ": " + setJsResult.getErrorMessage());
        return false;
    }

    SCE_LOG_DEBUG("DataModelInitHelper: Initialized {} from inline content", varId);
    return true;
}

ScriptValue SCE::DataModelInitHelper::inlineContentValue(IScriptEngine &jsEngine, const std::string &sessionId,
                                                         const ScriptSource &content) {
    // Try evaluating the children as an expression first. The LOWERED half
    // goes to the engine; under the ECMAScript target the two halves are the
    // same string, so this is the author's text.
    auto evalResult = jsEngine.evaluateExpression(sessionId, content);
    evalResult.wait();
    auto evalJsResult = evalResult.get();

    if (evalJsResult.isSuccess()) {
        return evalJsResult.getInternalValue();
    }

    // No expression: the author's text, whitespace-normalized, is a string.
    return ScriptValue(normalizeWhitespace(content.source()));
}

std::string SCE::DataModelInitHelper::inlineContentEventData(IScriptEngine &jsEngine, const std::string &sessionId,
                                                             const ScriptSource &content) {
    // Whether the children are an XML document is asked of the author's text,
    // as `initializeVariable` asks it; such children travel as written.
    if (isXMLContent(content.source())) {
        return content.source();
    }
    return EventDataHelper::scriptValueToJsonString(inlineContentValue(jsEngine, sessionId, content));
}

bool SCE::DataModelInitHelper::initializeVariableFromSrc(IScriptEngine &jsEngine, const std::string &sessionId,
                                                         const std::string &varId, const std::string &src,
                                                         const std::string &basePath,
                                                         std::function<void(const std::string &)> errorCallback) {
    // §scxml-5.3.1: Load content from external file
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
    // exactly the text a build cannot see. §scxml-5.3.1.
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
