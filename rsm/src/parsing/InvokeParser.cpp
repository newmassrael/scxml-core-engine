// InvokeParser.cpp
#include "InvokeParser.h"
#include "common/Logger.h"
#include "common/XmlSerializationHelper.h"
#include "parsing/ParsingCommon.h"

#ifndef __EMSCRIPTEN__
#include <libxml++/nodes/textnode.h>
#include <libxml/tree.h>
#endif

RSM::InvokeParser::InvokeParser(std::shared_ptr<RSM::NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    LOG_DEBUG("Creating invoke parser");
}

RSM::InvokeParser::~InvokeParser() {
    LOG_DEBUG("Destroying invoke parser");
}

std::shared_ptr<RSM::IInvokeNode>
RSM::InvokeParser::parseInvokeNode(const std::shared_ptr<IXMLElement> &invokeElement) {
    if (!invokeElement) {
        LOG_WARN("Null invoke element");
        return nullptr;
    }

    // W3C SCXML 6.4: Parse id attribute if present, otherwise leave empty for runtime generation
    std::string id;
    if (invokeElement->hasAttribute("id")) {
        id = invokeElement->getAttribute("id");
    }

    // Create InvokeNode
    auto invokeNode = nodeFactory_->createInvokeNode(id);

    // Process type attribute
    if (invokeElement->hasAttribute("type")) {
        invokeNode->setType(invokeElement->getAttribute("type"));
    } else if (invokeElement->hasAttribute("typeexpr")) {
        invokeNode->setTypeExpr(invokeElement->getAttribute("typeexpr"));
    }

    // Process src attribute
    if (invokeElement->hasAttribute("src")) {
        invokeNode->setSrc(invokeElement->getAttribute("src"));
    } else if (invokeElement->hasAttribute("srcexpr")) {
        std::string srcExpr = invokeElement->getAttribute("srcexpr");
        invokeNode->setSrcExpr(srcExpr);
        LOG_DEBUG("srcexpr attribute set: {}", srcExpr);
    }

    // Process idlocation attribute
    if (invokeElement->hasAttribute("idlocation")) {
        invokeNode->setIdLocation(invokeElement->getAttribute("idlocation"));
    }

    // Process namelist attribute
    if (invokeElement->hasAttribute("namelist")) {
        invokeNode->setNamelist(invokeElement->getAttribute("namelist"));
    }

    // Process autoforward attribute
    if (invokeElement->hasAttribute("autoforward") && invokeElement->getAttribute("autoforward") == "true") {
        invokeNode->setAutoForward(true);
    }

    // Parse param elements
    parseParamElements(invokeElement, invokeNode);

    // Parse content element
    parseContentElement(invokeElement, invokeNode);

    // Parse finalize element
    auto finalizeElement = RSM::ParsingCommon::findFirstChildElement(invokeElement, "finalize");
    if (finalizeElement) {
        parseFinalizeElement(finalizeElement, invokeNode);
    }

    LOG_DEBUG("Invoke node parsed successfully: {}", id);
    return invokeNode;
}

std::vector<std::shared_ptr<RSM::IInvokeNode>>
RSM::InvokeParser::parseInvokesInState(const std::shared_ptr<IXMLElement> &stateElement) {
    std::vector<std::shared_ptr<IInvokeNode>> invokeNodes;

    if (!stateElement) {
        LOG_WARN("Null state element");
        return invokeNodes;
    }

    auto invokeElements = RSM::ParsingCommon::findChildElements(stateElement, "invoke");
    LOG_DEBUG("Found {} invoke elements", invokeElements.size());

    for (const auto &invokeElement : invokeElements) {
        auto invokeNode = parseInvokeNode(invokeElement);
        if (invokeNode) {
            invokeNodes.push_back(invokeNode);
        }
    }

    return invokeNodes;
}

void RSM::InvokeParser::parseFinalizeElement(const std::shared_ptr<IXMLElement> &finalizeElement,
                                             std::shared_ptr<IInvokeNode> invokeNode) {
    if (!finalizeElement || !invokeNode) {
        return;
    }

    // W3C SCXML 6.4: Finalize can contain executable content
    // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
    std::string finalizeContent = XmlSerializationHelper::serializeContent(finalizeElement);
    invokeNode->setFinalize(finalizeContent);

    LOG_DEBUG("Finalize element parsed for invoke: {}, content: '{}'", invokeNode->getId(), finalizeContent);
}

void RSM::InvokeParser::parseParamElements(const std::shared_ptr<IXMLElement> &invokeElement,
                                           std::shared_ptr<IInvokeNode> invokeNode) {
    if (!invokeElement || !invokeNode) {
        return;
    }

    auto paramElements = RSM::ParsingCommon::findChildElements(invokeElement, "param");
    for (const auto &paramElement : paramElements) {
        std::string name, expr, location;

        if (paramElement->hasAttribute("name")) {
            name = paramElement->getAttribute("name");
        }

        if (paramElement->hasAttribute("expr")) {
            expr = paramElement->getAttribute("expr");
        }

        if (paramElement->hasAttribute("location")) {
            location = paramElement->getAttribute("location");
        }

        invokeNode->addParam(name, expr, location);

        LOG_DEBUG("Param parsed: name={}", name);
    }
}

std::vector<std::shared_ptr<RSM::IDataModelItem>>
RSM::InvokeParser::parseParamElementsAndCreateDataItems(const std::shared_ptr<IXMLElement> &invokeElement,
                                                        std::shared_ptr<IInvokeNode> invokeNode) {
    std::vector<std::shared_ptr<IDataModelItem>> dataItems;

    if (!invokeElement || !invokeNode) {
        return dataItems;
    }

    auto paramElements = RSM::ParsingCommon::findChildElements(invokeElement, "param");
    for (const auto &paramElement : paramElements) {
        std::string name, expr, location;

        if (paramElement->hasAttribute("name")) {
            name = paramElement->getAttribute("name");
        }

        if (paramElement->hasAttribute("expr")) {
            expr = paramElement->getAttribute("expr");
        }

        if (paramElement->hasAttribute("location")) {
            location = paramElement->getAttribute("location");
        }

        // Create data model item
        if (!name.empty() && (!expr.empty() || !location.empty())) {
            auto dataItem = nodeFactory_->createDataModelItem(name, expr.empty() ? location : expr);
            if (dataItem) {
                dataItems.push_back(dataItem);
            }
        }

        LOG_DEBUG("Data item created for param: name={}", name);
    }

    return dataItems;
}

void RSM::InvokeParser::parseContentElement(const std::shared_ptr<IXMLElement> &invokeElement,
                                            std::shared_ptr<IInvokeNode> invokeNode) {
    if (!invokeElement || !invokeNode) {
        return;
    }

    auto contentElement = RSM::ParsingCommon::findFirstChildElement(invokeElement, "content");
    if (contentElement) {
        if (contentElement->hasAttribute("expr")) {
            // W3C SCXML test 530: Store expr for dynamic evaluation during invoke execution
            std::string contentExpr = contentElement->getAttribute("expr");
            invokeNode->setContentExpr(contentExpr);
            LOG_DEBUG("Content element has expr attribute: '{}'", contentExpr);
            return;
        }

        // Serialize internal XML elements
        // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
        std::string content = XmlSerializationHelper::serializeContent(contentElement);
        invokeNode->setContent(content);
        LOG_DEBUG("Content element parsed with serialized XML");
    }
}
