// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/DataModelInitializer.h"

#include "common/DataModelInitHelper.h"
#include "common/FileLoadingHelper.h"
#include "core/LogMacros.h"
#include "runtime/BindingHelper.h"
#include "runtime/DataContentHelpers.h"
#include "scripting/IScriptEngine.h"
#include "scripting/ScriptResultUtils.h"
#include "scripting/SessionRegistry.h"
#include <string>

namespace SCE {

DataModelInitializer::DataModelInitializer(std::shared_ptr<SCXMLModel> model, const std::string &sessionId,
                                           IScriptEngine &scriptEngine)
    : scriptEngine_(scriptEngine), model_(std::move(model)), sessionId_(sessionId) {}

void DataModelInitializer::setEventRaiser(std::shared_ptr<IEventRaiser> eventRaiser) {
    eventRaiser_ = std::move(eventRaiser);
}

std::vector<DataModelInitializer::DataItemInfo> DataModelInitializer::collectAllDataItems() const {
    std::vector<DataItemInfo> allDataItems;

    if (!model_) {
        return allDataItems;
    }

    // Collect top-level datamodel items
    const auto &topLevelItems = model_->getDataModelItems();
    for (const auto &item : topLevelItems) {
        allDataItems.push_back(DataItemInfo{"", item});  // Empty stateId for top-level
    }
    SCE_LOG_DEBUG("DataModelInitializer: Collected {} top-level data items", topLevelItems.size());

    // Collect state-level data items from all states
    const auto &allStates = model_->getAllStates();
    for (const auto &state : allStates) {
        if (!state) {
            continue;
        }

        const auto &stateDataItems = state->getDataItems();
        if (!stateDataItems.empty()) {
            for (const auto &item : stateDataItems) {
                allDataItems.push_back(DataItemInfo{state->getId(), item});
            }
            SCE_LOG_DEBUG("DataModelInitializer: Collected {} data items from state '{}'", stateDataItems.size(),
                      state->getId());
        }
    }

    SCE_LOG_INFO("DataModelInitializer: Total data items collected: {} (for global scope initialization)",
             allDataItems.size());
    return allDataItems;
}

void DataModelInitializer::initializeDataItem(const std::shared_ptr<IDataModelItem> &item, bool assignValue) {
    if (!item) {
        return;
    }

    std::string id = item->getId();
    std::string expr = item->getExpr();
    std::string src = item->getSrc();
    std::string content = item->getContent();

    // §scxml-6.4: Check if variable was pre-initialized (e.g., by invoke namelist/param)
    // Skip this check for late binding value assignment (assignValue=true with late binding)
    // because late binding creates variables as undefined first, then assigns values on state entry
    bool isLateBindingAssignment = assignValue && model_ && (model_->getBinding() == "late");

    if (!isLateBindingAssignment && scriptEngine_.isVariablePreInitialized(sessionId_, id)) {
        SCE_LOG_INFO("DataModelInitializer: Skipping initialization for '{}' - pre-initialized by invoke data", id);
        return;
    }

    // §scxml-B-2-2: Late binding creates variables with undefined at init, assigns values on state entry
    if (!assignValue) {
        // Create variable with undefined value (both early and late binding)
        auto setVarFuture = scriptEngine_.setVariable(sessionId_, id, ScriptValue{});
        auto setResult = setVarFuture.get();

        if (!SCE::ScriptResultUtils::isSuccess(setResult)) {
            SCE_LOG_ERROR("DataModelInitializer: Failed to create unbound variable '{}': {}", id,
                      setResult.getErrorMessage());
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution",
                                         "Failed to create variable '" + id + "': " + setResult.getErrorMessage());
            }
            return;
        }

        SCE_LOG_DEBUG("DataModelInitializer: Created unbound variable '{}' (value assignment deferred for late binding)",
                  id);
        return;
    }

    // Early binding or late binding value assignment: Evaluate and assign
    if (!expr.empty()) {
        // ARCHITECTURE.MD: Zero Duplication - Use DataModelInitHelper (shared with AOT engine)
        // §scxml-B-2: For function expressions, use direct JavaScript assignment to preserve function type
        // Test 453: ECMAScript function literals must be stored as functions, not converted to C++
        bool isFunctionExpression = DataModelInitHelper::isFunctionExpression(expr);

        if (isFunctionExpression) {
            // Use direct JavaScript assignment to avoid function → C++ → function conversion loss
            std::string assignmentScript = id + " = " + expr;
            auto scriptFuture = scriptEngine_.executeScript(sessionId_, assignmentScript);
            auto scriptResult = scriptFuture.get();

            if (!SCE::ScriptResultUtils::isSuccess(scriptResult)) {
                SCE_LOG_ERROR(
                    "DataModelInitializer: Failed to assign function expression '{}' to variable '{}': {}", expr, id,
                    scriptResult.getErrorMessage());
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", "Failed to assign function expression for '" + id +
                                                                    "': " + scriptResult.getErrorMessage());
                }
                return;
            }
            SCE_LOG_DEBUG("DataModelInitializer: Initialized function variable '{}' from expression '{}'", id, expr);
        } else {
            // ARCHITECTURE.MD: Zero Duplication - Use DataModelInitHelper (shared with AOT engine)
            // §scxml-5.3: Use initializeVariableFromExpr for expr attribute
            // Test 277: expr evaluation failure must raise error.execution (no fallback)
            bool success = DataModelInitHelper::initializeVariableFromExpr(
                scriptEngine_, sessionId_, id, expr, [this](const std::string &msg) {
                    // §scxml-5.3: Raise error.execution on initialization failure
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution", msg);
                    }
                    SCE_LOG_ERROR("DataModelInitializer: {}", msg);
                });

            if (success) {
                SCE_LOG_DEBUG("DataModelInitializer: Initialized variable '{}' from expression '{}'", id, expr);
            } else {
                // Leave variable unbound (don't create it) so it can be assigned later
                return;
            }
        }
    } else if (!src.empty()) {
        // §scxml-5.3: Load data from external source (test 446)
        // ARCHITECTURE.MD: Zero Duplication - Use FileLoadingHelper (Single Source of Truth)

        std::string filePath = FileLoadingHelper::normalizePath(src);

        // Resolve relative path based on SCXML file location
        if (filePath[0] != '/') {  // Relative path
            std::string scxmlFilePath = SCE::SessionRegistry::instance().getSessionFilePath(sessionId_);
            if (!scxmlFilePath.empty()) {
                // Extract directory from SCXML file path
                size_t lastSlash = scxmlFilePath.find_last_of("/");
                if (lastSlash != std::string::npos) {
                    std::string directory = scxmlFilePath.substr(0, lastSlash + 1);
                    filePath = directory + filePath;
                }
            }
        }

        // Load file content using FileLoadingHelper
        std::string fileContent;
        bool success = FileLoadingHelper::loadFileContent(filePath, fileContent);

        if (!success) {
            SCE_LOG_ERROR("DataModelInitializer: Failed to load file '{}' for variable '{}'", filePath, id);
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution",
                                         "Failed to load file '" + filePath + "' for variable '" + id + "'");
            }
            return;
        }

        // §scxml-B-2: Check content type (XML/JSON/text) and handle appropriately
        if (isXMLContent(fileContent)) {
            // §scxml-B-2 test 557: Parse XML content as DOM object
            SCE_LOG_DEBUG("DataModelInitializer: Parsing XML content from file '{}' as DOM for variable '{}'", filePath,
                      id);

            auto setVarFuture = scriptEngine_.setVariableAsDOM(sessionId_, id, fileContent);
            auto setResult = setVarFuture.get();

            if (!SCE::ScriptResultUtils::isSuccess(setResult)) {
                SCE_LOG_ERROR(
                    "DataModelInitializer: Failed to set XML content from file '{}' for variable '{}': {}", filePath,
                    id, setResult.getErrorMessage());
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", "Failed to set XML content from file '" + filePath +
                                                                    "' for '" + id +
                                                                    "': " + setResult.getErrorMessage());
                }
                return;
            }

            SCE_LOG_DEBUG("DataModelInitializer: Set variable '{}' as XML DOM object from file '{}'", id, filePath);
        } else {
            // §scxml-B-2: Try evaluating as JSON/JS first (test 446), fall back to text (test 558)
            auto future = scriptEngine_.evaluateExpression(sessionId_, fileContent);
            auto result = future.get();

            if (SCE::ScriptResultUtils::isSuccess(result)) {
                // Successfully evaluated as JSON/JS expression
                auto setVarFuture =
                    scriptEngine_.setVariable(sessionId_, id, result.getInternalValue());
                auto setResult = setVarFuture.get();

                if (!SCE::ScriptResultUtils::isSuccess(setResult)) {
                    SCE_LOG_ERROR("DataModelInitializer: Failed to set variable '{}' from file '{}': {}", id, filePath,
                              setResult.getErrorMessage());
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution",
                                                 "Failed to set variable '" + id + "' from file '" + filePath +
                                                     "': " + setResult.getErrorMessage());
                    }
                    return;
                }

                SCE_LOG_DEBUG("DataModelInitializer: Initialized variable '{}' from file '{}'", id, filePath);
            } else {
                // §scxml-B-2 test 558: Non-JSON content - normalize whitespace and store as string
                std::string normalized = normalizeWhitespace(fileContent);

                auto setVarFuture =
                    scriptEngine_.setVariable(sessionId_, id, ScriptValue{normalized});
                auto setResult = setVarFuture.get();

                if (!SCE::ScriptResultUtils::isSuccess(setResult)) {
                    SCE_LOG_ERROR(
                        "DataModelInitializer: Failed to set normalized text from file '{}' for variable '{}': {}",
                        filePath, id, setResult.getErrorMessage());
                    if (eventRaiser_) {
                        eventRaiser_->raiseEvent("error.execution",
                                                 "Failed to set text content from file '" + filePath + "' for '" + id +
                                                     "': " + setResult.getErrorMessage());
                    }
                    return;
                }

                SCE_LOG_DEBUG("DataModelInitializer: Set variable '{}' with normalized text from file '{}': '{}'", id,
                          filePath, normalized);
            }
        }
    } else if (!content.empty()) {
        // §scxml-B-2: Initialize with inline content
        // ARCHITECTURE.md: Zero Duplication - Use DataModelInitHelper (shared with AOT engine)
        bool success = DataModelInitHelper::initializeVariable(
            scriptEngine_, sessionId_, id, content, [this](const std::string &msg) {
                SCE_LOG_ERROR("DataModelInitializer: {}", msg);
                if (eventRaiser_) {
                    eventRaiser_->raiseEvent("error.execution", msg);
                }
            });

        if (!success) {
            return;  // Error already handled by callback
        }

        SCE_LOG_DEBUG("DataModelInitializer: Initialized variable '{}' from content", id);
    } else {
        // §scxml-5.3: No expression or content - create variable with undefined value (test 445)
        auto setVarFuture = scriptEngine_.setVariable(sessionId_, id, ScriptValue{});
        auto setResult = setVarFuture.get();

        if (!SCE::ScriptResultUtils::isSuccess(setResult)) {
            SCE_LOG_ERROR("DataModelInitializer: Failed to create undefined variable '{}': {}", id,
                      setResult.getErrorMessage());
            if (eventRaiser_) {
                eventRaiser_->raiseEvent("error.execution",
                                         "Failed to create variable '" + id + "': " + setResult.getErrorMessage());
            }
            return;
        }

        SCE_LOG_DEBUG("DataModelInitializer: Created variable '{}' with undefined value", id);
    }
}

void DataModelInitializer::initializeAllDataItems(const std::string &binding) {
    if (!model_) {
        SCE_LOG_DEBUG("DataModelInitializer: No model available for data model initialization");
        return;
    }

    const auto allDataItems = collectAllDataItems();
    SCE_LOG_INFO("DataModelInitializer: Initializing {} total data items (global scope with {} binding)",
             allDataItems.size(), binding.empty() ? "early (default)" : binding);

    // Use BindingHelper to determine initialization strategy
    // This ensures §scxml-5.3 compliance through shared logic with AOT engine
    bool shouldAssignValue = BindingHelper::shouldAssignValueAtDocumentLoad(binding);

    for (const auto &dataInfo : allDataItems) {
        // Always call initializeDataItem (handles expr/src/content/undefined)
        // The assignValue flag controls whether to evaluate expr/src/content or use undefined
        initializeDataItem(dataInfo.dataItem, shouldAssignValue);
    }

    if (BindingHelper::isLateBinding(binding)) {
        SCE_LOG_DEBUG("DataModelInitializer: Late binding mode - values will be assigned on state entry");
    } else {
        SCE_LOG_DEBUG("DataModelInitializer: Early binding mode - all values assigned at init");
    }
}

void DataModelInitializer::initializeStateDataOnEntry(const std::string &stateId) {
    if (!model_) {
        return;
    }

    const std::string &binding = model_->getBinding();
    bool isFirstEntry = (initializedStates_.find(stateId) == initializedStates_.end());

    if (isFirstEntry) {
        // First entry to this state - check if we need to initialize variables
        auto stateNode = model_->findStateById(stateId);
        if (stateNode) {
            const auto &stateDataItems = stateNode->getDataItems();
            if (!stateDataItems.empty()) {
                SCE_LOG_DEBUG(
                    "DataModelInitializer: First entry to state '{}' - checking {} data items for late binding",
                    stateId, stateDataItems.size());

                for (const auto &item : stateDataItems) {
                    bool hasExpr = !item->getExpr().empty();

                    // Use BindingHelper to determine if value should be assigned on state entry
                    if (BindingHelper::shouldAssignValueOnStateEntry(binding, isFirstEntry, hasExpr)) {
                        // Late binding: assign value now
                        initializeDataItem(item, true);  // assignValue=true
                    }
                }

                initializedStates_.insert(stateId);  // Mark state as initialized
            }
        }
    }
}

}  // namespace SCE
