#include "parsing/TransitionParser.h"
#include "common/Logger.h"
#include "parsing/ParsingCommon.h"
#include <algorithm>
#include <cassert>
#include <sstream>

SCE::TransitionParser::TransitionParser(std::shared_ptr<SCE::NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    LOG_DEBUG("Creating transition parser");
}

SCE::TransitionParser::~TransitionParser() {
    LOG_DEBUG("Destroying transition parser");
}

void SCE::TransitionParser::setActionParser(std::shared_ptr<SCE::ActionParser> actionParser) {
    actionParser_ = actionParser;
    LOG_DEBUG("Action parser set");
}

std::shared_ptr<SCE::ITransitionNode>
SCE::TransitionParser::parseTransitionNode(const std::shared_ptr<IXMLElement> &transElement,
                                           SCE::IStateNode *stateNode) {
    if (!transElement || !stateNode) {
        LOG_WARN("Null transition element or state node");
        return nullptr;
    }

    std::string event = transElement->hasAttribute("event") ? transElement->getAttribute("event") : "";
    std::string target = transElement->hasAttribute("target") ? transElement->getAttribute("target") : "";

    LOG_DEBUG("Parsing transition: {} -> {}", (event.empty() ? "<no event>" : event),
              (target.empty() ? "<internal>" : target));

    // Treat as internal transition if target is empty
    bool isInternal = target.empty();

    // Create transition node
    std::shared_ptr<SCE::ITransitionNode> transition;

    if (isInternal) {
        LOG_DEBUG("Internal transition detected (no target)");

        // Create transition with empty target
        transition = nodeFactory_->createTransitionNode(event, "");

        // Explicitly clear target list
        transition->clearTargets();

        LOG_DEBUG("After clearTargets() - targets count: {}", transition->getTargets().size());
    } else {
        // Create with empty string on initialization
        transition = nodeFactory_->createTransitionNode(event, "");

        // Clear existing target list and start fresh
        transition->clearTargets();

        // Parse space-separated target string
        std::stringstream ss(target);
        std::string targetId;

        // Add individual targets
        while (ss >> targetId) {
            if (!targetId.empty()) {
                transition->addTarget(targetId);
                LOG_DEBUG("Added target: {}", targetId);
            }
        }
    }

    // Set internal transition
    transition->setInternal(isInternal);

    // Process type attribute
    if (transElement->hasAttribute("type")) {
        std::string type = transElement->getAttribute("type");
        transition->setAttribute("type", type);
        LOG_DEBUG("Type: {}", type);

        // Set as internal transition if type is "internal"
        if (type == "internal") {
            transition->setInternal(true);
            isInternal = true;  // Update isInternal variable
        }
    }

    // Process condition attribute
    if (transElement->hasAttribute("cond")) {
        std::string cond = transElement->getAttribute("cond");
        transition->setAttribute("cond", cond);
        transition->setGuard(cond);
        LOG_DEBUG("Condition: {}", cond);
    }

    // Process guard attribute
    if (transElement->hasAttribute("guard")) {
        std::string guard = transElement->getAttribute("guard");
        transition->setGuard(guard);
        LOG_DEBUG("Guard: {}", guard);
    }

    // Parse event list
    if (!event.empty()) {
        auto events = parseEventList(event);
        for (const auto &eventName : events) {
            transition->addEvent(eventName);
            LOG_DEBUG("Added event: {}", eventName);
        }
    }

    // Parse actions
    parseActions(transElement, transition);

    LOG_DEBUG("Transition parsed successfully with {} ActionNodes", transition->getActionNodes().size());
    return transition;
}

std::shared_ptr<SCE::ITransitionNode>
SCE::TransitionParser::parseInitialTransition(const std::shared_ptr<IXMLElement> &initialElement) {
    if (!initialElement) {
        LOG_WARN("Null initial element");
        return nullptr;
    }

    LOG_DEBUG("Parsing initial transition");

    // Find transition element within initial element
    auto transElement = ParsingCommon::findFirstChildElement(initialElement, "transition");
    if (!transElement) {
        LOG_WARN("No transition element found in initial");
        return nullptr;
    }

    if (!transElement->hasAttribute("target")) {
        LOG_WARN("Initial transition missing target attribute");
        return nullptr;
    }

    std::string target = transElement->getAttribute("target");
    LOG_DEBUG("Initial transition target: {}", target);

    // Create initial transition - no event
    auto transition = nodeFactory_->createTransitionNode("", target);

    // Set special attribute
    transition->setAttribute("initial", "true");

    // Parse actions
    parseActions(transElement, transition);

    LOG_DEBUG("Initial transition parsed successfully");
    return transition;
}

std::vector<std::shared_ptr<SCE::ITransitionNode>>
SCE::TransitionParser::parseTransitionsInState(const std::shared_ptr<IXMLElement> &stateElement,
                                               SCE::IStateNode *stateNode) {
    std::vector<std::shared_ptr<SCE::ITransitionNode>> transitions;

    if (!stateElement || !stateNode) {
        LOG_WARN("Null state element or node");
        return transitions;
    }

    LOG_DEBUG("Parsing transitions in state: {}", stateNode->getId());

    // Find all transition elements
    auto transElements = ParsingCommon::findChildElements(stateElement, "transition");
    for (const auto &transElement : transElements) {
        auto transition = parseTransitionNode(transElement, stateNode);
        if (transition) {
            transitions.push_back(transition);
        }
    }

    LOG_DEBUG("Found {} transitions", transitions.size());
    return transitions;
}

bool SCE::TransitionParser::isTransitionNode(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    return matchNodeName(nodeName, "transition");
}

void SCE::TransitionParser::parseActions(const std::shared_ptr<IXMLElement> &transElement,
                                         std::shared_ptr<SCE::ITransitionNode> transition) {
    if (!transElement || !transition) {
        return;
    }

    // ActionParser is required for SCXML parsing
    if (!actionParser_) {
        assert(false && "ActionParser is required for SCXML compliance");
        return;
    }

    {
        // SCXML specification compliance: Store ActionNode objects directly
        auto actionNodes = actionParser_->parseActionsInElement(transElement);
        for (const auto &actionNode : actionNodes) {
            if (actionNode) {
                transition->addActionNode(actionNode);
                LOG_DEBUG("Added ActionNode: {}", actionNode->getActionType());
            }
        }
    }
}

std::vector<std::string> SCE::TransitionParser::parseEventList(const std::string &eventStr) const {
    std::vector<std::string> events;
    std::stringstream ss(eventStr);
    std::string event;

    // Parse space-separated event list
    while (std::getline(ss, event, ' ')) {
        // Remove empty strings
        if (!event.empty()) {
            events.push_back(event);
        }
    }

    return events;
}

bool SCE::TransitionParser::matchNodeName(const std::string &nodeName, const std::string &searchName) const {
    return ParsingCommon::matchNodeName(nodeName, searchName);
}
