// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5, §11.8;
// ETSI ES 201 873-1 §8.2): the value a suite gives each, which every component
// that declares a module parameter of that id reads into its variable before
// its machine starts.
//
// A value is held as the one JSON value model a saved state is made of
// (sce/include/saved/SavedJson.h). The generated suite writes each default
// through the writer of its declared type, and the generated component reads it
// back through the reader of the type it declares, which are the reads a
// restored variable is held by: a value of another type, or past its bound, is
// refused by name rather than converted. The runtime knows the type of no
// module parameter, so it is written once for every suite.

#pragma once

#include <map>
#include <stdexcept>
#include <string>
#include <utility>

#include "saved/SavedJson.h"

namespace SCE::Test {

/// A module parameter's value: the saved form of a value of its declared type.
using ModuleParameterValue = SCE::Saved::JsonValue;

/**
 * @brief The module parameters a run hands its components, by id
 *
 * Generated from a `test-suite` with each parameter at its default, and given
 * other values by the caller of a run where it overrides one (-1 §8.2.1).
 */
class ModuleParameters {
public:
    /// Gives the module parameter @p id the value @p value, in place of any it had.
    void set(const std::string &id, ModuleParameterValue value) {
        values_.insert_or_assign(id, std::move(value));
    }

    /// Whether the module parameter @p id has a value.
    bool has(const std::string &id) const {
        return values_.count(id) != 0;
    }

    /// The value of the module parameter @p id; refused, naming it, when it has
    /// none, since no value is made up in its place.
    const ModuleParameterValue &of(const std::string &id) const {
        auto found = values_.find(id);
        if (found == values_.end()) {
            throw std::out_of_range("no value is given for the module parameter '" + id + "'");
        }
        return found->second;
    }

    /// Every module parameter that has a value, by id.
    const std::map<std::string, ModuleParameterValue> &values() const {
        return values_;
    }

private:
    std::map<std::string, ModuleParameterValue> values_;
};

}  // namespace SCE::Test
