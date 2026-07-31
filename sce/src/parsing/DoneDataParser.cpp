// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/DoneDataParser.h"
#include "core/LogMacros.h"
#include "parsing/ParsingCommon.h"

SCE::DoneDataParser::DoneDataParser(std::shared_ptr<NodeFactory> factory) : factory_(factory) {
    SCE_LOG_DEBUG("Creating DoneData parser");
}

bool SCE::DoneDataParser::parseDoneData(const std::shared_ptr<IXMLElement> &doneDataElement, IStateNode *stateNode) {
    if (!doneDataElement || !stateNode) {
        SCE_LOG_ERROR("Null doneData element or state node");
        return false;
    }

    SCE_LOG_DEBUG("Parsing <donedata> for state {}", stateNode->getId());

    bool hasContent = false;
    bool hasParam = false;

    // Parse <content> element
    auto contentElement = ParsingCommon::findFirstChildElement(doneDataElement, "content");
    if (contentElement) {
        hasContent = parseContent(contentElement, stateNode);
        SCE_LOG_DEBUG("Found <content> element: {}", (hasContent ? "valid" : "invalid"));
    }

    // Parse <param> elements
    auto paramElements = ParsingCommon::findChildElements(doneDataElement, "param");
    for (const auto &paramElement : paramElements) {
        if (parseParam(paramElement, stateNode)) {
            hasParam = true;
        }
    }

    SCE_LOG_DEBUG("Found {} <param> elements: {}", paramElements.size(), (hasParam ? "valid" : "invalid"));

    // §scxml-5.5.2: a conformant document names either a single <content> child or
    // one or more <param> children under <donedata>, never both.
    if (hasContent && hasParam) {
        SCE_LOG_ERROR("<content> and <param> cannot be used together in <donedata>");

        // Clear conflict to satisfy XOR condition
        if (hasContent) {
            // Keep content, remove param
            stateNode->clearDoneDataParams();
            hasParam = false;
        } else {
            // Keep param, remove content
            stateNode->setDoneDataContentLiteral("");
            hasContent = false;
        }

        return false;
    }

    return hasContent || hasParam;
}

bool SCE::DoneDataParser::parseContent(const std::shared_ptr<IXMLElement> &contentElement, IStateNode *stateNode) {
    if (!contentElement || !stateNode) {
        SCE_LOG_ERROR("Null content element or state node");
        return false;
    }

    // Check expr attribute
    std::string exprValue;
    if (contentElement->hasAttribute("expr")) {
        exprValue = contentElement->getAttribute("expr");
        SCE_LOG_DEBUG("Found 'expr' attribute: {}", exprValue);
    }

    // Check content
    std::string textContent = contentElement->getTextContent();
    textContent = ParsingCommon::trimString(textContent);
    if (!textContent.empty()) {
        SCE_LOG_DEBUG("Found text content: {}",
                      (textContent.length() > 30 ? textContent.substr(0, 27) + "..." : textContent));
    }

    // §scxml-5.6.2: a conformant document does not give <content> both an 'expr'
    // attribute and child content.
    if (!exprValue.empty() && !textContent.empty()) {
        SCE_LOG_ERROR("<content> cannot have both 'expr' attribute and child content");
        return false;
    }

    // §scxml-B-2-6: under the ECMAScript data model a <content> child of <donedata> is
    // interpreted as §scxml-B-2-8-1 defines _event.data, which is why the text body
    // becomes an expression here rather than a verbatim literal.
    // §scxml-5.5 + 5.6 + Appendix B.2.2:
    //   - `expr` attribute => Expression (evaluated against the datamodel).
    //   - Text body with ECMAScript datamodel => Expression (Appendix B.2.2:
    //     inline text is parsed as JSON; SCE delegates that parsing to the
    //     script engine so `<content>21</content>` yields number 21 for
    //     `_event.data == 21` style guards).
    //   - Text body with the "null" datamodel => Literal (children used
    //     **as the content value** verbatim — no script engine needed).
    if (!exprValue.empty()) {
        stateNode->setDoneDataContentExpression(exprValue);
        return true;
    } else if (!textContent.empty()) {
        if (datamodelType_ == "null") {
            stateNode->setDoneDataContentLiteral(textContent);
        } else {
            stateNode->setDoneDataContentExpression(textContent);
        }
        return true;
    }

    // Empty <content/> — treat as literal empty value.
    stateNode->setDoneDataContentLiteral("");
    return true;
}

bool SCE::DoneDataParser::parseParam(const std::shared_ptr<IXMLElement> &paramElement, IStateNode *stateNode) {
    if (!paramElement || !stateNode) {
        SCE_LOG_ERROR("Null param element or state node");
        return false;
    }

    // name attribute (required)
    if (!paramElement->hasAttribute("name")) {
        SCE_LOG_ERROR("<param> element must have 'name' attribute");
        return false;
    }

    std::string nameValue = paramElement->getAttribute("name");

    // Check expr and location attributes (only one can be used)
    bool hasExpr = paramElement->hasAttribute("expr");
    bool hasLocation = paramElement->hasAttribute("location");

    if (hasExpr && hasLocation) {
        SCE_LOG_ERROR("<param> cannot have both 'expr' and 'location' attributes");
        return false;
    }

    // Process location attribute
    if (hasLocation) {
        std::string locationValue = paramElement->getAttribute("location");
        stateNode->addDoneDataParam(nameValue, locationValue);
        SCE_LOG_DEBUG("Added param: {} with location: {}", nameValue, locationValue);
        return true;
    }

    // Process expr attribute
    if (hasExpr) {
        std::string exprValue = paramElement->getAttribute("expr");
        stateNode->addDoneDataParam(nameValue, exprValue);
        SCE_LOG_DEBUG("Added param: {} with expr: {}", nameValue, exprValue);
        return true;
    }

    SCE_LOG_ERROR("<param> must have either 'expr' or 'location' attribute");
    return false;
}
