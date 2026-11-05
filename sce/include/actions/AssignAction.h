// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "BaseAction.h"
#include <string>

namespace SCE {

/**
 * @brief SCXML <assign> action implementation
 *
 * Assigns a value to a variable in the SCXML data model.
 * This is equivalent to the <assign> element in SCXML specification.
 */
class AssignAction : public BaseAction {
public:
    /**
     * @brief Construct assign action
     * @param location Variable location to assign to (e.g., "myVar", "data.field")
     * @param expr Expression to evaluate and assign
     * @param id Optional action identifier
     */
    AssignAction(const std::string &location, const std::string &expr, const std::string &id = "");

    /**
     * @brief Destructor
     */
    virtual ~AssignAction() = default;

    // IActionNode implementation
    bool execute(IExecutionContext &context) override;
    std::string getActionType() const override;
    std::shared_ptr<IActionNode> clone() const override;

    /**
     * @brief Get assignment location
     * @return Variable location string
     */
    const std::string &getLocation() const;

    /**
     * @brief Set assignment location
     * @param location Variable location to assign to
     */
    void setLocation(const std::string &location);

    /**
     * @brief Get assignment expression
     * @return Expression string
     */
    const std::string &getExpr() const;

    /**
     * @brief Set assignment expression
     * @param expr Expression to evaluate and assign
     */
    void setExpr(const std::string &expr);

    /**
     * @brief The `<assign>`'s in-line children, when they give the value
     *
     * Set instead of an expression when the element has children and no
     * `expr`. They are not an expression to evaluate as written: the executor
     * reads them as every in-line value is read
     * (`DataModelInitHelper::inlineContentValue`) — the value of the
     * expression they are when they are one, and their space-normalized text
     * otherwise — which is the reading sce-build's `to_*_assign_content`
     * gives the generated machines.
     */
    const std::string &getInlineContent() const;

    /**
     * @brief Set the in-line children that give the value
     * @param content The children as `XmlSerializationHelper::serializeContentBody` reads them
     */
    void setInlineContent(const std::string &content);

    /**
     * @brief Whether the value is given by in-line children rather than `expr`
     */
    bool hasInlineContent() const;

    /**
     * @brief Get assignment type hint (optional)
     * @return Type hint string
     */
    const std::string &getType() const;

    /**
     * @brief Set assignment type hint
     * @param type Type hint ("string", "number", "boolean", etc.)
     */
    void setType(const std::string &type);

protected:
    // BaseAction implementation
    std::vector<std::string> validateSpecific() const override;
    std::string getSpecificDescription() const override;

private:
    std::string location_;
    std::string expr_;
    std::string inlineContent_;
    std::string type_;  // Optional type hint

    /**
     * @brief Validate variable location syntax
     * @param location Location string to validate
     * @return true if location is syntactically valid
     */
    bool isValidLocation(const std::string &location) const;
};

}  // namespace SCE