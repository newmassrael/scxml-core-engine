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

#include "common/EventDataHelper.h"
#include "scripting/IScriptEngine.h"
#include <functional>
#include <optional>
#include <sstream>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace SCE {

/**
 * @brief Single Source of Truth for donedata evaluation (W3C SCXML 5.5, 5.7)
 *
 * Shared by Interpreter engine and Static Code Generator to ensure Zero Duplication.
 *
 * W3C SCXML 5.5: "In cases where the SCXML Processor generates a 'done' event upon
 * entry into the final state, it MUST evaluate the donedata elements param or content
 * children and place the resulting data in the _event.data field."
 *
 * W3C SCXML 5.7: "If the processor cannot create an ECMAScript object for some reason,
 * the processor must place the error error.execution in the internal event queue."
 */
class DoneDataHelper {
public:
    /**
     * @brief Evaluate donedata content expression to _event.data value
     *
     * W3C SCXML 5.5: <content> sets the entire _event.data value
     *
     * @param jsEngine JSEngine instance for expression evaluation
     * @param sessionId Session ID for JSEngine context
     * @param contentExpr Content expression to evaluate
     * @param outEventData Output value (may be string, number, object)
     * @param onError Callback for error.execution events (optional)
     * @return true if evaluation succeeded or no content, false on critical error
     *
     * Usage:
     * ```cpp
     * // Interpreter engine (StateMachine.cpp)
     * std::string eventData;
     * if (!DoneDataHelper::evaluateContent(jsEngine, sessionId, content, eventData,
     *         [this](const std::string& msg) { eventRaiser_->raiseEvent("error.execution", msg); })) {
     *     return false;
     * }
     *
     * // Static Code Generator (generated code)
     * std::string eventData;
     * DoneDataHelper::evaluateContent(jsEngine, sessionId, "1+1", eventData,
     *     [&engine](const std::string& msg) { engine.raise(Event::Error_execution); });
     * ```
     */
    /**
     * @brief Emit a literal `<content>` body as _event.data (W3C SCXML 5.5)
     *
     * W3C SCXML 5.5 specifies that when the `<content>` element has no
     * `expr` attribute, "the children are used as the content value".
     * No evaluation happens — the inline text is the value itself.
     *
     * This path is script-engine-free: it is linkable against `sce_base`
     * and is the SSoT consumed by both the interpreter engine and the
     * static code generator's literal branch (Zero Duplication).
     *
     * @param literal Inline text content from `<content>literal</content>`
     * @param outEventData _event.data output as a canonical JSON scalar so the
     *                     wire path (mesh §9.6.2 wire-18) and the local fallback
     *                     parse in `EventRaiserImpl::raiseEventWithPriority` —
     *                     which hydrates `typedData` via
     *                     `EventDataHelper::jsonStringToScriptValue` when typed
     *                     data is absent — see the same bytes they can round-trip.
     * @param outTypedData Optional typed pipeline — a `ScriptValue(string)`
     *                     is stored so downstream consumers that inspect
     *                     `typedData` observe a string-kind value.
     */
    static void emitContentLiteral(const std::string &literal, std::string &outEventData,
                                   std::optional<ScriptValue> *outTypedData = nullptr) {
        ScriptValue value{literal};
        outEventData = EventDataHelper::scriptValueToJsonString(value);
        if (outTypedData) {
            *outTypedData = std::move(value);
        }
    }

    static bool evaluateContent(IScriptEngine &jsEngine, const std::string &sessionId, const std::string &contentExpr,
                                std::string &outEventData, std::function<void(const std::string &)> onError = nullptr,
                                std::optional<ScriptValue> *outTypedData = nullptr) {
        if (contentExpr.empty()) {
            outEventData = "";
            return true;
        }

        // W3C SCXML 5.5: Evaluate content as expression
        auto future = jsEngine.evaluateExpression(sessionId, contentExpr);
        auto result = future.get();

        if (result.isSuccess()) {
            const auto &value = result.getInternalValue();
            // W3C SCXML 5.5 + B.2: ship canonical JSON so the wire path
            // (mesh §9.6.2 wire-18) and the local fallback parse in
            // `EventRaiserImpl::raiseEventWithPriority` can round-trip
            // through `EventDataHelper::jsonStringToScriptValue`.
            // `scriptValueToJsonString` handles ScriptArray/ScriptObject
            // recursively, quoting strings and preserving numeric/bool
            // types — no `null`-then-falls-back-to-source-expression hack.
            outEventData = EventDataHelper::scriptValueToJsonString(value);

            // W3C SCXML 5.5: Preserve typed data for engine-agnostic pipeline
            if (outTypedData) {
                *outTypedData = value;
            }
            return true;
        }

        // W3C SCXML 5.10: Raise error.execution event for expression evaluation failure
        if (onError) {
            onError(result.getErrorMessage());
        }

        // W3C SCXML 5.5: Return empty data (not literal content) when evaluation fails
        outEventData = "";
        return true;
    }

    /**
     * @brief Evaluate donedata params to JSON object
     *
     * W3C SCXML 5.5: <param> elements create object with name:value pairs
     * W3C SCXML 5.7: Empty param location raises error.execution
     *
     * @param jsEngine JSEngine instance for expression evaluation
     * @param sessionId Session ID for JSEngine context
     * @param params Vector of (name, expr) pairs
     * @param outEventData Output JSON string (e.g., {"Var1":1})
     * @param onError Callback for error.execution events (optional)
     * @return false if structural error prevents done.state, true otherwise
     *
     * Error Handling:
     * - Structural error (empty location): returns false to prevent done.state event
     * - Runtime error (invalid expr): ignores failed param, continues with others
     *
     * Usage:
     * ```cpp
     * // Interpreter engine
     * std::vector<std::pair<std::string, std::string>> params = {{"x", "1"}, {"y", "Var1"}};
     * std::string eventData;
     * if (!DoneDataHelper::evaluateParams(jsEngine, sessionId, params, eventData,
     *         [this](const std::string& msg) { eventRaiser_->raiseEvent("error.execution", msg); })) {
     *     return false;  // Structural error, skip done.state
     * }
     *
     * // Static Code Generator
     * std::vector<std::pair<std::string, std::string>> params = {{"Var1", "1"}};
     * std::string eventData;
     * DoneDataHelper::evaluateParams(jsEngine, sessionId, params, eventData,
     *     [&engine](const std::string& msg) { engine.raise(Event::Error_execution); });
     * ```
     */
    static bool evaluateParams(IScriptEngine &jsEngine, const std::string &sessionId,
                               const std::vector<std::pair<std::string, std::string>> &params,
                               std::string &outEventData, std::function<void(const std::string &)> onError = nullptr,
                               std::optional<ScriptValue> *outTypedData = nullptr) {
        if (params.empty()) {
            outEventData = "";
            return true;
        }

        // W3C SCXML 5.5: <param> elements create an object with name:value pairs
        std::ostringstream jsonBuilder;
        jsonBuilder << "{";

        // W3C SCXML 5.5: Build ScriptObject for engine-agnostic typed data pipeline
        std::shared_ptr<ScriptObject> typedObj;
        if (outTypedData) {
            typedObj = std::make_shared<ScriptObject>();
        }

        bool first = true;
        for (const auto &param : params) {
            const std::string &paramName = param.first;
            const std::string &paramExpr = param.second;

            // W3C SCXML 5.7: Empty location is invalid (structural error)
            // Must raise error.execution and prevent done.state event generation
            if (paramExpr.empty()) {
                if (onError) {
                    onError("Empty param location or expression: " + paramName);
                }
                // W3C SCXML 5.7: Return false to skip done.state event generation
                return false;
            }

            // Evaluate param expression
            auto future = jsEngine.evaluateExpression(sessionId, paramExpr);
            auto result = future.get();

            if (result.isSuccess()) {
                // W3C SCXML 5.7: Successfully evaluated param
                if (!first) {
                    jsonBuilder << ",";
                }
                first = false;

                const auto &value = result.getInternalValue();
                // W3C SCXML 5.5 + B.2: Use canonical JSON serializer so nested
                // ScriptObject/ScriptArray param values round-trip through the
                // wire / local JSON-fallback paths identically to typedData.
                jsonBuilder << "\"" << escapeJsonString(paramName) << "\":"
                            << EventDataHelper::scriptValueToJsonString(value);

                // Preserve typed value for engine-agnostic pipeline
                if (typedObj) {
                    typedObj->properties[paramName] = value;
                }
            } else {
                // W3C SCXML 5.7: Invalid location or expression (runtime error)
                // Must raise error.execution and ignore this param, but continue with others
                if (onError) {
                    onError("Invalid param location or expression: " + paramName + " = " + paramExpr);
                }
                // Continue to next param without adding this one
            }
        }

        jsonBuilder << "}";
        outEventData = jsonBuilder.str();

        // W3C SCXML 5.5: Store typed data for done.state event
        if (outTypedData && typedObj) {
            *outTypedData = typedObj;
        }

        return true;
    }

    /**
     * @brief Escape special characters for JSON string
     *
     * @param str Input string
     * @return Escaped JSON string (without surrounding quotes)
     */
    static std::string escapeJsonString(const std::string &str) {
        std::ostringstream escaped;
        for (char c : str) {
            switch (c) {
            case '"':
                escaped << "\\\"";
                break;
            case '\\':
                escaped << "\\\\";
                break;
            case '\n':
                escaped << "\\n";
                break;
            case '\r':
                escaped << "\\r";
                break;
            case '\t':
                escaped << "\\t";
                break;
            case '\b':
                escaped << "\\b";
                break;
            case '\f':
                escaped << "\\f";
                break;
            default:
                escaped << c;
                break;
            }
        }
        return escaped.str();
    }

};

}  // namespace SCE
