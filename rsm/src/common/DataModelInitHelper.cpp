#include "common/DataModelInitHelper.h"
#include "common/FileLoadingHelper.h"
#include "common/Logger.h"
#include "scripting/JSEngine.h"
#include <algorithm>

bool RSM::DataModelInitHelper::isFunctionExpression(const std::string &expr) {
    // W3C SCXML B.2: Detect JavaScript function literals
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

bool RSM::DataModelInitHelper::initializeVariable(JSEngine &jsEngine, const std::string &sessionId,
                                                  const std::string &varId, const std::string &content,
                                                  std::function<void(const std::string &)> errorCallback) {
    // W3C SCXML 5.2.2 & B.2: Initialize datamodel variable with inline content or expression

    if (content.empty()) {
        // W3C SCXML B.2.2 test 445: Empty content - create variable with undefined value
        // ARCHITECTURE.md Zero Duplication: Matches Interpreter (StateMachine.cpp:1597)
        // setVariable with empty ScriptValue creates undefined variable
        auto result = jsEngine.setVariable(sessionId, varId, ScriptValue{});
        result.wait();
        auto jsResult = result.get();
        if (!jsResult.isSuccess()) {
            errorCallback("Failed to create unbound variable " + varId + ": " + jsResult.getErrorMessage());
            return false;
        }
        LOG_DEBUG("DataModelInitHelper: Created unbound variable {} (undefined)", varId);
        return true;
    }

    // W3C SCXML B.2: Detect XML content and create DOM object
    // Match Interpreter behavior (StateMachine.cpp:1756) and JSEngine logic (JSEngineImpl.cpp:359)
    size_t firstNonWhitespace = content.find_first_not_of(" \t\r\n");
    bool isXML = firstNonWhitespace != std::string::npos && content[firstNonWhitespace] == '<';

    if (isXML) {
        // W3C SCXML B.2: XML content → create DOM object using setVariableAsDOM
        // ARCHITECTURE.MD: Zero Duplication - Matches Interpreter (StateMachine.cpp:1756)
        auto result = jsEngine.setVariableAsDOM(sessionId, varId, content);
        result.wait();
        auto jsResult = result.get();

        if (!jsResult.isSuccess()) {
            errorCallback("Failed to initialize XML DOM for " + varId + ": " + jsResult.getErrorMessage());
            return false;
        }

        LOG_DEBUG("DataModelInitHelper: Initialized {} with XML DOM content", varId);
        return true;
    }

    // W3C SCXML B.2: Non-XML content - evaluate as JavaScript expression first (matches Interpreter)
    // ARCHITECTURE.md Zero Duplication: Matches StateMachine.cpp:1772-1778
    auto evalResult = jsEngine.evaluateExpression(sessionId, content);
    evalResult.wait();
    auto evalJsResult = evalResult.get();

    if (!evalJsResult.isSuccess()) {
        errorCallback("Failed to evaluate expression for " + varId + ": " + evalJsResult.getErrorMessage());
        return false;
    }

    // Set variable with evaluated result
    auto setResult = jsEngine.setVariable(sessionId, varId, evalJsResult.getInternalValue());
    setResult.wait();
    auto setJsResult = setResult.get();

    if (!setJsResult.isSuccess()) {
        errorCallback("Failed to set variable " + varId + ": " + setJsResult.getErrorMessage());
        return false;
    }

    LOG_DEBUG("DataModelInitHelper: Initialized {} with evaluated content", varId);
    return true;
}

bool RSM::DataModelInitHelper::initializeVariableFromSrc(JSEngine &jsEngine, const std::string &sessionId,
                                                         const std::string &varId, const std::string &src,
                                                         const std::string &basePath,
                                                         std::function<void(const std::string &)> errorCallback) {
    // W3C SCXML 5.2.2: Load content from external file
    // ARCHITECTURE.MD: Zero Duplication - Use FileLoadingHelper (Single Source of Truth)

    std::string content;
    std::string errorMsg;
    bool success = FileLoadingHelper::loadExternalScript(src, basePath, content, errorMsg);

    if (!success) {
        errorCallback(errorMsg);
        return false;
    }

    // Initialize with loaded content
    bool initSuccess = initializeVariable(jsEngine, sessionId, varId, content, errorCallback);
    if (initSuccess) {
        LOG_DEBUG("DataModelInitHelper: Loaded {} from external file: {}", varId, src);
    }
    return initSuccess;
}

bool RSM::DataModelInitHelper::initializeVariableFromExpr(JSEngine &jsEngine, const std::string &sessionId,
                                                          const std::string &varId, const std::string &expr,
                                                          std::function<void(const std::string &)> errorCallback) {
    // W3C SCXML 5.2.2: Evaluate expression and assign to variable
    auto result = jsEngine.setVariable(sessionId, varId, expr);
    result.wait();
    auto jsResult = result.get();

    if (!jsResult.isSuccess()) {
        errorCallback("Failed to initialize " + varId + " with expr: " + jsResult.getErrorMessage());
        return false;
    }

    LOG_DEBUG("DataModelInitHelper: Initialized {} with expr: {}", varId, expr);
    return true;
}
