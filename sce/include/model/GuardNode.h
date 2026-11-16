// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IGuardNode.h"
#include <string>
#include <unordered_map>
#include <vector>

/**
 * @brief Implementation class for guard node
 *
 * This class implements guard nodes that represent transition conditions.
 * Corresponds to <code:guard> element in SCXML documents.
 */

namespace SCE {

class GuardNode : public IGuardNode {
public:
    /**
     * @brief Constructor
     * @param id Guard identifier
     * @param target Target state ID
     */
    GuardNode(const std::string &id, const std::string &target);

    /**
     * @brief Destructor
     */
    virtual ~GuardNode();

    /**
     * @brief Return guard ID
     * @return Guard ID
     */
    virtual const std::string &getId() const override;

    /**
     * @brief Add dependency
     * @param property Property name to depend on
     */
    virtual void addDependency(const std::string &property) override;

    /**
     * @brief Return list of dependencies
     * @return List of dependent property names
     */
    virtual const std::vector<std::string> &getDependencies() const override;

    virtual void setAttribute(const std::string &name, const std::string &value) override;
    virtual const std::string &getAttribute(const std::string &name) const override;
    virtual const std::unordered_map<std::string, std::string> &getAttributes() const override;

    void setTargetState(const std::string &targetState) override;
    const std::string &getTargetState() const override;
    void setCondition(const std::string &condition) override;
    const std::string &getCondition() const override;

private:
    std::string id_;                                           // Guard ID
    std::string target_;                                       // Legacy field for compatibility
    std::string condition_;                                    // Condition expression
    std::string targetState_;                                  // Target state ID
    std::vector<std::string> dependencies_;                    // List of dependencies
    std::unordered_map<std::string, std::string> attributes_;  // Other attributes
    const std::string emptyString_;                            // For returning empty string
};

}  // namespace SCE