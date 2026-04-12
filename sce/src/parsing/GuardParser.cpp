// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/GuardParser.h"
#include "GuardUtils.h"
#include "ParsingCommon.h"
#include "core/LogMacros.h"
#include <algorithm>

SCE::GuardParser::GuardParser(std::shared_ptr<SCE::NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    SCE_LOG_DEBUG("Creating guard parser");
}

SCE::GuardParser::~GuardParser() {
    SCE_LOG_DEBUG("Destroying guard parser");
}

std::shared_ptr<SCE::IGuardNode> SCE::GuardParser::parseGuardNode(const std::shared_ptr<IXMLElement> &guardNode) {
    if (!guardNode) {
        SCE_LOG_WARN("Null guard node");
        return nullptr;
    }

    std::string id, target, condition;

    // Get id attribute
    if (guardNode->hasAttribute("id")) {
        id = guardNode->getAttribute("id");
    } else if (guardNode->hasAttribute("name")) {
        id = guardNode->getAttribute("name");
    }

    // Get target/condition attributes
    if (guardNode->hasAttribute("target")) {
        target = guardNode->getAttribute("target");
    } else if (guardNode->hasAttribute("condition")) {
        condition = guardNode->getAttribute("condition");
    } else if (guardNode->hasAttribute("to")) {
        target = guardNode->getAttribute("to");
    }

    if (id.empty() || (target.empty() && condition.empty())) {
        SCE_LOG_WARN("Guard node missing required attributes");
        SCE_LOG_DEBUG("Node name: {}", guardNode->getName());
        return nullptr;
    }

    // Create basic guard node
    auto guard = nodeFactory_->createGuardNode(id, "");

    // Process target attribute
    if (!target.empty()) {
        SCE_LOG_DEBUG("Guard: {} with target attribute: {}", id, target);

        if (GuardUtils::isConditionExpression(target)) {
            guard->setCondition(target);
            SCE_LOG_DEBUG("Set condition from target: {}", target);
        } else {
            guard->setTargetState(target);
            SCE_LOG_DEBUG("Set target state: {}", target);
        }
    }

    // Process condition attribute
    if (!condition.empty()) {
        guard->setCondition(condition);
        SCE_LOG_DEBUG("Set condition from attribute: {}", condition);
    }

    // Process <code:condition> or <condition> element
    auto conditionElement = SCE::ParsingCommon::findFirstChildElement(guardNode, "condition");
    if (conditionElement) {
        SCE_LOG_DEBUG("Found condition element");

        std::string conditionText = SCE::ParsingCommon::extractTextContent(conditionElement, true);
        SCE_LOG_DEBUG("Raw condition content: '{}'", conditionText);

        if (!conditionText.empty()) {
            guard->setCondition(conditionText);
            SCE_LOG_DEBUG("Set condition from element: {}", conditionText);
        }
    }

    // Parse dependencies
    parseDependencies(guardNode, guard);

    // Parse external implementation
    parseExternalImplementation(guardNode, guard);

    SCE_LOG_DEBUG("Guard parsed successfully");
    return guard;
}

std::shared_ptr<SCE::IGuardNode>
SCE::GuardParser::parseGuardFromTransition(const std::shared_ptr<IXMLElement> &transitionNode,
                                           const std::string &targetState) {
    if (!transitionNode) {
        SCE_LOG_WARN("Null transition node");
        return nullptr;
    }

    // Find guard attribute considering namespace prefix
    std::string guardId;
    if (transitionNode->hasAttribute("code:guard")) {
        guardId = transitionNode->getAttribute("code:guard");
    } else if (transitionNode->hasAttribute("guard")) {
        guardId = transitionNode->getAttribute("guard");
    }

    if (guardId.empty()) {
        return nullptr;
    }

    SCE_LOG_DEBUG("Parsing guard from transition: {} for state: {}", guardId, targetState);

    // Create basic guard node
    auto guard = nodeFactory_->createGuardNode(guardId, "");

    // Set target state explicitly
    guard->setTargetState(targetState);

    // Check if cond attribute exists
    if (transitionNode->hasAttribute("cond")) {
        std::string condition = transitionNode->getAttribute("cond");
        guard->setCondition(condition);
        SCE_LOG_DEBUG("Set condition from cond attribute: {}", condition);
    }

    SCE_LOG_DEBUG("Guard from transition parsed successfully");
    return guard;
}

std::vector<std::shared_ptr<SCE::IGuardNode>>
SCE::GuardParser::parseGuardsElement(const std::shared_ptr<IXMLElement> &guardsNode) {
    std::vector<std::shared_ptr<SCE::IGuardNode>> guards;

    if (!guardsNode) {
        SCE_LOG_WARN("Null guards node");
        return guards;
    }

    SCE_LOG_DEBUG("Parsing guards element");

    // Parse guard nodes
    auto guardNodes = SCE::ParsingCommon::findChildElements(guardsNode, "guard");

    for (const auto &guardElement : guardNodes) {
        auto guard = parseGuardNode(guardElement);
        if (guard) {
            guards.push_back(guard);
            SCE_LOG_DEBUG("Added guard: {}", guard->getId());
        }
    }

    SCE_LOG_DEBUG("Parsed {} guards", guards.size());
    return guards;
}

std::vector<std::shared_ptr<SCE::IGuardNode>>
SCE::GuardParser::parseAllGuards(const std::shared_ptr<IXMLElement> &scxmlNode) {
    std::vector<std::shared_ptr<SCE::IGuardNode>> allGuards;

    if (!scxmlNode) {
        SCE_LOG_WARN("Null SCXML node");
        return allGuards;
    }

    SCE_LOG_DEBUG("Parsing all guards in SCXML document");

    // 1. Parse guards within code:guards element
    auto guardsNode = SCE::ParsingCommon::findFirstChildElement(scxmlNode, "guards");
    if (guardsNode) {
        auto guards = parseGuardsElement(guardsNode);
        allGuards.insert(allGuards.end(), guards.begin(), guards.end());
    }

    // 2. Find guard attributes in transitions of all states
    auto stateNodes = SCE::ParsingCommon::findChildElements(scxmlNode, "state");
    auto parallelNodes = SCE::ParsingCommon::findChildElements(scxmlNode, "parallel");
    auto finalNodes = SCE::ParsingCommon::findChildElements(scxmlNode, "final");

    // Combine all state nodes
    std::vector<std::shared_ptr<IXMLElement>> allStateNodes;
    allStateNodes.insert(allStateNodes.end(), stateNodes.begin(), stateNodes.end());
    allStateNodes.insert(allStateNodes.end(), parallelNodes.begin(), parallelNodes.end());
    allStateNodes.insert(allStateNodes.end(), finalNodes.begin(), finalNodes.end());

    // Check guard attributes in transition elements of each state
    for (const auto &stateElement : allStateNodes) {
        // Get state ID
        if (!stateElement->hasAttribute("id")) {
            continue;
        }

        std::string stateId = stateElement->getAttribute("id");

        // Process transition elements
        auto transNodes = SCE::ParsingCommon::findChildElements(stateElement, "transition");
        for (const auto &transElement : transNodes) {
            if (transElement->hasAttribute("target")) {
                std::string target = transElement->getAttribute("target");
                auto guard = parseGuardFromTransition(transElement, target);
                if (guard) {
                    allGuards.push_back(guard);
                    SCE_LOG_DEBUG("Added guard from transition in state {}", stateId);
                }
            }
        }
    }

    // 3. Remove duplicates (based on ID)
    std::sort(allGuards.begin(), allGuards.end(),
              [](const std::shared_ptr<SCE::IGuardNode> &a, const std::shared_ptr<SCE::IGuardNode> &b) {
                  return a->getId() < b->getId();
              });

    allGuards.erase(std::unique(allGuards.begin(), allGuards.end(),
                                [](const std::shared_ptr<SCE::IGuardNode> &a,
                                   const std::shared_ptr<SCE::IGuardNode> &b) { return a->getId() == b->getId(); }),
                    allGuards.end());

    SCE_LOG_DEBUG("Found {} unique guards", allGuards.size());
    return allGuards;
}

bool SCE::GuardParser::isGuardNode(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    return SCE::ParsingCommon::matchNodeName(nodeName, "guard");
}

void SCE::GuardParser::parseDependencies(const std::shared_ptr<IXMLElement> &guardNode,
                                         std::shared_ptr<SCE::IGuardNode> guardObject) {
    if (!guardNode || !guardObject) {
        return;
    }

    // Parse dependencies
    auto depNodes = SCE::ParsingCommon::findChildElements(guardNode, "dependency");

    for (const auto &element : depNodes) {
        std::string property;
        if (element->hasAttribute("property")) {
            property = element->getAttribute("property");
        } else if (element->hasAttribute("prop")) {
            property = element->getAttribute("prop");
        }

        if (!property.empty()) {
            guardObject->addDependency(property);
            SCE_LOG_DEBUG("Added dependency: {}", property);
        }
    }
}

void SCE::GuardParser::parseExternalImplementation(const std::shared_ptr<IXMLElement> &guardNode,
                                                   std::shared_ptr<SCE::IGuardNode> guardObject) {
    if (!guardNode || !guardObject) {
        return;
    }

    auto implNode = SCE::ParsingCommon::findFirstChildElement(guardNode, "external-implementation");

    if (implNode) {
        if (implNode->hasAttribute("class")) {
            std::string className = implNode->getAttribute("class");
            guardObject->setExternalClass(className);
            SCE_LOG_DEBUG("External class: {}", className);
        }

        if (implNode->hasAttribute("factory")) {
            std::string factory = implNode->getAttribute("factory");
            guardObject->setExternalFactory(factory);
            SCE_LOG_DEBUG("External factory: {}", factory);
        }
    }
}
