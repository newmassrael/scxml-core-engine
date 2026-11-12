// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SCXMLTypes.h"
#include "model/IDataModelItem.h"
#include "model/SCXMLModel.h"
#include "runtime/IEventRaiser.h"
#include <memory>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace SCE {

class IScriptEngine;

/**
 * @brief §scxml-5.3: Data model initialization with binding mode support
 *
 * Handles all data model variable lifecycle:
 * - Collecting data items from document (top-level + state-level)
 * - Initializing variables from expr/src/content/undefined
 * - Early binding (all values at document load) vs late binding (values on state entry)
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Uses DataModelInitHelper, BindingHelper, FileLoadingHelper
 * - Single Responsibility: Data model initialization only
 * - No engine mixing: Works with IScriptEngine interface for expression evaluation
 */
class DataModelInitializer {
public:
    /**
     * @brief §scxml-5.3: Data item descriptor with containing state
     */
    struct DataItemInfo {
        std::string stateId;  // Empty for top-level datamodel, state ID for state-level data
        std::shared_ptr<IDataModelItem> dataItem;
    };

    /**
     * @brief The values the session that invoked this one handed it, name and value, in the order handed
     *
     * A name handed more than once means the last value handed under it.
     */
    using HandedValues = std::vector<std::pair<std::string, ScriptValue>>;

    /**
     * @brief Construct with required dependencies
     *
     * @param model SCXML model for data item lookups
     * @param sessionId Script session ID for variable operations
     * @param scriptEngine Script engine for expression evaluation and variable management
     */
    DataModelInitializer(std::shared_ptr<SCXMLModel> model, const std::string &sessionId, IScriptEngine &scriptEngine);

    /**
     * @brief Set EventRaiser for error.execution event reporting
     *
     * May be set after construction (EventRaiser lifecycle may differ).
     *
     * @param eventRaiser Shared pointer to event raiser
     */
    void setEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser);

    /**
     * @brief Collect all data items from SCXML document
     *
     * Gathers top-level datamodel items and state-level data items.
     *
     * @return Vector of all data items with their containing state IDs
     */
    std::vector<DataItemInfo> collectAllDataItems() const;

    /**
     * @brief §scxml-5.3: Initialize a single data item
     *
     * Handles expr (expression evaluation), src (file loading),
     * content (inline content), and undefined (no value).
     *
     * @param item Data item to initialize
     * @param assignValue If false, create variable as undefined (late binding deferred)
     */
    void initializeDataItem(const std::shared_ptr<IDataModelItem> &item, bool assignValue);

    /**
     * @brief §scxml-5.3: Initialize all data items at document load time
     *
     * Uses BindingHelper to determine early vs late binding strategy.
     *
     * A top-level item whose id `handedValues` names is declared with the
     * value handed for it in place of its own, whose expr, src or content is
     * then not evaluated; a value the session cannot hold is that item's
     * initialization error. A state's item never takes a handed value, and a
     * handed name that is the id of no top-level item is not declared.
     *
     * @param binding Binding mode string ("early" or "late")
     * @param handedValues The values the invoking session handed this one;
     *        empty for a session nothing invoked
     */
    void initializeAllDataItems(const std::string &binding, const HandedValues &handedValues = {});

    /**
     * @brief §scxml-5.3: Initialize state data on first entry (late binding)
     *
     * Called from enterState() to assign values when state is entered for the first time.
     * Uses BindingHelper to determine if values should be assigned.
     *
     * @param stateId State being entered
     */
    void initializeStateDataOnEntry(const std::string &stateId);

private:
    /**
     * @brief Declare the top-level item `id` with the value handed for it
     *
     * Raises error.execution and leaves the item undeclared when the session
     * cannot hold the value, as an item whose own value failed is left.
     */
    void declareHandedValue(const std::string &id, const ScriptValue &value);

    IScriptEngine &scriptEngine_;
    std::shared_ptr<SCXMLModel> model_;
    std::string sessionId_;
    std::shared_ptr<IEventRaiser> eventRaiser_;

    // §scxml-5.3: Track which states have initialized their data (for late binding)
    std::set<std::string> initializedStates_;
};

}  // namespace SCE
