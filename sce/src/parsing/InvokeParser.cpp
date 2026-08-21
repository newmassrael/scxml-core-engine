// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// InvokeParser.cpp
#include "InvokeParser.h"
#include "core/LogMacros.h"
#include "parsing/ParsingCommon.h"
#include "parsing/XmlSerializationHelper.h"

#ifndef __EMSCRIPTEN__
#endif

#include <sstream>

namespace {

/// §scxml-6.5.2: the executable content an EMPTY `<finalize>` stands for.
///
/// The clause spells the behaviour as content rather than as a rule — "update
/// the corresponding location as if by `<assign>` with any return value that
/// has a name that matches" — so writing that content is what implements it,
/// and this engine then runs it through the same action executor an authored
/// body would.
///
/// A `<param>` contributes only when it carries `location`, which is the
/// clause's own wording ("`<param>` children containing 'location'
/// attributes"), and the name it matches on is the param's `name` — which may
/// differ from the location it writes.
///
/// Returns an empty string when there is nothing to update, leaving an empty
/// `<finalize>` on a plain `<invoke>` exactly as inert as the clause does.
std::string synthesizeAutomaticFinalize(const std::shared_ptr<SCE::IInvokeNode> &invokeNode) {
    std::ostringstream out;

    auto emit = [&out](const std::string &name, const std::string &location) {
        // `&amp;&amp;` because this text is re-parsed as XML before it is
        // executed: a bare `&&` inside `cond` would end the document.
        out << "<if cond=\"_event.data &amp;&amp; _event.data." << name << " !== undefined\">"
            << "<assign location=\"" << location << "\" expr=\"_event.data." << name << "\"/>"
            << "</if>";
    };

    std::istringstream names(invokeNode->getNamelist());
    std::string item;
    while (names >> item) {
        emit(item, item);
    }

    for (const auto &param : invokeNode->getParams()) {
        const std::string &name = std::get<0>(param);
        const std::string &location = std::get<2>(param);
        if (name.empty() || location.empty()) {
            continue;
        }
        emit(name, location);
    }

    return out.str();
}

}  // namespace

SCE::InvokeParser::InvokeParser(std::shared_ptr<SCE::NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    SCE_LOG_DEBUG("Creating invoke parser");
}

SCE::InvokeParser::~InvokeParser() {
    SCE_LOG_DEBUG("Destroying invoke parser");
}

std::shared_ptr<SCE::IInvokeNode>
SCE::InvokeParser::parseInvokeNode(const std::shared_ptr<IXMLElement> &invokeElement) {
    if (!invokeElement) {
        SCE_LOG_WARN("Null invoke element");
        return nullptr;
    }

    // §scxml-6.4: Parse id attribute if present, otherwise leave empty for runtime generation
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
        SCE_LOG_DEBUG("srcexpr attribute set: {}", srcExpr);
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
    auto finalizeElement = SCE::ParsingCommon::findFirstChildElement(invokeElement, "finalize");
    if (finalizeElement) {
        parseFinalizeElement(finalizeElement, invokeNode);
    }

    SCE_LOG_DEBUG("Invoke node parsed successfully: {}", id);
    return invokeNode;
}

std::vector<std::shared_ptr<SCE::IInvokeNode>>
SCE::InvokeParser::parseInvokesInState(const std::shared_ptr<IXMLElement> &stateElement) {
    std::vector<std::shared_ptr<IInvokeNode>> invokeNodes;

    if (!stateElement) {
        SCE_LOG_WARN("Null state element");
        return invokeNodes;
    }

    auto invokeElements = SCE::ParsingCommon::findChildElements(stateElement, "invoke");
    SCE_LOG_DEBUG("Found {} invoke elements", invokeElements.size());

    for (const auto &invokeElement : invokeElements) {
        auto invokeNode = parseInvokeNode(invokeElement);
        if (invokeNode) {
            invokeNodes.push_back(invokeNode);
        }
    }

    return invokeNodes;
}

void SCE::InvokeParser::parseFinalizeElement(const std::shared_ptr<IXMLElement> &finalizeElement,
                                             std::shared_ptr<IInvokeNode> invokeNode) {
    if (!finalizeElement || !invokeNode) {
        return;
    }

    // §scxml-6.5.2: Finalize can contain executable content
    // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
    std::string finalizeContent = XmlSerializationHelper::serializeContent(finalizeElement);

    // §scxml-6.5.2: an EMPTY `<finalize>` is not an inert one. With no
    // executable content the clause requires the automatic update instead —
    // "for each item in the 'namelist' attribute and each such <param>
    // element, the Processor MUST update the corresponding location as if by
    // <assign> with any return value that has a name that matches" — and it
    // draws the line explicitly: "the automatic update does not take place if
    // the <finalize> element is absent as opposed to empty".
    //
    // The clause names the executable content the empty element stands for, so
    // synthesising that content is what makes it work here: this engine
    // re-parses the finalize body as SCXML executable content
    // (`StateMachine.cpp`), so an `<if>`/`<assign>` pair runs through the same
    // action executor an authored body would. The AOT side reaches the same
    // answer in `sce-build`'s parser, in JavaScript rather than XML, because
    // that is the form its `finalize_content` carries.
    //
    // "With ANY return value that has a name that matches" is a condition: an
    // event carrying no such name must leave the location alone, which is why
    // each assignment is guarded rather than unconditional.
    if (finalizeContent.find_first_not_of(" \t\r\n") == std::string::npos) {
        finalizeContent = synthesizeAutomaticFinalize(invokeNode);
    }

    invokeNode->setFinalize(finalizeContent);

    SCE_LOG_DEBUG("Finalize element parsed for invoke: {}, content: '{}'", invokeNode->getId(), finalizeContent);
}

void SCE::InvokeParser::parseParamElements(const std::shared_ptr<IXMLElement> &invokeElement,
                                           std::shared_ptr<IInvokeNode> invokeNode) {
    if (!invokeElement || !invokeNode) {
        return;
    }

    auto paramElements = SCE::ParsingCommon::findChildElements(invokeElement, "param");
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

        SCE_LOG_DEBUG("Param parsed: name={}", name);
    }
}

std::vector<std::shared_ptr<SCE::IDataModelItem>>
SCE::InvokeParser::parseParamElementsAndCreateDataItems(const std::shared_ptr<IXMLElement> &invokeElement,
                                                        std::shared_ptr<IInvokeNode> invokeNode) {
    std::vector<std::shared_ptr<IDataModelItem>> dataItems;

    if (!invokeElement || !invokeNode) {
        return dataItems;
    }

    auto paramElements = SCE::ParsingCommon::findChildElements(invokeElement, "param");
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

        SCE_LOG_DEBUG("Data item created for param: name={}", name);
    }

    return dataItems;
}

void SCE::InvokeParser::parseContentElement(const std::shared_ptr<IXMLElement> &invokeElement,
                                            std::shared_ptr<IInvokeNode> invokeNode) {
    if (!invokeElement || !invokeNode) {
        return;
    }

    auto contentElement = SCE::ParsingCommon::findFirstChildElement(invokeElement, "content");
    if (contentElement) {
        if (contentElement->hasAttribute("expr")) {
            // W3C SCXML test 530: Store expr for dynamic evaluation during invoke execution
            std::string contentExpr = contentElement->getAttribute("expr");
            invokeNode->setContentExpr(contentExpr);
            SCE_LOG_DEBUG("Content element has expr attribute: '{}'", contentExpr);
            return;
        }

        // Serialize internal XML elements
        // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
        std::string content = XmlSerializationHelper::serializeContent(contentElement);
        invokeNode->setContent(content);
        SCE_LOG_DEBUG("Content element parsed with serialized XML");
    }
}
