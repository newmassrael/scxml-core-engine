// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/ActionParser.h"
#include "actions/AssignAction.h"
#include "actions/CancelAction.h"
#include "actions/ForeachAction.h"
#include "actions/IfAction.h"
#include "actions/LogAction.h"
#include "actions/RaiseAction.h"
#include "actions/ScriptAction.h"
#include "actions/SendAction.h"
#include "backends/LogUtils.h"
#include "common/FileLoadingHelper.h"
#include "core/LogMacros.h"
#include "parsing/ParsingCommon.h"
#include "parsing/XmlSerializationHelper.h"
#include <algorithm>
#include <sstream>

#ifndef __EMSCRIPTEN__
#endif

SCE::ActionParser::ActionParser(std::shared_ptr<SCE::NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    SCE_LOG_DEBUG("Creating action parser");
}

SCE::ActionParser::~ActionParser() {
    SCE_LOG_DEBUG("Destroying action parser");
}

bool SCE::ActionParser::matchNodeName(const std::string &nodeName, const std::string &searchName) const {
    // Exact match
    if (nodeName == searchName) {
        return true;
    }

    // With namespace (e.g., "code:action")
    size_t colonPos = nodeName.find(':');
    if (colonPos != std::string::npos && colonPos + 1 < nodeName.length()) {
        std::string localName = nodeName.substr(colonPos + 1);
        return localName == searchName;
    }

    return false;
}

std::string SCE::ActionParser::getLocalName(const std::string &nodeName) const {
    // Remove namespace if present
    size_t colonPos = nodeName.find(':');
    if (colonPos != std::string::npos && colonPos + 1 < nodeName.length()) {
        return nodeName.substr(colonPos + 1);
    }
    return nodeName;
}

void SCE::ActionParser::setScxmlBasePath(const std::string &basePath) {
    scxmlBasePath_ = basePath;
}

// ============================================================================
// Platform-agnostic IXMLElement implementations
// ============================================================================

bool SCE::ActionParser::isActionNode(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    SCE_LOG_DEBUG("ActionParser: isActionNode checking element: '{}'", nodeName);

    // Custom action tags
    if (matchNodeName(nodeName, "action") || matchNodeName(nodeName, "code:action")) {
        return true;
    }

    // Standard SCXML executable content tags
    bool isStandardAction = matchNodeName(nodeName, "raise") || matchNodeName(nodeName, "assign") ||
                            matchNodeName(nodeName, "script") || matchNodeName(nodeName, "log") ||
                            matchNodeName(nodeName, "send") || matchNodeName(nodeName, "cancel");

    SCE_LOG_DEBUG("ActionParser: isActionNode result for '{}': {}", nodeName, isStandardAction);
    return isStandardAction;
}

bool SCE::ActionParser::isExternalActionNode(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    return matchNodeName(nodeName, "external-action") || matchNodeName(nodeName, "code:external-action");
}

bool SCE::ActionParser::isSpecialExecutableContent(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();

    // SCXML executable content requiring special processing
    // Note: else/elseif are only processed within if blocks, excluded here
    return matchNodeName(nodeName, "if") || matchNodeName(nodeName, "foreach") || matchNodeName(nodeName, "invoke") ||
           matchNodeName(nodeName, "finalize");
}

void SCE::ActionParser::parseExternalImplementation(const std::shared_ptr<IXMLElement> &element,
                                                    std::shared_ptr<SCE::IActionNode> actionNode) {
    if (!element || !actionNode) {
        return;
    }

    SCE_LOG_DEBUG("Parsing external implementation for action: {}", actionNode->getId());

    if (element->hasAttribute("class")) {
        std::string className = element->getAttribute("class");
        SCE_LOG_DEBUG("Class name: {}", className);
    }

    if (element->hasAttribute("factory")) {
        std::string factory = element->getAttribute("factory");
        SCE_LOG_DEBUG("Factory: {}", factory);
    }
}

void SCE::ActionParser::parseSpecialExecutableContent(const std::shared_ptr<IXMLElement> &element,
                                                      std::vector<std::shared_ptr<SCE::IActionNode>> &actions) {
    if (!element) {
        return;
    }

    std::string nodeName = element->getName();
    SCE_LOG_DEBUG("Parsing special content: {}", nodeName);

    // Special elements should use parseActionNode for proper parsing
    std::string localName = getLocalName(nodeName);

    if (localName == "if" || localName == "foreach") {
        // Use parseActionNode for proper IF/foreach parsing with all child actions
        auto specialAction = parseActionNode(element);
        if (specialAction) {
            actions.push_back(specialAction);
        }
    } else {
        // Other special elements treated as script actions
        auto specialAction = std::make_shared<SCE::ScriptAction>("", localName);
        actions.push_back(specialAction);
    }
}

std::vector<std::shared_ptr<SCE::IActionNode>>
SCE::ActionParser::parseActionsInElement(const std::shared_ptr<IXMLElement> &parentElement) {
    std::vector<std::shared_ptr<SCE::IActionNode>> actions;

    if (!parentElement) {
        SCE_LOG_WARN("Null parent element");
        return actions;
    }

    SCE_LOG_DEBUG("ActionParser: Parsing actions in element: {}", parentElement->getName());

    // Get direct children in document order
    auto children = parentElement->getChildren();
    SCE_LOG_DEBUG("ActionParser: Found {} child elements in {}", children.size(), parentElement->getName());

    for (const auto &element : children) {
        SCE_LOG_DEBUG("ActionParser: Processing child element: '{}'", element->getName());

        // Check action node
        if (isActionNode(element)) {
            SCE_LOG_DEBUG("ActionParser: '{}' is recognized as action node", element->getName());
            auto action = parseActionNode(element);
            if (action) {
                SCE_LOG_DEBUG("ActionParser: Successfully parsed action node: '{}'", element->getName());
                actions.push_back(action);
            } else {
                SCE_LOG_ERROR("ActionParser: Failed to parse action node: '{}'", element->getName());
            }
        } else {
            SCE_LOG_DEBUG("ActionParser: '{}' is NOT recognized as action node", element->getName());
        }
        // Check external executable action node
        if (isExternalActionNode(element)) {
            auto action = parseExternalActionNode(element);
            if (action) {
                actions.push_back(action);
            }
        }
        // SCXML elements requiring special processing (if/elseif/else, foreach, etc.)
        else if (isSpecialExecutableContent(element)) {
            // Process special elements - recursively parse child elements
            parseSpecialExecutableContent(element, actions);
        }
    }

    SCE_LOG_DEBUG("ActionParser: Parsed {} actions", actions.size());
    return actions;
}

std::shared_ptr<SCE::IActionNode>
SCE::ActionParser::parseExternalActionNode(const std::shared_ptr<IXMLElement> &externalActionNode) {
    if (!externalActionNode) {
        SCE_LOG_WARN("Null external action node");
        return nullptr;
    }

    // Get action ID (name)
    std::string id;
    if (externalActionNode->hasAttribute("name")) {
        id = externalActionNode->getAttribute("name");
    } else if (externalActionNode->hasAttribute("id")) {
        id = externalActionNode->getAttribute("id");
    }

    if (id.empty()) {
        SCE_LOG_WARN("External action node missing required name attribute");
        return nullptr;
    }

    SCE_LOG_DEBUG("Parsing external action: {}", id);

    // External actions are handled as ScriptAction (extend when external action support is added)
    auto action = std::make_shared<SCE::ScriptAction>("", id);

    // Ignore delay time (not supported in current implementation)
    if (externalActionNode->hasAttribute("delay")) {
        SCE_LOG_DEBUG("ActionParser: Delay attribute value: {}", externalActionNode->getAttribute("delay"));
    }

    // Parse external implementation element
    auto implNode = ParsingCommon::findFirstChildElement(externalActionNode, "code:external-implementation");
    if (!implNode) {
        // Try without namespace
        implNode = ParsingCommon::findFirstChildElement(externalActionNode, "external-implementation");
    }

    if (implNode) {
        parseExternalImplementation(implNode, action);
    }

    SCE_LOG_DEBUG("External action parsed successfully");
    return action;
}

std::shared_ptr<SCE::IActionNode>
SCE::ActionParser::parseActionNode(const std::shared_ptr<IXMLElement> &actionElement) {
    if (!actionElement) {
        SCE_LOG_WARN("Null action element");
        return nullptr;
    }

    // Determine action type from element name
    std::string elementName = actionElement->getName();
    size_t colonPos = elementName.find(':');
    if (colonPos != std::string::npos && colonPos + 1 < elementName.length()) {
        elementName = elementName.substr(colonPos + 1);
    }

    // Extract ID
    std::string id;
    if (actionElement->hasAttribute("name")) {
        id = actionElement->getAttribute("name");
    } else if (actionElement->hasAttribute("id")) {
        id = actionElement->getAttribute("id");
    } else {
        id = elementName;
    }

    SCE_LOG_DEBUG("ActionParser: Processing action with id: {}", id);

    // Create specific action objects by action type
    if (elementName == "script") {
        // §scxml-5.8: Check for external script source
        std::string content;

        // §scxml-5.8.1: 'src' gives the URI the script is downloaded from, and may not
        // occur when the element has children.
        // §scxml-5.8.2: a conformant document supplies either 'src' or child content,
        // never both; a src that cannot be downloaded makes the document non-conformant.
        if (actionElement->hasAttribute("src")) {
            // External script specified via 'src' attribute
            std::string srcPath = actionElement->getAttribute("src");
            std::string errorMsg;

            // ARCHITECTURE.md Zero Duplication: Use FileLoadingHelper
            // §scxml-5.8: Load external script with security validation
            if (!FileLoadingHelper::loadExternalScript(srcPath, scxmlBasePath_, content, errorMsg)) {
                SCE_LOG_ERROR("ActionParser: {}", errorMsg);
                return nullptr;
            }

            // Script loaded successfully via FileLoadingHelper
        } else {
            // Inline script: read text content
            content = actionElement->getTextContent();
        }

        // Return ScriptAction with loaded content (external or inline)
        return std::make_shared<SCE::ScriptAction>(content, id);

    } else if (elementName == "assign") {
        // §scxml-5.4.2: the child content of <assign> is an in-line
        // specification of the value, mutually exclusive with 'expr' — so the
        // text content is read only when 'expr' is absent.
        std::string location = actionElement->hasAttribute("location") ? actionElement->getAttribute("location") : "";
        std::string expr;

        if (actionElement->hasAttribute("expr")) {
            expr = actionElement->getAttribute("expr");
        } else {
            // W3C SCXML test 530: Use text content as expression
            // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
            std::string xmlContent = XmlSerializationHelper::serializeContent(actionElement);
            if (!xmlContent.empty()) {
                expr = XmlSerializationHelper::escapeForJavaScript(xmlContent);
            }
        }

        return std::make_shared<SCE::AssignAction>(location, expr, id);

    } else if (elementName == "log") {
        // §scxml-4.7.2: both <log> attributes are optional — 'expr' is the
        // value to log, 'label' a string identifying the message. This build
        // logs one string, preferring 'expr' and falling back to 'label',
        // which §scxml-4.7.3 permits by leaving the manner of logging
        // platform-dependent.
        std::string message;
        if (actionElement->hasAttribute("expr")) {
            message = actionElement->getAttribute("expr");
        } else if (actionElement->hasAttribute("label")) {
            message = actionElement->getAttribute("label");
        }
        return std::make_shared<SCE::LogAction>(message, id);

    } else if (elementName == "raise") {
        // §scxml-4.2.1: 'event' is <raise>'s only attribute and is required.
        std::string event = actionElement->hasAttribute("event") ? actionElement->getAttribute("event") : "";
        return std::make_shared<SCE::RaiseAction>(event, id);

    } else if (elementName == "if") {
        // §scxml-4.3.1: 'cond' is required on <if>; §scxml-4.4.1 gives <elseif>
        // its own required 'cond' and §scxml-4.5.1 makes <else> an unguarded
        // partition, which is why the branch walk below reads a condition for
        // the first two and none for the third.
        std::string condition = actionElement->hasAttribute("cond") ? actionElement->getAttribute("cond") : "";
        auto ifAction = std::make_shared<SCE::IfAction>(condition, id);

        // Get DIRECT children only (not recursive descendants)
        // Get direct children in document order
        auto children = actionElement->getChildren();

        SCE::IfAction::ConditionalBranch *currentBranch = nullptr;

        SCE_LOG_DEBUG("IF action: found {} children, condition='{}'", children.size(), condition);

        // Process children sequentially to maintain branch context
        int childIndex = 0;
        for (const auto &element : children) {
            std::string childName = element->getName();
            // Remove namespace prefix if present
            size_t colonPos = childName.find(':');
            if (colonPos != std::string::npos && colonPos + 1 < childName.length()) {
                childName = childName.substr(colonPos + 1);
            }

            SCE_LOG_DEBUG("  Child {}: name='{}', currentBranch={}", childIndex, childName,
                          currentBranch ? "else/elseif" : "if");

            if (childName == "elseif") {
                std::string elseifCondition = element->hasAttribute("cond") ? element->getAttribute("cond") : "";
                currentBranch = &ifAction->addElseIfBranch(elseifCondition);
                SCE_LOG_DEBUG("    Added elseif branch with condition='{}'", elseifCondition);
            } else if (childName == "else") {
                currentBranch = &ifAction->addElseBranch();
                SCE_LOG_DEBUG("    Added else branch");
            } else if (isActionNode(element)) {
                auto childAction = parseActionNode(element);
                if (childAction) {
                    if (currentBranch) {
                        // Add to current elseif/else branch
                        currentBranch->actions.push_back(childAction);
                        SCE_LOG_DEBUG("    Added {} action to current branch (size now: {})", childName,
                                      currentBranch->actions.size());
                    } else {
                        // Add to main if branch (before any elseif/else)
                        ifAction->addIfAction(childAction);
                        SCE_LOG_DEBUG("    Added {} action to main if branch", childName);
                    }
                } else {
                    SCE_LOG_WARN("    parseActionNode returned nullptr for '{}'", childName);
                }
            } else {
                SCE_LOG_DEBUG("    Skipping non-action element '{}'", childName);
            }
            childIndex++;
        }

        SCE_LOG_DEBUG("IF action complete: {} branches", ifAction->getBranchCount());
        return ifAction;

    } else if (elementName == "send") {
        std::string event = actionElement->hasAttribute("event") ? actionElement->getAttribute("event") : "";

        auto sendAction = std::make_shared<SCE::SendAction>(event, id);

        // §scxml-6.2.2: every <send> attribute is optional and each of event /
        // target / type / delay / id has an "expr" twin the table declares
        // mutually exclusive with it. Both members of each pair are stored
        // rather than collapsed here, so SendAction::validateSpecific can
        // report a document that specifies both instead of the parser
        // silently picking one.
        // Parse idlocation attribute for W3C compliance
        if (actionElement->hasAttribute("idlocation")) {
            sendAction->setIdLocation(actionElement->getAttribute("idlocation"));
        }

        // Parse other send attributes
        if (actionElement->hasAttribute("target")) {
            sendAction->setTarget(actionElement->getAttribute("target"));
        }

        if (actionElement->hasAttribute("targetexpr")) {
            sendAction->setTargetExpr(actionElement->getAttribute("targetexpr"));
        }

        if (actionElement->hasAttribute("eventexpr")) {
            sendAction->setEventExpr(actionElement->getAttribute("eventexpr"));
        }

        if (actionElement->hasAttribute("delay")) {
            sendAction->setDelay(actionElement->getAttribute("delay"));
        }

        if (actionElement->hasAttribute("delayexpr")) {
            sendAction->setDelayExpr(actionElement->getAttribute("delayexpr"));
        }

        if (actionElement->hasAttribute("type")) {
            sendAction->setType(actionElement->getAttribute("type"));
        }

        // §scxml-6.2: Parse typeexpr attribute for dynamic type evaluation (Test 174)
        if (actionElement->hasAttribute("typeexpr")) {
            sendAction->setTypeExpr(actionElement->getAttribute("typeexpr"));
        }

        // §scxml-C-1: Parse namelist attribute for event data
        if (actionElement->hasAttribute("namelist")) {
            sendAction->setNamelist(actionElement->getAttribute("namelist"));
        }

        // W3C SCXML: send element uses 'id' attribute for sendid (for cancellation reference)
        if (actionElement->hasAttribute("id")) {
            sendAction->setSendId(actionElement->getAttribute("id"));
        }

        // §scxml-5.10 & C.2: Parse content child element for event data
        auto contentElements = ParsingCommon::findChildElements(actionElement, "content");
        if (!contentElements.empty()) {
            auto contentElement = contentElements[0];

            // §scxml-5.10: Check for expr attribute (dynamic content evaluation)
            if (contentElement->hasAttribute("expr")) {
                // Use expr attribute for dynamic content
                std::string contentExpr = contentElement->getAttribute("expr");
                sendAction->setContentExpr(contentExpr);
                SCE_LOG_DEBUG("ActionParser: Parsed send content expr: '{}'", contentExpr);
            } else {
                // §scxml-5.10: Use child content as literal
                // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
                std::string contentText = XmlSerializationHelper::serializeContent(contentElement);

                if (!contentText.empty()) {
                    sendAction->setContent(contentText);
                    SCE_LOG_DEBUG("ActionParser: Parsed send content literal: '{}'", contentText);
                }
            }
        }

        // §scxml-5.7.1: <param> requires 'name' and carries either 'expr' or
        // 'location'; both forms are accepted here so a location-valued param
        // reaches the event the same way it does on the AOT path, which stores
        // it too (`sce-build/src/parser.rs` Param::location). §scxml-5.6.1
        // gives <content> the same expr-or-child-content split, handled above.
        auto paramElements = ParsingCommon::findChildElements(actionElement, "param");
        for (const auto &paramElement : paramElements) {
            if (!paramElement->hasAttribute("name")) {
                SCE_LOG_WARN("ActionParser: send param element missing required name attribute");
                continue;
            }

            std::string paramName = paramElement->getAttribute("name");
            if (paramElement->hasAttribute("expr")) {
                std::string paramExpr = paramElement->getAttribute("expr");
                sendAction->addParamWithExpr(paramName, paramExpr);
                SCE_LOG_DEBUG("ActionParser: Added send param '{}' with expr '{}'", paramName, paramExpr);
            } else if (paramElement->hasAttribute("location")) {
                std::string paramLocation = paramElement->getAttribute("location");
                sendAction->addParamWithLocation(paramName, paramLocation);
                SCE_LOG_DEBUG("ActionParser: Added send param '{}' with location '{}'", paramName, paramLocation);
            } else {
                SCE_LOG_WARN("ActionParser: send param '{}' has neither expr nor location", paramName);
            }
        }

        return sendAction;

    } else if (elementName == "cancel") {
        // §scxml-6.3.1: 'sendid' and 'sendidexpr' are both optional and
        // mutually exclusive; each is stored as written so the evaluation step
        // resolves the expression form at execution time.
        std::string sendid = actionElement->hasAttribute("sendid") ? actionElement->getAttribute("sendid") : "";
        auto cancelAction = std::make_shared<SCE::CancelAction>(sendid, id);

        // W3C SCXML 1.0: Handle sendidexpr attribute for dynamic send ID evaluation
        if (actionElement->hasAttribute("sendidexpr")) {
            cancelAction->setSendIdExpr(actionElement->getAttribute("sendidexpr"));
        }

        return cancelAction;

    } else if (elementName == "foreach") {
        // §scxml-4.6.2: 'array' and 'item' are required, 'index' optional —
        // 'item' and 'index' name locations the loop declares, so they are
        // carried as written rather than evaluated here.
        std::string array = actionElement->hasAttribute("array") ? actionElement->getAttribute("array") : "";
        std::string item = actionElement->hasAttribute("item") ? actionElement->getAttribute("item") : "";
        std::string index = actionElement->hasAttribute("index") ? actionElement->getAttribute("index") : "";

        SCE_LOG_DEBUG("Parsing foreach: array='{}', item='{}', index='{}'", array, item, index);

        auto foreachAction = std::make_shared<SCE::ForeachAction>(array, item, index, id);

        // Parse nested actions (executable content inside foreach)
        auto childActions = parseActionsInElement(actionElement);
        for (const auto &childAction : childActions) {
            if (childAction) {
                foreachAction->addIterationAction(childAction);
            }
        }

        SCE_LOG_DEBUG("Foreach action created with {} child actions", childActions.size());

        return foreachAction;

    } else {
        SCE_LOG_WARN("Unknown action type: {}, creating ScriptAction", elementName);
        return std::make_shared<SCE::ScriptAction>("", id);
    }
}
