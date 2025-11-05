// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
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
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include <set>
#include <string>
#include <vector>

namespace RSM {

/**
 * @brief Helper for W3C SCXML 6.3.2 datamodel validation
 *
 * Single Source of Truth for child datamodel variable validation.
 * Used by both Interpreter (InvokeExecutor.cpp) and AOT engines (generated code).
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Shared validation logic across engines
 * - Helper Function Pattern: Follows SendHelper, ForeachHelper design
 * - W3C SCXML 6.3.2: "If the name of a param element or the key of a namelist item
 *   do not match the name of a data element in the invoked process, the Processor
 *   MUST NOT add the value to the invoked session's data model"
 */
class DatamodelValidationHelper {
public:
    /**
     * @brief Build set of child datamodel variable names
     *
     * W3C SCXML 6.3.2: Extract variable names from child's datamodel for validation.
     * Used to validate namelist and param bindings.
     *
     * @param varNames Vector of variable names from child's datamodel
     * @return std::set for O(log n) lookup during validation
     */
    static std::set<std::string> buildChildDatamodelSet(const std::vector<std::string> &varNames) {
        return std::set<std::string>(varNames.begin(), varNames.end());
    }

    /**
     * @brief Check if variable is declared in child's datamodel
     *
     * W3C SCXML 6.3.2: Validate that variable exists in child before binding.
     * Prevents creating undeclared variables in child session.
     *
     * ARCHITECTURE.md Zero Duplication:
     * - Matches InvokeExecutor.cpp:337-340 logic
     * - Shared by Interpreter and AOT engines
     *
     * @param varName Variable name to validate
     * @param childDatamodel Set of declared variable names in child
     * @return true if variable is declared in child, false otherwise
     */
    static bool isVariableDeclaredInChild(const std::string &varName, const std::set<std::string> &childDatamodel) {
        return childDatamodel.find(varName) != childDatamodel.end();
    }
};

}  // namespace RSM
