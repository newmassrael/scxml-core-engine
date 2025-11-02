#include "parsing/GuardParser.h"
#include "GuardUtils.h"
#include "ParsingCommon.h"
#include "common/Logger.h"
#include <algorithm>

RSM::GuardParser::GuardParser(std::shared_ptr<RSM::NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    LOG_DEBUG("Creating guard parser");
}

RSM::GuardParser::~GuardParser() {
    LOG_DEBUG("Destroying guard parser");
}

std::shared_ptr<RSM::IGuardNode> RSM::GuardParser::parseGuardNode(const std::shared_ptr<IXMLElement> &guardNode) {
    if (!guardNode) {
        LOG_WARN("Null guard node");
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
        LOG_WARN("Guard node missing required attributes");
        LOG_DEBUG("Node name: {}", guardNode->getName());
        return nullptr;
    }

    // Create basic guard node
    auto guard = nodeFactory_->createGuardNode(id, "");

    // Process target attribute
    if (!target.empty()) {
        LOG_DEBUG("Guard: {} with target attribute: {}", id, target);

        if (GuardUtils::isConditionExpression(target)) {
            guard->setCondition(target);
            LOG_DEBUG("Set condition from target: {}", target);
        } else {
            guard->setTargetState(target);
            LOG_DEBUG("Set target state: {}", target);
        }
    }

    // Process condition attribute
    if (!condition.empty()) {
        guard->setCondition(condition);
        LOG_DEBUG("Set condition from attribute: {}", condition);
    }

    // Process <code:condition> or <condition> element
    auto conditionElement = RSM::ParsingCommon::findFirstChildElement(guardNode, "condition");
    if (conditionElement) {
        LOG_DEBUG("Found condition element");

        std::string conditionText = RSM::ParsingCommon::extractTextContent(conditionElement, true);
        LOG_DEBUG("Raw condition content: '{}'", conditionText);

        if (!conditionText.empty()) {
            guard->setCondition(conditionText);
            LOG_DEBUG("Set condition from element: {}", conditionText);
        }
    }

    // Parse dependencies
    parseDependencies(guardNode, guard);

    // Parse external implementation
    parseExternalImplementation(guardNode, guard);

    LOG_DEBUG("Guard parsed successfully");
    return guard;
}

std::shared_ptr<RSM::IGuardNode>
RSM::GuardParser::parseGuardFromTransition(const std::shared_ptr<IXMLElement> &transitionNode,
                                           const std::string &targetState) {
    if (!transitionNode) {
        LOG_WARN("Null transition node");
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

    LOG_DEBUG("Parsing guard from transition: {} for state: {}", guardId, targetState);

    // Create basic guard node
    auto guard = nodeFactory_->createGuardNode(guardId, "");

    // Set target state explicitly
    guard->setTargetState(targetState);

    // Check if cond attribute exists
    if (transitionNode->hasAttribute("cond")) {
        std::string condition = transitionNode->getAttribute("cond");
        guard->setCondition(condition);
        LOG_DEBUG("Set condition from cond attribute: {}", condition);
    }

    LOG_DEBUG("Guard from transition parsed successfully");
    return guard;
}

std::shared_ptr<RSM::IGuardNode>
RSM::GuardParser::parseReactiveGuard(const std::shared_ptr<IXMLElement> &reactiveGuardNode) {
    if (!reactiveGuardNode) {
        LOG_WARN("Null reactive guard node");
        return nullptr;
    }

    std::string id, target, condition;

    if (reactiveGuardNode->hasAttribute("id")) {
        id = reactiveGuardNode->getAttribute("id");
    }
    if (reactiveGuardNode->hasAttribute("target")) {
        target = reactiveGuardNode->getAttribute("target");
    }
    if (reactiveGuardNode->hasAttribute("condition")) {
        condition = reactiveGuardNode->getAttribute("condition");
    }

    if (id.empty() || (target.empty() && condition.empty())) {
        LOG_WARN("Reactive guard node missing required attributes");
        return nullptr;
    }

    // Create basic guard node
    auto guard = nodeFactory_->createGuardNode(id, "");

    // Set reactive attributes
    guard->setReactive(true);
    guard->setAttribute("reactive", "true");

    // Process target attribute
    if (!target.empty()) {
        LOG_DEBUG("Reactive guard: {} with target: {}", id, target);

        if (GuardUtils::isConditionExpression(target)) {
            guard->setCondition(target);
            LOG_DEBUG("Set condition from target: {}", target);
        } else {
            guard->setTargetState(target);
            LOG_DEBUG("Set target state: {}", target);
        }
    }

    // Process condition attribute
    if (!condition.empty()) {
        guard->setCondition(condition);
        LOG_DEBUG("Set condition from attribute: {}", condition);
    }

    // Parse dependencies
    parseDependencies(reactiveGuardNode, guard);

    // Parse external implementation
    parseExternalImplementation(reactiveGuardNode, guard);

    LOG_DEBUG("Reactive guard parsed successfully");
    return guard;
}

std::vector<std::shared_ptr<RSM::IGuardNode>>
RSM::GuardParser::parseGuardsElement(const std::shared_ptr<IXMLElement> &guardsNode) {
    std::vector<std::shared_ptr<RSM::IGuardNode>> guards;

    if (!guardsNode) {
        LOG_WARN("Null guards node");
        return guards;
    }

    LOG_DEBUG("Parsing guards element");

    // Parse guard nodes
    auto guardNodes = RSM::ParsingCommon::findChildElements(guardsNode, "guard");

    for (const auto &guardElement : guardNodes) {
        auto guard = parseGuardNode(guardElement);
        if (guard) {
            guards.push_back(guard);
            LOG_DEBUG("Added guard: {}", guard->getId());
        }
    }

    LOG_DEBUG("Parsed {} guards", guards.size());
    return guards;
}

std::vector<std::shared_ptr<RSM::IGuardNode>>
RSM::GuardParser::parseAllGuards(const std::shared_ptr<IXMLElement> &scxmlNode) {
    std::vector<std::shared_ptr<RSM::IGuardNode>> allGuards;

    if (!scxmlNode) {
        LOG_WARN("Null SCXML node");
        return allGuards;
    }

    LOG_DEBUG("Parsing all guards in SCXML document");

    // 1. Parse guards within code:guards element
    auto guardsNode = RSM::ParsingCommon::findFirstChildElement(scxmlNode, "guards");
    if (guardsNode) {
        auto guards = parseGuardsElement(guardsNode);
        allGuards.insert(allGuards.end(), guards.begin(), guards.end());
    }

    // 2. Find guard attributes in transitions of all states
    auto stateNodes = RSM::ParsingCommon::findChildElements(scxmlNode, "state");
    auto parallelNodes = RSM::ParsingCommon::findChildElements(scxmlNode, "parallel");
    auto finalNodes = RSM::ParsingCommon::findChildElements(scxmlNode, "final");

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
        auto transNodes = RSM::ParsingCommon::findChildElements(stateElement, "transition");
        for (const auto &transElement : transNodes) {
            if (transElement->hasAttribute("target")) {
                std::string target = transElement->getAttribute("target");
                auto guard = parseGuardFromTransition(transElement, target);
                if (guard) {
                    allGuards.push_back(guard);
                    LOG_DEBUG("Added guard from transition in state {}", stateId);
                }
            }
        }

        // Process reactive guards
        auto reactiveGuardNodes = RSM::ParsingCommon::findChildElements(stateElement, "reactive-guard");
        for (const auto &guardElement : reactiveGuardNodes) {
            auto guard = parseReactiveGuard(guardElement);
            if (guard) {
                allGuards.push_back(guard);
                LOG_DEBUG("Added reactive guard from state {}", stateId);
            }
        }
    }

    // 3. Remove duplicates (based on ID)
    std::sort(allGuards.begin(), allGuards.end(),
              [](const std::shared_ptr<RSM::IGuardNode> &a, const std::shared_ptr<RSM::IGuardNode> &b) {
                  return a->getId() < b->getId();
              });

    allGuards.erase(std::unique(allGuards.begin(), allGuards.end(),
                                [](const std::shared_ptr<RSM::IGuardNode> &a,
                                   const std::shared_ptr<RSM::IGuardNode> &b) { return a->getId() == b->getId(); }),
                    allGuards.end());

    LOG_DEBUG("Found {} unique guards", allGuards.size());
    return allGuards;
}

bool RSM::GuardParser::isGuardNode(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    return RSM::ParsingCommon::matchNodeName(nodeName, "guard");
}

bool RSM::GuardParser::isReactiveGuardNode(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    return RSM::ParsingCommon::matchNodeName(nodeName, "reactive-guard");
}

void RSM::GuardParser::parseDependencies(const std::shared_ptr<IXMLElement> &guardNode,
                                         std::shared_ptr<RSM::IGuardNode> guardObject) {
    if (!guardNode || !guardObject) {
        return;
    }

    // Parse dependencies
    auto depNodes = RSM::ParsingCommon::findChildElements(guardNode, "dependency");

    for (const auto &element : depNodes) {
        std::string property;
        if (element->hasAttribute("property")) {
            property = element->getAttribute("property");
        } else if (element->hasAttribute("prop")) {
            property = element->getAttribute("prop");
        }

        if (!property.empty()) {
            guardObject->addDependency(property);
            LOG_DEBUG("Added dependency: {}", property);
        }
    }
}

void RSM::GuardParser::parseExternalImplementation(const std::shared_ptr<IXMLElement> &guardNode,
                                                   std::shared_ptr<RSM::IGuardNode> guardObject) {
    if (!guardNode || !guardObject) {
        return;
    }

    auto implNode = RSM::ParsingCommon::findFirstChildElement(guardNode, "external-implementation");

    if (implNode) {
        if (implNode->hasAttribute("class")) {
            std::string className = implNode->getAttribute("class");
            guardObject->setExternalClass(className);
            LOG_DEBUG("External class: {}", className);
        }

        if (implNode->hasAttribute("factory")) {
            std::string factory = implNode->getAttribute("factory");
            guardObject->setExternalFactory(factory);
            LOG_DEBUG("External factory: {}", factory);
        }
    }
}
