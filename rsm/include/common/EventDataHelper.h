// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <map>
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
};

}  // namespace SCE
