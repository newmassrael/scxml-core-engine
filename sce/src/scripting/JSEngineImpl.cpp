// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/DataModelInitHelper.h"
#include "core/LogMacros.h"
#include "quickjs.h"
#include "scripting/DOMBinding.h"
#include "scripting/JSEngine.h"
#include <climits>
#include <cmath>
#include <cstdio>
#include <iostream>

namespace SCE {

// Global the C++ side parks system-variable values on so the setup script can
// read them as values. The script removes it before returning; the name is
// underscore-prefixed to stay clear of anything an SCXML document can declare.
static constexpr const char *SYSTEM_VARS_STASH = "__sce_system_vars_stash";

// === Internal JavaScript Execution Methods ===

ScriptResult JSEngine::executeScriptInternal(const std::string &sessionId, const std::string &script) {
    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    JSContext *ctx = session->jsContext;

    // §scxml-B-2-10: the content of <script> may be any ECMAScript program, so it is
    // evaluated as global code rather than restricted to a statement subset.
    // Execute script with QuickJS global evaluation
    SCE_LOG_DEBUG("JSEngine: Executing script with QuickJS...");

    ::JSValue result = JS_Eval(ctx, script.c_str(), script.length(), "<script>", JS_EVAL_TYPE_GLOBAL);

    SCE_LOG_DEBUG("JSEngine: JS_Eval completed, checking result...");

    if (JS_IsException(result)) {
        SCE_LOG_DEBUG("JSEngine: Exception occurred in script execution");
        ScriptResult error = createErrorFromException(ctx);
        SCE_LOG_ERROR("JSEngine::executeScriptInternal - QuickJS exception: {}", error.getErrorMessage());
        JS_FreeValue(ctx, result);
        return error;
    }

    SCE_LOG_DEBUG("JSEngine: Script execution successful, converting result...");
    ScriptValue jsResult = quickJSToJSValue(ctx, result);
    JS_FreeValue(ctx, result);
    SCE_LOG_DEBUG("JSEngine: Result conversion completed, returning success");
    return ScriptResult::createSuccess(jsResult);
}

ScriptResult JSEngine::evaluateExpressionInternal(const std::string &sessionId, const std::string &expression) {
    SCE_LOG_DEBUG("JSEngine::evaluateExpressionInternal - Evaluating expression '{}' in session '{}'", expression,
                  sessionId);

    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        SCE_LOG_ERROR("JSEngine::evaluateExpressionInternal - Session not found: {}", sessionId);
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    SCE_LOG_DEBUG("JSEngine::evaluateExpressionInternal - Session found, context valid");

    JSContext *ctx = session->jsContext;

    // §scxml-B-2-5: any ECMAScript expression is a legal value expression, so the text
    // is handed to the engine unrestricted rather than matched against a subset.
    // First try to evaluate as-is
    ::JSValue result = JS_Eval(ctx, expression.c_str(), expression.length(), "<expression>", JS_EVAL_TYPE_GLOBAL);

    // If it failed and the expression starts with '{', try wrapping in parentheses for object literals
    if (JS_IsException(result) && !expression.empty() && expression[0] == '{') {
        SCE_LOG_DEBUG("JSEngine::evaluateExpressionInternal - First evaluation failed, trying wrapped expression for "
                      "object literal");
        JS_FreeValue(ctx, result);  // Free the exception

        std::string wrappedExpression = "(" + expression + ")";
        result =
            JS_Eval(ctx, wrappedExpression.c_str(), wrappedExpression.length(), "<expression>", JS_EVAL_TYPE_GLOBAL);
    }

    // ARCHITECTURE.MD: Zero Duplication - Use DataModelInitHelper (shared with Interpreter/AOT)
    // §scxml-B-2: If it failed and the expression starts with 'function', try wrapping in parentheses for function
    // expressions Test 453: ECMAScript function literals must be accepted as value expressions
    if (JS_IsException(result) && DataModelInitHelper::isFunctionExpression(expression)) {
        SCE_LOG_DEBUG("JSEngine::evaluateExpressionInternal - First evaluation failed, trying wrapped expression for "
                      "function literal");
        JS_FreeValue(ctx, result);  // Free the exception

        std::string wrappedExpression = "(" + expression + ")";
        result =
            JS_Eval(ctx, wrappedExpression.c_str(), wrappedExpression.length(), "<expression>", JS_EVAL_TYPE_GLOBAL);
    }

    if (JS_IsException(result)) {
        SCE_LOG_ERROR("JSEngine::evaluateExpressionInternal - Final JS_Eval failed for expression '{}'", expression);

        // Root cause analysis: Check _event object state when _event.data.aParam fails
        if (expression.find("_event.data") != std::string::npos) {
            SCE_LOG_ERROR("JSEngine: _event.data access failed - debugging info:");

            // Check _event object existence
            ::JSValue eventCheck = JS_Eval(ctx, "_event", 6, "<debug>", JS_EVAL_TYPE_GLOBAL);
            if (JS_IsException(eventCheck)) {
                SCE_LOG_ERROR("JSEngine: _event object does not exist");
                JS_FreeValue(ctx, eventCheck);
            } else if (JS_IsUndefined(eventCheck)) {
                SCE_LOG_ERROR("JSEngine: _event is undefined");
                JS_FreeValue(ctx, eventCheck);
            } else {
                SCE_LOG_DEBUG("JSEngine: _event object exists");

                // Check _event.data
                ::JSValue dataCheck = JS_Eval(ctx, "_event.data", 11, "<debug>", JS_EVAL_TYPE_GLOBAL);
                if (JS_IsException(dataCheck)) {
                    SCE_LOG_ERROR("JSEngine: _event.data access failed");
                    JS_FreeValue(ctx, dataCheck);
                } else if (JS_IsUndefined(dataCheck)) {
                    SCE_LOG_ERROR("JSEngine: _event.data is undefined");
                    JS_FreeValue(ctx, dataCheck);
                } else {
                    SCE_LOG_DEBUG("JSEngine: _event.data exists");
                }
                JS_FreeValue(ctx, dataCheck);
                JS_FreeValue(ctx, eventCheck);
            }
        }

        ScriptResult error = createErrorFromException(ctx);
        SCE_LOG_ERROR("JSEngine::evaluateExpressionInternal - QuickJS exception: {}", error.getErrorMessage());
        JS_FreeValue(ctx, result);
        return error;
    }

    SCE_LOG_DEBUG("JSEngine::evaluateExpressionInternal - JS_Eval succeeded for expression '{}'", expression);

    ScriptValue jsResult = quickJSToJSValue(ctx, result);
    JS_FreeValue(ctx, result);

    // Debug logging for ScriptValue conversion
    std::string debug_type = "unknown";
    std::string debug_value = "unknown";
    if (std::holds_alternative<ScriptUndefined>(jsResult)) {
        debug_type = "ScriptUndefined";
        debug_value = "undefined";
    } else if (std::holds_alternative<ScriptNull>(jsResult)) {
        debug_type = "ScriptNull";
        debug_value = "null";
    } else if (std::holds_alternative<bool>(jsResult)) {
        debug_type = "bool";
        debug_value = std::get<bool>(jsResult) ? "true" : "false";
    } else if (std::holds_alternative<int64_t>(jsResult)) {
        debug_type = "int64_t";
        debug_value = std::to_string(std::get<int64_t>(jsResult));
    } else if (std::holds_alternative<double>(jsResult)) {
        debug_type = "double";
        debug_value = std::to_string(std::get<double>(jsResult));
    } else if (std::holds_alternative<std::string>(jsResult)) {
        debug_type = "string";
        debug_value = "\"" + std::get<std::string>(jsResult) + "\"";
    }

    SCE_LOG_TRACE("JSEngine::evaluateExpressionInternal - Expression='{}', type={}, value={}", expression, debug_type,
                  debug_value);

    return ScriptResult::createSuccess(jsResult);
}

ScriptResult JSEngine::validateExpressionInternal(const std::string &sessionId, const std::string &expression) {
    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    JSContext *ctx = session->jsContext;

    // Try compiling as JavaScript expression to check for syntax errors
    ::JSValue result = JS_Eval(ctx, ("(function(){return (" + expression + ");})").c_str(), expression.length() + 23,
                               "<validation>", JS_EVAL_FLAG_COMPILE_ONLY);

    if (JS_IsException(result)) {
        ScriptResult error = createErrorFromException(ctx);
        JS_FreeValue(ctx, result);
        return error;
    }

    JS_FreeValue(ctx, result);
    return ScriptResult::createSuccess();
}

ScriptResult JSEngine::setVariableInternal(const std::string &sessionId, const std::string &name,
                                           const ScriptValue &value) {
    SCE_LOG_DEBUG("JSEngine::setVariableInternal - Setting variable '{}' in session '{}'", name, sessionId);

    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        SCE_LOG_ERROR("JSEngine::setVariableInternal - Session not found: {}", sessionId);
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    JSContext *ctx = session->jsContext;
    ::JSValue global = JS_GetGlobalObject(ctx);

    // §scxml-B-2-1: each <data> element becomes an ECMAScript variable object whose
    // name is the element's 'id', holding the JSON, DOM or string value it was given.
    // Log the value using variant visit pattern
    std::string valueStr = std::visit(
        [](const auto &v) -> std::string {
            using T = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<T, std::string>) {
                return "STRING: '" + v + "'";
            } else if constexpr (std::is_same_v<T, bool>) {
                return "BOOLEAN: " + std::string(v ? "true" : "false");
            } else if constexpr (std::is_same_v<T, int64_t>) {
                return "NUMBER(int64): " + std::to_string(v);
            } else if constexpr (std::is_same_v<T, double>) {
                return "NUMBER(double): " + std::to_string(v);
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptArray>>) {
                return "ARRAY: [" + std::to_string(v->elements.size()) + " elements]";
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
                return "OBJECT: [" + std::to_string(v->properties.size()) + " properties]";
            } else if constexpr (std::is_same_v<T, ScriptNull>) {
                return "NULL";
            } else if constexpr (std::is_same_v<T, ScriptUndefined>) {
                return "UNDEFINED";
            } else {
                return "UNKNOWN_TYPE";
            }
        },
        value);

    SCE_LOG_DEBUG("JSEngine::setVariableInternal - Variable '{}' value: {}", name, valueStr);

    ::JSValue qjsValue = jsValueToQuickJS(ctx, value);

    // Check if conversion was successful
    if (JS_IsException(qjsValue)) {
        SCE_LOG_ERROR(
            "JSEngine::setVariableInternal - Failed to convert ScriptValue to QuickJS value for variable '{}'", name);
        JS_FreeValue(ctx, global);
        return createErrorFromException(ctx);
    }

    // Set the property
    int result = JS_SetPropertyStr(ctx, global, name.c_str(), qjsValue);

    if (result < 0) {
        // §scxml-5.10: Check if this is a read-only system variable error
        ::JSValue exc = JS_GetException(ctx);
        if (!JS_IsNull(exc)) {
            // Get error message to check if it's a read-only error
            const char *errStr = JS_ToCString(ctx, exc);
            std::string errorMsg = errStr ? errStr : "Unknown error";
            JS_FreeCString(ctx, errStr);
            JS_FreeValue(ctx, exc);

            SCE_LOG_ERROR("JSEngine::setVariableInternal - Failed to set property '{}': {}", name, errorMsg);
            JS_FreeValue(ctx, global);
            return ScriptResult::createError("Failed to set variable " + name + ": " + errorMsg);
        }

        SCE_LOG_ERROR("JSEngine::setVariableInternal - Failed to set property '{}' in global object", name);
        JS_FreeValue(ctx, global);
        return ScriptResult::createError("Failed to set variable: " + name);
    }

    JS_FreeValue(ctx, global);

    // Track pre-initialized variable for datamodel initialization optimization
    // Thread-safe: protect access to SessionContext::preInitializedVars
    {
        std::lock_guard<std::mutex> lock(sessionsMutex_);
        session->preInitializedVars.insert(name);
    }

    SCE_LOG_DEBUG("JSEngine::setVariableInternal - Successfully set variable '{}' in session '{}'", name, sessionId);
    return ScriptResult::createSuccess();
}

ScriptResult JSEngine::getVariableInternal(const std::string &sessionId, const std::string &name) {
    SCE_LOG_DEBUG("JSEngine::getVariableInternal - Getting variable '{}' from session '{}'", name, sessionId);

    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        SCE_LOG_ERROR("JSEngine::getVariableInternal - Session not found: {}", sessionId);
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    SCE_LOG_DEBUG("JSEngine::getVariableInternal - Session found, context valid");

    JSContext *ctx = session->jsContext;
    ::JSValue global = JS_GetGlobalObject(ctx);

    // First check if the property exists before getting it
    JSAtom atom = JS_NewAtom(ctx, name.c_str());
    [[maybe_unused]] int hasProperty = JS_HasProperty(ctx, global, atom);
    SCE_LOG_DEBUG("JSEngine::getVariableInternal - JS_HasProperty('{}') returned: {}", name, hasProperty);
    JS_FreeAtom(ctx, atom);

    ::JSValue qjsValue = JS_GetPropertyStr(ctx, global, name.c_str());

    if (JS_IsException(qjsValue)) {
        SCE_LOG_ERROR("JSEngine::getVariableInternal - JS_GetPropertyStr failed for variable '{}'", name);
        JS_FreeValue(ctx, global);
        return createErrorFromException(ctx);
    }

    // Check if the property actually exists (not just undefined)
    if (JS_IsUndefined(qjsValue)) {
        SCE_LOG_DEBUG("JSEngine::getVariableInternal - Variable '{}' is undefined, checking if property exists", name);
        // Use JS_HasProperty to distinguish between "not set" and "set to
        // undefined"
        JSAtom atom2 = JS_NewAtom(ctx, name.c_str());
        int hasProperty2 = JS_HasProperty(ctx, global, atom2);
        JS_FreeAtom(ctx, atom2);  // Free the atom to prevent memory leak
        SCE_LOG_DEBUG("JSEngine::getVariableInternal - Second JS_HasProperty('{}') returned: {}", name, hasProperty2);
        if (hasProperty2 <= 0) {
            // Property doesn't exist - this is not an error, caller will handle
            SCE_LOG_DEBUG("JSEngine::getVariableInternal - Variable '{}' does not exist in global context", name);
            JS_FreeValue(ctx, qjsValue);
            JS_FreeValue(ctx, global);
            return ScriptResult::createError("Variable not found: " + name);
        }
        // Property exists but is undefined - this is valid, continue with existing
        // qjsValue
        SCE_LOG_DEBUG("JSEngine::getVariableInternal - Variable '{}' exists but is set to undefined", name);
    } else {
        SCE_LOG_DEBUG("JSEngine::getVariableInternal - Variable '{}' found with value", name);
    }

    ScriptValue result = quickJSToJSValue(ctx, qjsValue);
    JS_FreeValue(ctx, qjsValue);
    JS_FreeValue(ctx, global);

    SCE_LOG_DEBUG("JSEngine::getVariableInternal - Successfully retrieved variable '{}'", name);
    return ScriptResult::createSuccess(result);
}

/// What the ladder in `parseEventData` produced, and which of its rungs
/// produced it.
///
/// The rung is returned rather than re-derived by the caller. A caller can
/// tell a string from an object afterwards, but not a string the ladder chose
/// because the content was prose from one it chose because a structured read
/// failed — and that difference is the whole point of `PayloadReading`.
struct ParsedEventData {
    ::JSValue value;
    SCE::PayloadReading reading;
};

static ParsedEventData parseEventData(JSContext *ctx, const std::string &dataStr) {
    // §scxml-B-2: parse the event data as JSON, an XML DOM, or a
    // space-normalized string.
    // §scxml-B-2-8-1: _event.data carries what the event supplied — key-value pairs
    // become named properties, JSON becomes the corresponding object, valid XML becomes
    // a DOM, and anything else becomes a space-normalized string.
    // Whether the content OPENS like a document, skipping leading whitespace.
    // It is a guess about which reading applies, not the reading itself: the
    // clause conditions the DOM rung on the content being one — "if the
    // Processor can interpret the content as a valid XML document, it MUST
    // create the corresponding DOM structure" — and closes with "Otherwise,
    // the Processor MUST treat the content as a space-normalized string
    // literal". So a guess that turns out wrong falls through to the readings
    // below rather than ending here.
    size_t firstNonWhitespace = dataStr.find_first_not_of(" \t\r\n");
    bool opensLikeXML = firstNonWhitespace != std::string::npos && dataStr[firstNonWhitespace] == '<';

    if (opensLikeXML) {
        ::JSValue dom = SCE::DOMBinding::createDOMObject(ctx, dataStr);
        if (!JS_IsException(dom)) {
            return {dom, SCE::PayloadReading::Dom};
        }
        // `createDOMObject` refuses with `JS_ThrowSyntaxError`, which leaves a
        // pending exception on the context. Clearing it here is not tidying:
        // this reading is being ABANDONED rather than failing, and a pending
        // exception left behind surfaces as an unrelated failure at whatever
        // the session evaluates next. It used to be reported to the caller
        // instead, which stored `_event.data` as `undefined` and logged an
        // ERROR for a payload the clause calls perfectly ordinary — every
        // `error.*` message this repository raises names the SCXML construct
        // that failed, so every one of them opens with `<`.
        JS_FreeValue(ctx, JS_GetException(ctx));
    }

    // Try to parse as JSON
    ::JSValue jsonValue = JS_ParseJSON(ctx, dataStr.c_str(), dataStr.length(), "<event-data>");
    if (!JS_IsException(jsonValue)) {
        return {jsonValue, SCE::PayloadReading::Structured};
    }

    // §scxml-B-2 test 562: If not XML or JSON, create space-normalized string
    // "processor creates space normalized string when receiving anything other than KVPs or XML"
    JS_FreeValue(ctx, jsonValue);  // Free the exception

    // Space normalization: collapse multiple whitespace (spaces, tabs, newlines) into single spaces
    std::string normalized;
    normalized.reserve(dataStr.length());
    bool inWhitespace = false;
    bool hasContent = false;

    for (char c : dataStr) {
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
            if (hasContent && !inWhitespace) {
                normalized += ' ';
                inWhitespace = true;
            }
        } else {
            normalized += c;
            inWhitespace = false;
            hasContent = true;
        }
    }

    // Remove trailing whitespace if any
    if (!normalized.empty() && normalized.back() == ' ') {
        normalized.pop_back();
    }

    // Which of the two third-rung readings this is. The clause treats them the
    // same and a host does not: prose arriving as text is the ladder working
    // (W3C test 562), and a payload that opened with a brace and would not
    // parse is a payload whose fields have just stopped existing. Only here is
    // that difference still visible, because only here was the structured read
    // attempted.
    return {JS_NewString(ctx, normalized.c_str()), SCE::payloadReadingOfText(dataStr)};
}

SetCurrentEventResult JSEngine::setCurrentEventInternal(const std::string &sessionId,
                                                        const std::shared_ptr<Event> &event) {
    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        return {ScriptResult::createError("Session not found: " + sessionId), PayloadReading::Absent};
    }

    // Which rung of §scxml-B-2-8-1 the payload ended up on. Recorded as the
    // ladder walks rather than re-derived afterwards: a caller can tell a
    // string from an object, but not a string chosen because the content was
    // prose from one chosen because a structured read failed.
    PayloadReading reading = PayloadReading::Absent;

    JSContext *ctx = session->jsContext;
    ::JSValue global = JS_GetGlobalObject(ctx);
    ::JSValue eventObj = JS_NewObject(ctx);

    if (event) {
        // §scxml-B-2-8: _event is an object with one property per field of §scxml-5.10.1 —
        // name, type, sendid, origin, origintype and invokeid are Strings while data may
        // hold any type; a field this specification leaves unset is ECMAScript undefined.
        // Set event properties
        JS_SetPropertyStr(ctx, eventObj, "name", JS_NewString(ctx, event->getName().c_str()));
        JS_SetPropertyStr(ctx, eventObj, "type", JS_NewString(ctx, event->getType().c_str()));
        JS_SetPropertyStr(ctx, eventObj, "sendid", JS_NewString(ctx, event->getSendId().c_str()));
        JS_SetPropertyStr(ctx, eventObj, "origin", JS_NewString(ctx, event->getOrigin().c_str()));
        JS_SetPropertyStr(ctx, eventObj, "origintype", JS_NewString(ctx, event->getOriginType().c_str()));
        JS_SetPropertyStr(ctx, eventObj, "invokeid", JS_NewString(ctx, event->getInvokeId().c_str()));

        // Set event data
        if (event->hasData()) {
            std::string dataStr = event->getDataAsString();
            ParsedEventData parsed = parseEventData(ctx, dataStr);
            if (!JS_IsException(parsed.value)) {
                JS_SetPropertyStr(ctx, eventObj, "data", parsed.value);
                reading = parsed.reading;
            } else {
                JS_SetPropertyStr(ctx, eventObj, "data", JS_UNDEFINED);
                SCE_LOG_ERROR("JSEngine: Failed to parse event data for eventObj");
            }
        } else {
            JS_SetPropertyStr(ctx, eventObj, "data", JS_UNDEFINED);
        }

        // Store event in session
        session->currentEvent = event;
    } else {
        // Clear event
        JS_SetPropertyStr(ctx, eventObj, "name", JS_NewString(ctx, ""));
        JS_SetPropertyStr(ctx, eventObj, "type", JS_NewString(ctx, ""));
        JS_SetPropertyStr(ctx, eventObj, "sendid", JS_NewString(ctx, ""));
        JS_SetPropertyStr(ctx, eventObj, "origin", JS_NewString(ctx, ""));
        JS_SetPropertyStr(ctx, eventObj, "origintype", JS_NewString(ctx, ""));
        JS_SetPropertyStr(ctx, eventObj, "invokeid", JS_NewString(ctx, ""));
        JS_SetPropertyStr(ctx, eventObj, "data", JS_UNDEFINED);

        session->currentEvent.reset();
    }

    // §scxml-5.10: Lazy initialization of _event on first event
    ::JSValue eventDataProperty;
    if (!session->eventObjectInitialized) {
        SCE_LOG_DEBUG("JSEngine: First event detected - initializing _event object per W3C SCXML 5.10 for session: {}",
                      sessionId);
        // Setup _event object now that first event is being processed
        setupEventObject(ctx, sessionId);
        session->eventObjectInitialized = true;
        // Get the newly created __eventData
        eventDataProperty = JS_GetPropertyStr(ctx, global, "__eventData");
        if (!JS_IsObject(eventDataProperty)) {
            JS_FreeValue(ctx, eventDataProperty);
            JS_FreeValue(ctx, eventObj);
            JS_FreeValue(ctx, global);
            SCE_LOG_ERROR("JSEngine: Failed to initialize _event object on first event - sessionId: {}", sessionId);
            return {ScriptResult::createError("Failed to create __eventData object for session: " + sessionId),
                    PayloadReading::Absent};
        }
        SCE_LOG_DEBUG("JSEngine: _event object successfully initialized for session: {}", sessionId);
    } else {
        eventDataProperty = JS_GetPropertyStr(ctx, global, "__eventData");
        if (!JS_IsObject(eventDataProperty)) {
            JS_FreeValue(ctx, eventDataProperty);
            JS_FreeValue(ctx, eventObj);
            JS_FreeValue(ctx, global);
            return {ScriptResult::createError("__eventData object not found for session: " + sessionId),
                    PayloadReading::Absent};
        }
    }

    if (event) {
        // Set event properties on the internal data object
        JS_SetPropertyStr(ctx, eventDataProperty, "name", JS_NewString(ctx, event->getName().c_str()));
        JS_SetPropertyStr(ctx, eventDataProperty, "type", JS_NewString(ctx, event->getType().c_str()));
        JS_SetPropertyStr(ctx, eventDataProperty, "sendid", JS_NewString(ctx, event->getSendId().c_str()));
        JS_SetPropertyStr(ctx, eventDataProperty, "origin", JS_NewString(ctx, event->getOrigin().c_str()));
        JS_SetPropertyStr(ctx, eventDataProperty, "origintype", JS_NewString(ctx, event->getOriginType().c_str()));
        JS_SetPropertyStr(ctx, eventDataProperty, "invokeid", JS_NewString(ctx, event->getInvokeId().c_str()));

        // Parse and set event data as JSON or DOM object for XML
        if (event->hasData()) {
            std::string dataStr = event->getDataAsString();
            SCE_LOG_DEBUG("JSEngine: Setting event data from string: '{}'", dataStr);
            ParsedEventData parsed = parseEventData(ctx, dataStr);
            if (!JS_IsException(parsed.value)) {
                JS_SetPropertyStr(ctx, eventDataProperty, "data", parsed.value);
                reading = parsed.reading;
                SCE_LOG_DEBUG("JSEngine: Successfully set event data");
            } else {
                JS_SetPropertyStr(ctx, eventDataProperty, "data", JS_UNDEFINED);
                SCE_LOG_ERROR("JSEngine: Failed to parse event data for eventDataProperty");
            }

            // W3C SCXML testing extension: _event.raw for manual inspection
            // Used by W3C test 178 (manual visual verification of duplicate param keys)
            // Contains raw event data string (pre-parsing) for debugging and test validation
            // Not part of §scxml-5.10 specification - implementation-specific extension
            JS_SetPropertyStr(ctx, eventDataProperty, "raw", JS_NewString(ctx, dataStr.c_str()));
            SCE_LOG_DEBUG("JSEngine: Set _event.raw = '{}'", dataStr);
        } else {
            SCE_LOG_DEBUG("JSEngine: Event has no data, setting _event.data to undefined");
            JS_SetPropertyStr(ctx, eventDataProperty, "data", JS_UNDEFINED);
            // W3C SCXML testing extension: Set _event.raw as empty string when no data
            JS_SetPropertyStr(ctx, eventDataProperty, "raw", JS_NewString(ctx, ""));
        }
    } else {
        // Reset all event properties to empty/undefined values
        const char *props[] = {"name", "type", "sendid", "origin", "origintype", "invokeid"};
        for (int i = 0; i < 6; i++) {
            JS_SetPropertyStr(ctx, eventDataProperty, props[i], JS_NewString(ctx, ""));
        }
        JS_SetPropertyStr(ctx, eventDataProperty, "data", JS_UNDEFINED);
        // W3C SCXML testing extension: Reset _event.raw to empty string on event clear
        JS_SetPropertyStr(ctx, eventDataProperty, "raw", JS_NewString(ctx, ""));
    }

    JS_FreeValue(ctx, eventDataProperty);
    JS_FreeValue(ctx, eventObj);
    JS_FreeValue(ctx, global);

    return {ScriptResult::createSuccess(), reading};
}

ScriptResult JSEngine::setupSystemVariablesInternal(const std::string &sessionId, const std::string &sessionName,
                                                    const std::vector<IOProcessorDescriptor> &ioProcessors) {
    SessionContext *session = getSession(sessionId);
    if (!session || !session->jsContext) {
        return ScriptResult::createError("Session not found: " + sessionId);
    }

    JSContext *ctx = session->jsContext;
    ::JSValue global = JS_GetGlobalObject(ctx);

    // Register _queueErrorEvent function for error.execution raising from read-only property setters
    ::JSValue queueErrorFunc = JS_NewCFunction(ctx, queueErrorEventWrapper, "_queueErrorEvent", 2);
    JS_SetPropertyStr(ctx, global, "_queueErrorEvent", queueErrorFunc);

    // §scxml-5.10: System variables must be read-only and raise error.execution on modification attempts
    // Use JavaScript code to define read-only properties with error handlers (tests 322, 326, 346)
    // §scxml-B-2-8: every system variable is a read-only ECMAScript variable — _sessionid
    // and _name are Strings, and _ioprocessors is an object with one named property per
    // supported Event I/O Processor.

    // The three values below all originate outside the engine: the session id
    // is embedder- or <invoke>-supplied, the session name is the <scxml>
    // element's 'name' attribute, and each processor location is declared by
    // the deployment. Building them as QuickJS values and handing the setup
    // script a single stash object keeps them out of the evaluated source, so
    // no quote in any of them can terminate a literal and be read as code.
    ::JSValue stash = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, stash, "sessionid", JS_NewString(ctx, sessionId.c_str()));
    JS_SetPropertyStr(ctx, stash, "name", JS_NewString(ctx, sessionName.c_str()));

    // §scxml-C-1-1 / §scxml-C-2-3: each entry carries a 'location' field naming
    // the address that reaches this session through that processor.
    ::JSValue ioProcessorsObj = JS_NewObject(ctx);
    for (const auto &processor : ioProcessors) {
        ::JSValue entry = JS_NewObject(ctx);
        JS_SetPropertyStr(ctx, entry, "location", JS_NewString(ctx, processor.location.c_str()));
        JS_SetPropertyStr(ctx, ioProcessorsObj, processor.name.c_str(), entry);
    }
    JS_SetPropertyStr(ctx, stash, "ioprocessors", ioProcessorsObj);
    JS_SetPropertyStr(ctx, global, SYSTEM_VARS_STASH, stash);

    std::string setupCode = R"(
        (function() {
            // Take the stashed values and drop the global, so the only route to
            // them from here on is the read-only accessors defined below.
            var __systemVars = this[')" +
                            std::string(SYSTEM_VARS_STASH) + R"('];
            delete this[')" +
                            std::string(SYSTEM_VARS_STASH) +
                            R"('];
            var sessionId = __systemVars.sessionid;

            // W3C SCXML 5.10: Define read-only _sessionid with error.execution on write
            Object.defineProperty(this, '_sessionid', {
                get: function() { return __systemVars.sessionid; },
                set: function(value) {
                    console.log('SCE Error: Attempt to assign to read-only system variable _sessionid');
                    _queueErrorEvent(sessionId, 'error.execution');
                    throw new Error('Cannot assign to read-only system variable _sessionid');
                },
                enumerable: true,
                configurable: false
            });

            // W3C SCXML 5.10: Define read-only _name with error.execution on write
            Object.defineProperty(this, '_name', {
                get: function() { return __systemVars.name; },
                set: function(value) {
                    console.log('SCE Error: Attempt to assign to read-only system variable _name');
                    _queueErrorEvent(sessionId, 'error.execution');
                    throw new Error('Cannot assign to read-only system variable _name');
                },
                enumerable: true,
                configurable: false
            });

            // W3C SCXML 5.10: Define read-only _ioprocessors with error.execution on write
            Object.defineProperty(this, '_ioprocessors', {
                get: function() { return __systemVars.ioprocessors; },
                set: function(value) {
                    console.log('SCE Error: Attempt to assign to read-only system variable _ioprocessors');
                    _queueErrorEvent(sessionId, 'error.execution');
                    throw new Error('Cannot assign to read-only system variable _ioprocessors');
                },
                enumerable: true,
                configurable: false
            });

            return true;
        })();
    )";

    ::JSValue result =
        JS_Eval(ctx, setupCode.c_str(), setupCode.length(), "<system_variables_setup>", JS_EVAL_TYPE_GLOBAL);
    if (JS_IsException(result)) {
        SCE_LOG_ERROR("JSEngine: Failed to setup read-only system variables");
        ::JSValue exception = JS_GetException(ctx);
        const char *errorStr = JS_ToCString(ctx, exception);
        if (errorStr) {
            SCE_LOG_ERROR("JSEngine: System variables setup error: {}", errorStr);
            JS_FreeCString(ctx, errorStr);
        }
        JS_FreeValue(ctx, exception);
        JS_FreeValue(ctx, result);
        JS_FreeValue(ctx, global);
        return ScriptResult::createError("Failed to setup read-only system variables");
    }
    JS_FreeValue(ctx, result);
    JS_FreeValue(ctx, global);

    // Store in session
    session->sessionName = sessionName;
    session->ioProcessors = ioProcessors;

    return ScriptResult::createSuccess();
}

// === Type Conversion ===

ScriptValue JSEngine::quickJSToJSValue(JSContext *ctx, JSValue qjsValue) {
    // SCXML W3C Compliance: Handle null and undefined distinctly
    if (JS_IsUndefined(qjsValue)) {
        return ScriptUndefined{};
    } else if (JS_IsNull(qjsValue)) {
        return ScriptNull{};
    } else if (JS_IsBool(qjsValue)) {
        return JS_ToBool(ctx, qjsValue) ? true : false;
    } else if (JS_IsNumber(qjsValue)) {
        // JavaScript numbers are always double (IEEE 754)
        double d;
        JS_ToFloat64(ctx, &d, qjsValue);

        SCE_LOG_TRACE("JSEngine::quickJSToJSValue - JS_IsNumber=true, extracted double={}", d);

        // SCXML W3C compliance: Return as int64_t if it's a whole number within range
        const double llong_min_d = static_cast<double>(LLONG_MIN);
        const double llong_max_d = static_cast<double>(LLONG_MAX);
        if (d == floor(d) && d >= llong_min_d && d <= llong_max_d) {
            int64_t int_result = static_cast<int64_t>(d);
            SCE_LOG_TRACE("JSEngine::quickJSToJSValue - Converting to int64_t={}", int_result);
            return int_result;
        }
        SCE_LOG_TRACE("JSEngine::quickJSToJSValue - Returning as double={}", d);
        return d;
    } else if (JS_IsString(qjsValue)) {
        const char *str = JS_ToCString(ctx, qjsValue);
        std::string result(str ? str : "");
        if (str) {
            JS_FreeCString(ctx, str);
        }
        return result;
    } else if (JS_IsArray(qjsValue)) {
        auto scriptArray = std::make_shared<ScriptArray>();
        JSValue lengthVal = JS_GetPropertyStr(ctx, qjsValue, "length");
        int64_t length = 0;
        JS_ToInt64(ctx, &length, lengthVal);
        JS_FreeValue(ctx, lengthVal);

        scriptArray->elements.reserve(static_cast<size_t>(length));
        for (int64_t i = 0; i < length; ++i) {
            JSValue element = JS_GetPropertyUint32(ctx, qjsValue, static_cast<uint32_t>(i));
            scriptArray->elements.push_back(quickJSToJSValue(ctx, element));
            JS_FreeValue(ctx, element);
        }
        return scriptArray;
    } else if (JS_IsObject(qjsValue) && !JS_IsFunction(ctx, qjsValue)) {
        auto scriptObject = std::make_shared<ScriptObject>();
        JSPropertyEnum *props = nullptr;
        uint32_t propCount = 0;

        if (JS_GetOwnPropertyNames(ctx, &props, &propCount, qjsValue, JS_GPN_STRING_MASK | JS_GPN_ENUM_ONLY) == 0) {
            for (uint32_t i = 0; i < propCount; ++i) {
                const char *key = JS_AtomToCString(ctx, props[i].atom);
                if (key) {
                    JSValue propValue = JS_GetProperty(ctx, qjsValue, props[i].atom);
                    scriptObject->properties[key] = quickJSToJSValue(ctx, propValue);
                    JS_FreeValue(ctx, propValue);
                    JS_FreeCString(ctx, key);
                }
                JS_FreeAtom(ctx, props[i].atom);
            }
            js_free(ctx, props);
        }
        return scriptObject;
    }

    // Default fallback for unknown types
    return ScriptUndefined{};
}

JSValue JSEngine::jsValueToQuickJS(JSContext *ctx, const ScriptValue &value) {
    return std::visit(
        [this, ctx](const auto &v) -> JSValue {
            using T = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<T, ScriptUndefined>) {
                return JS_UNDEFINED;
            } else if constexpr (std::is_same_v<T, ScriptNull>) {
                return JS_NULL;
            } else if constexpr (std::is_same_v<T, bool>) {
                return JS_NewBool(ctx, v);
            } else if constexpr (std::is_same_v<T, int64_t>) {
                return JS_NewInt64(ctx, v);
            } else if constexpr (std::is_same_v<T, double>) {
                return JS_NewFloat64(ctx, v);
            } else if constexpr (std::is_same_v<T, std::string>) {
                return JS_NewString(ctx, v.c_str());
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptArray>>) {
                JSValue jsArray = JS_NewArray(ctx);
                for (size_t i = 0; i < v->elements.size(); ++i) {
                    JSValue element = jsValueToQuickJS(ctx, v->elements[i]);
                    JS_SetPropertyUint32(ctx, jsArray, static_cast<uint32_t>(i), element);
                }
                return jsArray;
            } else if constexpr (std::is_same_v<T, std::shared_ptr<ScriptObject>>) {
                JSValue jsObject = JS_NewObject(ctx);
                for (const auto &[key, val] : v->properties) {
                    JSValue propValue = jsValueToQuickJS(ctx, val);
                    JS_SetPropertyStr(ctx, jsObject, key.c_str(), propValue);
                }
                return jsObject;
            } else {
                return JS_UNDEFINED;
            }
        },
        value);
}

// === Error Handling ===

ScriptResult JSEngine::createErrorFromException(JSContext *ctx) {
    ::JSValue exception = JS_GetException(ctx);
    if (JS_IsNull(exception)) {
        return ScriptResult::createError("JavaScript error: Exception is null");
    }
    const char *errorStr = JS_ToCString(ctx, exception);
    std::string errorMessage;
    if (errorStr) {
        errorMessage = std::string("JavaScript error: ") + errorStr;
        JS_FreeCString(ctx, errorStr);
    } else {
        errorMessage = "Unknown JavaScript error - could not get error string";
    }
    // Try to get stack trace
    ::JSValue stack = JS_GetPropertyStr(ctx, exception, "stack");
    if (!JS_IsUndefined(stack)) {
        const char *stackStr = JS_ToCString(ctx, stack);
        if (stackStr) {
            errorMessage += "\nStack: " + std::string(stackStr);
            JS_FreeCString(ctx, stackStr);
        }
    }
    JS_FreeValue(ctx, stack);
    JS_FreeValue(ctx, exception);
    return ScriptResult::createError(errorMessage);
}
}  // namespace SCE
