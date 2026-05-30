// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include <stdexcept>
#include <string>

namespace SCE::Validation {

/**
 * @brief Validates foreach loop attributes according to §scxml-4.6 specification
 *
 * §scxml-4.6 Requirements:
 * - 'array' attribute is required and must not be empty
 * - 'item' attribute is required and must not be empty
 * - 'index' attribute is optional
 *
 * @param array The array expression to iterate over
 * @param item The variable name for each iteration item
 * @param errorMessage Output parameter for error description if validation fails
 * @return true if validation passes, false otherwise
 *
 * Platform Safety: Returns bool instead of throwing to ensure WASM pthread exception safety
 */
inline bool validateForeachAttributes(const std::string &array, const std::string &item, std::string &errorMessage) {
    // §scxml-4.6: array attribute is required
    if (array.empty()) {
        errorMessage = "Foreach array attribute is missing or empty";
        return false;
    }

    // §scxml-4.6: item attribute is required
    if (item.empty()) {
        errorMessage = "Foreach item attribute is missing or empty";
        return false;
    }

    return true;
}

}  // namespace SCE::Validation
