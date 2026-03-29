#pragma once

#include "model/IDataModelItem.h"
#include "model/SCXMLModel.h"
#include "runtime/IEventRaiser.h"
#include <memory>
#include <set>
#include <string>
#include <vector>

namespace SCE {

/**
 * @brief W3C SCXML 5.3: Data model initialization with binding mode support
 *
 * Handles all data model variable lifecycle:
 * - Collecting data items from document (top-level + state-level)
 * - Initializing variables from expr/src/content/undefined
 * - Early binding (all values at document load) vs late binding (values on state entry)
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Uses DataModelInitHelper, BindingHelper, FileLoadingHelper
 * - Single Responsibility: Data model initialization only
 * - No engine mixing: Works with JSEngine singleton for expression evaluation
 */
class DataModelInitializer {
public:
    /**
     * @brief W3C SCXML 5.3: Data item descriptor with containing state
     */
    struct DataItemInfo {
        std::string stateId;  // Empty for top-level datamodel, state ID for state-level data
        std::shared_ptr<IDataModelItem> dataItem;
    };

    /**
     * @brief Construct with required dependencies
     *
     * @param model SCXML model for data item lookups
     * @param sessionId JavaScript session ID for variable operations
     */
    DataModelInitializer(std::shared_ptr<SCXMLModel> model, const std::string &sessionId);

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
     * @brief W3C SCXML 5.3: Initialize a single data item
     *
     * Handles expr (expression evaluation), src (file loading),
     * content (inline content), and undefined (no value).
     *
     * @param item Data item to initialize
     * @param assignValue If false, create variable as undefined (late binding deferred)
     */
    void initializeDataItem(const std::shared_ptr<IDataModelItem> &item, bool assignValue);

    /**
     * @brief W3C SCXML 5.3: Initialize all data items at document load time
     *
     * Uses BindingHelper to determine early vs late binding strategy.
     *
     * @param binding Binding mode string ("early" or "late")
     */
    void initializeAllDataItems(const std::string &binding);

    /**
     * @brief W3C SCXML 5.3: Initialize state data on first entry (late binding)
     *
     * Called from enterState() to assign values when state is entered for the first time.
     * Uses BindingHelper to determine if values should be assigned.
     *
     * @param stateId State being entered
     */
    void initializeStateDataOnEntry(const std::string &stateId);

private:
    std::shared_ptr<SCXMLModel> model_;
    std::string sessionId_;
    std::shared_ptr<IEventRaiser> eventRaiser_;

    // W3C SCXML 5.3: Track which states have initialized their data (for late binding)
    std::set<std::string> initializedStates_;
};

}  // namespace SCE
