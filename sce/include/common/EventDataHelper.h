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

#include "SCXMLTypes.h"
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief Helper functions for W3C SCXML event data construction
 *
 * Single Source of Truth for event data JSON building shared between:
 * - Interpreter engine (InternalEventTarget::buildEventData)
 * - AOT engine (StaticCodeGenerator - generated send param code)
 *
 * W3C SCXML References:
 * - 5.10: Event data structure (_event.data)
 * - 6.2: Send element with param elements
 * - Test 176: Event data from send params
 * - Test 178: Duplicate param names (multiple values)
 */
class EventDataHelper {
public:
    /**
     * @brief Build JSON string from evaluated params
     *
     * Single Source of Truth for event data JSON construction.
     * Used by both Interpreter and AOT engines to ensure consistent behavior.
     *
     * W3C SCXML 5.10: Construct event data from params.
     * W3C Test 178: Support duplicate param names - multiple values stored as array.
     *
     * @param params Map of param names to values (vector supports duplicates)
     * @return JSON string representation (compact format)
     *
     * @example
     * // Single value
     * params["name"] = {"value"};
     * buildJsonFromParams(params) → {"name":"value"}
     *
     * // Duplicate param names (W3C Test 178)
     * params["data"] = {"first", "second"};
     * buildJsonFromParams(params) → {"data":["first","second"]}
     */
    static std::string buildJsonFromParams(const std::map<std::string, std::vector<std::string>> &params);

    /**
     * @brief Build ScriptValue object from typed params
     *
     * Engine-agnostic alternative to buildJsonFromParams. Creates a ScriptObject
     * directly from typed param values, avoiding JSON serialization/deserialization.
     *
     * @param typedParams Map of param names to ScriptValue values
     * @return ScriptValue containing a ScriptObject with param properties
     */
    static ScriptValue buildScriptValueFromParams(const std::map<std::string, ScriptValue> &typedParams);

    /**
     * @brief Build type-preserving JSON string from typed params
     *
     * W3C SCXML 5.10: Constructs JSON that preserves all ScriptValue types:
     * - Primitives: int/double remain unquoted, bool as true/false, string as quoted
     * - Structures: ScriptArray as JSON array, ScriptObject as JSON object (recursive)
     * - Nullish: ScriptNull and ScriptUndefined both map to JSON null
     *
     * Used for scheduler event data and child-to-parent communication where typedData cannot be
     * passed directly as ScriptValue (e.g., PullScheduler stores string-only event data).
     *
     * @note ScriptUndefined is serialized as JSON null (JSON has no undefined concept).
     *       Roundtrip via jsonStringToScriptValue will recover it as ScriptNull, not ScriptUndefined.
     *
     * @param typedParams Map of param names to ScriptValue values
     * @return JSON string with type-preserving values (e.g., {"aParam":1,"arr":[1,2],"obj":{"k":"v"}})
     */
    static std::string buildJsonFromTypedParams(const std::map<std::string, ScriptValue> &typedParams);

    /**
     * @brief Build event data JSON, preferring type-preserving form when typed params exist.
     *
     * Single decision point shared by every send-action call site (parent send,
     * scheduler, immediate send) and by mesh transports (shm/someip/zenoh) where
     * `data` is the only channel and must encode int/double/bool natively for the
     * receiver to compare values without coercion.
     *
     * @param stringParams Map of param names to stringified values
     * @param typedParams  Map of param names to typed ScriptValues (preferred)
     * @return Type-preserving JSON when typedParams non-empty; string-only JSON otherwise
     */
    static std::string buildEventDataJson(const std::map<std::string, std::vector<std::string>> &stringParams,
                                          const std::map<std::string, ScriptValue> &typedParams);

    /**
     * @brief Convert ScriptValue to JSON string
     *
     * W3C SCXML B.2: Recursive ScriptValue -> JSON string conversion.
     * Symmetric counterpart to jsonStringToScriptValue.
     * Handles all ScriptValue types including nested ScriptArray/ScriptObject.
     *
     * Unlike jsonStringToScriptValue (which returns optional due to parse failure),
     * this always succeeds because every ScriptValue variant is JSON-representable.
     *
     * @note ScriptUndefined maps to JSON null; roundtrip recovers as ScriptNull.
     *
     * @param value ScriptValue to serialize
     * @return JSON string representation (compact format)
     */
    static std::string scriptValueToJsonString(const ScriptValue &value);

    /**
     * @brief Parse JSON string into ScriptValue (engine-agnostic)
     *
     * W3C SCXML B.2: Converts JSON event data to typed ScriptValue at the pipeline level,
     * avoiding engine-specific JSON parsing workarounds.
     *
     * Used when typedParams are unavailable (e.g., content elements, HTTP responses).
     *
     * @param jsonStr JSON string to parse
     * @return ScriptValue if valid JSON, std::nullopt otherwise
     */
    static std::optional<ScriptValue> jsonStringToScriptValue(const std::string &jsonStr);
};

}  // namespace SCE
