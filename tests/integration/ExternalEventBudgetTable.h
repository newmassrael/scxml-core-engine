// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The numbers of the external-event budget (ARCHITECTURE.md, "External-Event
// Budget"), read from tests/external_event_budget/budget_outcomes.json, the
// table every engine's test of the document
// tests/integration/external_chain_is_bounded.scxml is held to. The C++
// Interpreter driver and its AOT twin read it through this one reader, so a
// budget or a counter is stated in the table and nowhere else.
//
// A key the table does not have is an exception that names it, which the test
// that asked reports as a failure: there is no default to fall back to, because
// a test that supplied its own number for a missing key would pass against a
// table that lost it.

#pragma once

#include <cstdint>
#include <fstream>
#include <initializer_list>
#include <limits>
#include <nlohmann/json.hpp>
#include <stdexcept>
#include <string>

#ifndef SCE_EXTERNAL_EVENT_BUDGET_TABLE
#error "SCE_EXTERNAL_EVENT_BUDGET_TABLE must name tests/external_event_budget/budget_outcomes.json"
#endif

namespace SCE::Tests {

/// The parsed table, read once per process.
inline const nlohmann::json &externalEventBudgetTable() {
    static const nlohmann::json table = [] {
        std::ifstream file(SCE_EXTERNAL_EVENT_BUDGET_TABLE);
        if (!file.is_open()) {
            throw std::runtime_error(std::string("cannot read the external-event budget table ") +
                                     SCE_EXTERNAL_EVENT_BUDGET_TABLE);
        }
        return nlohmann::json::parse(file);
    }();
    return table;
}

/// The dotted spelling of @p path, for a message that names the key.
inline std::string externalEventBudgetKeyName(std::initializer_list<const char *> path) {
    std::string name;
    for (const char *key : path) {
        if (!name.empty()) {
            name += ".";
        }
        name += key;
    }
    return name;
}

/// The value at @p path (`{"exact", "budget"}`), or an exception naming the
/// dotted path of the key the table does not have.
inline const nlohmann::json &externalEventBudgetValue(std::initializer_list<const char *> path) {
    const nlohmann::json *at = &externalEventBudgetTable();
    for (const char *key : path) {
        if (!at->is_object() || !at->contains(key)) {
            throw std::runtime_error("the external-event budget table has no key `" + externalEventBudgetKeyName(path) +
                                     "`");
        }
        at = &at->at(key);
    }
    return *at;
}

/// An integer of the table, signed because `host.refused` lists negative budgets.
inline int64_t externalEventBudgetInt(std::initializer_list<const char *> path) {
    const nlohmann::json &value = externalEventBudgetValue(path);
    if (!value.is_number_integer()) {
        throw std::runtime_error("the external-event budget table key `" + externalEventBudgetKeyName(path) +
                                 "` is not an integer");
    }
    return value.get<int64_t>();
}

/// A budget or a counter the engine takes or reports as an unsigned 32-bit value.
inline uint32_t externalEventBudgetCount(std::initializer_list<const char *> path) {
    const int64_t value = externalEventBudgetInt(path);
    if (value < 0 || value > static_cast<int64_t>(std::numeric_limits<uint32_t>::max())) {
        throw std::runtime_error("the external-event budget table key `" + externalEventBudgetKeyName(path) +
                                 "` does not fit an unsigned count");
    }
    return static_cast<uint32_t>(value);
}

}  // namespace SCE::Tests
